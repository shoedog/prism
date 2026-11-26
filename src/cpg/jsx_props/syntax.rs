use super::*;
use props_binding::{first_expression, PropsValue, PropsWriter};

pub(super) fn exact_node(parsed: &ParsedFile, span: Range<usize>) -> Option<Node<'_>> {
    parsed
        .tree
        .root_node()
        .descendant_for_byte_range(span.start, span.end)
        .filter(|node| node.byte_range() == span)
}

pub(super) fn opening_at(parsed: &ParsedFile, span: Range<usize>) -> Option<Node<'_>> {
    exact_node(parsed, span).filter(|node| {
        matches!(
            node.kind(),
            "jsx_opening_element" | "jsx_self_closing_element"
        )
    })
}

// Both grammars mark the braced spread as an attribute field.
pub(super) fn attributes(opening: Node<'_>) -> Vec<Node<'_>> {
    opening
        .children_by_field_name("attribute", &mut opening.walk())
        .collect()
}

fn expression(container: Node<'_>) -> Option<Node<'_>> {
    container
        .named_children(&mut container.walk())
        .find(|child| !child.is_extra())
}

pub(super) fn entries(caller: &ParsedFile, span: Range<usize>) -> Option<Vec<PropsEntry>> {
    let opening = opening_at(caller, span)?;
    if opening.has_error() {
        return None;
    }
    let mut writes = Vec::new();
    for attribute in attributes(opening) {
        if attribute.kind() == "jsx_expression" {
            if expression(attribute).is_some_and(|node| node.kind() == "spread_element") {
                writes.push(PropsEntry::Spread);
                continue;
            }
            return None;
        }
        if attribute.kind() != "jsx_attribute" {
            return None;
        }
        let mut cursor = attribute.walk();
        let mut parts = attribute
            .named_children(&mut cursor)
            .filter(|node| !node.is_extra());
        let name = parts.next()?;
        let key = caller.node_text(&name);
        if matches!(key, "key" | "ref") {
            continue;
        }
        if !key.is_ascii() || key.contains('\\') {
            writes.push(PropsEntry::Wildcard);
            continue;
        }
        let value = if name.kind() == "jsx_namespace_name" || key == "__proto__" {
            PropsValue::Opaque
        } else if matches!(name.kind(), "property_identifier" | "identifier") {
            object_literal_args::parse_value(
                caller,
                parts
                    .next()
                    .filter(|part| part.kind() == "jsx_expression")
                    .and_then(expression),
                true,
            )?
        } else {
            writes.push(PropsEntry::Wildcard);
            continue;
        };
        writes.push(PropsEntry::Named(PropsWriter {
            key: key.into(),
            legacy: false,
            value,
        }));
    }
    if opening.kind() == "jsx_opening_element"
        && opening.parent().is_some_and(|element| {
            element
                .named_children(&mut element.walk())
                .any(|child| match child.kind() {
                    "jsx_element"
                    | "jsx_self_closing_element"
                    | "jsx_fragment"
                    | "html_character_reference" => true,
                    "jsx_expression" => expression(child).is_some(),
                    "jsx_text" => {
                        let text = caller.node_text(&child);
                        !text.trim().is_empty() || !text.contains(['\n', '\r'])
                    }
                    _ => false,
                })
        })
    {
        writes.push(PropsEntry::Named(PropsWriter {
            key: "children".into(),
            legacy: false,
            value: PropsValue::Opaque,
        }));
    }
    Some(writes)
}

pub(super) fn operand_root(mut node: Node<'_>) -> Option<Node<'_>> {
    loop {
        node = crate::ast::js_ts::props_keys::strip_transparent_wrappers(node)?;
        match node.kind() {
            "identifier" | "shorthand_property_identifier" | "this" | "super" => return Some(node),
            "member_expression" | "subscript_expression" => {
                node = node.child_by_field_name("object")?;
            }
            _ => return None,
        }
    }
}

pub(super) fn receiver_chain<'tree>(
    parsed: &ParsedFile,
    root: Node<'tree>,
) -> (Node<'tree>, Vec<String>, bool) {
    let mut current = root;
    let mut fields = Vec::new();
    let mut bracket = false;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "member_expression" if parent.child_by_field_name("object") == Some(current) => {
                let Some(property) = parent.child_by_field_name("property") else {
                    break;
                };
                fields.push(parsed.node_text(&property).to_owned());
            }
            "subscript_expression" if parent.child_by_field_name("object") == Some(current) => {
                bracket = true;
            }
            kind if object_literal_args::is_transparent_wrapper(kind)
                && first_expression(parent) == Some(current) => {}
            _ => break,
        }
        current = parent;
    }
    (current, fields, bracket)
}

pub(super) fn execution_owner<'tree>(
    parsed: &ParsedFile,
    node: Node<'tree>,
) -> Option<Node<'tree>> {
    let mut current = node;
    let boundaries = parsed.language.callable_boundary_node_types();
    while let Some(parent) = current.parent() {
        if callable_region(&boundaries, parent, current) {
            return Some(parent);
        }
        current = parent;
    }
    None
}

fn callable_region(boundaries: &[&str], parent: Node<'_>, child: Node<'_>) -> bool {
    boundaries.contains(&parent.kind())
        && ["body", "parameters", "parameter"]
            .into_iter()
            .any(|field| parent.child_by_field_name(field) == Some(child))
}

pub(super) fn is_class_receiver_region(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "field_definition" | "public_field_definition" | "class_static_block"
    )
}

/// Arrows inherit the receiver. Computed class names and heritage are evaluated
/// outside the receiver regions introduced by field values and static blocks.
pub(super) fn receiver_owner<'tree>(parsed: &ParsedFile, node: Node<'tree>) -> Option<Node<'tree>> {
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut current = node;
    while let Some(parent) = current.parent() {
        if (is_class_receiver_region(parent)
            && (parent.kind() == "class_static_block"
                || parent.child_by_field_name("value") == Some(current)))
            || (parent.kind() != "arrow_function" && callable_region(&boundaries, parent, current))
        {
            return Some(parent);
        }
        current = parent;
    }
    None
}

pub(super) fn is_value_read(parsed: &ParsedFile, node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "identifier" | "shorthand_property_identifier" | "this" | "super"
    ) && !parsed.js_ts_identifier_is_not_a_read(node)
        && !node.parent().is_some_and(|parent| {
            matches!(
                parent.kind(),
                "jsx_attribute"
                    | "jsx_opening_element"
                    | "jsx_self_closing_element"
                    | "jsx_closing_element"
            ) || (parent.kind() == "member_expression"
                && parent.parent().is_some_and(|tag| {
                    tag.child_by_field_name("name") == Some(parent)
                        && matches!(
                            tag.kind(),
                            "jsx_opening_element"
                                | "jsx_self_closing_element"
                                | "jsx_closing_element"
                        )
                }))
        })
}

pub(super) fn walk<'tree>(
    mut pending: Vec<Node<'tree>>,
    mut visit: impl FnMut(Node<'tree>) -> bool,
) {
    while let Some(node) = pending.pop() {
        if !node.is_extra() && visit(node) {
            pending.extend(node.named_children(&mut node.walk()));
        }
    }
}
