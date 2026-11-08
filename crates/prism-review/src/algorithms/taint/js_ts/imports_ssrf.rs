//! JS/TS import/require bindings and SSRF client matching.

use crate::algorithms::taint::js_ts::bindings::{
    js_ts_assignment_visible_before_context, js_ts_identifier_binds_imported_member_at_call,
    js_ts_identifier_binds_module_at_call,
};
use crate::algorithms::taint::js_ts::sinks::collect_nodes_of_kind;
use prism::ast::tree_helpers::{call_path_matches, unwrap_parenthesized};
use prism::ast::ParsedFile;
use prism::frameworks::js_ts::call_classify::{
    collect_js_ts_assignment_like_nodes, js_ts_require_call_module,
};
use tree_sitter::Node;

pub(super) fn js_ts_import_statement_binds_member(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    let Some(source) = node.child_by_field_name("source") else {
        return false;
    };
    let source_text = parsed
        .node_text(&source)
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string();
    if source_text != module_name {
        return false;
    }
    let mut specs = Vec::new();
    collect_nodes_of_kind(*node, "import_specifier", &mut specs);
    for spec in specs {
        let Some(name) = spec.child_by_field_name("name") else {
            continue;
        };
        if parsed.node_text(&name) != imported_member {
            continue;
        }
        let local = spec
            .child_by_field_name("alias")
            .map(|alias| parsed.node_text(&alias))
            .unwrap_or_else(|| parsed.node_text(&name));
        if local == local_name {
            return true;
        }
    }
    false
}

