//! Python framework request sources (Flask, Django, FastAPI, pydantic).

use prism::access_path::AccessPath;
use crate::algorithms::taint::{
    PythonDjangoRequestDataSources, PythonFlaskRequestDataSources, PythonParam, TaintSeed,
};
use prism::ast::tree_helpers::call_path_text;
use prism::ast::ParsedFile;
use prism::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn detect_python_framework_sources(
    file_path: &str,
    parsed: &ParsedFile,
    sources: &mut Vec<TaintSeed>,
) {
    let pydantic_models = collect_python_pydantic_models(parsed);
    let flask_receivers = prism::frameworks::python::flask::route_receivers(parsed);
    // Compute FastAPI route receivers once per file rather than per function;
    // the AST walk is O(tree_size) and `function_has_route_decorator_with_receivers`
    // re-uses the result for each handler check.
    let fastapi_receivers = prism::frameworks::python::fastapi::route_receivers(parsed);
    for func in parsed.all_functions() {
        if python_is_inner_decorated_function(&func) {
            continue;
        }
        let line = func.start_position().row + 1;
        let params = python_function_params(parsed, &func);
        let has_request_param = params.iter().any(|param| param.name == "request");
        let django_request_data = if has_request_param {
            python_django_request_data_sources(parsed, &func)
        } else {
            PythonDjangoRequestDataSources::default()
        };
        let has_django_import_context =
            parsed.source.contains("django") || parsed.source.contains("rest_framework");
        let is_fastapi_route =
            prism::frameworks::python::fastapi::function_has_route_decorator_with_receivers(
                parsed,
                &func,
                &fastapi_receivers,
            );
        let is_flask_route =
            prism::frameworks::python::flask::function_has_route_decorator_with_receivers(
                parsed,
                &func,
                &flask_receivers,
            );
        let is_drf_or_django_view = has_request_param
            && (has_django_import_context
                || (django_request_data.has_access
                    && python_looks_like_standalone_django_view(parsed, &func)));

        if is_fastapi_route {
            for param in &params {
                if param.name == "self" {
                    continue;
                }
                let annotation = param.annotation.as_deref().unwrap_or("");
                if annotation.contains("Request")
                    || annotation.contains("Query")
                    || annotation.contains("Path")
                    || annotation.contains("Body")
                    || annotation.contains("Header")
                    || annotation.contains("Form")
                    || annotation.contains("File")
                    || pydantic_models.contains(annotation)
                {
                    sources.push(TaintSeed::target(
                        file_path.to_string(),
                        line,
                        AccessPath::simple(param.name.as_str()),
                    ));
                }
            }
        } else if is_drf_or_django_view {
            for param in &params {
                if param.name == "request" {
                    sources.push(TaintSeed::target(
                        file_path.to_string(),
                        line,
                        AccessPath::simple("request"),
                    ));
                }
            }
            for (source_line, target) in django_request_data.targets {
                sources.push(TaintSeed::target(
                    file_path.to_string(),
                    source_line,
                    target,
                ));
            }
        } else if is_flask_route {
            let flask_request_data = python_flask_request_data_sources(parsed, &func);
            for source_line in flask_request_data.lines {
                sources.push(TaintSeed::line(file_path.to_string(), source_line));
            }
            for (source_line, target) in flask_request_data.targets {
                sources.push(TaintSeed::target(
                    file_path.to_string(),
                    source_line,
                    target,
                ));
            }
        }
    }
}

fn python_django_request_data_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
) -> PythonDjangoRequestDataSources {
    let mut sources = PythonDjangoRequestDataSources::default();
    collect_django_request_data_sources(parsed, *func, &mut sources);
    sources
}

fn python_flask_request_data_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
) -> PythonFlaskRequestDataSources {
    let mut sources = PythonFlaskRequestDataSources::default();
    collect_flask_request_data_sources(parsed, *func, &mut sources);
    sources
}

