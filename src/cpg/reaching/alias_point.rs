//! #783 option A: does a certified alias's declaration run before the point where a write runs, on a real route?
//!
//! Reaching-definition sets are may-sets over every CFG edge, including the edges the generic builder draws from
//! one `if` arm into the next line (a main CFG defect: the then-arm's last statement flows into the else-arm's
//! first line), so "the alias def is in the entry set of a node" is no proof that the alias reaches that node. This
//! module answers from the syntax instead, which has exactly the structured routes and none of the spurious ones.
//!
//! The alias declaration `A` is one statement in a statement list `L` (a block). It runs before a point `P` when
//! `P` lies in a statement that follows `A`'s statement in `L`, or, leaving `L` through the compound statement
//! that holds it (an `if` arm, a loop or `try` body, a bare or labeled block), in a statement that follows that
//! statement in its own list, and so on up to the owner's body. `P` in another arm of the same `if`, in a
//! `catch` or `finally` clause, or before `A` is not on such a route and is not proven (the answer is `false` and
//! the caller keeps the rule-disabled label). A statement that leaves unconditionally (`return`, `throw`, `break`,
//! `continue`) before `A` in its list, before the compound statement that holds it in the list it climbs
//! through, or between `A` and the exit of a list ends the route (the alias never runs, or the route is cut). No retained CFG node is consulted, so a
//! statement that shares a line with another, or has no node (a class declaration), cannot stand in for `P`.
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// What the proof reads: the parsed file and the owner's byte range.
pub(super) struct Reach<'a> {
    pub parsed: &'a ParsedFile,
    pub owner: (usize, usize),
}

const ABRUPT: [&str; 4] = [
    "return_statement",
    "throw_statement",
    "break_statement",
    "continue_statement",
];

impl Reach<'_> {
    fn strictly_inside(&self, n: Node<'_>) -> bool {
        !(n.start_byte() <= self.owner.0 && n.end_byte() >= self.owner.1)
    }

    /// The declaration at byte `alias_start` runs before the point at byte `probe` on a structured route.
    pub(super) fn reaches(&self, alias_start: usize, probe: usize) -> bool {
        let root = self.parsed.tree.root_node();
        let Some(mut cur) = root.descendant_for_byte_range(alias_start, alias_start) else {
            return false;
        };
        // The declaration's own statement.
        while !matches!(cur.kind(), "lexical_declaration" | "variable_declaration") {
            match cur.parent() {
                Some(p) if self.strictly_inside(p) => cur = p,
                _ => return false,
            }
        }
        loop {
            let Some(list) = cur.parent().filter(|l| l.kind() == "statement_block") else {
                return false;
            };
            // An unconditional exit before `cur` in its list makes `cur` (and the alias, if it is inside) dead.
            let mut before = cur.prev_named_sibling();
            while let Some(b) = before {
                if ABRUPT.contains(&b.kind()) {
                    return false;
                }
                before = b.prev_named_sibling();
            }
            // The statements after `cur` in its list: `probe` in one of them, with no unconditional exit before it.
            let mut next = cur.next_named_sibling();
            while let Some(t) = next {
                if t.start_byte() <= probe && probe < t.end_byte() {
                    return true;
                }
                if ABRUPT.contains(&t.kind()) {
                    return false;
                }
                next = t.next_named_sibling();
            }
            // Leaving the list: the statement that holds it. Its remaining statements must not exit.
            let Some(g) = list.parent() else { return false };
            cur = match g.kind() {
                "statement_block" => list,
                "labeled_statement" | "if_statement" | "for_statement" | "for_in_statement"
                | "while_statement" | "try_statement" => g,
                "else_clause" | "catch_clause" | "finally_clause" => match g.parent() {
                    Some(p) => p,
                    None => return false,
                },
                _ => return false,
            };
            if !self.strictly_inside(cur) {
                return false;
            }
        }
    }
}
