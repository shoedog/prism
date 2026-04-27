//! Python FastAPI framework spec.

use crate::ast::ParsedFile;
use crate::languages::Language;

use super::super::FrameworkSpec;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "fastapi",
    detect,
    sources: &[],
    sinks: &[],
    sanitizers: &[],
};

fn detect(parsed: &ParsedFile) -> bool {
    if parsed.language != Language::Python {
        return false;
    }
    let receivers = route_receivers(parsed);
    !receivers.is_empty()
        && parsed
            .all_functions()
            .iter()
            .any(|func| function_has_route_decorator_with_receivers(parsed, func, &receivers))
}

const ROUTE_METHODS: &[&str] = &[
    "get",
    "post",
    "put",
    "delete",
    "patch",
    "head",
    "options",
    "api_route",
];

/// Return receiver names bound to FastAPI route-bearing objects in code, not in
/// comments or strings. Supports canonical, type-annotated, and tuple assignment
/// shapes: `app = FastAPI()`, `app: FastAPI = FastAPI()`,
/// `app, router = FastAPI(), APIRouter()`, and `(app, router) = FastAPI(), APIRouter()`.
///
/// Constructor calls are validated against the file's import map so a local
/// `class FastAPI: ...` shadow or an unrelated `unrelated.FastAPI()` call cannot
/// register a receiver: the namespace prefix (or the bare basename, for
/// `from fastapi import FastAPI` shapes) must resolve to the `fastapi` module.
pub fn route_receivers(parsed: &ParsedFile) -> BTreeSet<String> {
    let imports = parsed.extract_imports();
    if !imports.values().any(|module| is_fastapi_module(module)) {
        return BTreeSet::new();
    }

    let mut receivers = BTreeSet::new();
    collect_route_receivers(parsed, &imports, parsed.tree.root_node(), &mut receivers);
    receivers
}

/// Convenience wrapper that recomputes `route_receivers` per call. For code that
/// queries many functions in the same file, prefer `route_receivers` once and
/// `function_has_route_decorator_with_receivers` per function to avoid O(N×tree)
/// recomputation.
pub fn function_has_route_decorator(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    let receivers = route_receivers(parsed);
    function_has_route_decorator_with_receivers(parsed, func, &receivers)
}

/// Per-function variant that takes a pre-computed `receivers` set. Use when
/// iterating many functions in the same file (e.g. taint source enumeration).
pub fn function_has_route_decorator_with_receivers(
    parsed: &ParsedFile,
    func: &Node<'_>,
    receivers: &BTreeSet<String>,
) -> bool {
    if receivers.is_empty() {
        return false;
    }
    function_route_receiver(parsed, func)
        .as_ref()
        .is_some_and(|receiver| receivers.contains(receiver))
}

fn collect_route_receivers(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
    receivers: &mut BTreeSet<String>,
) {
    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            collect_receivers_from_assignment(parsed, imports, lhs, rhs, receivers);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_route_receivers(parsed, imports, child, receivers);
    }
}

fn collect_receivers_from_assignment(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    lhs: Node<'_>,
    rhs: Node<'_>,
    receivers: &mut BTreeSet<String>,
) {
    let lhs_elements = sequence_elements(lhs);
    let rhs_elements = sequence_elements(rhs);
    if !lhs_elements.is_empty()
        && !rhs_elements.is_empty()
        && lhs_elements.len() == rhs_elements.len()
    {
        for (lhs_element, rhs_element) in lhs_elements.into_iter().zip(rhs_elements) {
            if is_fastapi_constructor_call(parsed, imports, rhs_element) {
                collect_identifier_names(parsed, lhs_element, receivers);
            }
        }
        return;
    }

    if is_fastapi_constructor_call(parsed, imports, rhs) {
        collect_identifier_names(parsed, lhs, receivers);
    }
}

fn function_route_receiver(parsed: &ParsedFile, func: &Node<'_>) -> Option<String> {
    let decorated = decorated_definition_node(*func)?;
    let mut cursor = decorated.walk();
    for child in decorated.children(&mut cursor) {
        if child.kind() != "decorator" {
            continue;
        }
        if let Some((receiver, method)) = decorator_receiver_and_method(parsed, child) {
            if ROUTE_METHODS.contains(&method.as_str()) {
                return Some(receiver);
            }
        }
    }
    None
}

