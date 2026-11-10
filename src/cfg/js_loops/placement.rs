//! The placement rule of the loop plan (#693, #697; option B, slice 2).
//!
//! Reaching definitions places every definition and use of an owner on the statement node that its
//! LINE resolves to through `StatementLookup` (the shortest retained span holding the line). A loop
//! rewrite is built on the line nodes of its head, so it is representable only when the placement agrees
//! with the loop: every header execution occurrence (a `for` initializer, test and update, a `while`
//! test, an iteration binding and iterated expression, a `do` condition) resolves to the head node (a
//! `do`'s dedicated condition node), and no body execution occurrence resolves to it. An occurrence is
//! a named syntax node of the header or the body, at the line it starts on; a nested callable is one
//! occurrence (its own statements belong to its own owner) and a body block is no occurrence.
use super::is_loop;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Where the occurrences of a loop tree are not placed as the rewrite needs, or `None` when they all are.
/// `root` is the byte range of the tree's outermost loop, `owner` its callable.
pub(super) fn violation(
    parsed: &ParsedFile,
    owner: Node<'_>,
    root: (usize, usize),
) -> Option<String> {
    let boundaries = parsed.language.callable_boundary_node_types();
    let top = parsed
        .tree
        .root_node()
        .named_descendant_for_byte_range(root.0, root.1)
        .filter(|n| is_loop(n.kind()) && (n.start_byte(), n.end_byte()) == root)?;
    let mut loops = Vec::new();
    let mut stack = vec![top];
    while let Some(n) = stack.pop() {
        if n.id() != top.id() && boundaries.contains(&n.kind()) {
            continue;
        }
        if is_loop(n.kind()) {
            loops.push(n);
        }
        let mut cursor = n.walk();
        stack.extend(n.named_children(&mut cursor));
    }
    loops.sort_by_key(|n| n.start_byte());
    // Every loop's head and its header and body rows, then ONE lookup over all the rows of the tree.
    let mut parts = Vec::new();
    let mut all = BTreeSet::new();
    for l in loops {
        let body = l.child_by_field_name("body");
        let head = if l.kind() == "do_statement" {
            parsed.do_geometry_line(&l)?
        } else {
            l.start_position().row + 1
        };
        let (mut header, mut inside) = (BTreeSet::new(), BTreeSet::new());
        let mut cursor = l.walk();
        for part in l.named_children(&mut cursor) {
            if body.is_some_and(|b| b.id() == part.id()) {
                rows(part, &boundaries, true, &mut inside);
            } else {
                rows(part, &boundaries, false, &mut header);
            }
        }
        all.extend(header.iter().chain(&inside));
        parts.push((l.kind(), head, header, inside));
    }
    let wanted: Vec<usize> = all.into_iter().collect();
    let placed: BTreeMap<usize, Option<usize>> = wanted
        .iter()
        .copied()
        .zip(crate::cpg::placed_lines(parsed, &owner, &wanted))
        .collect();
    for (kind, head, header, inside) in parts {
        for row in &header {
            if placed[row] != Some(head) {
                return Some(format!(
                    "{kind}@L{head}: header occurrence on L{row} is placed at {:?}",
                    placed[row]
                ));
            }
        }
        for row in &inside {
            if placed[row] == Some(head) {
                return Some(format!(
                    "{kind}@L{head}: body occurrence on L{row} is placed at the head"
                ));
            }
        }
    }
    None
}

/// The start rows of the named nodes under `node` (a nested callable's inside excluded; a block, a
/// comment and, in a body, nothing else structural, are no occurrence).
fn rows(node: Node<'_>, boundaries: &[&str], body: bool, out: &mut BTreeSet<usize>) {
    let mut stack = vec![node];
    while let Some(n) = stack.pop() {
        let kind = n.kind();
        if kind != "comment" && !(body && kind == "statement_block") {
            out.insert(n.start_position().row + 1);
        }
        if boundaries.contains(&kind) {
            continue;
        }
        let mut cursor = n.walk();
        stack.extend(n.named_children(&mut cursor));
    }
}
