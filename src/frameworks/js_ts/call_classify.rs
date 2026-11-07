//! Framework-aware JS/TS call classification (express/fastify/koa receivers,
//! route-call shapes, `require` modules, assignment-like nodes).
//!
//! Moved verbatim out of `algorithms/taint.rs` (#702) so core modules such as
//! `framework_entries` do not import the review layer.

use crate::ast::tree_helpers::{call_arg_node, unwrap_parenthesized};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// `pub(crate)` (P9): reused as-is by `framework_entries::express_route_candidates`
/// to compute the express receiver set for registration-first extraction —
/// no logic change, just visibility.
pub(crate) fn js_ts_framework_receiver_names(
    parsed: &ParsedFile,
    framework: &str,
) -> BTreeSet<String> {
    let imports = parsed.extract_imports();
    let mut receivers = BTreeSet::new();
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    for assignment in assignments {
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if lhs.kind() != "identifier" {
            continue;
        }
        if js_ts_expr_constructs_framework_receiver(parsed, rhs, framework, &imports) {
            receivers.insert(parsed.node_text(&lhs).to_string());
        }
    }
    receivers
}

/// `pub(crate)` (P9): reused by `framework_entries::express_route_candidates`
/// to collect every call node in a file before filtering to route
/// registrations.
pub(crate) fn collect_js_ts_call_nodes<'a>(
    node: Node<'a>,
    parsed: &ParsedFile,
    out: &mut Vec<Node<'a>>,
) {
    if parsed.language.is_call_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_call_nodes(child, parsed, out);
    }
}

/// `pub(crate)` (P9): reused by `framework_entries` to test a call's method
/// name against the express route-method set.
pub(crate) fn js_ts_framework_route_method(framework: &str, method: &str) -> bool {
    match framework {
        "express" => matches!(
            method,
            "get" | "post" | "put" | "patch" | "delete" | "all" | "use"
        ),
        "fastify" => matches!(
            method,
            "get" | "post" | "put" | "patch" | "delete" | "options" | "head" | "all" | "route"
        ),
        "koa" => matches!(
            method,
            "use" | "get" | "post" | "put" | "patch" | "delete" | "all"
        ),
        _ => false,
    }
}

/// `pub(crate)` (P9): reused by `framework_entries` to test whether a route
/// call's receiver is a direct express instance (vs. a `.route()` builder).
pub(crate) fn js_ts_receiver_expr_is_framework_instance(
    parsed: &ParsedFile,
    receiver: Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
) -> bool {
    let receiver = unwrap_parenthesized(receiver);
    if receiver.kind() == "identifier" {
        return framework_receivers.contains(parsed.node_text(&receiver));
    }
    js_ts_expr_constructs_framework_receiver(parsed, receiver, framework, imports)
}

/// `pub(crate)` (P9): reused by `framework_entries` to detect the
/// `.route(path).get(handler)` builder shape (arg positioning differs from
/// the direct-instance shape — see `framework_entries::express_route_candidates`).
pub(crate) fn js_ts_receiver_expr_is_route_builder(
    parsed: &ParsedFile,
    receiver: Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
) -> bool {
    let receiver = unwrap_parenthesized(receiver);
    if !parsed.language.is_call_node(receiver.kind()) {
        return false;
    }
    let Some(function) = receiver.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    if parsed.node_text(&property) != "route" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    js_ts_receiver_expr_is_framework_instance(
        parsed,
        object,
        framework,
        framework_receivers,
        imports,
    )
}

fn js_ts_expr_constructs_framework_receiver(
    parsed: &ParsedFile,
    expr: Node<'_>,
    framework: &str,
    imports: &BTreeMap<String, String>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if !matches!(expr.kind(), "call_expression" | "new_expression") {
        return false;
    }
    let Some(callee) = js_ts_call_or_new_callee(expr) else {
        return false;
    };
    js_ts_callee_constructs_framework_receiver(parsed, callee, framework, imports)
}

