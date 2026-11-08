//! JS/TS framework route handlers, decorators and request parameters.

use prism::access_path::AccessPath;
use crate::algorithms::taint::js_ts::request_aliases::js_ts_request_data_assignment_sources;
use crate::algorithms::taint::{JsTsParam, TaintSeed};
use prism::ast::tree_helpers::unwrap_parenthesized;
use prism::ast::ParsedFile;
use prism::frameworks::js_ts::call_classify::{
    collect_js_ts_call_nodes, js_ts_call_or_new_callee, js_ts_framework_receiver_names,
    js_ts_framework_route_method, js_ts_receiver_expr_is_framework_instance,
    js_ts_receiver_expr_is_route_builder,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn detect_js_ts_framework_sources(
    file_path: &str,
    parsed: &ParsedFile,
    sources: &mut Vec<TaintSeed>,
) {
    let framework = match parsed.framework() {
        Some(spec) => spec.name,
        None => return,
    };
    if !matches!(framework, "nestjs" | "fastify" | "express" | "koa") {
        return;
    }
    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);

    for func in parsed.all_functions() {
        let line = func.start_position().row + 1;
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }

        let source_params =
            js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
        if source_params.is_empty() {
            continue;
        }

        for param in &source_params {
            sources.push(TaintSeed::target(
                file_path.to_string(),
                line,
                AccessPath::simple(param.as_str()),
            ));
        }

        let assignment_sources =
            js_ts_request_data_assignment_sources(parsed, &func, framework, &source_params);
        for source in assignment_sources {
            sources.push(TaintSeed::target_scoped(
                file_path.to_string(),
                source.line,
                source.target,
                source.start_byte,
                source.scope,
                source.byte_scope,
            ));
        }
    }
}

pub(super) fn js_ts_function_params(parsed: &ParsedFile, func: &Node<'_>) -> Vec<JsTsParam> {
    let params = match find_js_ts_parameters_node(*func) {
        Some(n) => n,
        None => return Vec::new(),
    };
    let mut out = Vec::new();
    let mut cursor = params.walk();
    for child in params.named_children(&mut cursor) {
        if let Some(name) = js_ts_param_name(parsed, &child) {
            let has_nest_source_decorator = js_ts_param_has_nest_source_decorator(parsed, &child);
            out.push(JsTsParam {
                name,
                has_nest_source_decorator,
            });
        }
    }
    out
}

