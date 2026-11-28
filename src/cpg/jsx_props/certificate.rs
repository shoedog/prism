//! Rule R, the positive certificate P2 (DESIGN 5.3.4): may a representative read
//! stand for the operand? Judged by spelling over the whole execution owner.
use super::syntax;
use crate::ast::ParsedFile;
use std::borrow::Cow;
use tree_sitter::Node;

/// The clauses a root failed, in trace order (`c`, `a2`, `b2`), with the first offender.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Certificate {
    pub(super) failed: Vec<&'static str>,
    pub(super) a2_form: Option<&'static str>,
    pub(super) b2_position: Option<String>,
}

impl Certificate {
    pub(super) fn keeps(&self) -> bool {
        self.failed.is_empty()
    }
}

/// The grammars break lines on `\n` only, while the language also breaks on a lone CR, U+2028 and
/// U+2029 and reads `<!--` and `-->` as comments: a statement can hide or appear across them.
fn line_terminator_divergence(text: &str) -> bool {
    text.contains(['\u{2028}', '\u{2029}'])
        || text.contains("<!--")
        || text.contains("-->")
        || text
            .match_indices('\r')
            .any(|(at, _)| text.as_bytes().get(at + 1) != Some(&b'\n'))
}

/// Clause c: no parse error, no `with`, no name spelled `eval` or `arguments`.
fn clause_c(parsed: &ParsedFile, owner: Node<'_>) -> bool {
    let mut clean = !owner.has_error() && !line_terminator_divergence(parsed.node_text(&owner));
    syntax::walk(vec![owner], |node| {
        clean &= node.kind() != "with_statement";
        if name_token(node) {
            let text = parsed.node_text(&node);
            // A token spelled with an escape the decoder cannot read (`\u{00000078}`) may spell
            // the root, `eval` or `arguments`: fail closed.
            clean &=
                !(text.contains('\\') && ParsedFile::js_ts_decode_identifier_text(text).is_none());
            clean &= !matches!(spelling(text).as_ref(), "arguments" | "eval");
        }
        true
    });
    clean
}

pub(super) fn certify(parsed: &ParsedFile, root: Node<'_>, owner: Node<'_>) -> Certificate {
    let failed_c = !clause_c(parsed, owner);
    let name = spelling(parsed.node_text(&root));
    let mut occurrences = Vec::new();
    syntax::walk(vec![owner], |node| {
        if matches!(
            node.kind(),
            "identifier"
                | "undefined"
                | "shorthand_property_identifier"
                | "shorthand_property_identifier_pattern"
                | "type_identifier"
        ) && spelling(parsed.node_text(&node)) == name
            && variable_token(node, owner, &name)
        {
            occurrences.push((node, declaration(node, owner)));
        }
        true
    });
    let mut declarations = occurrences
        .iter()
        .filter_map(|&(node, form)| Some((node, form?)));
    let first = declarations.next();
    let sole = first.filter(|_| declarations.next().is_none());
    let a2_form = if matches!(name.as_ref(), "this" | "super" | "arguments") {
        Some("this_super_arguments")
    } else {
        match sole {
            Some((_, Declaration::Eligible)) => None,
            Some((_, Declaration::Ineligible(form))) => Some(form),
            None if first.is_some() => Some("multiple_declarations"),
            None => Some("none_found"),
        }
    };
    let b2_position = occurrences
        .into_iter()
        .map(|(node, _)| node)
        .filter(|&node| {
            !sole.is_some_and(|(decl, _)| decl == node)
                && (in_loop_head(node, owner) || !read_position(node))
        })
        .min_by_key(|node| node.start_byte())
        .and_then(|node| {
            let parent = node.parent()?;
            Some(format!("{}/{}", parent.kind(), field(parent, node)))
        });

    Certificate::new(failed_c, a2_form, b2_position)
}

impl Certificate {
    fn new(failed_c: bool, a2_form: Option<&'static str>, b2_position: Option<String>) -> Self {
        let failed = [
            ("c", failed_c),
            ("a2", a2_form.is_some()),
            ("b2", b2_position.is_some()),
        ]
        .into_iter()
        .filter_map(|(clause, failed)| failed.then_some(clause))
        .collect();
        Self {
            failed,
            a2_form,
            b2_position,
        }
    }
}

