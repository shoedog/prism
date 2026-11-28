//! Python `for`/`while`/`else` phases of the I1 route walk (lane 10, #847).
//!
//! A loop is walked as initializer-once, test or advance, target binding only on a successful
//! entry, body, and exit. A loop that does not enclose W is entered at most once (further
//! iterations can only add kills). A loop that encloses W is never entered again: body
//! completion and `continue` resume at the header, and only its exhaustion or false test leaves.
use super::python::kids;
use super::route::{truth, Flow, Phase, Truth, Walk};
use tree_sitter::Node;

/// What a `for` iterable literally holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Items {
    Empty,
    NonEmpty,
    Unknown,
}

fn items(n: Node) -> Items {
    let mut n = n;
    while n.kind() == "parenthesized_expression" {
        let named: Vec<_> = kids(n).into_iter().filter(|c| c.is_named()).collect();
        match named[..] {
            [only] => n = only,
            _ => break,
        }
    }
    if !matches!(n.kind(), "list" | "tuple" | "set" | "dictionary") {
        return Items::Unknown;
    }
    let named: Vec<_> = kids(n).into_iter().filter(|c| c.is_named()).collect();
    let splat = |c: &Node| matches!(c.kind(), "list_splat" | "dictionary_splat");
    match (named.is_empty(), named.iter().any(splat)) {
        (true, _) => Items::Empty,
        (_, true) => Items::Unknown,
        _ => Items::NonEmpty,
    }
}

impl<'a> Walk<'a> {
    /// The loop's `else` suite on exhaustion or a false test, else the plain exit.
    fn loop_else(&self, l: Node<'a>, t: super::route::Trace) -> Flow {
        let body = kids(l)
            .into_iter()
            .find(|c| c.kind() == "else_clause")
            .and_then(|e| e.child_by_field_name("body").map(|b| (e, b)));
        match body {
            Some((e, b)) => self.exec_block(b, t.push(Phase::LoopElse, e)),
            None => Flow::only(Some(t.push(Phase::Exit, l))),
        }
    }

    pub(super) fn exec_while(&self, n: Node<'a>, t: super::route::Trace) -> Flow {
        let (Some(cond), Some(body)) = (
            n.child_by_field_name("condition"),
            n.child_by_field_name("body"),
        ) else {
            return Flow::default();
        };
        let lit = truth(cond, self.src);
        let Some(s) = self.atom(cond, t.push(Phase::Test, cond)) else {
            return Flow::default();
        };
        let mut out = Flow::default();
        if lit != Truth::True {
            out.absorb(self.loop_else(n, s.push(Phase::Skip, n)));
        }
        if lit != Truth::False {
            let f = self.exec_block(body, s.push(Phase::Enter, n));
            out.absorb(self.loop_back(n, f, false));
        }
        out
    }

    pub(super) fn exec_for(&self, n: Node<'a>, t: super::route::Trace) -> Flow {
        let (Some(iter), Some(left), Some(body)) = (
            n.child_by_field_name("right"),
            n.child_by_field_name("left"),
            n.child_by_field_name("body"),
        ) else {
            return Flow::default();
        };
        let held = items(iter);
        let Some(s) = self.atom(iter, t.push(Phase::Test, iter)) else {
            return Flow::default();
        };
        let mut out = Flow::default();
        if held != Items::NonEmpty {
            out.absorb(self.loop_else(n, s.push(Phase::Skip, n)));
        }
        if held == Items::Empty {
            return out;
        }
        if self.tr.kills_target(left, self.src) {
            self.killed.set(true);
            return out;
        }
        let f = self.exec_block(body, s.push(Phase::Enter, n));
        out.absorb(self.loop_back(n, f, false));
        out
    }

    /// Resolve the jumps of one body execution at loop `l`, then resume at its header.
    /// `encloses` marks a loop around W: its re-entry is cut and recorded.
    pub(super) fn loop_back(&self, l: Node<'a>, mut f: Flow, encloses: bool) -> Flow {
        let mut out = Flow::default();
        if let Some(t) = f.brk.take() {
            let jump = t.last();
            out.normal = Some(t.push_to(Phase::Resolve, l, jump));
        }
        let cont = f.cont.take().map(|t| {
            let jump = t.last();
            t.push_to(Phase::Resolve, l, jump)
        });
        let Some(t) = f.normal.take().or(cont) else {
            return out;
        };
        let is_while = l.kind() == "while_statement";
        let cond = l.child_by_field_name("condition").filter(|_| is_while);
        let lit = cond.map_or(Truth::Unknown, |c| truth(c, self.src));
        if encloses && lit != Truth::False {
            self.cut.borrow_mut().push(super::python::span(l));
        }
        let header = match cond {
            Some(c) => self.atom(c, t.push(Phase::Test, c)),
            None => Some(t.push(Phase::Advance, l)),
        };
        if let (Some(s), true) = (header, lit != Truth::True) {
            out.absorb(self.loop_else(l, s.push(Phase::Exhaust, l)));
        }
        out
    }

    /// W may be in the loop's body (not when the test is never true or the iterable is empty)
    /// or in its `else` (not when a `while True` can only leave through `break`).
    pub(super) fn loop_part_reachable(&self, l: Node<'a>, in_else: bool) -> bool {
        if l.kind() == "while_statement" {
            let lit = l
                .child_by_field_name("condition")
                .map_or(Truth::Unknown, |c| truth(c, self.src));
            return lit != if in_else { Truth::True } else { Truth::False };
        }
        let held = l.child_by_field_name("right").map_or(Items::Unknown, items);
        in_else || held != Items::Empty
    }
}
