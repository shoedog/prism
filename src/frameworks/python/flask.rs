//! Python Flask framework spec.

use crate::ast::ParsedFile;
use crate::languages::Language;

use super::super::FrameworkSpec;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "flask",
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
    "route", "get", "post", "put", "delete", "patch", "head", "options",
];

/// Return receiver names bound to Flask route-bearing objects. Supports
/// `app = Flask(...)`, `bp = Blueprint(...)`, namespaced constructors, and
/// tuple/type-annotated assignment shapes.
pub fn route_receivers(parsed: &ParsedFile) -> BTreeSet<String> {
    let imports = parsed.extract_imports();
    if !imports.values().any(|module| is_flask_module(module)) {
        return BTreeSet::new();
    }

    let mut receivers = BTreeSet::new();
    collect_route_receivers(parsed, &imports, parsed.tree.root_node(), &mut receivers);
    receivers
}

pub fn function_has_route_decorator_with_receivers(
    parsed: &ParsedFile,
    func: &Node<'_>,
    receivers: &BTreeSet<String>,
) -> bool {
    if receivers.is_empty() {
        return false;
    }
    let Some(decorated) = decorated_definition_node(*func) else {
        return false;
    };
    let mut cursor = decorated.walk();
    for child in decorated.children(&mut cursor) {
        if child.kind() != "decorator" {
            continue;
        }
        if let Some((receiver, method)) = decorator_receiver_and_method(parsed, child) {
            if ROUTE_METHODS.contains(&method.as_str()) && receivers.contains(&receiver) {
                return true;
            }
        }
    }
    false
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
            if is_flask_constructor_call(parsed, imports, rhs_element) {
                collect_receiver_identifier(parsed, lhs_element, receivers);
            }
        }
        return;
    }

    if is_flask_constructor_call(parsed, imports, rhs) {
        collect_receiver_identifier(parsed, lhs, receivers);
    }
}

fn is_flask_constructor_call(
    parsed: &ParsedFile,
    imports: &BTreeMap<String, String>,
    node: Node<'_>,
) -> bool {
    let node = unwrap_parens(node);
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
    if basename != "Flask" && basename != "Blueprint" {
        return false;
    }
    match namespace {
        Some(ns) => {
            let head = ns.split('.').next().unwrap_or(ns);
            imports
                .get(head)
                .map(|module| is_flask_module(module))
                .unwrap_or(false)
        }
        None => imports
            .get(basename)
            .map(|module| is_flask_module(module))
            .unwrap_or(false),
    }
}

fn is_flask_module(module: &str) -> bool {
    module == "flask" || module.starts_with("flask.")
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

fn decorated_definition_node(func: Node<'_>) -> Option<Node<'_>> {
    if func.kind() == "decorated_definition" {
        return Some(func);
    }
    func.parent()
        .filter(|parent| parent.kind() == "decorated_definition")
}

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

fn collect_receiver_identifier(
    parsed: &ParsedFile,
    node: Node<'_>,
    receivers: &mut BTreeSet<String>,
) {
    let node = unwrap_parens(node);
    if node.kind() == "identifier" {
        receivers.insert(parsed.node_text(&node).to_string());
    }
}