fn find_js_ts_parameters_node(node: Node<'_>) -> Option<Node<'_>> {
    if let Some(params) = node.child_by_field_name("parameters") {
        return Some(params);
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "formal_parameters");
    found
}

fn js_ts_param_name(parsed: &ParsedFile, param: &Node<'_>) -> Option<String> {
    for field in ["pattern", "name", "parameter", "left"] {
        if let Some(node) = param.child_by_field_name(field) {
            if let Some(name) = js_ts_first_identifier(parsed, node) {
                return Some(name);
            }
        }
    }
    if parsed.language.is_identifier_node(param.kind()) {
        return Some(parsed.node_text(param).to_string());
    }
    let mut names = Vec::new();
    collect_identifier_names(parsed, *param, &mut names);
    names.into_iter().find(|name| {
        !matches!(
            name.as_str(),
            "Body"
                | "Query"
                | "Param"
                | "Headers"
                | "Req"
                | "Request"
                | "Get"
                | "Post"
                | "Put"
                | "Patch"
                | "Delete"
                | "All"
        )
    })
}

fn js_ts_first_identifier(parsed: &ParsedFile, node: Node<'_>) -> Option<String> {
    if parsed.language.is_identifier_node(node.kind()) {
        return Some(parsed.node_text(&node).to_string());
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(name) = js_ts_first_identifier(parsed, child) {
            return Some(name);
        }
    }
    None
}

fn collect_identifier_names(parsed: &ParsedFile, node: Node<'_>, out: &mut Vec<String>) {
    if parsed.language.is_identifier_node(node.kind()) {
        out.push(parsed.node_text(&node).to_string());
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_identifier_names(parsed, child, out);
    }
}

pub(super) fn js_ts_framework_source_params(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    params: &[JsTsParam],
    framework_receivers: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    match framework {
        "nestjs" => {
            if !js_ts_has_route_decorator(parsed, func) {
                return out;
            }
            for param in params {
                if param.has_nest_source_decorator {
                    out.insert(param.name.clone());
                }
            }
        }
        "fastify" => {
            if !js_ts_is_framework_route_handler(parsed, func, framework, framework_receivers) {
                return out;
            }
            if let Some(first) = params.first() {
                if matches!(first.name.as_str(), "request" | "req") {
                    out.insert(first.name.clone());
                }
            }
        }
        "express" => {
            if !js_ts_is_framework_route_handler(parsed, func, framework, framework_receivers) {
                return out;
            }
            if params.len() >= 2
                && matches!(params[0].name.as_str(), "req" | "request")
                && matches!(params[1].name.as_str(), "res" | "response")
            {
                out.insert(params[0].name.clone());
            }
        }
        "koa" => {
            if !js_ts_is_framework_route_handler(parsed, func, framework, framework_receivers) {
                return out;
            }
            if let Some(first) = params.first() {
                if first.name == "ctx" || first.name == "context" {
                    out.insert(first.name.clone());
                }
            }
        }
        _ => {}
    }
    out
}

fn js_ts_is_framework_route_handler(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
) -> bool {
    let imports = parsed.extract_imports();
    let mut current = func.parent();
    while let Some(node) = current {
        if parsed.language.is_call_node(node.kind())
            && js_ts_call_args_contain_function(parsed, &node, func.id())
            && js_ts_route_call_matches_framework(
                parsed,
                &node,
                framework,
                framework_receivers,
                &imports,
            )
        {
            return true;
        }
        current = node.parent();
    }

    let binding_names = js_ts_function_binding_names(parsed, func);
    !binding_names.is_empty()
        && js_ts_registered_route_references_handler(
            parsed,
            framework,
            framework_receivers,
            &imports,
            &binding_names,
        )
}

fn js_ts_function_binding_names(parsed: &ParsedFile, func: &Node<'_>) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if let Some(name) = parsed.language.function_name(func) {
        if parsed.language.is_identifier_node(name.kind())
            || matches!(
                name.kind(),
                "property_identifier" | "shorthand_property_identifier" | "field_identifier"
            )
        {
            names.insert(parsed.node_text(&name).to_string());
        }
    }
    names
}

fn js_ts_registered_route_references_handler(
    parsed: &ParsedFile,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
    binding_names: &BTreeSet<String>,
) -> bool {
    let mut calls = Vec::new();
    collect_js_ts_call_nodes(parsed.tree.root_node(), parsed, &mut calls);
    calls.iter().any(|call| {
        js_ts_route_call_matches_framework(parsed, call, framework, framework_receivers, imports)
            && js_ts_call_args_reference_handler_name(parsed, call, binding_names)
    })
}

fn js_ts_call_args_reference_handler_name(
    parsed: &ParsedFile,
    call: &Node<'_>,
    binding_names: &BTreeSet<String>,
) -> bool {
    let Some(args) = parsed.language.call_arguments(call) else {
        return false;
    };
    js_ts_node_references_handler_name(parsed, args, binding_names)
}

