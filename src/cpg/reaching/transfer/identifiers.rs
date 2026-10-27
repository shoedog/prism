use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

pub(in crate::cpg::reaching) fn introduced_identifiers<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
) -> Vec<Node<'a>> {
    fn collect<'a>(parsed: &ParsedFile, node: Node<'a>, identifiers: &mut Vec<Node<'a>>) {
        if node.kind() == "shorthand_property_identifier_pattern" {
            identifiers.push(node);
            return;
        }
        if parsed.language.is_identifier_node(node.kind()) {
            identifiers.push(node);
            return;
        }
        match node.kind() {
            "init_declarator"
            | "pointer_declarator"
            | "array_declarator"
            | "function_declarator"
                if parsed.language == Language::C =>
            {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    collect(parsed, declarator, identifiers);
                }
            }
            "parenthesized_declarator" | "attributed_declarator"
                if parsed.language == Language::C =>
            {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    collect(parsed, child, identifiers);
                }
            }
            "as_pattern" | "aliased_import" if parsed.language == Language::Python => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    collect(parsed, alias, identifiers);
                }
            }
            "except_group_clause" if parsed.language == Language::Python => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "as_pattern"
                        || child
                            .prev_sibling()
                            .is_some_and(|previous| previous.kind() == "as")
                    {
                        collect(parsed, child, identifiers);
                    }
                }
            }
            "delete_statement" | "global_statement" | "nonlocal_statement"
                if parsed.language == Language::Python =>
            {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    collect(parsed, child, identifiers);
                }
            }
            "dotted_name" if parsed.language == Language::Python => {
                let mut cursor = node.walk();
                let mut children: Vec<_> = node.named_children(&mut cursor).collect();
                let selected = if node
                    .parent()
                    .is_some_and(|parent| parent.kind() == "import_statement")
                {
                    children.drain(..).next()
                } else {
                    children.pop()
                };
                if let Some(identifier) = selected {
                    collect(parsed, identifier, identifiers);
                }
            }
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    collect(parsed, value, identifiers);
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    collect(parsed, left, identifiers);
                }
            }
            "object_pattern"
            | "array_pattern"
            | "rest_pattern"
            | "parenthesized_expression"
            | "parenthesized_pattern"
            | "expression_list"
            | "as_pattern_target"
            | "pattern_list"
            | "tuple"
            | "tuple_pattern"
            | "list"
            | "list_pattern"
            | "list_splat_pattern"
            | "splat_pattern" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    collect(parsed, child, identifiers);
                }
            }
            _ => {}
        }
    }

    let mut identifiers = Vec::new();
    collect(parsed, node, &mut identifiers);
    identifiers
}

pub(in crate::cpg::reaching) fn row_introduced_identifiers<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    fields: &[&str],
) -> Vec<Node<'a>> {
    let mut identifiers = Vec::new();
    for field in fields {
        let mut cursor = node.walk();
        for target in node.children_by_field_name(field, &mut cursor) {
            if parsed.language == Language::Python
                && matches!(node.kind(), "except_clause" | "with_item")
                && *field == "value"
                && target.kind() != "as_pattern"
            {
                continue;
            }
            identifiers.extend(introduced_identifiers(parsed, target));
        }
    }
    if parsed.language == Language::Python
        && fields.is_empty()
        && matches!(
            node.kind(),
            "except_group_clause" | "delete_statement" | "global_statement" | "nonlocal_statement"
        )
    {
        identifiers.extend(introduced_identifiers(parsed, node));
    }
    identifiers.sort_by_key(Node::start_byte);
    identifiers.dedup_by_key(|identifier| identifier.start_byte());
    identifiers
}

pub(in crate::cpg::reaching) fn row_excludes_identifier(
    parsed: &ParsedFile,
    node: Node<'_>,
    fields: &[&str],
    byte: usize,
) -> bool {
    let identifiers = row_introduced_identifiers(parsed, node, fields);
    let owns_byte = identifiers.iter().any(|identifier| {
        if identifier.start_byte() == byte {
            return true;
        }
        if parsed.language != Language::C || byte > identifier.start_byte() {
            return false;
        }
        std::iter::successors(identifier.parent(), |ancestor| ancestor.parent())
            .take_while(|ancestor| ancestor.id() != node.id())
            .any(|ancestor| {
                ancestor.start_byte() <= byte && identifier.end_byte() <= ancestor.end_byte()
            })
    });
    !fields.is_empty() && !owns_byte
}
