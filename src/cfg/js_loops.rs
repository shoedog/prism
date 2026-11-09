//! JS/TS/TSX loop exits (#693) and the `do`..`while` condition (#697).
//!
//! The sequential edge and the `if` branch edge join a statement to "the next
//! line in the sorted universe", so the last statement of a loop body also
//! reached the code after the loop without passing the loop header, and a
//! header write (a `for` update, a `while` condition) never killed a body def.
//! A `do` condition shared the `do` header's node, so the body's writes did not
//! reach it in order. For every loop whose whole shape is known (a rewired
//! loop, below) this module:
//!
//! * drops every main edge from the loop's lines to a line after it (the
//!   bypass, and the header's own exit, which is re-added from syntax) and
//!   every edge from a body line back to the loop's first line (main's back
//!   edge, re-added from syntax);
//! * adds body completion -> continue point (the header line; a `do`'s
//!   condition line), header -> `after(loop)`, and for a `do` condition ->
//!   body entry; a `for`..`in`/`of` body also completes into `after(loop)`
//!   (its head writes on entering an iteration, never on the exit);
//! * adds the edges main never had for `break`, labeled `break` and
//!   `continue`, from a jump inside a rewired loop to its target.
//!
//! A loop is rewired only when its continue point, its exit, its body's
//! normal completions and every loop nested in it are known; anything else
//! (a statement kind with no completion rule, an unadmitted `do`) keeps main's
//! edges for that loop and every loop around it. Targets come from syntax
//! (`after`), never from the sorted line list. Every edge added here is
//! `Structured`. `with_main_loop_cfg` turns the whole module and the JS `do`
//! condition admission off: the `PRISM_RD_CENSUS_TRACE` control pass.
use super::{CfgEdge, EdgeSink};
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

std::thread_local! {
    static MAIN_LOOP_CFG: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with JS/TS/TSX loops keeping main's CFG and statement universe. Used only by the
/// trace's control pass; production never sets it.
pub(crate) fn with_main_loop_cfg<T>(f: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            MAIN_LOOP_CFG.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(MAIN_LOOP_CFG.with(|c| c.replace(true)));
    f()
}

pub(crate) fn main_loop_cfg() -> bool {
    MAIN_LOOP_CFG.with(|c| c.get())
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

fn end_line(node: Node<'_>) -> usize {
    node.end_position().row + 1
}

fn is_loop(kind: &str) -> bool {
    matches!(
        kind,
        "for_statement" | "for_in_statement" | "while_statement" | "do_statement"
    )
}

/// A declaration that is no statement of the CFG's universe (a hoisted function, a class, a type):
/// it completes nowhere and is passed over when looking for the last or next statement.
fn is_skipped(kind: &str) -> bool {
    kind == "comment"
        || kind == "empty_statement"
        || (kind.ends_with("_declaration")
            && !matches!(kind, "lexical_declaration" | "variable_declaration"))
}

/// One loop of the owner and what rewiring needs of it.
struct Loop<'tree> {
    node: Node<'tree>,
    /// The continue point: the header line, or an admitted `do`'s condition line.
    head: usize,
    /// Whether the loop is rewired (see the module doc).
    rewired: bool,
    /// The body's normal completions, as lines.
    completions: Vec<usize>,
    /// The line control reaches when the loop ends normally.
    exit: Next,
    /// The first statement of the body (a `do`'s condition enters it).
    entry: Next,
}

struct Cx<'a, 'tree> {
    parsed: &'a ParsedFile,
    lines: &'a [usize],
    body: Node<'tree>,
    boundaries: &'a [&'static str],
}