fn js_ts_node_references_handler_name(
    parsed: &ParsedFile,
    node: Node<'_>,
    binding_names: &BTreeSet<String>,
) -> bool {
    if parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    if parsed.language.is_identifier_node(node.kind())
        && binding_names.contains(parsed.node_text(&node))
    {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .any(|child| js_ts_node_references_handler_name(parsed, child, binding_names));
    found
}

fn js_ts_call_args_contain_function(parsed: &ParsedFile, call: &Node<'_>, func_id: usize) -> bool {
    let Some(args) = parsed.language.call_arguments(call) else {
        return false;
    };
    js_ts_node_contains_function_without_intervening_function(parsed, args, func_id)
}

fn js_ts_node_contains_function_without_intervening_function(
    parsed: &ParsedFile,
    node: Node<'_>,
    func_id: usize,
) -> bool {
    if node.id() == func_id {
        return true;
    }
    if parsed.language.function_node_types().contains(&node.kind()) {
        return false;
    }
    let mut cursor = node.walk();
    let found = node.named_children(&mut cursor).any(|child| {
        js_ts_node_contains_function_without_intervening_function(parsed, child, func_id)
    });
    found
}

fn js_ts_route_call_matches_framework(
    parsed: &ParsedFile,
    call: &Node<'_>,
    framework: &str,
    framework_receivers: &BTreeSet<String>,
    imports: &BTreeMap<String, String>,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "member_expression" {
        return false;
    }
    let Some(property) = function.child_by_field_name("property") else {
        return false;
    };
    let method = parsed
        .node_text(&property)
        .trim_matches(|c| c == '\'' || c == '"' || c == '`');
    if !js_ts_framework_route_method(framework, method) {
        return false;
    }
    let Some(receiver) = function.child_by_field_name("object") else {
        return false;
    };
    js_ts_receiver_expr_is_framework_instance(
        parsed,
        receiver,
        framework,
        framework_receivers,
        imports,
    ) || js_ts_receiver_expr_is_route_builder(
        parsed,
        receiver,
        framework,
        framework_receivers,
        imports,
    )
}

fn js_ts_param_has_nest_source_decorator(parsed: &ParsedFile, param: &Node<'_>) -> bool {
    js_ts_node_has_direct_decorator_named(
        parsed,
        *param,
        &["Body", "Query", "Param", "Headers", "Req", "Request"],
    )
}

fn js_ts_has_route_decorator(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    let route_names = ["Get", "Post", "Put", "Patch", "Delete", "All"];
    js_ts_node_has_direct_decorator_named(parsed, *func, &route_names)
        || js_ts_function_header_has_decorator_named(parsed, func, &route_names)
}

fn js_ts_node_has_direct_decorator_named(
    parsed: &ParsedFile,
    node: Node<'_>,
    names: &[&str],
) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "decorator" && js_ts_decorator_name_is_one_of(parsed, child, names) {
            return true;
        }
    }
    false
}

fn js_ts_decorator_name_is_one_of(
    parsed: &ParsedFile,
    decorator: Node<'_>,
    names: &[&str],
) -> bool {
    js_ts_decorator_name(parsed, decorator)
        .is_some_and(|decorator_name| names.iter().any(|name| decorator_name == *name))
}

fn js_ts_function_header_has_decorator_named(
    parsed: &ParsedFile,
    func: &Node<'_>,
    names: &[&str],
) -> bool {
    let start_line = func.start_position().row + 1;
    let params_line = find_js_ts_parameters_node(*func)
        .map(|params| params.start_position().row + 1)
        .unwrap_or(start_line);
    let window_start = start_line.saturating_sub(5).max(1);
    for line in window_start..=params_line {
        if names.iter().any(|name| {
            let decorator = format!("@{}", name);
            parsed.line_has_code_text(line, &decorator)
        }) {
            return true;
        }
    }
    false
}

fn js_ts_decorator_name(parsed: &ParsedFile, decorator: Node<'_>) -> Option<String> {
    let target = js_ts_first_decorator_target(parsed, decorator)?;
    let target = unwrap_parenthesized(target);
    let target = if parsed.language.is_call_node(target.kind()) {
        js_ts_call_or_new_callee(target)?
    } else {
        target
    };
    let target = unwrap_parenthesized(target);
    let name_node = if target.kind() == "member_expression" {
        target.child_by_field_name("property")?
    } else {
        target
    };
    if !parsed.language.is_identifier_node(name_node.kind()) {
        return None;
    }
    Some(
        parsed
            .node_text(&name_node)
            .trim_start_matches('@')
            .to_string(),
    )
}

fn js_ts_first_decorator_target<'a>(parsed: &ParsedFile, decorator: Node<'a>) -> Option<Node<'a>> {
    let mut cursor = decorator.walk();
    let target = decorator.named_children(&mut cursor).find(|child| {
        parsed.language.is_call_node(child.kind())
            || child.kind() == "member_expression"
            || parsed.language.is_identifier_node(child.kind())
    });
    target
}
