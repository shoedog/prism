//! C/C++ unlabeled `break`/`continue`, `switch` dispatch and the `do`..`while`
//! condition (#645; the Go counterpart is `go_jumps.rs`).
//!
//! `is_terminator` stops the sequential edge after a jump and nothing replaced
//! it, so defs before a jump reached nothing after it. Every edge here is
//! `Structured`: only the sequential loop may invent a join between two arms.
//! Targets are computed from syntax (`after`), never by scanning the sorted
//! line list, so a terminal statement resumes its loop or leaves its arm
//! without entering a sibling arm. Nested callables (lambdas, local functions)
//! are never entered: a jump cannot cross them.
use super::{CfgEdge, EdgeSink};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// What the builder saw, for the measurement report; `build` ignores it.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) struct CJumpStats {
    pub jump_edges: usize,
    /// Emitted jump edges whose source line holds more than one statement (#549).
    pub jump_edges_on_multi_statement_lines: usize,
    pub dispatch_switches: usize,
    /// Switches with a `case` outside the body's direct children: left to main.
    pub duff_switches: usize,
    pub rewired_do_loops: usize,
    /// `do`..`while` whose condition shares a line with the header or body: left to main.
    pub one_line_do_loops: usize,
    /// Rewired `do` loops whose legacy body-last → header back edge was dropped.
    pub do_back_edges_removed: usize,
}

enum Next {
    Line(usize),
    /// Control leaves the function.
    Exit,
    Unknown,
}

fn line(node: Node<'_>) -> usize {
    node.start_position().row + 1
}

use crate::ast::is_c_cpp_statement_kind as is_statement;

fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_statement" | "while_statement" | "do_statement" | "for_range_loop"
    )
}

/// Totals over a file's functions, for the measurement report.
#[cfg(test)]
pub(crate) fn c_jump_stats(parsed: &ParsedFile) -> CJumpStats {
    use crate::languages::Language;
    let mut total = CJumpStats::default();
    if !matches!(parsed.language, Language::C | Language::Cpp) {
        return total;
    }
    for owner in parsed.all_functions() {
        let lines: Vec<usize> = parsed
            .statements_in_function(&owner)
            .iter()
            .map(|(l, _)| *l)
            .collect();
        let s = stats(owner, parsed, &lines);
        total.jump_edges += s.jump_edges;
        total.jump_edges_on_multi_statement_lines += s.jump_edges_on_multi_statement_lines;
        total.dispatch_switches += s.dispatch_switches;
        total.duff_switches += s.duff_switches;
        total.rewired_do_loops += s.rewired_do_loops;
        total.one_line_do_loops += s.one_line_do_loops;
        total.do_back_edges_removed += s.do_back_edges_removed;
    }
    total
}

pub(super) fn build(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize], edges: &mut EdgeSink) {
    scan(owner, parsed, lines, edges);
}

#[cfg(test)]
/// Run the builder into a scratch sink and report what it did.
pub(super) fn stats(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize]) -> CJumpStats {
    scan(owner, parsed, lines, &mut EdgeSink::new())
}

fn scan(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize], edges: &mut EdgeSink) -> CJumpStats {
    let mut stats = CJumpStats::default();
    let Some(body) = owner.child_by_field_name("body") else {
        return stats;
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut cx = Cx {
        parsed,
        lines,
        body,
        boundaries: &boundaries,
    };
    let (jumps, switches, dos, per_line) = cx.collect();
    let mut emit = |from: Node<'_>, to: usize, stats: &mut CJumpStats, jump: bool| {
        edges.push(CfgEdge {
            file: parsed.path.clone(),
            from_line: line(from),
            to_line: to,
        });
        if jump {
            stats.jump_edges += 1;
            if per_line.get(&line(from)).copied().unwrap_or(0) > 1 {
                stats.jump_edges_on_multi_statement_lines += 1;
            }
        }
    };
    for jump in jumps {
        let is_continue = jump.kind() == "continue_statement";
        let Some(target) = cx.enclosing(jump, is_continue) else {
            continue;
        };
        let to = if is_continue {
            do_aware_continue(target, parsed)
        } else {
            match cx.after(target) {
                Next::Line(to) => Some(to),
                _ => None,
            }
        };
        if let Some(to) = to {
            emit(jump, to, &mut stats, true);
        }
    }
    for switch in switches {
        match cx.dispatch(switch) {
            Dispatch::Targets(targets) => {
                stats.dispatch_switches += 1;
                for to in targets.into_iter().filter(|&to| to != line(switch)) {
                    emit(switch, to, &mut stats, false);
                }
            }
            Dispatch::Duff => stats.duff_switches += 1,
            Dispatch::None => {}
        }
    }
    for node in dos {
        let Some(cond) = do_condition_line(node, parsed) else {
            stats.one_line_do_loops += 1;
            continue;
        };
        stats.rewired_do_loops += 1;
        if do_back_edge_removed(node, parsed, lines) {
            stats.do_back_edges_removed += 1;
        }
        let cond_node = node.child_by_field_name("condition").unwrap_or(node);
        if let Some(Next::Line(first)) = node.child_by_field_name("body").map(|b| cx.enter(b)) {
            emit(cond_node, first, &mut stats, false);
        }
        // Normal completion of the body reaches the condition: the sorted
        // sequential edge supplies it only for a plain last statement.
        if let Some(body) = node.child_by_field_name("body") {
            let mut ends = Vec::new();
            cx.completions(body, &mut ends);
            for from in ends {
                if line(from) != cond {
                    emit(from, cond, &mut stats, false);
                }
            }
        }
        if let Next::Line(exit) = cx.after(node) {
            if exit != cond {
                emit(cond_node, exit, &mut stats, false);
            }
        }
    }
    stats
}

