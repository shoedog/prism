//! Python HTTP client receivers (aiohttp, urllib3) and module resolution.

use crate::algorithms::taint::collect_assignments;
use crate::algorithms::taint::python::sources::python_assignment_items;
use crate::ast::ParsedFile;
use crate::ast::tree_helpers::unwrap_parenthesized;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn python_is_aiohttp_request_call(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let imports = parsed.extract_imports();
    let function = unwrap_parenthesized(function);
    if function.kind() == "identifier" {
        let name = parsed.node_text(&function);
        return name == "request" && python_imports_resolve_to_module(&imports, name, "aiohttp");
    }
    if function.kind() != "attribute" {
        return false;
    }
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if parsed.node_text(&attribute) != "request" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    python_expression_resolves_to_module(parsed, &imports, object, "aiohttp")
}

pub(in crate::algorithms::taint) fn python_is_aiohttp_client_session_method_call(
    parsed: &ParsedFile,
    call: &Node<'_>,
    expected_method: &str,
) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "attribute" {
        return false;
    }
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if parsed.node_text(&attribute) != expected_method {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let object = unwrap_parenthesized(object);
    if python_is_aiohttp_client_session_constructor_call(parsed, object) {
        return true;
    }
    if object.kind() != "identifier" {
        return false;
    }
    let receiver = parsed.node_text(&object);
    python_aiohttp_client_session_vars(parsed, call.start_position().row + 1).contains(receiver)
}

pub(in crate::algorithms::taint) fn python_is_urllib3_pool_manager_request_call(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let Some(function) = call.child_by_field_name("function") else {
        return false;
    };
    let function = unwrap_parenthesized(function);
    if function.kind() != "attribute" {
        return false;
    }
    let Some(attribute) = function.child_by_field_name("attribute") else {
        return false;
    };
    if parsed.node_text(&attribute) != "request" {
        return false;
    }
    let Some(object) = function.child_by_field_name("object") else {
        return false;
    };
    let object = unwrap_parenthesized(object);
    if python_is_urllib3_pool_manager_constructor_call(parsed, object) {
        return true;
    }
    if object.kind() != "identifier" {
        return false;
    }
    let receiver = parsed.node_text(&object);
    python_urllib3_pool_manager_vars(parsed, call.start_position().row + 1).contains(receiver)
}

fn python_aiohttp_client_session_vars(parsed: &ParsedFile, sink_line: usize) -> BTreeSet<String> {
    python_constructor_receiver_vars(parsed, sink_line, "aiohttp", "ClientSession", true)
}

fn python_urllib3_pool_manager_vars(parsed: &ParsedFile, sink_line: usize) -> BTreeSet<String> {
    python_constructor_receiver_vars(parsed, sink_line, "urllib3", "PoolManager", false)
}

fn python_constructor_receiver_vars(
    parsed: &ParsedFile,
    sink_line: usize,
    module_name: &str,
    constructor_name: &str,
    include_with_aliases: bool,
) -> BTreeSet<String> {
    let Some(func_node) = parsed.enclosing_function(sink_line) else {
        return BTreeSet::new();
    };
    let imports = parsed.extract_imports();
    let mut names = BTreeSet::new();

    let mut assignments = Vec::new();
    collect_assignments(func_node, parsed, &mut assignments);
    for assignment in assignments {
        if assignment.start_position().row + 1 > sink_line {
            continue;
        }
        let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&assignment),
            parsed.language.assignment_value(&assignment),
        ) else {
            continue;
        };
        let lhs_items = python_assignment_items(lhs);
        let rhs_items = python_assignment_items(rhs);
        if lhs_items.len() == rhs_items.len() && lhs_items.len() > 1 {
            for (lhs_item, rhs_item) in lhs_items.into_iter().zip(rhs_items) {
                if python_is_constructor_call_from_module(
                    parsed,
                    &imports,
                    rhs_item,
                    module_name,
                    constructor_name,
                ) {
                    collect_bare_identifier_name(parsed, lhs_item, &mut names);
                }
            }
        } else if python_is_constructor_call_from_module(
            parsed,
            &imports,
            rhs,
            module_name,
            constructor_name,
        ) {
            collect_bare_identifier_name(parsed, lhs, &mut names);
        }
    }

    if include_with_aliases {
        collect_constructor_with_aliases_from_with(
            parsed,
            &imports,
            func_node,
            sink_line,
            module_name,
            constructor_name,
            &mut names,
        );
    }
    names
}

fn collect_bare_identifier_name(parsed: &ParsedFile, node: Node<'_>, names: &mut BTreeSet<String>) {
    let node = unwrap_parenthesized(node);
    if node.kind() == "identifier" {
        names.insert(parsed.node_text(&node).to_string());
    }
}