fn variable_token(mut node: Node<'_>, owner: Node<'_>, name: &str) -> bool {
    // TS aliases runtime class binding names to the same kind as type names.
    if node.kind() == "type_identifier"
        && !node.parent().is_some_and(|parent| {
            matches!(
                parent.kind(),
                "class" | "class_declaration" | "abstract_class_declaration"
            ) && parent.child_by_field_name("name") == Some(node)
        })
    {
        return false;
    }

    // Arguments of a call inside an otherwise excluded type run once the types are erased.
    let mut in_arguments = false;
    while node != owner {
        let Some(parent) = node.parent() else {
            break;
        };
        in_arguments |= parent.kind() == "arguments";
        match parent.kind() {
            "asserts"
            | "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "type_query"
            | "type_predicate"
            | "type_alias_declaration"
            | "nested_type_identifier"
            | "generic_type"
            | "tuple_type"
            | "interface_declaration"
            | "object_type"
            | "implements_clause"
            | "method_signature"
            | "abstract_method_signature"
            | "index_signature"
            | "function_type"
            | "constructor_type"
                if in_arguments => {}
            "jsx_namespace_name"
            | "jsx_closing_element"
            | "asserts"
            | "type_annotation"
            | "type_arguments"
            | "type_parameters"
            | "type_query"
            | "type_predicate"
            | "type_alias_declaration"
            | "nested_type_identifier"
            | "generic_type"
            | "tuple_type"
            | "interface_declaration"
            | "object_type"
            | "implements_clause"
            | "method_signature"
            | "abstract_method_signature"
            | "index_signature"
            | "function_type"
            | "constructor_type"
            | "namespace_export" => return false,
            "as_expression" | "satisfies_expression"
                if !in_arguments && first_named_child(parent) != Some(node) =>
            {
                return false;
            }
            // The vendored Flow-style `declare module.exports: T` has no type wrapper.
            "ambient_declaration" if has_keyword(parent, "module") => return false,
            "function_signature" if parent.child_by_field_name("parameters") == Some(node) => {
                return false;
            }
            "jsx_opening_element" | "jsx_self_closing_element"
                if node.kind() == "identifier"
                    && parent.child_by_field_name("name") == Some(node)
                    && (name.starts_with(|c: char| c.is_ascii_lowercase())
                        || name.contains('-')) =>
            {
                return false;
            }
            "import_specifier"
                if has_keyword(parent, "type")
                    || (parent.child_by_field_name("name") == Some(node)
                        && parent.child_by_field_name("alias").is_some()) =>
            {
                return false;
            }
            "import_statement" if has_keyword(parent, "type") || has_keyword(parent, "typeof") => {
                return false;
            }
            "export_specifier" if parent.child_by_field_name("alias") == Some(node) => {
                return false
            }
            "export_statement"
                if parent.child_by_field_name("source").is_some()
                    || has_keyword(parent, "type")
                    || has_keyword(parent, "namespace") =>
            {
                return false;
            }
            _ => {}
        }
        node = parent;
    }
    true
}

#[derive(Clone, Copy)]
enum Declaration {
    Eligible,
    Ineligible(&'static str),
}

fn declaration(mut node: Node<'_>, owner: Node<'_>) -> Option<Declaration> {
    use Declaration::{Eligible, Ineligible};

    let mut destructured = false;
    let mut rest = false;
    while node != owner {
        let parent = node.parent()?;
        let form = match (parent.kind(), field(parent, node)) {
            ("array_pattern" | "object_pattern", "<none>") => {
                destructured = true;
                node = parent;
                continue;
            }
            ("rest_pattern", "<none>") => {
                rest = true;
                node = parent;
                continue;
            }
            ("pair_pattern", "value")
            | ("assignment_pattern" | "object_assignment_pattern", "left")
            | ("required_parameter" | "optional_parameter", "pattern" | "name") => {
                node = parent;
                continue;
            }
            ("variable_declarator", "name") => const_form(parent),
            ("formal_parameters", "<none>") if parent.parent() != Some(owner) => {
                Ineligible("param_not_owner")
            }
            ("formal_parameters", "<none>") if rest && !destructured => {
                Ineligible("param_non_leaf")
            }
            ("formal_parameters", "<none>") => Eligible,
            ("arrow_function", "parameter") if parent == owner => Eligible,
            ("arrow_function", "parameter") => Ineligible("param_not_owner"),
            ("for_in_statement", "left") if parent.child_by_field_name("kind").is_some() => {
                Ineligible("loop_head")
            }
            ("assignment_expression", "left") if has_keyword(parent, "using") => {
                Ineligible("using")
            }
            ("catch_clause", "parameter") => Ineligible("catch"),
            (
                "function_declaration"
                | "function_expression"
                | "function_signature"
                | "generator_function"
                | "generator_function_declaration",
                "name",
            ) => Ineligible("function"),
            ("abstract_class_declaration" | "class" | "class_declaration", "name") => {
                Ineligible("class")
            }
            ("enum_declaration", "name") => Ineligible("enum"),
            ("nested_identifier", "object") => {
                node = parent;
                continue;
            }
            ("internal_module" | "module", "name") => Ineligible("namespace"),
            ("import_specifier", "alias" | "name")
            | ("import_clause" | "import_require_clause" | "namespace_import", "<none>") => {
                Ineligible("import")
            }
            ("import_alias", "<none>") if first_named_child(parent) == Some(node) => {
                Ineligible("import")
            }
            _ => return None,
        };
        return Some(form);
    }
    None
}

fn const_form(declarator: Node<'_>) -> Declaration {
    use Declaration::{Eligible, Ineligible};

    let Some(statement) = declarator.parent() else {
        return Ineligible("variable_declarator");
    };
    if statement.parent().is_some_and(|parent| {
        parent.kind() == "for_statement"
            && parent.child_by_field_name("initializer") == Some(statement)
    }) {
        return Ineligible("loop_head");
    }
    match statement
        .child_by_field_name("kind")
        .map(|node| node.kind())
        .unwrap_or(statement.kind())
    {
        "variable_declaration" => Ineligible("var"),
        "const" if ancestors(declarator).any(|node| is_loop(node.kind())) => {
            Ineligible("const_in_loop")
        }
        "const" if declarator.child_by_field_name("value").is_none() => {
            Ineligible("const_without_initializer")
        }
        "const" => Eligible,
        kind => Ineligible(kind),
    }
}

fn first_named_child(node: Node<'_>) -> Option<Node<'_>> {
    node.named_children(&mut node.walk())
        .find(|child| !child.is_extra())
}

fn last_named_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    let last = node
        .named_children(&mut cursor)
        .filter(|child| !child.is_extra())
        .last();
    last
}

fn has_keyword(node: Node<'_>, keyword: &str) -> bool {
    node.children(&mut node.walk())
        .any(|child| !child.is_named() && child.kind() == keyword)
}

fn spelling(text: &str) -> Cow<'_, str> {
    if text.contains('\\') {
        if let Some(name) = ParsedFile::js_ts_decode_identifier_text(text) {
            return Cow::Owned(name);
        }
    }
    Cow::Borrowed(text)
}

