//! Function-local Go goto, break and continue targets.
//!
//! Labels already occupy CFG statement lines, including labels on their own
//! line. Break and continue share the same enclosing-construct lookup for both
//! their named and unnamed forms. Nested callables have separate label scopes.
use super::{CfgEdge, EdgeOrigin, EdgeSink};
use crate::ast::ParsedFile;
use crate::go_statements;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) fn build(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize], edges: &mut EdgeSink) {
    let Some(body) = owner.child_by_field_name("body") else {
        return;
    };
    let mut nodes = Vec::new();
    let boundaries = parsed.language.callable_boundary_node_types();
    collect(body, &boundaries, &mut nodes);
    let mut labels = BTreeMap::new();
    let mut defers = Vec::new();
    for &node in &nodes {
        if node.kind() == "labeled_statement" {
            if let Some(name) = node.child_by_field_name("label") {
                labels.insert(parsed.node_text(&name), node);
            }
        } else if node.kind() == "defer_statement" {
            defers.push(node.start_position().row + 1);
        }
    }
    for node in nodes {
        let kind = node.kind();
        let target = match kind {
            "goto_statement" => label_name(node, parsed)
                .and_then(|name| labels.get(name))
                .map(|label| label.start_position().row + 1),
            "break_statement" | "continue_statement" => {
                let is_continue = kind == "continue_statement";
                let Some(target) = construct(node, owner, parsed, is_continue) else {
                    continue;
                };
                if is_continue {
                    Some(target.start_position().row + 1)
                } else {
                    match successor(target, owner, lines) {
                        Successor::Line(line) => Some(line),
                        Successor::Exit => {
                            // Return has no explicit exit node. Exiting break
                            // uses the same incomplete dispatch to owner defers.
                            for &to in &defers {
                                push(node, to, parsed, edges, EdgeOrigin::GoDeferExit);
                            }
                            None
                        }
                        Successor::Unknown => None,
                    }
                }
            }
            _ => continue,
        };
        if let Some(to) = target {
            push(node, to, parsed, edges, EdgeOrigin::Structured);
        }
    }
}

fn collect<'tree>(node: Node<'tree>, boundaries: &[&str], out: &mut Vec<Node<'tree>>) {
    if boundaries.contains(&node.kind()) {
        return;
    }
    out.push(node);
    for child in go_statements::children(node, Language::Go).filter(Node::is_named) {
        collect(child, boundaries, out);
    }
}

fn label_name<'a>(node: Node<'_>, parsed: &'a ParsedFile) -> Option<&'a str> {
    go_statements::children(node, Language::Go)
        .find(|child| child.kind() == "label_name")
        .map(|name| parsed.node_text(&name))
}

fn is_construct(kind: &str, is_continue: bool) -> bool {
    kind == "for_statement"
        || (!is_continue
            && matches!(
                kind,
                "expression_switch_statement" | "type_switch_statement" | "select_statement"
            ))
}

/// Named and unnamed jumps resolve through one ancestor walk. A named target
/// must label an enclosing construct of the right kind; labels in another
/// callable (or on an unrelated statement) can never be jump targets here.
fn construct<'tree>(
    jump: Node<'tree>,
    owner: Node<'tree>,
    parsed: &ParsedFile,
    is_continue: bool,
) -> Option<Node<'tree>> {
    let name = label_name(jump, parsed);
    let mut at = go_statements::parent(jump);
    while let Some(node) = at {
        if node.id() == owner.id() {
            break;
        }
        if is_construct(node.kind(), is_continue) {
            match name {
                None => return Some(node),
                Some(name) => {
                    if go_statements::parent(node).is_some_and(|parent| {
                        parent.kind() == "labeled_statement"
                            && parent
                                .child_by_field_name("label")
                                .is_some_and(|label| parsed.node_text(&label) == name)
                    }) {
                        return Some(node);
                    }
                }
            }
        }
        at = go_statements::parent(node);
    }
    None
}

pub(super) enum Successor {
    Line(usize),
    Exit,
    Unknown,
}

/// Leave syntax, not a globally sorted line range: a terminal statement may
/// resume a loop or leave an arm without ever entering the alternative arm.
pub(super) fn successor(node: Node<'_>, owner: Node<'_>, lines: &[usize]) -> Successor {
    target(node, owner, lines, false)
}

pub(super) fn entry(node: Node<'_>, owner: Node<'_>, lines: &[usize]) -> Successor {
    target(node, owner, lines, true)
}

/// Project syntax onto the existing statement universe. The four non-anchor
/// leaf statement kinds are transparent; blocks are entered structurally.
/// Never scan past an unrecognized node or into a sibling arm. Iteration keeps
/// a long sequence of unanchored statements from consuming the call stack.
fn target(mut node: Node<'_>, owner: Node<'_>, lines: &[usize], mut enter: bool) -> Successor {
    loop {
        if enter {
            if node.kind() == "block" {
                if let Some(child) = go_statements::children(node, Language::Go)
                    .find(|n| n.is_named() && n.kind() != "comment")
                {
                    node = child;
                    continue;
                }
            } else if lines
                .binary_search(&(node.start_position().row + 1))
                .is_ok()
            {
                return Successor::Line(node.start_position().row + 1);
            } else if !matches!(
                node.kind(),
                "inc_statement" | "dec_statement" | "empty_statement" | "type_declaration"
            ) {
                return Successor::Unknown;
            }
            enter = false;
        }
        let Some(parent) = go_statements::parent(node) else {
            return Successor::Unknown;
        };
        if parent.id() == owner.id() {
            return if node.kind() == "block" {
                Successor::Exit
            } else {
                Successor::Unknown
            };
        }
        match parent.kind() {
            "block" | "expression_case" | "type_case" | "communication_case" | "default_case" => {
                let mut next = go_statements::next_named_sibling(node);
                while next.is_some_and(|n| n.kind() == "comment") {
                    next = next.and_then(go_statements::next_named_sibling);
                }
                if let Some(sibling) = next {
                    node = sibling;
                    enter = true;
                    continue;
                }
            }
            "for_statement" if node.kind() == "block" => {
                return Successor::Line(parent.start_position().row + 1);
            }
            "if_statement" if matches!(node.kind(), "block" | "if_statement") => {}
            "labeled_statement" => {}
            "expression_switch_statement" | "type_switch_statement" | "select_statement"
                if matches!(
                    node.kind(),
                    "expression_case" | "type_case" | "communication_case" | "default_case"
                ) => {}
            _ => return Successor::Unknown,
        }
        node = parent;
    }
}

fn push(jump: Node<'_>, to: usize, parsed: &ParsedFile, edges: &mut EdgeSink, origin: EdgeOrigin) {
    // Record the origin at emission. Ordinary jumps are structural; synthetic
    // exit dispatch must remain incomplete, like existing return->defer joins.
    edges.edges.push(CfgEdge {
        file: parsed.path.clone(),
        from_line: jump.start_position().row + 1,
        to_line: to,
    });
    edges.origins.push(origin);
}

#[cfg(test)]
#[path = "go_jumps_tests.rs"]
mod tests;