/// The `continue` target of a loop: header line, or a `do`'s condition line.
/// A `do` whose condition shares a line is left to main (no edge).
fn do_aware_continue(target: Node<'_>, parsed: &ParsedFile) -> Option<usize> {
    if target.kind() == "do_statement" {
        do_condition_line(target, parsed)
    } else {
        Some(line(target))
    }
}

/// The admitted condition line of a `do`..`while` (see
/// `ParsedFile::c_do_condition_line`); `None` leaves main's edges standing.
pub(super) fn do_condition_line(node: Node<'_>, parsed: &ParsedFile) -> Option<usize> {
    parsed.c_do_condition_line(&node)
}

/// Whether the legacy body-last → `do` header back edge must go: the loop is
/// rewired and the condition → first-statement edge is emitted, so the body
/// completes through the condition. Without that edge the back edge stays.
pub(super) fn do_back_edge_removed(node: Node<'_>, parsed: &ParsedFile, lines: &[usize]) -> bool {
    let Some(body) = node.child_by_field_name("body") else {
        return false;
    };
    if do_condition_line(node, parsed).is_none() {
        return false;
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    let cx = Cx {
        parsed,
        lines,
        body: node,
        boundaries: &boundaries,
    };
    matches!(cx.enter(body), Next::Line(_))
}

enum Dispatch {
    Targets(Vec<usize>),
    Duff,
    None,
}

struct Cx<'a, 'tree> {
    parsed: &'a ParsedFile,
    lines: &'a [usize],
    body: Node<'tree>,
    boundaries: &'a [&'static str],
}

type Collected<'tree> = (
    Vec<Node<'tree>>,
    Vec<Node<'tree>>,
    Vec<Node<'tree>>,
    BTreeMap<usize, usize>,
);