pub(super) fn build(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize], edges: &mut EdgeSink) {
    if main_loop_cfg() {
        return;
    }
    let Some(body) = owner.child_by_field_name("body") else {
        return;
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let cx = Cx {
        parsed,
        lines,
        body,
        boundaries: &boundaries,
    };
    let (nodes, jumps) = cx.collect();
    if nodes.is_empty() {
        return;
    }
    let mut loops: Vec<Loop<'_>> = nodes.into_iter().map(|n| cx.analyze(n)).collect();
    // A loop is rewired only when every loop nested in it is: nested spans are contiguous in `loops`
    // (sorted by start byte), so one pass from the innermost end settles them.
    for i in (0..loops.len()).rev() {
        let end = loops[i].node.end_byte();
        let nested_ok = loops[i + 1..]
            .iter()
            .take_while(|inner| inner.node.start_byte() < end)
            .all(|inner| inner.rewired);
        loops[i].rewired &= nested_ok;
    }
    let ranges: Vec<(usize, usize)> = loops
        .iter()
        .filter(|l| l.rewired)
        .map(|l| (line(l.node), end_line(l.node)))
        .collect();
    if ranges.is_empty() {
        return;
    }
    // Drop main's routes: out of a rewired loop (or its header's exit) to a later line, and the back
    // edge from a body line to the first line (the new ones come from syntax).
    edges.retain(|e| {
        !ranges.iter().any(|&(s, t)| {
            (s <= e.from_line && e.from_line <= t && e.to_line > t)
                || (s < e.from_line && e.from_line <= t && e.to_line == s)
        })
    });
    let mut have: BTreeSet<(usize, usize)> = edges
        .edges
        .iter()
        .map(|e| (e.from_line, e.to_line))
        .collect();
    let mut emit = |from: usize, to: usize, edges: &mut EdgeSink| {
        if have.insert((from, to)) {
            edges.push(CfgEdge {
                file: parsed.path.clone(),
                from_line: from,
                to_line: to,
            });
        }
    };
    for l in loops.iter().filter(|l| l.rewired) {
        for &from in &l.completions {
            emit(from, l.head, edges);
            // A `for`..`in`/`of` head rebinds its target on entering an iteration, not when the
            // iterator is exhausted: the body's end leaves the loop with no header write on the way.
            if let (true, Next::Line(to)) = (l.node.kind() == "for_in_statement", &l.exit) {
                emit(from, *to, edges);
            }
        }
        if let Next::Line(to) = l.exit {
            emit(l.head, to, edges);
        }
        if let (true, Next::Line(first)) = (l.node.kind() == "do_statement", &l.entry) {
            emit(l.head, *first, edges);
        }
    }
    for jump in jumps {
        let inside_rewired = loops.iter().any(|l| {
            l.rewired
                && l.node.start_byte() <= jump.start_byte()
                && jump.end_byte() <= l.node.end_byte()
        });
        if !inside_rewired {
            continue;
        }
        let is_continue = jump.kind() == "continue_statement";
        let Some(target) = cx.target(jump, is_continue) else {
            continue;
        };
        let to = if is_continue {
            cx.continue_point(target)
        } else {
            match cx.after(target) {
                Next::Line(to) => Some(to),
                _ => None,
            }
        };
        if let Some(to) = to {
            emit(line(jump), to, edges);
        }
    }
}

impl<'a, 'tree> Cx<'a, 'tree> {
    /// One walk of the owner's body, never entering a nested callable: the loops and the
    /// `break`/`continue` statements, each in source order.
    fn collect(&self) -> (Vec<Node<'tree>>, Vec<Node<'tree>>) {
        let (mut loops, mut jumps) = (Vec::new(), Vec::new());
        let mut stack = vec![self.body];
        while let Some(n) = stack.pop() {
            let kind = n.kind();
            if self.boundaries.contains(&kind) {
                continue;
            }
            if is_loop(kind) {
                loops.push(n);
            } else if matches!(kind, "break_statement" | "continue_statement") {
                jumps.push(n);
            }
            let mut cursor = n.walk();
            let children: Vec<_> = n.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        loops.sort_by_key(|n| n.start_byte());
        jumps.sort_by_key(|n| n.start_byte());
        (loops, jumps)
    }

    /// The loop's continue point, exit, entry and completions, and whether all of them are known.
    fn analyze(&self, node: Node<'tree>) -> Loop<'tree> {
        let mut out = Loop {
            node,
            head: line(node),
            rewired: false,
            completions: Vec::new(),
            exit: Next::Unknown,
            entry: Next::Unknown,
        };
        let Some(body) = node.child_by_field_name("body") else {
            return out;
        };
        let head = if node.kind() == "do_statement" {
            match self.parsed.do_condition_line(&node) {
                Some(cond) => cond,
                None => return out,
            }
        } else {
            line(node)
        };
        out.head = head;
        out.exit = self.after(node);
        out.entry = self.enter(body);
        // Only a `do` condition needs the body's entry: the other loops' headers reach it already.
        let entry_known = node.kind() != "do_statement" || !matches!(out.entry, Next::Unknown);
        let known = self.lines.binary_search(&head).is_ok()
            && !matches!(out.exit, Next::Unknown)
            && entry_known
            && self.completions(body, &mut out.completions);
        out.rewired = known;
        out
    }

    /// The construct a `break` or `continue` binds to: the innermost loop (or, for `break`, `switch`),
    /// or the labeled statement its label names. Never crosses a callable.
    fn target(&self, jump: Node<'tree>, is_continue: bool) -> Option<Node<'tree>> {
        let label = jump
            .child_by_field_name("label")
            .map(|l| &self.parsed.source[l.start_byte()..l.end_byte()]);
        let mut at = jump.parent();
        while let Some(n) = at {
            if n.id() == self.body.id() || self.boundaries.contains(&n.kind()) {
                return None;
            }
            match label {
                None if is_loop(n.kind()) || (!is_continue && n.kind() == "switch_statement") => {
                    return Some(n);
                }
                Some(want) if n.kind() == "labeled_statement" => {
                    let name = n.child_by_field_name("label")?;
                    if &self.parsed.source[name.start_byte()..name.end_byte()] == want {
                        // `continue label` needs a loop under the label.
                        return if is_continue {
                            n.child_by_field_name("body").filter(|b| is_loop(b.kind()))
                        } else {
                            Some(n)
                        };
                    }
                }
                _ => {}
            }
            at = n.parent();
        }
        None
    }

    /// Where a `continue` of loop `target` resumes: the header line, or an admitted `do`'s condition.
    fn continue_point(&self, target: Node<'tree>) -> Option<usize> {
        if target.kind() == "do_statement" {
            self.parsed.do_condition_line(&target)
        } else {
            Some(line(target))
        }
    }

    /// The statements whose normal completion ends `node`, by explicit rule: an atomic statement
    /// completes at its own line; the last statement of a block or case, both arms of an `if` (the
    /// `if` itself without `else`), a loop header (its false exit), a nested `do`'s condition, a
    /// labeled statement's inner statement, `try` through its `finally` (else its body and `catch`),
    /// a `switch` through its last case and, without `default`, its header. Jumps complete nowhere.
    /// Returns false for a kind with no rule, so the caller keeps main's edges.
    fn completions(&self, node: Node<'tree>, out: &mut Vec<usize>) -> bool {
        match node.kind() {
            "statement_block" => match Self::last_statement(node) {
                Some(last) => self.completions(last, out),
                None => true,
            },
            "if_statement" => {
                let arm = match node.child_by_field_name("alternative") {
                    Some(alt) => match Self::last_statement(alt) {
                        Some(inner) => self.completions(inner, out),
                        None => false,
                    },
                    None => {
                        out.push(line(node));
                        true
                    }
                };
                arm && node
                    .child_by_field_name("consequence")
                    .is_some_and(|then| self.completions(then, out))
            }
            "labeled_statement" | "with_statement" => node
                .child_by_field_name("body")
                .is_some_and(|inner| self.completions(inner, out)),
            "return_statement" | "throw_statement" | "break_statement" | "continue_statement" => {
                true
            }
            "try_statement" => {
                if let Some(finalizer) = node.child_by_field_name("finalizer") {
                    return finalizer
                        .child_by_field_name("body")
                        .is_some_and(|b| self.completions(b, out));
                }
                let body_ok = node
                    .child_by_field_name("body")
                    .is_some_and(|b| self.completions(b, out));
                let handler_ok = node.child_by_field_name("handler").is_none_or(|h| {
                    h.child_by_field_name("body")
                        .is_some_and(|b| self.completions(b, out))
                });
                body_ok && handler_ok
            }
            "switch_statement" => {
                let Some(body) = node.child_by_field_name("body") else {
                    return false;
                };
                let mut cursor = body.walk();
                let cases: Vec<_> = body.named_children(&mut cursor).collect();
                let has_default = cases.iter().any(|c| c.kind() == "switch_default");
                if !has_default {
                    out.push(line(node));
                }
                match cases.iter().rev().find(|c| c.kind() != "comment") {
                    Some(last) => match Self::last_statement(*last) {
                        Some(inner) => self.completions(inner, out),
                        None => {
                            out.push(line(node));
                            true
                        }
                    },
                    None => {
                        out.push(line(node));
                        true
                    }
                }
            }
            "while_statement" | "for_statement" | "for_in_statement"
                if self.lines.binary_search(&line(node)).is_ok() =>
            {
                out.push(line(node));
                true
            }
            "do_statement" => match self.parsed.do_condition_line(&node) {
                Some(cond) => {
                    out.push(cond);
                    true
                }
                None => false,
            },
            "expression_statement"
            | "lexical_declaration"
            | "variable_declaration"
            | "debugger_statement"
                if self.lines.binary_search(&line(node)).is_ok() =>
            {
                out.push(line(node));
                true
            }
            _ => false,
        }
    }

    /// The statements of a block, `else` clause or `case`: named children that are not the case's
    /// `value`, comments or declarations outside the universe.
    fn statements(node: Node<'tree>) -> Vec<Node<'tree>> {
        let value = node.child_by_field_name("value").map(|v| v.id());
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|c| !is_skipped(c.kind()) && Some(c.id()) != value)
            .collect()
    }

    fn last_statement(node: Node<'tree>) -> Option<Node<'tree>> {
        Self::statements(node).pop()
    }

    fn next_statement(node: Node<'tree>) -> Option<Node<'tree>> {
        let mut next = node.next_named_sibling();
        while next.is_some_and(|n| is_skipped(n.kind())) {
            next = next.and_then(|n| n.next_named_sibling());
        }
        next
    }

    /// The statement line control reaches on entering `node`; an empty block or case enters what
    /// follows it.
    fn enter(&self, node: Node<'tree>) -> Next {
        match node.kind() {
            "statement_block" | "switch_case" | "switch_default" => {
                match Self::statements(node).first() {
                    Some(first) => self.enter(*first),
                    None => self.after(node),
                }
            }
            _ if self.lines.binary_search(&line(node)).is_ok() => Next::Line(line(node)),
            _ => Next::Unknown,
        }
    }

    /// The line control reaches when `node` completes normally: the next statement; at the end of a
    /// loop body that loop's continue point; out of an arm, the construct's own exit. Function-final
    /// is `Exit`.
    fn after(&self, mut node: Node<'tree>) -> Next {
        loop {
            if node.id() == self.body.id() {
                return Next::Exit;
            }
            let Some(parent) = node.parent() else {
                return Next::Unknown;
            };
            match parent.kind() {
                "statement_block" | "switch_case" | "switch_default" | "switch_body" => {
                    if let Some(next) = Self::next_statement(node) {
                        return self.enter(next);
                    }
                }
                "for_statement" | "for_in_statement" | "while_statement" => {
                    let is_body = parent
                        .child_by_field_name("body")
                        .is_some_and(|b| b.id() == node.id());
                    return if is_body {
                        Next::Line(line(parent))
                    } else {
                        Next::Unknown
                    };
                }
                "do_statement" => {
                    return self
                        .parsed
                        .do_condition_line(&parent)
                        .map_or(Next::Unknown, Next::Line);
                }
                "try_statement" => {
                    let is_block = parent
                        .child_by_field_name("body")
                        .is_some_and(|b| b.id() == node.id())
                        || parent
                            .child_by_field_name("handler")
                            .is_some_and(|h| h.id() == node.id());
                    if let (true, Some(finalizer)) =
                        (is_block, parent.child_by_field_name("finalizer"))
                    {
                        return match finalizer.child_by_field_name("body") {
                            Some(body) => self.enter(body),
                            None => Next::Unknown,
                        };
                    }
                }
                "if_statement" | "else_clause" | "labeled_statement" | "switch_statement"
                | "with_statement" | "catch_clause" | "finally_clause" => {}
                _ => return Next::Unknown,
            }
            node = parent;
        }
    }
}

#[cfg(test)]
#[path = "js_loops_tests.rs"]
mod tests;