fn decorated_definition_node(func: Node<'_>) -> Option<Node<'_>> {
    if func.kind() == "decorated_definition" {
        return Some(func);
    }
    func.parent()
        .filter(|parent| parent.kind() == "decorated_definition")
}

/// AST-based extraction of `(receiver, method)` from a `@receiver.method(...)`
/// or `@receiver.method` decorator. Returns `None` when the decorator target is
/// not a bare `receiver.method` shape — nested attribute access (`@some.app.get`),
/// bare-identifier decorators (`@cached`), or call-of-call decorators do not have
/// a receiver that can match a route-receiver binding.
fn decorator_receiver_and_method(
    parsed: &ParsedFile,
    decorator: Node<'_>,
) -> Option<(String, String)> {
    let target = first_decorator_target(decorator)?;
    let function = if target.kind() == "call" {
        target.child_by_field_name("function")?
    } else {
        target
    };
    if function.kind() != "attribute" {
        return None;
    }
    let object = function.child_by_field_name("object")?;
    let attribute = function.child_by_field_name("attribute")?;
    if object.kind() != "identifier" || attribute.kind() != "identifier" {
        return None;
    }
    Some((
        parsed.node_text(&object).to_string(),
        parsed.node_text(&attribute).to_string(),
    ))
}

fn first_decorator_target(decorator: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = decorator.walk();
    let target = decorator
        .named_children(&mut cursor)
        .find(|child| child.kind() == "call" || child.kind() == "attribute");
    target
}

/// Returns true when `node` is a `FastAPI(...)` or `APIRouter(...)` constructor
/// whose namespace resolves to the `fastapi` package per the file's import map.
///
/// Resolution rules:
/// - Bare `FastAPI()` / `APIRouter()`: the basename must appear as an alias in
///   the import map and resolve to `fastapi` (or a `fastapi.*` submodule). This
///   matches `from fastapi import FastAPI` while rejecting a local class shadow.
/// - Namespaced `<ns>.FastAPI()` / `<ns>.APIRouter()`: the leading namespace
///   segment must resolve to `fastapi` (e.g. `import fastapi as fa; fa.FastAPI()`
///   or `import fastapi.applications; applications.FastAPI()`).
/// - `parenthesized_expression` is unwrapped recursively before the kind check.
fn is_fastapi_constructor_call(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
) -> bool {
    if node.kind() == "parenthesized_expression" {
        let mut cursor = node.walk();
        return node
            .named_children(&mut cursor)
            .any(|child| is_fastapi_constructor_call(parsed, imports, child));
    }
    if node.kind() != "call" {
        return false;
    }
    let function = match node.child_by_field_name("function") {
        Some(function) => function,
        None => return false,
    };
    let callee = parsed.node_text(&function).trim();
    let (namespace, basename) = match callee.rsplit_once('.') {
        Some((ns, name)) => (Some(ns.trim()), name.trim()),
        None => (None, callee),
    };
    if basename != "FastAPI" && basename != "APIRouter" {
        return false;
    }
    match namespace {
        Some(ns) => {
            // Use the leading segment so `applications.FastAPI()` (where
            // `applications` came from `import fastapi.applications`) resolves.
            let head = ns.split('.').next().unwrap_or(ns);
            imports
                .get(head)
                .map(|module| is_fastapi_module(module))
                .unwrap_or(false)
        }
        None => imports
            .get(basename)
            .map(|module| is_fastapi_module(module))
            .unwrap_or(false),
    }
}

fn is_fastapi_module(module: &str) -> bool {
    module == "fastapi" || module.starts_with("fastapi.")
}

/// Return the elements of a tuple/list/pattern-list assignment side. Unwraps
/// `parenthesized_expression` so that `(app, router) = ...` (and the symmetric
/// RHS form) decomposes to its inner identifiers/calls. Returns an empty Vec
/// for non-sequence nodes.
fn sequence_elements(node: Node<'_>) -> Vec<Node<'_>> {
    let node = unwrap_parens(node);
    if !matches!(
        node.kind(),
        "pattern_list" | "expression_list" | "tuple" | "list" | "tuple_pattern" | "list_pattern"
    ) {
        return Vec::new();
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn unwrap_parens(mut node: Node<'_>) -> Node<'_> {
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

fn collect_identifier_names(parsed: &ParsedFile, node: Node<'_>, receivers: &mut BTreeSet<String>) {
    if node.kind() == "identifier" {
        receivers.insert(parsed.node_text(&node).to_string());
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_identifier_names(parsed, child, receivers);
    }
}
