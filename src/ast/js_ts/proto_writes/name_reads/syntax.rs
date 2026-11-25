//! Grammar positions for the value inventory and the syntax-only fallback.
use super::super::{eager_owner_inputs, has_token, parameter_decorator, path};
use super::{
    children, field, is_owner, is_scope, pattern_parts, peel, Analysis, Context, Node, ParsedFile,
    State, Version,
};

impl<'a> Analysis<'a> {
    // Preserve occurrences and lexical identity after refusing the transfer.
    // The inventory walk visits each syntax node once without copying State.
    pub(super) fn inventory(&mut self, root: Node<'a>, context: Context<'a>, state: &mut State) {
        let mut work = vec![(root, context)];
        while let Some((node, outer)) = work.pop() {
            if is_owner(node) && node != outer.owner {
                self.function(node, outer, &mut State::default(), None);
                for child in eager_owner_inputs(node) {
                    work.push((child, outer));
                }
                continue;
            }
            let mut context = outer;
            if is_scope(node) {
                context.scope = node;
            }
            let target = match node.kind() {
                "assignment_expression" | "for_in_statement" => field(node, "left"),
                "variable_declarator" => field(node, "name"),
                "catch_clause" => field(node, "parameter"),
                _ => None,
            };
            for target in target.into_iter().chain(super::super::parameters(node)) {
                work.extend(
                    pattern_inputs(target)
                        .into_iter()
                        .map(|child| (child, context)),
                );
            }
            if let Some((base, path)) = path(self.file, node) {
                self.read_path(node, path, base, context, Version::default(), true);
                work.extend(field(node, "index").map(|child| (child, context)));
                if let Some(object) = field(node, "object") {
                    let mut object = peel(object);
                    while matches!(object.kind(), "member_expression" | "subscript_expression") {
                        work.extend(field(object, "index").map(|child| (child, context)));
                        let Some(base) = field(object, "object") else {
                            break;
                        };
                        object = peel(base);
                    }
                }
                continue;
            }
            for child in value_children(self.file, node).filter(|child| {
                Some(*child) != target
                    && !parameter_decorator(*child, node)
                    && (!is_owner(node)
                        || !matches!(child.kind(), "decorator" | "computed_property_name"))
            }) {
                let child_context =
                    if node.kind() == "with_statement" && field(node, "object") == Some(child) {
                        outer
                    } else {
                        context
                    };
                work.push((child, child_context));
            }
        }
        *state = State::default();
    }
}

fn pattern_inputs(target: Node<'_>) -> Vec<Node<'_>> {
    let mut pending = vec![target];
    let mut reads = Vec::new();
    while let Some(part) = pending.pop().map(peel) {
        match part.kind() {
            "member_expression" | "subscript_expression" => {
                reads.extend(
                    [field(part, "object"), field(part, "index")]
                        .into_iter()
                        .flatten(),
                );
            }
            "assignment_pattern"
            | "object_assignment_pattern"
            | "required_parameter"
            | "optional_parameter" => {
                reads.extend(field(part, "right").or_else(|| field(part, "value")));
            }
            "pair_pattern" => reads
                .extend(field(part, "key").filter(|key| key.kind() == "computed_property_name")),
            _ => {}
        }
        pending.extend(pattern_parts(part));
    }
    reads
}

// Tree-sitter exposes declarations, type syntax and a JSX tag's two spellings as
// named children. Dispatch on the grammar position before visiting value nodes.
pub(in super::super) fn value_children<'a>(
    file: &'a ParsedFile,
    node: Node<'a>,
) -> impl Iterator<Item = Node<'a>> {
    children(node).filter(move |child| match node.kind() {
        "import_statement"
        | "type_alias_declaration"
        | "interface_declaration"
        | "type_annotation"
        | "type_arguments"
        | "type_parameters"
        | "function_signature"
        | "method_signature"
        | "abstract_method_signature"
        | "jsx_closing_element" => false,
        "export_specifier" => {
            Some(*child) == field(node, "name")
                && !has_token(node, "type")
                && node
                    .parent()
                    .and_then(|clause| clause.parent())
                    .is_none_or(|export| {
                        field(export, "source").is_none() && !has_token(export, "type")
                    })
        }
        "jsx_opening_element" | "jsx_self_closing_element"
            if Some(*child) == field(node, "name") =>
        {
            // An identifier node also represents lowercase intrinsic names. A
            // dotted tag is a value expression even when its root is lowercase.
            child.kind() == "member_expression"
                || (child.kind() == "identifier"
                    && !file
                        .node_text(child)
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_lowercase)
                    && !file.node_text(child).contains('-'))
        }
        "function_declaration"
        | "generator_function_declaration"
        | "function_expression"
        | "generator_function"
        | "arrow_function"
        | "method_definition"
        | "class"
        | "class_declaration"
        | "abstract_class_declaration" => {
            Some(*child) != field(node, "name")
                && Some(*child) != field(node, "parameters")
                && Some(*child) != field(node, "parameter")
        }
        "field_definition" | "public_field_definition" => {
            let name = field(node, "property").or_else(|| field(node, "name"));
            Some(*child) != name || child.kind() == "computed_property_name"
        }
        _ => !matches!(
            child.kind(),
            "type_annotation"
                | "type_arguments"
                | "type_parameters"
                | "type_identifier"
                | "property_identifier"
                | "private_property_identifier"
        ),
    })
}
