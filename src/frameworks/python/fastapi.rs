//! Python FastAPI framework spec.

use crate::ast::ParsedFile;
use crate::languages::Language;

use super::super::FrameworkSpec;
use std::collections::BTreeSet;
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
    !route_receivers(parsed).is_empty()
        && parsed
            .all_functions()
            .iter()
            .any(|func| function_has_route_decorator(parsed, func))
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
/// shapes: `app = FastAPI()`, `app: FastAPI = FastAPI()`, and
/// `app, router = FastAPI(), APIRouter()`.
pub fn route_receivers(parsed: &ParsedFile) -> BTreeSet<String> {
    let imports = parsed.extract_imports();
    if !imports.values().any(|module| module == "fastapi") {
        return BTreeSet::new();
    }

    let mut receivers = BTreeSet::new();
    collect_route_receivers(parsed, parsed.tree.root_node(), &mut receivers);
    receivers
}

pub fn function_has_route_decorator(parsed: &ParsedFile, func: &Node<'_>) -> bool {
    let receivers = route_receivers(parsed);
    if receivers.is_empty() {
        return false;
    }
    function_route_receiver(parsed, func)
        .as_ref()
        .is_some_and(|receiver| receivers.contains(receiver))
}

fn collect_route_receivers(parsed: &ParsedFile, node: Node<'_>, receivers: &mut BTreeSet<String>) {
    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            collect_receivers_from_assignment(parsed, lhs, rhs, receivers);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_route_receivers(parsed, child, receivers);
    }
}

fn collect_receivers_from_assignment(
    parsed: &ParsedFile,
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
            if is_fastapi_constructor_call(parsed, rhs_element) {
                collect_identifier_names(parsed, lhs_element, receivers);
            }
        }
        return;
    }

    if is_fastapi_constructor_call(parsed, rhs) {
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
    let text = parsed.node_text(&function).trim();
    let (receiver, method) = text.rsplit_once('.')?;
    Some((receiver.trim().to_string(), method.trim().to_string()))
}

fn first_decorator_target(decorator: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = decorator.walk();
    let target = decorator
        .named_children(&mut cursor)
        .find(|child| child.kind() == "call" || child.kind() == "attribute");
    target
}

fn is_fastapi_constructor_call(parsed: &ParsedFile, node: Node<'_>) -> bool {
    if node.kind() == "parenthesized_expression" {
        let mut cursor = node.walk();
        return node
            .named_children(&mut cursor)
            .any(|child| is_fastapi_constructor_call(parsed, child));
    }
    if node.kind() != "call" {
        return false;
    }
    let function = match node.child_by_field_name("function") {
        Some(function) => function,
        None => return false,
    };
    let callee = parsed.node_text(&function).trim();
    let basename = callee.rsplit('.').next().unwrap_or(callee);
    basename == "FastAPI" || basename == "APIRouter"
}

fn sequence_elements(node: Node<'_>) -> Vec<Node<'_>> {
    if !matches!(
        node.kind(),
        "pattern_list" | "expression_list" | "tuple" | "list" | "parenthesized_expression"
    ) {
        return Vec::new();
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
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