pub(crate) fn js_ts_call_or_new_callee(node: Node<'_>) -> Option<Node<'_>> {
    node.child_by_field_name("function")
        .or_else(|| node.child_by_field_name("constructor"))
        .or_else(|| node.child_by_field_name("name"))
        .or_else(|| node.named_child(0))
}

fn js_ts_callee_constructs_framework_receiver(
    parsed: &ParsedFile,
    callee: Node<'_>,
    framework: &str,
    imports: &BTreeMap<String, String>,
) -> bool {
    let callee = unwrap_parenthesized(callee);
    if callee.kind() == "identifier" {
        let local = parsed.node_text(&callee);
        return imports
            .get(local)
            .is_some_and(|module| js_ts_module_matches_framework(module, framework));
    }

    if parsed.language.is_call_node(callee.kind()) {
        return js_ts_require_call_module(parsed, &callee)
            .is_some_and(|module| js_ts_module_matches_framework(&module, framework));
    }

    if callee.kind() == "member_expression" {
        let Some(property) = callee.child_by_field_name("property") else {
            return false;
        };
        let Some(object) = callee.child_by_field_name("object") else {
            return false;
        };
        let property_text = parsed.node_text(&property);
        return (framework == "express" && property_text == "Router")
            && js_ts_expr_resolves_to_framework_module(parsed, object, framework, imports);
    }

    false
}

fn js_ts_expr_resolves_to_framework_module(
    parsed: &ParsedFile,
    expr: Node<'_>,
    framework: &str,
    imports: &BTreeMap<String, String>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if expr.kind() == "identifier" {
        let local = parsed.node_text(&expr);
        return imports
            .get(local)
            .is_some_and(|module| js_ts_module_matches_framework(module, framework));
    }
    if parsed.language.is_call_node(expr.kind()) {
        return js_ts_require_call_module(parsed, &expr)
            .is_some_and(|module| js_ts_module_matches_framework(&module, framework));
    }
    false
}

fn js_ts_module_matches_framework(module: &str, framework: &str) -> bool {
    match framework {
        "express" => module == "express",
        "fastify" => module == "fastify",
        "koa" => matches!(module, "koa" | "@koa/router" | "koa-router"),
        _ => false,
    }
}

pub(crate) fn js_ts_require_call_module(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let node = unwrap_parenthesized(*node);
    if !parsed.language.is_call_node(node.kind()) {
        return None;
    }
    let function = parsed.call_function_name(&node)?;
    if parsed.node_text(&function) != "require" {
        return None;
    }
    let arg = call_arg_node(&node, 0)?;
    js_ts_literal_string_value(parsed, &arg)
}

pub(crate) fn js_ts_literal_string_value(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let node = unwrap_parenthesized(*node);
    let text = parsed.node_text(&node).trim();
    if !(matches!(node.kind(), "string" | "template_string")
        || text.starts_with('"')
        || text.starts_with('\'')
        || text.starts_with('`'))
    {
        return None;
    }
    if text.starts_with('`') && text.contains("${") {
        return None;
    }
    text.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
        .or_else(|| text.strip_prefix('`').and_then(|s| s.strip_suffix('`')))
        .map(ToString::to_string)
}

pub(crate) fn collect_js_ts_assignment_like_nodes<'a>(
    node: Node<'a>,
    parsed: &ParsedFile,
    out: &mut Vec<Node<'a>>,
) {
    if node.kind() == "variable_declarator" || parsed.language.is_assignment_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_assignment_like_nodes(child, parsed, out);
    }
}

pub(crate) fn js_ts_assignment_target_and_value<'a>(
    parsed: &ParsedFile,
    node: &Node<'a>,
) -> Option<(Node<'a>, Node<'a>)> {
    Some((
        js_ts_assignment_target(parsed, node)?,
        js_ts_assignment_value(parsed, node)?,
    ))
}

pub(crate) fn js_ts_assignment_target<'a>(
    parsed: &ParsedFile,
    node: &Node<'a>,
) -> Option<Node<'a>> {
    if node.kind() == "variable_declarator" {
        return node.child_by_field_name("name");
    }
    parsed.language.assignment_target(node)
}

pub(crate) fn js_ts_assignment_value<'a>(parsed: &ParsedFile, node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() == "variable_declarator" {
        return node.child_by_field_name("value");
    }
    parsed.language.assignment_value(node)
}