fn collect_constructor_with_aliases_from_with(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    sink_line: usize,
    module_name: &str,
    constructor_name: &str,
    names: &mut BTreeSet<String>,
) {
    if node.start_position().row + 1 > sink_line {
        return;
    }
    if node.kind() == "with_statement" {
        collect_constructor_as_pattern_aliases(
            parsed,
            imports,
            node,
            module_name,
            constructor_name,
            names,
        );
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_constructor_with_aliases_from_with(
            parsed,
            imports,
            child,
            sink_line,
            module_name,
            constructor_name,
            names,
        );
    }
}

fn collect_constructor_as_pattern_aliases(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    with_node: Node<'_>,
    module_name: &str,
    constructor_name: &str,
    names: &mut BTreeSet<String>,
) {
    let header_end = with_node
        .child_by_field_name("body")
        .map(|body| body.start_byte())
        .unwrap_or(with_node.end_byte());
    collect_constructor_as_patterns_before_body(
        parsed,
        imports,
        with_node,
        header_end,
        module_name,
        constructor_name,
        names,
    );
}

fn collect_constructor_as_patterns_before_body(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    header_end: usize,
    module_name: &str,
    constructor_name: &str,
    names: &mut BTreeSet<String>,
) {
    if node.start_byte() >= header_end {
        return;
    }
    if node.kind() == "as_pattern" {
        if let Some(alias) =
            constructor_as_pattern_alias(parsed, imports, node, module_name, constructor_name)
        {
            names.insert(alias);
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_constructor_as_patterns_before_body(
            parsed,
            imports,
            child,
            header_end,
            module_name,
            constructor_name,
            names,
        );
    }
}

fn constructor_as_pattern_alias(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    as_pattern: Node<'_>,
    module_name: &str,
    constructor_name: &str,
) -> Option<String> {
    let mut alias = None;
    let mut has_constructor = false;
    let mut cursor = as_pattern.walk();
    for child in as_pattern.children(&mut cursor) {
        if child.kind() == "as_pattern_target" {
            let name = parsed.node_text(&child).trim();
            if is_python_identifier(name) {
                alias = Some(name.to_string());
            }
            continue;
        }
        if node_contains_constructor_call_from_module(
            parsed,
            imports,
            child,
            module_name,
            constructor_name,
        ) {
            has_constructor = true;
        }
    }
    has_constructor.then_some(alias).flatten()
}

fn node_contains_constructor_call_from_module(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    module_name: &str,
    constructor_name: &str,
) -> bool {
    if python_is_constructor_call_from_module(parsed, imports, node, module_name, constructor_name)
    {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_constructor_call_from_module(
            parsed,
            imports,
            child,
            module_name,
            constructor_name,
        ) {
            return true;
        }
    }
    false
}

fn python_is_aiohttp_client_session_constructor_call(parsed: &ParsedFile, node: Node<'_>) -> bool {
    let imports = parsed.extract_imports();
    python_is_constructor_call_from_module(parsed, &imports, node, "aiohttp", "ClientSession")
}

fn python_is_urllib3_pool_manager_constructor_call(parsed: &ParsedFile, node: Node<'_>) -> bool {
    let imports = parsed.extract_imports();
    python_is_constructor_call_from_module(parsed, &imports, node, "urllib3", "PoolManager")
}

fn python_is_constructor_call_from_module(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    module_name: &str,
    constructor_name: &str,
) -> bool {
    let node = unwrap_parenthesized(node);
    if node.kind() != "call" {
        return false;
    }
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    let callee = parsed.node_text(&function).trim();
    let (namespace, basename) = match callee.rsplit_once('.') {
        Some((ns, name)) => (Some(ns.trim()), name.trim()),
        None => (None, callee),
    };
    if basename != constructor_name {
        return false;
    }
    match namespace {
        Some(ns) => python_expression_text_resolves_to_module(imports, ns, module_name),
        None => python_imports_resolve_to_module(imports, basename, module_name),
    }
}

fn is_python_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

fn python_expression_resolves_to_module(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    module_name: &str,
) -> bool {
    let node = unwrap_parenthesized(node);
    if node.kind() != "identifier" && node.kind() != "attribute" {
        return false;
    }
    python_expression_text_resolves_to_module(imports, parsed.node_text(&node).trim(), module_name)
}

fn python_expression_text_resolves_to_module(
    imports: &BTreeMap<String, String>,
    text: &str,
    module_name: &str,
) -> bool {
    let head = text.split('.').next().unwrap_or(text);
    python_imports_resolve_to_module(imports, head, module_name)
}

fn python_imports_resolve_to_module(
    imports: &BTreeMap<String, String>,
    name: &str,
    module_name: &str,
) -> bool {
    imports
        .get(name)
        .is_some_and(|module| python_module_matches(module, module_name))
}

fn python_module_matches(module: &str, expected: &str) -> bool {
    module == expected || module.starts_with(&format!("{}.", expected))
}
