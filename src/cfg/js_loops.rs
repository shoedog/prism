//! JS/TS/TSX loop exits (#693) and the `do`..`while` condition (#697).
//!
//! The sequential edge and the `if` branch edge join a statement to "the next
//! line in the sorted universe", so the last statement of a loop body also
//! reached the code after the loop without passing the loop header, and a
//! header write (a `for` update, a `while` condition) never killed a body def.
//! A `do` condition shared the `do` header's node, so the body's writes did not
//! reach it in order.
//!
//! The unit is the loop TREE: an outermost loop of the owner (never inside a
//! nested callable) with every loop nested in it. Which trees are rewired is
//! decided ONCE per file by the plan (`plan`, #693/#697 option B slice 2): a
//! tree is admitted only when main's own routing below accepts it, the placement
//! rule holds (`placement`) and no CFG record intersecting it is unresolved
//! (`js_disposition`); a tree the plan refuses keeps main's edges and universe
//! and its OWNER is refused whole: it keeps main's universe, edges, labels and availability exactly.
//! Main's own routing: a tree is REWIRED, all of it
//! or none of it, when it holds no form this module cannot route soundly:
//!
//! * every loop's continue point (the header line; an admitted `do`'s
//!   condition line), exit (`after`, from syntax, never the sorted line list),
//!   body completions (an explicit rule list) and `do` body entry are known;
//! * no `break`/`continue` leaves a `try` body or `catch` whose `finally` does
//!   not enclose the jump's target (a direct edge would skip the `finally`'s
//!   writes: a wrong `exact`);
//! * no `for` initializer writes a name that the loop's body writes and that
//!   neither its test nor its update writes (the initializer runs once but
//!   shares the header node, so a back route would re-run its kill).
//!
//! In a rewired tree every main edge the loops own is dropped: edges from a
//! loop's interior lines to a later line (the bypass), edges from an interior
//! line back to the loop's first line (main's back edge), and one copy of the
//! header's own exit. The edges are added from syntax: body completion ->
//! continue point, header -> `after(loop)` (a nested loop exits to the outer
//! continue point), a `do` condition -> body entry, and for `break`, labeled
//! `break`, `continue` and labeled `continue` an edge to the target (main had
//! none). A `for`..`in`/`of` head writes on entering an iteration, never when
//! the iterator is exhausted, so a body end and a `continue` also reach
//! `after(loop)` directly. A tree that is not rewired keeps main's edges and
//! main's statement universe, its `do` conditions included
//! (`do_tree_rewirable` is the admission predicate of the `do` condition
//! statement). Every edge added here is `Structured`. `with_main_loop_cfg`
//! turns the module and the JS `do` condition admission off: the
//! `PRISM_RD_CENSUS_TRACE` control pass.
use super::js_ownership::{Emit, Producer, Role};
use super::{CfgEdge, EdgeSink};
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

mod facts;
mod for_init;
mod placement;
mod plan;
mod replace;
mod tryflow;
pub(super) use facts::{tree_facts, LoopFacts, TreeFacts};
pub(super) use plan::any_admitted_within as plan_any_admitted_within;
#[cfg(test)]
pub(super) use plan::legacy_candidates;
pub(crate) use plan::LoopPlan;
#[cfg(test)]
pub(crate) use plan::Why;
#[cfg(test)]
pub(super) use plan::{ownerships, with_admission};
pub(crate) use replace::wrapper_resolvable;

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

/// Whether `text` holds `for`, `while` or `do` as a whole word: every loop statement kind starts with one of these
/// keywords, so text without them holds no loop. Conservative (comments and strings count; a byte of a multi-byte
/// character is no identifier character, so it never hides a keyword) and a pure speed gate for `Cx::outermost`.
fn may_hold_loop(text: &[u8]) -> bool {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'$';
    let at = |i: usize, kw: &[u8]| {
        text[i..].starts_with(kw)
            && (i == 0 || !word(text[i - 1]))
            && text.get(i + kw.len()).is_none_or(|b| !word(*b))
    };
    (0..text.len()).any(|i| match text[i] {
        b'f' => at(i, b"for"),
        b'w' => at(i, b"while"),
        b'd' => at(i, b"do"),
        _ => false,
    })
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

/// A normal completion of a body: the line it leaves, and the construct that completes (the statement,
/// the `if` without `else`, the loop whose header exits, the `do` whose condition exits, or the
/// construct that entered an empty arm).
#[derive(Clone, Copy)]
struct Comp<'tree> {
    line: usize,
    node: Node<'tree>,
}

