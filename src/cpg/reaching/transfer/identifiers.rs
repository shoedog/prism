use super::super::binding_table::select_binding_row;
use super::super::DefSite;
use super::cpp;
use super::java;
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
        if java::collect_introduced_identifiers(parsed.language, node, identifiers) {
            return;
        }
        if cpp::collect_introduced_identifiers(parsed, node, identifiers) {
            return;
        }
        match node.kind() {
            "init_declarator"
            | "pointer_declarator"
            | "array_declarator"
            | "function_declarator"
                if super::super::binding_table::c_slice3_wave_applies(parsed) =>
            {
                if let Some(declarator) = node.child_by_field_name("declarator") {
                    collect(parsed, declarator, identifiers);
                }
            }
            "parenthesized_declarator" | "attributed_declarator"
                if super::super::binding_table::c_slice3_wave_applies(parsed) =>
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
        if parsed.language == Language::Lua && *field == "name" {
            identifiers.extend(super::lua::variable_names(parsed, node));
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
    cpp::collect_row(parsed, node, fields, &mut identifiers);
    identifiers.sort_by_key(Node::start_byte);
    identifiers.dedup_by_key(|identifier| identifier.start_byte());
    identifiers
}

fn identifier_owns_definition(
    parsed: &ParsedFile,
    node: Node<'_>,
    identifier: Node<'_>,
    byte: usize,
) -> bool {
    if identifier.start_byte() == byte {
        return true;
    }
    if !super::super::binding_table::c_slice3_wave_applies(parsed) || byte > identifier.start_byte()
    {
        return false;
    }
    std::iter::successors(identifier.parent(), |ancestor| ancestor.parent())
        .take_while(|ancestor| ancestor.id() != node.id())
        .any(|ancestor| {
            ancestor.start_byte() <= byte && identifier.end_byte() <= ancestor.end_byte()
        })
}

pub(in crate::cpg::reaching) fn row_identifier_for_def<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    fields: &[&str],
    def: &DefSite,
) -> Option<Node<'a>> {
    row_introduced_identifiers(parsed, node, fields)
        .into_iter()
        .find(|identifier| {
            (!super::super::binding_table::c_slice3_wave_applies(parsed)
                || parsed.node_text(identifier) == def.path.base)
                && identifier_owns_definition(parsed, node, *identifier, def.start_byte)
        })
}

pub(in crate::cpg::reaching) fn row_excludes_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    fields: &[&str],
    def: &DefSite,
) -> bool {
    if parsed.language == Language::Cpp {
        return cpp::row_excludes_def(parsed, node, fields, def);
    }
    !fields.is_empty() && row_identifier_for_def(parsed, node, fields, def).is_none()
}

pub(in crate::cpg::reaching) fn row_identifier_owns_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    identifier: Node<'_>,
    def: &DefSite,
) -> bool {
    if parsed.language == Language::Cpp {
        return cpp::identifier_owns_def(parsed, node, identifier, def);
    }
    parsed.node_text(&identifier) == def.path.base
        && identifier_owns_definition(parsed, node, identifier, def.start_byte)
}

pub(in crate::cpg::reaching) fn normalize_c_declaration_facts(
    parsed: &ParsedFile,
    defs: &mut [DefSite],
) {
    if !super::super::binding_table::c_slice3_wave_applies(parsed) {
        return;
    }
    for def in defs.iter_mut().filter(|def| !def.alias_derived) {
        if c_side_effect_target(parsed, def) {
            continue;
        }
        let Some(end_byte) = def.start_byte.checked_add(1) else {
            continue;
        };
        let Some(mut node) = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(def.start_byte, end_byte)
        else {
            continue;
        };
        loop {
            if let Some(row) =
                select_binding_row(parsed, node).filter(|row| row.declaration.is_some())
            {
                if row_identifier_for_def(parsed, node, row.fields, def).is_some() {
                    break;
                }
                if node.kind() == "declaration" {
                    def.alias_derived = true;
                    break;
                }
            }
            let Some(parent) = node.parent() else {
                break;
            };
            node = parent;
        }
    }
}

fn c_side_effect_target(parsed: &ParsedFile, def: &DefSite) -> bool {
    let Some(end_byte) = def.start_byte.checked_add(1) else {
        return false;
    };
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, end_byte)
    else {
        return false;
    };
    for ancestor in std::iter::successors(Some(node), |node| node.parent()) {
        match ancestor.kind() {
            "update_expression" => return true,
            "assignment_expression"
                if ancestor.child_by_field_name("left").is_some_and(|left| {
                    left.start_byte() <= def.start_byte && def.start_byte < left.end_byte()
                }) =>
            {
                return true;
            }
            "declaration" => break,
            _ => {}
        }
    }
    false
}