fn collect_flask_request_data_sources(
    parsed: &ParsedFile,
    node: Node<'_>,
    sources: &mut PythonFlaskRequestDataSources,
) {
    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            collect_flask_request_assignment_targets(parsed, lhs, rhs, sources);
        }
        return;
    }

    if let Some((target, value)) = python_named_expression_parts(parsed, node) {
        if node_contains_flask_request_data_access(parsed, value) {
            collect_flask_request_targets(parsed, target, node.start_position().row + 1, sources);
        }
        return;
    }

    if parsed.language.is_call_node(node.kind()) {
        if let Some(path) = call_path_text(parsed, &node) {
            if python_is_flask_request_source_call(path.trim()) {
                sources.lines.insert(node.start_position().row + 1);
                return;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_flask_request_data_sources(parsed, child, sources);
    }
}

fn collect_flask_request_assignment_targets(
    parsed: &ParsedFile,
    lhs: Node<'_>,
    rhs: Node<'_>,
    sources: &mut PythonFlaskRequestDataSources,
) {
    let lhs_items = python_assignment_items(lhs);
    let rhs_items = python_assignment_items(rhs);
    if lhs_items.len() == rhs_items.len() && lhs_items.len() > 1 {
        for (lhs_item, rhs_item) in lhs_items.into_iter().zip(rhs_items) {
            if node_contains_flask_request_data_access(parsed, rhs_item) {
                collect_flask_request_targets(
                    parsed,
                    lhs_item,
                    rhs_item.start_position().row + 1,
                    sources,
                );
            }
        }
        return;
    }

    if node_contains_flask_request_data_access(parsed, rhs) {
        collect_flask_request_targets(parsed, lhs, rhs.start_position().row + 1, sources);
    }
}

fn collect_flask_request_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    sources: &mut PythonFlaskRequestDataSources,
) {
    match node.kind() {
        "identifier" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                sources.targets.insert((line, AccessPath::simple(name)));
            }
        }
        "pattern_list"
        | "tuple_pattern"
        | "list_pattern"
        | "tuple"
        | "list"
        | "parenthesized_expression" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_flask_request_targets(parsed, child, line, sources);
            }
        }
        _ => {}
    }
}

pub(super) fn python_assignment_items(node: Node<'_>) -> Vec<Node<'_>> {
    let node = unwrap_python_parenthesized_expression(node);
    if !matches!(
        node.kind(),
        "pattern_list" | "expression_list" | "tuple" | "list" | "tuple_pattern" | "list_pattern"
    ) {
        return vec![node];
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn unwrap_python_parenthesized_expression(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" {
        let mut cursor = node.walk();
        let next = node.named_children(&mut cursor).next();
        match next {
            Some(child) => node = child,
            None => return node,
        }
    }
    node
}

fn python_named_expression_parts<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
) -> Option<(Node<'a>, Node<'a>)> {
    if parsed.language != Language::Python
        || !matches!(node.kind(), "named_expression" | "assignment_expression")
    {
        return None;
    }
    let mut cursor = node.walk();
    let children: Vec<Node<'a>> = node.named_children(&mut cursor).collect();
    let target = children.first().copied()?;
    let value = children.last().copied()?;
    if target.id() == value.id() {
        return None;
    }
    Some((target, value))
}

fn collect_django_request_data_sources(
    parsed: &ParsedFile,
    node: Node<'_>,
    sources: &mut PythonDjangoRequestDataSources,
) {
    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            if node_contains_django_request_data_access(parsed, rhs) {
                sources.has_access = true;
                collect_bare_identifier_targets(
                    parsed,
                    lhs,
                    node.start_position().row + 1,
                    sources,
                );
            }
        }
    }

    if node.kind() == "attribute" {
        let text = parsed.node_text(&node);
        if python_is_django_request_data_access(text.trim()) {
            sources.has_access = true;
        }
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_django_request_data_sources(parsed, child, sources);
    }
}

