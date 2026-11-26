//! Timing check for a lexical value read before its declarator has finished.
//!
//! A read at or after the end of the declarator passes. An earlier read passes
//! only when the nearest function-like ancestor inside the binding's scope is a
//! function expression or arrow literal that holds the read in its runtime body
//! or a parameter default, and that literal is delivered through a closed list of
//! storage boundaries. Everything else is refused, so the caller counts one
//! `refused_pre_initializer_unproved`.

use crate::ast::is_js_ts_function_like;
use tree_sitter::Node;

use super::props_keys::{is_transparent_wrapper, strip_transparent_wrappers};

pub(super) fn evaluated_after(site: Node<'_>, decl: Node<'_>, scope: Node<'_>) -> bool {
    if site.start_byte() >= decl.end_byte() {
        return true;
    }
    let Some((literal, child)) = nearest_function(site, scope) else {
        return false;
    };
    matches!(
        literal.kind(),
        "function_expression" | "generator_function" | "arrow_function"
    ) && ["body", "parameters", "parameter"]
        .iter()
        .any(|field| literal.child_by_field_name(field) == Some(child))
        && delivered_by_listed_use(literal)
}

/// The nearest function-like ancestor strictly inside `scope`, with the child of
/// it that holds `site`.
fn nearest_function<'t>(site: Node<'t>, scope: Node<'t>) -> Option<(Node<'t>, Node<'t>)> {
    let mut child = site;
    while child != scope {
        let parent = child.parent()?;
        if parent == scope {
            return None;
        }
        if is_js_ts_function_like(parent.kind()) {
            return Some((parent, child));
        }
        child = parent;
    }
    None
}

fn first_expression(parent: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = parent.walk();
    let first = parent
        .named_children(&mut cursor)
        .find(|node| !node.is_extra());
    first
}

/// The literal, followed outward through its value operand in transparent
/// wrappers, must be the whole value of exactly one of three uses: a
/// single-identifier declarator initializer, a return operand or concise arrow
/// body, or the braced value of a named JSX attribute.
fn delivered_by_listed_use(literal: Node<'_>) -> bool {
    let mut current = literal;
    while let Some(parent) = current.parent() {
        if is_transparent_wrapper(parent.kind())
            && strip_transparent_wrappers(parent) == strip_transparent_wrappers(current)
        {
            current = parent;
            continue;
        }
        return match parent.kind() {
            "variable_declarator" => {
                parent.child_by_field_name("value") == Some(current)
                    && parent
                        .child_by_field_name("name")
                        .is_some_and(|name| name.kind() == "identifier")
            }
            "return_statement" => first_expression(parent) == Some(current),
            "arrow_function" => parent.child_by_field_name("body") == Some(current),
            "jsx_expression" => {
                first_expression(parent) == Some(current)
                    && parent
                        .parent()
                        .is_some_and(|attribute| attribute.kind() == "jsx_attribute")
            }
            _ => false,
        };
    }
    false
}
