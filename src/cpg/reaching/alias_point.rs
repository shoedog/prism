//! #783 option A: does a certified alias's def reach the point where a write with no CFG node of its own runs?
//!
//! A write inside a class (a static field value, a static block, a computed key, a decorator) runs when the
//! class is evaluated, and it has no statement node: the census cannot place it. Its evaluation point is
//! therefore found from the syntax, and the alias def must reach that point (design 2.6 item 5), never a node
//! after a join:
//! - a class expression runs with the statement that contains it: the def must be in that statement's node entry
//!   set (or that statement has no node of its own, below);
//! - a class declaration is no statement of the CFG: its point is the end of the preceding sibling statement
//!   (a simple statement with a node: its out set), or, when it is first in its block, the header of the `if`
//!   whose arm holds the block (out set of the header node), or the point before a bare block that holds it.
//!
//! Anything else (a compound sibling, a loop, `try`, `switch`, the function's own body start, a statement
//! sharing its line with another) is not established: the answer is `false` and the caller keeps the 4a label.
use super::graph::BitSet;
use super::Line;
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// What the proof reads: the solved owner.
pub(super) struct Reach<'a> {
    pub parsed: &'a ParsedFile,
    pub owner: (usize, usize),
    pub in_sets: &'a [BitSet],
    pub mapped_defs: &'a [Option<usize>],
    pub line_index: &'a BTreeMap<Line, usize>,
}

const CLASSES: [&str; 3] = ["class", "class_declaration", "abstract_class_declaration"];
const SIMPLE: [&str; 3] = [
    "expression_statement",
    "lexical_declaration",
    "variable_declaration",
];

fn is_statement(kind: &str) -> bool {
    kind.ends_with("_statement") || kind.ends_with("_declaration")
}

impl Reach<'_> {
    /// Alias def `def` reaches the point where the write at byte `probe` runs.
    pub(super) fn reaches(
        &self,
        probe: usize,
        def: usize,
        node_of: &mut impl FnMut(Line) -> Option<usize>,
    ) -> bool {
        let Some(mut cur) = self
            .parsed
            .tree
            .root_node()
            .descendant_for_byte_range(probe, probe)
        else {
            return false;
        };
        // The outermost class below the owner, else the innermost statement.
        let (mut class, mut stmt) = (None, None);
        loop {
            if cur.start_byte() <= self.owner.0 && cur.end_byte() >= self.owner.1 {
                break;
            }
            if CLASSES.contains(&cur.kind()) {
                class = Some(cur);
            } else if stmt.is_none() && is_statement(cur.kind()) {
                stmt = Some(cur);
            }
            let Some(p) = cur.parent() else { break };
            cur = p;
        }
        let point = match class {
            Some(c) if c.kind() != "class" => Some(c),
            Some(c) => climb_to_statement(c, self.owner),
            None => stmt,
        };
        let Some(st) = point else { return false };
        if st.kind() != "class_declaration" && st.kind() != "abstract_class_declaration" {
            if let Some(n) = node_of(st.start_position().row + 1) {
                return self.in_sets[n].contains(def);
            }
        }
        self.before(st, def, node_of)
    }

    fn out_has(&self, node: usize, def: usize) -> bool {
        self.in_sets[node].contains(def) || self.mapped_defs[def] == Some(node)
    }

    /// The def reaches the point just before statement `st` (which has no node of its own). `first`: `st` is the
    /// class's own statement, whose next retained sibling in the same block is entered from that point alone.
    fn before(
        &self,
        mut st: Node<'_>,
        def: usize,
        node_of: &mut impl FnMut(Line) -> Option<usize>,
    ) -> bool {
        let mut first = true;
        loop {
            // The next retained statement of the same block: its entry set is the state before `st` (the
            // statements between are class declarations, which have no node), never a node after a join.
            if first {
                let mut next = st.next_named_sibling();
                while let Some(n) = next {
                    if n.kind() != "comment" && is_statement(n.kind()) {
                        if let Some(node) = node_of(n.start_position().row + 1) {
                            return self.in_sets[node].contains(def);
                        }
                    }
                    next = n.next_named_sibling();
                }
            }
            let mut prev = st.prev_named_sibling();
            while prev.is_some_and(|p| {
                p.kind() == "comment" || node_of(p.start_position().row + 1).is_none()
            }) {
                prev = prev.and_then(|p| p.prev_named_sibling());
            }
            if let Some(p) = prev {
                // A simple statement: its out set. A compound one (an `if`, a loop, `try`, ...): the alias is
                // declared once and never rewritten, so it reaches the point after the statement exactly when it
                // reaches the statement's entry (a declarator inside it would not be in the entry set).
                return node_of(p.start_position().row + 1).is_some_and(|n| {
                    if SIMPLE.contains(&p.kind()) {
                        self.out_has(n, def)
                    } else {
                        self.in_sets[n].contains(def)
                    }
                });
            }
            // First in its block: the point is the entry of the statement that holds the block.
            let Some(block) = st.parent().filter(|b| b.kind() == "statement_block") else {
                return false;
            };
            let Some(mut g) = block.parent() else {
                return false;
            };
            match g.kind() {
                "statement_block" => {
                    st = block;
                    first = false;
                    continue;
                }
                "labeled_statement" => {
                    st = g;
                    first = false;
                    continue;
                }
                "else_clause" | "catch_clause" | "finally_clause" => match g.parent() {
                    Some(p) => g = p,
                    None => return false,
                },
                _ => {}
            }
            return COMPOUND.contains(&g.kind())
                && node_of(g.start_position().row + 1)
                    .is_some_and(|n| self.in_sets[n].contains(def));
        }
    }
}

const COMPOUND: [&str; 5] = [
    "if_statement",
    "for_statement",
    "for_in_statement",
    "while_statement",
    "try_statement",
];

fn climb_to_statement<'t>(mut n: Node<'t>, owner: (usize, usize)) -> Option<Node<'t>> {
    while let Some(p) = n.parent() {
        if p.start_byte() <= owner.0 && p.end_byte() >= owner.1 {
            return None;
        }
        if is_statement(p.kind()) {
            return Some(p);
        }
        n = p;
    }
    None
}
