//! Exact positional parameter-slot extraction.
//!
//! Binding occurrence extraction deliberately has a different contract: it may
//! expose names which cannot be mapped to an argument position. This module is
//! for consumers that need positional identity and therefore stops before an
//! unsafe boundary instead of compressing it away.

use crate::ast::{ParameterOccurrence, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(crate) fn slots(parsed: &ParsedFile, function: &Node<'_>) -> Option<Vec<ParameterOccurrence>> {
    let params = parsed
        .find_parameters_node(function)
        .or_else(|| function.child_by_field_name("parameter"))?;
    if contains_recovery(params) || parsed.language == Language::Java {
        return None;
    }
    if has_duplicate_js_ts_bindings(parsed, params) {
        return None;
    }
    if params.kind() == "identifier" {
        return unique(parsed, vec![params]);
    }

    let nodes = match parsed.language {
        Language::JavaScript => javascript_slots(parsed, params),
        Language::TypeScript | Language::Tsx => typescript_slots(parsed, params),
        Language::Go => go_slots(parsed, params),
        Language::Rust => rust_slots(parsed, params),
        Language::Python => python_slots(parsed, params),
        _ => fallback_slots(parsed, params),
    }?;
    unique(parsed, nodes)
}

fn unique(parsed: &ParsedFile, nodes: Vec<Node<'_>>) -> Option<Vec<ParameterOccurrence>> {
    let mut names = BTreeSet::new();
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        let name = parsed.node_text(&node).to_string();
        if !names.insert(name.clone()) {
            return None;
        }
        out.push((name, node.start_byte(), node.end_byte()));
    }
    Some(out)
}

fn contains_recovery(node: Node<'_>) -> bool {
    if node.is_error() || node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    let has_recovery = node.children(&mut cursor).any(contains_recovery);
    has_recovery
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn identifier(node: Node<'_>) -> Option<Node<'_>> {
    (node.kind() == "identifier").then_some(node)
}

/// Duplicates invalidate a JS/TS parameter list even if an earlier pattern
/// already truncates the deterministic positional prefix. This must inspect
/// binding patterns (not all identifiers) so `a = initializer` does not treat
/// an initializer reference as a second parameter binding.
fn has_duplicate_js_ts_bindings(parsed: &ParsedFile, params: Node<'_>) -> bool {
    if !matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return false;
    }

    let mut names = BTreeSet::new();
    let mut duplicate = false;
    let parameter_nodes = if params.kind() == "identifier" {
        vec![params]
    } else {
        named_children(params)
    };
    for parameter in parameter_nodes {
        collect_js_ts_parameter_bindings(parsed, parameter, &mut names, &mut duplicate);
    }
    duplicate
}

fn collect_js_ts_parameter_bindings(
    parsed: &ParsedFile,
    node: Node<'_>,
    names: &mut BTreeSet<String>,
    duplicate: &mut bool,
) {
    if let Some(pattern) = node.child_by_field_name("pattern") {
        collect_js_ts_binding_pattern_names(parsed, pattern, names, duplicate);
        return;
    }
    if let Some(name) = node.child_by_field_name("name") {
        collect_js_ts_binding_pattern_names(parsed, name, names, duplicate);
        return;
    }
    if let Some(left) = node.child_by_field_name("left") {
        collect_js_ts_binding_pattern_names(parsed, left, names, duplicate);
        return;
    }

    match node.kind() {
        "object_pattern" | "array_pattern" | "identifier" | "rest_pattern" => {
            collect_js_ts_binding_pattern_names(parsed, node, names, duplicate);
        }
        kind if kind.contains("parameter") => {
            for child in named_children(node) {
                if !matches!(child.kind(), "type_annotation" | "return_type") {
                    collect_js_ts_binding_pattern_names(parsed, child, names, duplicate);
                }
            }
        }
        _ => {}
    }
}

fn collect_js_ts_binding_pattern_names(
    parsed: &ParsedFile,
    node: Node<'_>,
    names: &mut BTreeSet<String>,
    duplicate: &mut bool,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node).to_string();
            *duplicate |= !names.insert(name);
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_js_ts_binding_pattern_names(parsed, value, names, duplicate);
            }
        }
        "rest_pattern" => {
            for child in named_children(node) {
                collect_js_ts_binding_pattern_names(parsed, child, names, duplicate);
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node.child_by_field_name("left") {
                collect_js_ts_binding_pattern_names(parsed, left, names, duplicate);
            }
        }
        "object_pattern"
        | "array_pattern"
        | "parenthesized_expression"
        | "parenthesized_pattern" => {
            for child in named_children(node) {
                collect_js_ts_binding_pattern_names(parsed, child, names, duplicate);
            }
        }
        _ => {}
    }
}