fn node_contains_django_request_data_access(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if node.kind() == "attribute" {
        let text = parsed.node_text(&node);
        if python_is_django_request_data_access(text.trim()) {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_django_request_data_access(parsed, child) {
            return true;
        }
    }
    false
}

pub(super) fn node_contains_flask_request_data_access(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if parsed.language.is_call_node(node.kind()) {
        if let Some(path) = call_path_text(parsed, &node) {
            if python_is_flask_request_source_call(path.trim()) {
                return true;
            }
        }
    }
    if node.kind() == "attribute" {
        let text = parsed.node_text(&node);
        if python_is_flask_request_data_access(text.trim()) {
            return true;
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_flask_request_data_access(parsed, child) {
            return true;
        }
    }
    false
}

fn collect_bare_identifier_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    sources: &mut PythonDjangoRequestDataSources,
) {
    match node.kind() {
        "identifier" => {
            sources
                .targets
                .insert((line, AccessPath::simple(parsed.node_text(&node))));
        }
        "pattern_list" | "tuple_pattern" | "list_pattern" | "tuple" | "list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_bare_identifier_targets(parsed, child, line, sources);
            }
        }
        _ => {}
    }
}

fn python_is_django_request_data_access(path: &str) -> bool {
    const ACCESSORS: &[&str] = &["GET", "POST", "FILES", "COOKIES", "META", "body", "method"];
    ACCESSORS.iter().any(|accessor| {
        let prefix = format!("request.{accessor}");
        path == prefix || path.starts_with(&format!("{prefix}."))
    })
}

fn python_is_flask_request_source_call(path: &str) -> bool {
    python_is_flask_request_data_access(path)
        || matches!(path, "request.get_json" | "request.get_data")
}

fn python_is_flask_request_data_access(path: &str) -> bool {
    const ACCESSORS: &[&str] = &[
        "args", "form", "values", "cookies", "headers", "files", "json", "data",
    ];
    ACCESSORS.iter().any(|accessor| {
        let prefix = format!("request.{accessor}");
        path == prefix || path.starts_with(&format!("{prefix}."))
    })
}

fn python_looks_like_standalone_django_view(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    parsed.path.ends_with("views.py")
        || parsed
            .language
            .function_name(func)
            .map(|name| parsed.node_text(&name).contains("view"))
            .unwrap_or(false)
}

pub(in crate::algorithms::taint) fn python_is_inner_decorated_function(func: &Node<'_>) -> bool {
    func.kind() == "function_definition"
        && func
            .parent()
            .is_some_and(|parent| parent.kind() == "decorated_definition")
}

fn python_function_params(parsed: &ParsedFile, func: &Node<'_>) -> Vec<PythonParam> {
    let mut out = Vec::new();
    let function_node = if func.kind() == "decorated_definition" {
        let mut cursor = func.walk();
        let found = func
            .children(&mut cursor)
            .find(|child| child.kind() == "function_definition")
            .unwrap_or(*func);
        found
    } else {
        *func
    };
    let params = match function_node.child_by_field_name("parameters") {
        Some(p) => p,
        None => return out,
    };
    let mut cursor = params.walk();
    for child in params.named_children(&mut cursor) {
        let text = parsed.node_text(&child).trim();
        if text.is_empty() || text == "/" || text == "*" {
            continue;
        }
        let text = text.trim_start_matches('*');
        let (name_part, rest) = text
            .split_once(':')
            .map(|(n, r)| (n.trim(), Some(r.trim())))
            .unwrap_or((text.trim(), None));
        let name = name_part
            .split('=')
            .next()
            .unwrap_or(name_part)
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let annotation = rest.map(|r| {
            r.split('=')
                .next()
                .unwrap_or(r)
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        });
        out.push(PythonParam { name, annotation });
    }
    out
}

fn collect_python_pydantic_models(parsed: &ParsedFile) -> BTreeSet<String> {
    let mut models = BTreeSet::new();
    for line in parsed.source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("class ") || !trimmed.contains("BaseModel") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("class ") {
            let name = rest
                .split(['(', ':'])
                .next()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() {
                models.insert(name);
            }
        }
    }
    models
}
