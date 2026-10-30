//! Exact finite-set form of the function-body binding predicate.
//! Each helper unions precisely the leaves accepted by its boolean counterpart;
//! unsupported nodes and failed escape decoding contribute the empty set.
use super::*;
use std::collections::HashSet;

pub(super) fn collect(p: &ParsedFile, body: Node<'_>, boundaries: &[&str]) -> HashSet<String> {
    #[cfg(test)]
    binding_verdict_memo::probe::observe("function_body", body.id(), 0, "");
    let mut names = HashSet::new();
    let mut stack = vec![body];
    while let Some(node) = stack.pop() {
        if node.id() != body.id()
            && (boundaries.contains(&node.kind())
                || matches!(
                    node.kind(),
                    "class_static_block" | "class" | "class_declaration"
                ))
        {
            continue;
        }
        if node.kind() == "variable_declaration" {
            declaration(p, node, &mut names);
        }
        if node.kind() == "for_in_statement"
            && node
                .child_by_field_name("kind")
                .is_some_and(|k| p.node_text(&k) == "var")
        {
            if let Some(left) = node.child_by_field_name("left") {
                pattern(p, left, &mut names);
            }
        }
        if node.id() == body.id() && node.kind() == "statement_block" {
            let mut cursor = node.walk();
            for statement in node.named_children(&mut cursor) {
                block_statement(p, statement, &mut names);
            }
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
    names
}

pub(super) fn declaration(p: &ParsedFile, node: Node<'_>, names: &mut HashSet<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            if let Some(name) = child.child_by_field_name("name") {
                pattern(p, name, names);
            }
        }
    }
}

pub(super) fn block_statement(p: &ParsedFile, node: Node<'_>, names: &mut HashSet<String>) {
    match node.kind() {
        "labeled_statement" => {
            if let Some(body) = node.child_by_field_name("body") {
                block_statement(p, body, names);
            }
        }
        "lexical_declaration" => declaration(p, node, names),
        "function_declaration"
        | "generator_function_declaration"
        | "class_declaration"
        | "abstract_class_declaration"
        | "enum_declaration" => {
            if let Some(name) = node.child_by_field_name("name") {
                pattern(p, name, names);
            }
        }
        "switch_case" | "switch_default" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                block_statement(p, child, names);
            }
        }
        _ => {}
    }
}

pub(super) fn pattern(p: &ParsedFile, node: Node<'_>, names: &mut HashSet<String>) {
    match node.kind() {
        "identifier"
        | "undefined"
        | "type_identifier"
        | "shorthand_property_identifier_pattern" => {
            let text = p.node_text(&node);
            // Match the fence's raw fast path; the general BoundNames helper
            // always validates and therefore is not an equivalent collector.
            if !text.contains('\\') {
                names.insert(text.to_owned());
            } else if let Some(decoded) = ParsedFile::js_ts_decoded_identifier(text) {
                names.insert(decoded);
            }
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                pattern(p, value, names);
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                pattern(p, left, names);
            }
        }
        "required_parameter" | "optional_parameter" => {
            if let Some(binding) = node.child_by_field_name("pattern") {
                pattern(p, binding, names);
            }
        }
        "formal_parameters" | "object_pattern" | "array_pattern" | "rest_pattern" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                pattern(p, child, names);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
#[path = "js_scope_bindings_tests.rs"]
mod tests;