impl<'tree> Comp<'tree> {
    fn at(node: Node<'tree>) -> Self {
        Self {
            line: line(node),
            node,
        }
    }
}

/// One loop of a tree and what rewiring needs of it.
struct Loop<'tree> {
    node: Node<'tree>,
    /// The continue point: the header line, or an admitted `do`'s condition line.
    head: usize,
    /// The body's normal completions.
    completions: Vec<Comp<'tree>>,
    /// The line control reaches when the loop ends normally.
    exit: Next,
    /// The first statement of the body (a `do` condition enters it).
    entry: Next,
}

/// Why main's policy leaves a loop tree unrewired (the fallbacks that predate the plan's checks).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Legacy {
    /// A `for` initializer writes a name the body writes and neither test nor update does.
    ForInit,
    /// A `break`/`continue` leaves a `try` body or `catch` whose `finally` does not enclose its target.
    Finally,
    /// A continue point, exit, entry or body completion is unknown (or a `do` condition shares a line).
    Unrouted,
}

/// A rewired loop tree: its loops (source order) and the jumps inside it.
struct Tree<'tree> {
    loops: Vec<Loop<'tree>>,
    jumps: Vec<Node<'tree>>,
}

struct Cx<'a, 'tree> {
    parsed: &'a ParsedFile,
    body: Node<'tree>,
    boundaries: &'a [&'static str],
}

/// Whether the loop tree holding `do_node` is admitted by the plan (`plan`): the admission predicate of a
/// JS-family `do` condition as a CFG statement (an admitted condition in an unrewired tree would leave
/// the `do` header an empty node that main's exit edge bypasses the condition from). It reads syntax
/// only: the tree's outermost loop inside the owning callable, looked up in the one immutable plan.
pub(crate) fn do_tree_rewirable(parsed: &ParsedFile, do_node: &Node<'_>) -> bool {
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut outer = *do_node;
    let mut at = do_node.parent();
    while let Some(n) = at {
        if boundaries.contains(&n.kind()) {
            return plan::admits(parsed, (outer.start_byte(), outer.end_byte()));
        }
        if is_loop(n.kind()) {
            outer = n;
        }
        at = n.parent();
    }
    false
}

pub(super) fn build<'p>(owner: Node<'p>, parsed: &'p ParsedFile, edges: &mut EdgeSink) {
    build_inner(owner, parsed, edges);
    replace::trace_emitted(parsed, owner, edges);
}