pub(super) fn js_ts_import_statement_binds_module(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    let Some(source) = node.child_by_field_name("source") else {
        return false;
    };
    let source_text = parsed
        .node_text(&source)
        .trim_matches(|c| c == '\'' || c == '"')
        .to_string();
    if source_text != module_name {
        return false;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "import_clause" => {
                if js_ts_import_clause_binds_module(parsed, &child, local_name) {
                    return true;
                }
            }
            "identifier" => {
                if parsed.node_text(&child) == local_name {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn js_ts_import_clause_binds_module(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "identifier" => {
                if parsed.node_text(&child) == local_name {
                    return true;
                }
            }
            "namespace_import" => {
                if child
                    .child_by_field_name("name")
                    .is_some_and(|name| parsed.node_text(&name) == local_name)
                {
                    return true;
                }
                let mut inner = child.walk();
                if child
                    .children(&mut inner)
                    .any(|n| n.kind() == "identifier" && parsed.node_text(&n) == local_name)
                {
                    return true;
                }
            }
            // Named imports bind members, not the module namespace object.
            "named_imports" => {}
            _ => {}
        }
    }
    false
}

pub(super) fn js_ts_require_declarator_binds_member(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
    imported_member: &str,
) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let Some(value) = node.child_by_field_name("value") else {
        return false;
    };
    if js_ts_require_call_module(parsed, &value).is_some_and(|module| module == module_name) {
        return js_ts_pattern_binds_member(parsed, &name, imported_member, local_name);
    }
    if name.kind() == "identifier" && parsed.node_text(&name) == local_name {
        return js_ts_require_member_expression(parsed, &value)
            .is_some_and(|(module, member)| module == module_name && member == imported_member);
    }
    false
}

pub(super) fn js_ts_require_declarator_binds_module(
    parsed: &ParsedFile,
    node: &Node<'_>,
    local_name: &str,
    module_name: &str,
) -> bool {
    let Some(name) = node.child_by_field_name("name") else {
        return false;
    };
    let Some(value) = node.child_by_field_name("value") else {
        return false;
    };
    name.kind() == "identifier"
        && parsed.node_text(&name) == local_name
        && js_ts_require_call_module(parsed, &value).is_some_and(|module| module == module_name)
}

pub(super) fn js_ts_pattern_binds_member(
    parsed: &ParsedFile,
    pattern: &Node<'_>,
    imported_member: &str,
    local_name: &str,
) -> bool {
    let mut cursor = pattern.walk();
    for child in pattern.children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                if imported_member == local_name && parsed.node_text(&child) == imported_member {
                    return true;
                }
            }
            "pair_pattern" => {
                let key_matches = child
                    .child_by_field_name("key")
                    .is_some_and(|key| parsed.node_text(&key) == imported_member);
                let value_matches = child.child_by_field_name("value").is_some_and(|value| {
                    if value.kind() == "identifier" {
                        parsed.node_text(&value) == local_name
                    } else if value.kind() == "object_pattern" || value.kind() == "array_pattern" {
                        js_ts_pattern_binds_member(parsed, &value, imported_member, local_name)
                    } else {
                        false
                    }
                });
                if key_matches && value_matches {
                    return true;
                }
            }
            "object_pattern" | "array_pattern" => {
                if js_ts_pattern_binds_member(parsed, &child, imported_member, local_name) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

pub(super) fn js_ts_require_member_expression(
    parsed: &ParsedFile,
    node: &Node<'_>,
) -> Option<(String, String)> {
    let node = unwrap_parenthesized(*node);
    if node.kind() != "member_expression" {
        return None;
    }
    let object = node.child_by_field_name("object")?;
    let property = node.child_by_field_name("property")?;
    let module = js_ts_require_call_module(parsed, &object)?;
    let member = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
        .to_string();
    Some((module, member))
}

pub(in crate::algorithms::taint) fn js_ts_ssrf_sink_call_path_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
    expected: &str,
) -> bool {
    match expected {
        "fetch" => js_ts_ssrf_fetch_call_matches(parsed, call, actual),
        "axios" => js_ts_ssrf_direct_module_call_matches(parsed, call, &["axios"]),
        "got" => js_ts_ssrf_direct_module_call_matches(parsed, call, &["got"]),
        "get" | "post" | "request" => js_ts_ssrf_method_call_matches(parsed, call, expected),
        _ => call_path_matches(parsed, actual, expected),
    }
}

fn js_ts_ssrf_fetch_call_matches(parsed: &ParsedFile, call: &Node<'_>, actual: &str) -> bool {
    if matches!(actual, "fetch" | "globalThis.fetch" | "window.fetch") {
        return true;
    }
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() == "identifier" {
        let local = parsed.node_text(&function);
        return js_ts_identifier_binds_any_module_at_call(
            parsed,
            call,
            local,
            &["node-fetch", "cross-fetch", "isomorphic-fetch"],
        ) || js_ts_identifier_binds_any_imported_member_at_call(
            parsed,
            call,
            local,
            &["undici"],
            "fetch",
        );
    }
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    if parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`')
        != "fetch"
    {
        return false;
    }
    function
        .child_by_field_name("object")
        .is_some_and(|object| {
            js_ts_expr_binds_any_module_at_call(parsed, call, object, &["undici"])
        })
}

fn js_ts_ssrf_direct_module_call_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    modules: &[&str],
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    js_ts_expr_binds_any_module_at_call(parsed, call, function, modules)
}

fn js_ts_ssrf_method_call_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expected_method: &str,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() == "identifier" {
        let local = parsed.node_text(&function);
        return js_ts_identifier_binds_any_imported_member_at_call(
            parsed,
            call,
            local,
            js_ts_ssrf_method_modules(expected_method),
            expected_method,
        );
    }
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    let method = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    if method != expected_method {
        return false;
    }
    let Some(receiver) = function.child_by_field_name("object") else {
        return false;
    };
    js_ts_expr_binds_any_module_at_call(
        parsed,
        call,
        receiver,
        js_ts_ssrf_method_modules(expected_method),
    ) || js_ts_expr_is_ssrf_client_factory_call(parsed, call, receiver)
        || js_ts_expr_is_ssrf_factory_client_binding(parsed, call, receiver)
}

fn js_ts_ssrf_method_modules(method: &str) -> &'static [&'static str] {
    match method {
        "get" | "post" | "request" => &[
            "axios",
            "got",
            "superagent",
            "http",
            "node:http",
            "https",
            "node:https",
            "undici",
        ],
        _ => &[],
    }
}

fn js_ts_identifier_binds_any_module_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    modules: &[&str],
) -> bool {
    modules
        .iter()
        .any(|module| js_ts_identifier_binds_module_at_call(parsed, call, local_name, module))
}

fn js_ts_identifier_binds_any_imported_member_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    local_name: &str,
    modules: &[&str],
    imported_member: &str,
) -> bool {
    modules.iter().any(|module| {
        js_ts_identifier_binds_imported_member_at_call(
            parsed,
            call,
            local_name,
            module,
            imported_member,
        )
    })
}

fn js_ts_expr_binds_any_module_at_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expr: Node<'_>,
    modules: &[&str],
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if expr.kind() == "identifier" {
        return js_ts_identifier_binds_any_module_at_call(
            parsed,
            call,
            parsed.node_text(&expr),
            modules,
        );
    }
    if let Some(module) = js_ts_require_call_module(parsed, &expr) {
        return modules.iter().any(|expected| module == *expected);
    }
    false
}

fn js_ts_expr_is_ssrf_client_factory_call(
    parsed: &ParsedFile,
    context_call: &Node<'_>,
    expr: Node<'_>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if !parsed.language.is_call_node(expr.kind()) {
        return false;
    }
    let Some(function) = expr.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    let factory = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    match factory {
        "create" => js_ts_expr_binds_any_module_at_call(parsed, context_call, object, &["axios"]),
        "extend" => js_ts_expr_binds_any_module_at_call(parsed, context_call, object, &["got"]),
        "agent" => {
            js_ts_expr_binds_any_module_at_call(parsed, context_call, object, &["superagent"])
        }
        _ => false,
    }
}

fn js_ts_expr_is_ssrf_factory_client_binding(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expr: Node<'_>,
) -> bool {
    let expr = unwrap_parenthesized(expr);
    if expr.kind() != "identifier" {
        return false;
    }
    let local_name = parsed.node_text(&expr);
    let mut declarations = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut declarations);
    declarations.iter().any(|decl| {
        decl.kind() == "variable_declarator"
            && decl.child_by_field_name("name").is_some_and(|name| {
                name.kind() == "identifier" && parsed.node_text(&name) == local_name
            })
            && js_ts_assignment_visible_before_context(parsed, call, decl)
            && decl
                .child_by_field_name("value")
                .is_some_and(|value| js_ts_expr_is_ssrf_client_factory_call(parsed, call, value))
    })
}
