//! Function-local Go goto, break and continue targets.
//!
//! Labels already occupy CFG statement lines, including labels on their own
//! line. Break and continue share the same enclosing-construct lookup for both
//! their named and unnamed forms. Nested callables have separate label scopes.
use super::{CfgEdge, EdgeOrigin, EdgeSink};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) fn build(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize], edges: &mut EdgeSink) {
    let Some(body) = owner.child_by_field_name("body") else {
        return;
    };
    let mut nodes = Vec::new();
    collect(body, parsed, &mut nodes);
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
                    let next = after(target, lines);
                    if next.is_none() {
                        // Return has no explicit exit node. Like return, an
                        // exiting break only has edges to this owner's defers.
                        for &to in &defers {
                            push(node, to, parsed, edges, EdgeOrigin::GoDeferExit);
                        }
                    }
                    next
                }
            }
            _ => continue,
        };
        if let Some(to) = target {
            push(node, to, parsed, edges, EdgeOrigin::Structured);
        }
    }
}

fn collect<'tree>(node: Node<'tree>, parsed: &ParsedFile, out: &mut Vec<Node<'tree>>) {
    if parsed
        .language
        .callable_boundary_node_types()
        .contains(&node.kind())
    {
        return;
    }
    out.push(node);
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, parsed, out);
    }
}

fn label_name<'a>(node: Node<'_>, parsed: &'a ParsedFile) -> Option<&'a str> {
    let mut cursor = node.walk();
    let name = node
        .named_children(&mut cursor)
        .find(|child| child.kind() == "label_name")
        .map(|name| parsed.node_text(&name));
    name
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
    let mut at = jump.parent();
    while let Some(node) = at {
        if node.id() == owner.id() {
            break;
        }
        if is_construct(node.kind(), is_continue) {
            match name {
                None => return Some(node),
                Some(name) => {
                    if node.parent().is_some_and(|parent| {
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
        at = node.parent();
    }
    None
}

fn after(node: Node<'_>, lines: &[usize]) -> Option<usize> {
    let end = node.end_position().row + 1;
    lines
        .get(lines.partition_point(|&line| line <= end))
        .copied()
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