fn build_inner<'p>(owner: Node<'p>, parsed: &'p ParsedFile, edges: &mut EdgeSink) {
    if main_loop_cfg() {
        return;
    }
    let Some(body) = owner.child_by_field_name("body") else {
        return;
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let cx = Cx {
        parsed,
        body,
        boundaries: &boundaries,
    };
    #[cfg(test)]
    edges.mark_pre_rewrite();
    // A refused owner (one of its trees is refused, so all are) keeps main's geometry exactly: no rewrite, no
    // delegation, no drop of a nested admitted tree's copies (final refusal policy).
    let roots = cx.outermost();
    if roots
        .iter()
        .any(|t| !plan::admits(parsed, (t.start_byte(), t.end_byte())))
    {
        return;
    }
    // The enclosing owner's copies of what a nested callable's admitted tree owns go first.
    replace::drop_nested_copies(owner, parsed, edges);
    let trees: Vec<Tree<'_>> = roots
        .into_iter()
        .filter(|t| plan::admits(parsed, (t.start_byte(), t.end_byte())))
        .filter_map(|t| cx.tree(t))
        .collect();
    if trees.is_empty() {
        return;
    }
    // Take out the records an admitted tree of this owner owns (back edges, exits, body completions that
    // bypass the loop, delegated then-ends), by record, and nothing else.
    let Some(drops) = replace::own_drops(owner, parsed, edges) else {
        return;
    };
    replace::trace_drops(
        parsed,
        owner,
        edges.live_records().unwrap_or(&[]),
        &drops.keep,
        "own",
    );
    edges.retain_flags(&drops.keep);
    let mut have: BTreeSet<(usize, usize)> = edges
        .edges
        .iter()
        .map(|e| (e.from_line, e.to_line))
        .collect();
    let mut emit = |from: usize, to: usize, edges: &mut EdgeSink, why: Emit<'_>| {
        let edge = CfgEdge {
            file: parsed.path.clone(),
            from_line: from,
            to_line: to,
        };
        if have.insert((from, to)) {
            edges.push_rec(edge, why);
        } else {
            edges.suppress(&edge, why);
        }
    };
    for tree in &trees {
        let root = tree.loops[0].node;
        for l in &tree.loops {
            for c in &l.completions {
                let repeat = Emit::new(Producer::JsLoops, Role::Repetition, l.node)
                    .from(c.node)
                    .to(Some(l.node))
                    .depends_on(l.node)
                    .tree(root);
                emit(c.line, l.head, edges, repeat);
                // A `for`..`in`/`of` head rebinds its target on entering an iteration, not when the
                // iterator is exhausted: the body's end leaves the loop with no header write.
                if let (true, Next::Line(to)) = (l.node.kind() == "for_in_statement", &l.exit) {
                    let why = Emit::new(Producer::JsLoops, Role::LoopExit, l.node)
                        .from(c.node)
                        .depends_on(l.node)
                        .tree(root);
                    emit(c.line, *to, edges, why);
                }
            }
            if let Next::Line(to) = l.exit {
                let why = Emit::new(Producer::JsLoops, Role::LoopExit, l.node)
                    .from(l.node)
                    .depends_on(l.node)
                    .tree(root);
                emit(l.head, to, edges, why);
            }
            if let (true, Next::Line(first)) = (l.node.kind() == "do_statement", &l.entry) {
                let why = Emit::new(Producer::JsLoops, Role::Repetition, l.node)
                    .from(l.node)
                    .to(l.node.child_by_field_name("body"))
                    .depends_on(l.node)
                    .tree(root);
                emit(l.head, *first, edges, why);
            }
        }
        for &jump in &tree.jumps {
            let is_continue = jump.kind() == "continue_statement";
            let Some((target, _)) = cx.target(jump, is_continue) else {
                continue;
            };
            let jumps = |role: Role| {
                Emit::new(Producer::Jump, role, jump)
                    .from(jump)
                    .depends_on(target)
                    .tree(root)
            };
            if is_continue {
                if let Some(to) = cx.continue_point(target) {
                    emit(
                        line(jump),
                        to,
                        edges,
                        jumps(Role::Continue).to(Some(target)),
                    );
                }
                // Exhaustion after a `continue` performs no head write either.
                if target.kind() == "for_in_statement" {
                    if let Next::Line(to) = cx.after(target) {
                        emit(line(jump), to, edges, jumps(Role::Continue));
                    }
                }
            } else if let Next::Line(to) = cx.after(target) {
                emit(line(jump), to, edges, jumps(Role::Break));
            }
        }
    }
    // A delegated then-end (a wrapper ends an `if` arm and holds, or leaves, a loop of a tree) is replaced
    // by the wrapper's own completions reaching what follows the arm.
    for (wrapper, root) in &drops.then_ends {
        let Some(w) = replace::node_of(parsed, *wrapper) else {
            continue;
        };
        let Some(tree) = trees.iter().find(|t| {
            let n = t.loops[0].node;
            n.start_byte() == root.start && n.end_byte() == root.end
        }) else {
            continue;
        };
        let Some(delegated) = cx.wrapper_completion(w, (root.start, root.end)) else {
            continue;
        };
        replace::trace_inbound(parsed, owner, edges, w, &delegated);
        for c in &delegated.completions {
            let why = Emit::new(Producer::JsLoops, Role::NormalCompletion, w)
                .from(c.node)
                .depends_on(w)
                .tree(tree.loops[0].node);
            emit(c.line, delegated.to, edges, why);
        }
        for &(jump, to) in &delegated.jumps {
            let why = Emit::new(Producer::Jump, Role::Break, jump)
                .from(jump)
                .depends_on(w)
                .tree(tree.loops[0].node);
            emit(line(jump), to, edges, why);
        }
    }
}