fn name_token(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "identifier"
            | "type_identifier"
            | "property_identifier"
            | "private_property_identifier"
            | "statement_identifier"
            | "shorthand_property_identifier"
            | "shorthand_property_identifier_pattern"
            | "undefined"
            | "this"
            | "super"
    )
}

fn field(parent: Node<'_>, child: Node<'_>) -> &'static str {
    let mut cursor = parent.walk();
    cursor
        .goto_first_child_for_byte(child.start_byte())
        .filter(|_| cursor.node() == child)
        .and_then(|_| cursor.field_name())
        .unwrap_or("<none>")
}

fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    std::iter::successors(node.parent(), Node::parent)
}

fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
    )
}

fn in_loop_head(mut node: Node<'_>, owner: Node<'_>) -> bool {
    while node != owner {
        let Some(parent) = node.parent() else {
            break;
        };
        if is_loop(parent.kind()) && parent.child_by_field_name("body") != Some(node) {
            return true;
        }
        node = parent;
    }
    false
}

// Keep this table in sync with READ-POSITIONS.md in the pricing evidence.
fn read_position(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        let position = field(parent, node);
        match (parent.kind(), position) {
            ("member_expression" | "subscript_expression", "object") => node = parent,
            ("parenthesized_expression", "<none>")
                if parent.child_by_field_name("type").is_none() =>
            {
                node = parent;
            }
            ("unary_expression", "argument") => {
                return parent
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() != "delete");
            }
            ("object", "<none>") => return node.kind() == "shorthand_property_identifier",
            // A result position forwards the value: follow it to its consumer, so a store
            // through `(0, x).p`, `(c ? x : y).p` or `(x || y).p` is not a read of `x`.
            ("binary_expression", "left" | "right")
                if parent
                    .child_by_field_name("operator")
                    .is_some_and(|op| matches!(op.kind(), "&&" | "||" | "??")) =>
            {
                node = parent;
            }
            ("ternary_expression", "consequence" | "alternative") => node = parent,
            // `await x` yields `x` itself for a non-thenable value.
            ("await_expression", "<none>") => node = parent,
            ("sequence_expression", "<none>") => {
                if last_named_child(parent) == Some(node) {
                    node = parent;
                } else {
                    return true;
                }
            }
            ("binary_expression", "left" | "right")
            | ("ternary_expression", "condition")
            | ("call_expression", "function")
            | ("new_expression", "constructor")
            | ("subscript_expression", "index")
            | ("if_statement", "condition")
            | ("switch_statement" | "switch_case", "value")
            | (
                "pair" | "variable_declarator" | "field_definition" | "public_field_definition",
                "value",
            )
            | ("arrow_function", "body")
            | (
                "arguments"
                | "array"
                | "expression_statement"
                | "jsx_expression"
                | "return_statement"
                | "spread_element"
                | "template_substitution"
                | "throw_statement"
                | "yield_expression",
                "<none>",
            ) => return true,
            _ => return false,
        }
    }
    false
}

#[cfg(test)]
#[path = "certificate_tests.rs"]
mod tests;