impl<'tree> Cx<'_, 'tree> {
    /// One walk of the owner's body, never entering a nested callable.
    fn collect(&mut self) -> Collected<'tree> {
        let (mut jumps, mut switches, mut dos) = (Vec::new(), Vec::new(), Vec::new());
        let mut per_line = BTreeMap::new();
        let mut stack = vec![self.body];
        while let Some(n) = stack.pop() {
            let kind = n.kind();
            if self.boundaries.contains(&kind) {
                continue;
            }
            match kind {
                "break_statement" | "continue_statement" => jumps.push(n),
                "switch_statement" => switches.push(n),
                "do_statement" => dos.push(n),
                _ => {}
            }
            if is_statement(kind) {
                *per_line.entry(line(n)).or_insert(0) += 1;
            }
            let mut cursor = n.walk();
            let children: Vec<_> = n.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        jumps.sort_by_key(|n| n.start_byte());
        (jumps, switches, dos, per_line)
    }

    /// Innermost enclosing construct a jump binds to; a `switch` is skipped by
    /// `continue`. Never crosses a callable boundary.
    fn enclosing(&self, jump: Node<'tree>, is_continue: bool) -> Option<Node<'tree>> {
        let mut at = jump.parent();
        while let Some(n) = at {
            if n.id() == self.body.id() || self.boundaries.contains(&n.kind()) {
                return None;
            }
            if is_loop(n.kind()) || (!is_continue && n.kind() == "switch_statement") {
                return Some(n);
            }
            at = n.parent();
        }
        None
    }

    /// Header line to every `case`'s first statement; without a `default` also
    /// to the switch exit. Cases outside the body's direct children (Duff's
    /// device) give no dispatch.
    fn dispatch(&self, switch: Node<'tree>) -> Dispatch {
        let Some(body) = switch.child_by_field_name("body") else {
            return Dispatch::None;
        };
        let mut cases = Vec::new();
        let mut stack = vec![body];
        while let Some(n) = stack.pop() {
            let mut cursor = n.walk();
            for child in n.named_children(&mut cursor) {
                match child.kind() {
                    "switch_statement" => {}
                    k if self.boundaries.contains(&k) => {}
                    "case_statement" => {
                        cases.push(child);
                        stack.push(child);
                    }
                    _ => stack.push(child),
                }
            }
        }
        if cases.is_empty() {
            return Dispatch::None;
        }
        if cases
            .iter()
            .any(|c| c.parent().is_none_or(|p| p.id() != body.id()))
        {
            return Dispatch::Duff;
        }
        cases.sort_by_key(|c| c.start_byte());
        let mut targets = Vec::new();
        for case in &cases {
            if let Next::Line(to) = self.enter(*case) {
                targets.push(to);
            }
        }
        if cases
            .iter()
            .all(|c| c.child_by_field_name("value").is_some())
        {
            if let Next::Line(to) = self.after(switch) {
                targets.push(to);
            }
        }
        Dispatch::Targets(targets)
    }

    /// Statements whose normal completion ends `node`: the last statement of a
    /// block, both arms of an `if` (the `if` itself when it has no `else`), a
    /// loop header (its false exit), the last case of a `switch` and a
    /// `switch` without `default`. Jumps are not completions.
    fn completions(&self, node: Node<'tree>, out: &mut Vec<Node<'tree>>) {
        let last_child = |n: Node<'tree>| {
            let mut cursor = n.walk();
            let children: Vec<_> = n.named_children(&mut cursor).collect();
            children.into_iter().rev().find(|c| c.kind() != "comment")
        };
        match node.kind() {
            "compound_statement" | "case_statement" => {
                if let Some(last) = last_child(node) {
                    self.completions(last, out);
                }
            }
            "labeled_statement" => {
                if let Some(last) = last_child(node) {
                    self.completions(last, out);
                }
            }
            "if_statement" => {
                match node.child_by_field_name("alternative") {
                    Some(alt) => {
                        if let Some(inner) = last_child(alt) {
                            self.completions(inner, out);
                        }
                    }
                    None => out.push(node),
                }
                if let Some(then) = node.child_by_field_name("consequence") {
                    self.completions(then, out);
                }
            }
            "switch_statement" => {
                out.push(node);
                if let Some(body) = node.child_by_field_name("body") {
                    if let Some(last) = last_child(body) {
                        self.completions(last, out);
                    }
                }
            }
            "break_statement" | "continue_statement" | "return_statement" | "goto_statement"
            | "throw_statement" => {}
            "do_statement" => {
                if let Some(cond) = node.child_by_field_name("condition") {
                    out.push(cond);
                }
            }
            _ if self.lines.binary_search(&line(node)).is_ok() => out.push(node),
            _ => {}
        }
    }

    fn next_statement(node: Node<'tree>) -> Option<Node<'tree>> {
        let mut next = node.next_named_sibling();
        while next.is_some_and(|n| n.kind() == "comment") {
            next = next.and_then(|n| n.next_named_sibling());
        }
        next
    }

    /// The statement line control reaches on entering `node`. An empty block
    /// or case enters whatever follows it, as fallthrough would.
    fn enter(&self, node: Node<'tree>) -> Next {
        match node.kind() {
            "compound_statement" | "case_statement" => {
                let value = node.child_by_field_name("value").map(|v| v.id());
                let mut cursor = node.walk();
                let first = node
                    .named_children(&mut cursor)
                    .find(|c| c.kind() != "comment" && Some(c.id()) != value);
                match first {
                    Some(first) => self.enter(first),
                    None => self.after(node),
                }
            }
            _ if self.lines.binary_search(&line(node)).is_ok() => Next::Line(line(node)),
            _ => Next::Unknown,
        }
    }

    /// The line control reaches when `node` completes normally: the next
    /// statement; at the end of a loop body that loop's continue target; out of
    /// an arm, the construct's own exit. Function-final is `Exit`.
    fn after(&self, mut node: Node<'tree>) -> Next {
        loop {
            if node.id() == self.body.id() {
                return Next::Exit;
            }
            let Some(parent) = node.parent() else {
                return Next::Unknown;
            };
            match parent.kind() {
                "compound_statement" | "case_statement" => {
                    if let Some(next) = Self::next_statement(node) {
                        return self.enter(next);
                    }
                }
                "for_statement" | "while_statement" | "for_range_loop" => {
                    return Next::Line(line(parent));
                }
                "do_statement" => {
                    return do_condition_line(parent, self.parsed)
                        .map_or(Next::Unknown, Next::Line);
                }
                "switch_statement" | "labeled_statement" | "if_statement" | "else_clause" => {}
                _ => return Next::Unknown,
            }
            node = parent;
        }
    }
}

#[cfg(test)]
#[path = "c_jumps_tests.rs"]
mod tests;