impl<'a, 'tree> Cx<'a, 'tree> {
    /// The owner's outermost loops (never inside a nested callable), in source order.
    fn outermost(&self) -> Vec<Node<'tree>> {
        let mut out = Vec::new();
        // A body with a parse error is walked whatever its text: error recovery can make a loop of text
        // (`1while (c) {}`) the keyword gate would not see.
        if !self.body.has_error()
            && !may_hold_loop(&self.parsed.source.as_bytes()[self.body.byte_range()])
        {
            return out;
        }
        let mut stack = vec![self.body];
        while let Some(n) = stack.pop() {
            let kind = n.kind();
            if self.boundaries.contains(&kind) {
                continue;
            }
            if is_loop(kind) {
                out.push(n);
                continue;
            }
            let mut cursor = n.walk();
            let children: Vec<_> = n.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    /// The loop tree rooted at `root`, or `None` when it is not rewired (see the module doc).
    fn tree(&self, root: Node<'tree>) -> Option<Tree<'tree>> {
        self.tree_checked(root).ok()
    }

    /// `tree`, naming the main-policy fallback that refused it.
    fn tree_checked(&self, root: Node<'tree>) -> Result<Tree<'tree>, Legacy> {
        let (mut nodes, mut jumps) = (Vec::new(), Vec::new());
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            let kind = n.kind();
            if n.id() != root.id() && self.boundaries.contains(&kind) {
                continue;
            }
            if is_loop(kind) {
                nodes.push(n);
            } else if matches!(kind, "break_statement" | "continue_statement") {
                jumps.push(n);
            }
            let mut cursor = n.walk();
            let children: Vec<_> = n.named_children(&mut cursor).collect();
            stack.extend(children.into_iter().rev());
        }
        nodes.sort_by_key(|n| n.start_byte());
        jumps.sort_by_key(|n| n.start_byte());
        let loops = nodes
            .into_iter()
            .map(|n| self.analyze(n))
            .collect::<Result<Vec<_>, _>>()?;
        for &jump in &jumps {
            let is_continue = jump.kind() == "continue_statement";
            if matches!(self.target(jump, is_continue), Some((_, true))) {
                return Err(Legacy::Finally);
            }
        }
        Ok(Tree { loops, jumps })
    }

    /// The loop's continue point, exit, entry and completions, or `None` when one is unknown or the
    /// loop holds a form this module does not route.
    fn analyze(&self, node: Node<'tree>) -> Result<Loop<'tree>, Legacy> {
        let body = node.child_by_field_name("body").ok_or(Legacy::Unrouted)?;
        let is_do = node.kind() == "do_statement";
        let head = if is_do {
            self.parsed
                .do_geometry_line(&node)
                .ok_or(Legacy::Unrouted)?
        } else {
            line(node)
        };
        if node.kind() == "for_statement" && self.for_init_hazard(node) {
            return Err(Legacy::ForInit);
        }
        let exit = self.after(node);
        let entry = self.enter(body);
        // Only a `do` condition needs the body's entry: the other loops' headers reach it already.
        if matches!(exit, Next::Unknown) || (is_do && matches!(entry, Next::Unknown)) {
            return Err(Legacy::Unrouted);
        }
        let mut completions = Vec::new();
        let empty = body.kind() == "empty_statement"
            || (body.kind() == "statement_block" && Self::last_statement(body).is_none());
        if !empty && !self.completions(body, &mut completions) {
            return Err(Legacy::Unrouted);
        }
        Ok(Loop {
            node,
            head,
            completions,
            exit,
            entry,
        })
    }

    /// The construct a `break` or `continue` binds to: the innermost loop (or, for `break`, `switch`),
    /// or the labeled statement its label names, and whether the jump leaves a `try` body or `catch`
    /// whose `finally` does not enclose that target. Never crosses a callable.
    fn target(&self, jump: Node<'tree>, is_continue: bool) -> Option<(Node<'tree>, bool)> {
        let label = jump
            .child_by_field_name("label")
            .map(|l| &self.parsed.source[l.start_byte()..l.end_byte()]);
        let mut crossing = false;
        let mut prev = jump;
        let mut at = jump.parent();
        while let Some(n) = at {
            if n.id() == self.body.id() || self.boundaries.contains(&n.kind()) {
                return None;
            }
            match label {
                None if is_loop(n.kind()) || (!is_continue && n.kind() == "switch_statement") => {
                    return Some((n, crossing));
                }
                Some(want) if n.kind() == "labeled_statement" => {
                    let name = n.child_by_field_name("label")?;
                    if &self.parsed.source[name.start_byte()..name.end_byte()] == want {
                        // `continue label` needs a loop under the label.
                        return if is_continue {
                            n.child_by_field_name("body")
                                .filter(|b| is_loop(b.kind()))
                                .map(|b| (b, crossing))
                        } else {
                            Some((n, crossing))
                        };
                    }
                }
                _ => {}
            }
            if n.kind() == "try_statement"
                && n.child_by_field_name("finalizer").is_some()
                && (n
                    .child_by_field_name("body")
                    .is_some_and(|b| b.id() == prev.id())
                    || n.child_by_field_name("handler")
                        .is_some_and(|h| h.id() == prev.id()))
            {
                crossing = true;
            }
            prev = n;
            at = n.parent();
        }
        None
    }

    /// Where a `continue` of loop `target` resumes: the header line, or an admitted `do`'s condition.
    fn continue_point(&self, target: Node<'tree>) -> Option<usize> {
        if target.kind() == "do_statement" {
            self.parsed.do_geometry_line(&target)
        } else {
            Some(line(target))
        }
    }

    /// The statements whose normal completion ends `node`, by explicit rule: an atomic statement
    /// completes at its own line; the last statement of a block or case, both arms of an `if` (the
    /// `if` itself without `else`), a loop header (its false exit), a nested `do`'s condition, a
    /// labeled statement's inner statement, `try` through its `finally` (else its body and `catch`;
    /// nowhere when neither completes), a `switch` through its last case and, without `default`, its
    /// header. An empty arm completes where it was entered. Jumps complete nowhere. Returns false for
    /// a kind with no rule, so the caller keeps main's edges.
    fn completions(&self, node: Node<'tree>, out: &mut Vec<Comp<'tree>>) -> bool {
        match node.kind() {
            "statement_block" => match Self::last_statement(node) {
                Some(last) => self.completions(last, out),
                None => false,
            },
            "if_statement" => {
                let arm = match node.child_by_field_name("alternative") {
                    Some(alt) => match Self::last_statement(alt) {
                        Some(inner) => self.arm(inner, node, out),
                        None => {
                            out.push(Comp::at(node));
                            true
                        }
                    },
                    None => {
                        out.push(Comp::at(node));
                        true
                    }
                };
                arm && node
                    .child_by_field_name("consequence")
                    .is_some_and(|then| self.arm(then, node, out))
            }
            "labeled_statement" | "with_statement" => node
                .child_by_field_name("body")
                .is_some_and(|inner| self.arm(inner, node, out)),
            "return_statement" | "throw_statement" | "break_statement" | "continue_statement" => {
                true
            }
            "try_statement" => {
                let mut inner = Vec::new();
                let body_ok = node
                    .child_by_field_name("body")
                    .is_some_and(|b| self.arm(b, node, &mut inner));
                let handler_ok = node.child_by_field_name("handler").is_none_or(|h| {
                    h.child_by_field_name("body")
                        .is_some_and(|b| self.arm(b, node, &mut inner))
                });
                if !(body_ok && handler_ok) {
                    return false;
                }
                let Some(finalizer) = node.child_by_field_name("finalizer") else {
                    out.extend(inner);
                    return true;
                };
                let Some(fin) = finalizer.child_by_field_name("body") else {
                    return false;
                };
                if Self::last_statement(fin).is_none() {
                    // An empty `finally` changes nothing.
                    out.extend(inner);
                    true
                } else if inner.is_empty() {
                    // Neither the body nor the `catch` completes: the `finally` completes only on the
                    // way out of a jump or a throw.
                    true
                } else {
                    self.completions(fin, out)
                }
            }
            "switch_statement" => {
                let Some(body) = node.child_by_field_name("body") else {
                    return false;
                };
                let mut cursor = body.walk();
                let cases: Vec<_> = body.named_children(&mut cursor).collect();
                if !cases.iter().any(|c| c.kind() == "switch_default") {
                    out.push(Comp::at(node));
                }
                match cases.iter().rev().find(|c| c.kind() != "comment") {
                    Some(last) => match Self::last_statement(*last) {
                        Some(inner) => self.arm(inner, node, out),
                        None => {
                            out.push(Comp::at(node));
                            true
                        }
                    },
                    None => {
                        out.push(Comp::at(node));
                        true
                    }
                }
            }
            "while_statement" | "for_statement" | "for_in_statement" => {
                out.push(Comp::at(node));
                true
            }
            "do_statement" => match self.parsed.do_geometry_line(&node) {
                Some(cond) => {
                    out.push(Comp { line: cond, node });
                    true
                }
                None => false,
            },
            "expression_statement" | "lexical_declaration" | "variable_declaration" => {
                out.push(Comp::at(node));
                true
            }
            _ => false,
        }
    }

    /// The completions of an arm entered from `entry`: an empty block completes at the construct that
    /// entered it.
    fn arm(&self, node: Node<'tree>, entry: Node<'tree>, out: &mut Vec<Comp<'tree>>) -> bool {
        if node.kind() == "statement_block" && Self::last_statement(node).is_none() {
            out.push(Comp::at(entry));
            true
        } else {
            self.completions(node, out)
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
            kind if self.parsed.language.is_statement_node(kind) => Next::Line(line(node)),
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
                        .do_geometry_line(&parent)
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

#[cfg(test)]
#[path = "js_loops/for_init_tests.rs"]
mod for_init_tests;