fn field_or_first_named<'a>(node: Node<'a>, fields: &[&str]) -> Option<Node<'a>> {
    fields
        .iter()
        .find_map(|field| node.child_by_field_name(field))
        .or_else(|| named_children(node).into_iter().next())
}

fn simple_pattern(node: Node<'_>) -> Option<Node<'_>> {
    match node.kind() {
        "identifier" => Some(node),
        "assignment_pattern" => node.child_by_field_name("left").and_then(identifier),
        "mutable_pattern" => named_children(node).into_iter().find_map(identifier),
        _ => None,
    }
}

fn javascript_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "rest_pattern" | "object_pattern" | "array_pattern" => break,
            _ => match simple_pattern(parameter) {
                Some(name) => out.push(name),
                None => break,
            },
        }
    }
    Some(out)
}

fn typescript_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        if matches!(
            parameter.kind(),
            "rest_pattern" | "object_pattern" | "array_pattern"
        ) {
            break;
        }
        let pattern = match parameter.kind() {
            "required_parameter" | "optional_parameter" => {
                field_or_first_named(parameter, &["pattern", "name"])
            }
            _ => Some(parameter),
        };
        let Some(pattern) = pattern else { break };
        if pattern.kind() == "this" {
            continue;
        }
        let Some(name) = simple_pattern(pattern) else {
            break;
        };
        out.push(name);
    }
    Some(out)
}

fn go_slots<'a>(parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for declaration in named_children(params) {
        if declaration.kind() == "variadic_parameter_declaration" {
            break;
        }
        if declaration.kind() != "parameter_declaration" {
            break;
        }
        let Some(ty) = declaration.child_by_field_name("type") else {
            break;
        };
        let names: Vec<_> = named_children(declaration)
            .into_iter()
            .filter(|node| node.kind() == "identifier" && node.end_byte() <= ty.start_byte())
            .collect();
        if names.is_empty() {
            break;
        }
        let mut hit_blank = false;
        for name in names {
            if parsed.node_text(&name) == "_" {
                hit_blank = true;
                break;
            }
            out.push(name);
        }
        if hit_blank {
            break;
        }
    }
    Some(out)
}

fn rust_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "self_parameter" => continue,
            "parameter" => {
                let Some(pattern) = parameter.child_by_field_name("pattern") else {
                    break;
                };
                let Some(name) = simple_pattern(pattern) else {
                    break;
                };
                out.push(name);
            }
            "identifier" => {
                if _parsed.node_text(&parameter) == "_" {
                    break;
                }
                out.push(parameter);
            }
            _ => break,
        }
    }
    Some(out)
}

fn python_slots<'a>(_parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        match parameter.kind() {
            "positional_separator" => continue,
            "keyword_separator" | "list_splat_pattern" | "dictionary_splat_pattern" => break,
            "identifier" => out.push(parameter),
            "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                let Some(pattern) = field_or_first_named(parameter, &["name", "pattern"]) else {
                    break;
                };
                let Some(name) = simple_pattern(pattern) else {
                    break;
                };
                out.push(name);
            }
            _ => break,
        }
    }
    Some(out)
}

fn fallback_slots<'a>(parsed: &ParsedFile, params: Node<'a>) -> Option<Vec<Node<'a>>> {
    let mut out = Vec::new();
    for parameter in named_children(params) {
        let Some(name) = parsed.extract_param_name_node(&parameter) else {
            break;
        };
        out.push(name);
    }
    Some(out)
}
