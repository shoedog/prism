//! Record-based replacement of a loop tree's legacy transitions (#693, #697; option B, slice 3).
//!
//! Slice 2 decided WHICH trees are rewired; this decides WHICH EDGES a rewired tree takes out. A record of
//! the owner's build is dropped only when an admitted tree owns it (`Authority::owns`: the record is the
//! tree's back edge, exit, a body completion that bypasses the loop, or the then-end of an arm whose
//! completion the tree delegates), never because its line happens to lie between the loop's first and last
//! lines (the inclusive-span rule slice 2 still used). Records of other trees, of statements outside the
//! tree and of unmodelled emitters are unchanged. Everything a dropped record stood for is emitted from
//! syntax by `js_loops::build` (completions to the continue point, header to `after`, jumps), and a
//! delegated then-end is replaced by the completions of its wrapper (`Cx::wrapper_completion`).
//!
//! The same authority applies across callables. An enclosing callable's generic collectors also draw the
//! legacy loop edges of loops inside the callables nested in it (the owner's RD selects edges by endpoint
//! lines, not by who drew them). When the nested callable's tree is admitted its own build replaces those
//! edges, so the enclosing owner drops ITS copies of exactly the records the nested tree owns (rule (b) of
//! the slice-3 brief: a copy that survived next to the replacement kept a bypass, e.g. a `do`'s back edge
//! past its condition).
use super::{Comp, Cx, Next};
use crate::ast::ParsedFile;
use crate::cfg::js_disposition::Authority;
use crate::cfg::js_ownership::{Anchor, EdgeRecord, Producer};
use crate::cfg::EdgeSink;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// The syntax node an anchor names (its range and kind), or `None` when the file has none.
pub(in crate::cfg) fn node_of<'p>(parsed: &'p ParsedFile, a: Anchor) -> Option<Node<'p>> {
    let mut n = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(a.start, a.end)?;
    loop {
        if n.start_byte() == a.start && n.end_byte() == a.end && n.kind() == a.kind {
            return Some(n);
        }
        n = n.parent()?;
        if n.start_byte() > a.start || n.end_byte() < a.end {
            return None;
        }
    }
}

/// Whether the then-end wrapper `w` (a compound last statement of an `if` arm) has a resolvable completion
/// for the tree rooted at `root`, as seen from `owner`.
pub(crate) fn wrapper_resolvable<'p>(
    parsed: &'p ParsedFile,
    owner: Node<'p>,
    w: Anchor,
    root: Anchor,
) -> bool {
    let Some(body) = owner.child_by_field_name("body") else {
        return false;
    };
    let boundaries = parsed.language.callable_boundary_node_types();
    let cx = Cx {
        parsed,
        body,
        boundaries: &boundaries,
    };
    node_of(parsed, w).is_some_and(|n| cx.wrapper_completion(n, (root.start, root.end)).is_some())
}

/// Whether the then-end's `last` statement is a loop, or a label directly on one.
fn is_direct_loop(parsed: &ParsedFile, a: Anchor) -> bool {
    let Some(n) = node_of(parsed, a) else {
        return false;
    };
    let body = if n.kind() == "labeled_statement" {
        n.child_by_field_name("body")
    } else {
        Some(n)
    };
    body.is_some_and(|b| super::is_loop(b.kind()))
}

/// What replaces a then-end wrapper's edge.
pub(super) struct Delegated<'tree> {
    /// The wrapper's normal completions.
    pub(super) completions: Vec<Comp<'tree>>,
    /// The line control reaches when the wrapper completes.
    pub(super) to: usize,
    /// The jumps in the wrapper that no tree routes but whose target is inside it (a `break` of a `switch`
    /// in the wrapper): the wrapper emits their edges, each with the line its target completes to.
    pub(super) jumps: Vec<(Node<'tree>, usize)>,
}

impl<'a, 'tree> Cx<'a, 'tree> {
    /// What replaces the then-end edge drawn from the entry of the wrapper `w`: its normal completions
    /// (`completions`, a closed rule list), the line control reaches when `w` completes (`after`) and the
    /// edges of its own jumps. `None` is "unsupported": a form without a rule, an unknown completion target,
    /// or a `break`/`continue` in `w` that no admitted tree routes and whose target is not inside `w` (it
    /// would have no edge, so dropping the then-end would lose its flow instead of replacing it).
    pub(super) fn wrapper_completion(
        &self,
        w: Node<'tree>,
        root: (usize, usize),
    ) -> Option<Delegated<'tree>> {
        if !self.try_flow_accounted(w) {
            return None;
        }
        let jumps = self.wrapper_jumps(w, root)?;
        let mut completions = Vec::new();
        if !self.completions(w, &mut completions) {
            return None;
        }
        let Next::Line(to) = self.after(w) else {
            return None;
        };
        Some(Delegated {
            completions,
            to,
            jumps,
        })
    }

    /// Whether every `try` under `w` (never inside a nested callable) that has both a `catch` and a `finally`
    /// has flow the admission accounts for. `build_try_catch_edges` draws `catch` -> `finally` from the
    /// catch's FIRST statement, an unmodelled record that lies outside every loop footprint and so neither
    /// refuses nor is replaced; the emitted completion leaves the `finally`, which makes
    /// first-catch-statement -> `finally` -> after a route that skips the rest of the catch (fold 1, `tc`:
    /// a wrong `exact` across `x = next()` / `x = other()`). It is accounted only when the catch is empty or
    /// one simple statement (nothing after it to skip). Anything else is refused: the exception flow is not
    /// modelled, so the wrapper's completion cannot be called resolved.
    fn try_flow_accounted(&self, w: Node<'tree>) -> bool {
        let mut stack = vec![w];
        while let Some(n) = stack.pop() {
            if n.id() != w.id() && self.boundaries.contains(&n.kind()) {
                continue;
            }
            if n.kind() == "try_statement" && n.child_by_field_name("finalizer").is_some() {
                if let Some(h) = n.child_by_field_name("handler") {
                    let body = h.child_by_field_name("body");
                    let stmts = body.map(Self::statements).unwrap_or_default();
                    let simple = stmts.is_empty()
                        || (stmts.len() == 1
                            && matches!(
                                stmts[0].kind(),
                                "expression_statement"
                                    | "lexical_declaration"
                                    | "variable_declaration"
                                    | "return_statement"
                                    | "throw_statement"
                            ));
                    if !simple {
                        return false;
                    }
                }
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        true
    }

    /// The jumps under `w` (never inside a nested callable) that the wrapper itself must route, or `None`
    /// when one cannot be routed by anyone. A jump inside the tree rooted at `root`, or inside another
    /// outermost loop the plan admits, is that tree's (it resolves without crossing a `finally`); any
    /// other must be a `break` whose target (a `switch` or a labeled statement) is inside `w`.
    fn wrapper_jumps(
        &self,
        w: Node<'tree>,
        root: (usize, usize),
    ) -> Option<Vec<(Node<'tree>, usize)>> {
        let admitted: Vec<(usize, usize)> = self
            .outermost()
            .into_iter()
            .map(|n| (n.start_byte(), n.end_byte()))
            .filter(|r| *r == root || super::plan::admits(self.parsed, *r))
            .collect();
        let (mut own, mut stack) = (Vec::new(), vec![w]);
        while let Some(n) = stack.pop() {
            let kind = n.kind();
            if n.id() != w.id() && self.boundaries.contains(&kind) {
                continue;
            }
            if matches!(kind, "break_statement" | "continue_statement") {
                let is_continue = kind == "continue_statement";
                let resolved = self.target(n, is_continue);
                let in_tree = admitted
                    .iter()
                    .any(|r| r.0 <= n.start_byte() && n.end_byte() <= r.1);
                if in_tree {
                    if !matches!(resolved, Some((_, false))) {
                        return None;
                    }
                } else {
                    let (Some((target, false)), false) = (resolved, is_continue) else {
                        return None;
                    };
                    let inside =
                        w.start_byte() <= target.start_byte() && target.end_byte() <= w.end_byte();
                    let Next::Line(to) = self.after(target) else {
                        return None;
                    };
                    if !inside {
                        return None;
                    }
                    own.push((n, to));
                }
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        Some(own)
    }
}

/// The edges of the owner's build a rewired tree takes out, and the delegated then-end wrappers whose
/// completions the tree must emit.
pub(super) struct Drops {
    /// `keep[i]` is the verdict on the i-th live edge.
    pub(super) keep: Vec<bool>,
    /// Each dropped then-end record's wrapper and the root of the tree that owns it.
    pub(super) then_ends: Vec<(Anchor, Anchor)>,
}

/// The owner's live records an admitted tree of it owns. `None` when no tree is rewired or no record is
/// recorded (records are always built for a JS-family owner outside tests).
pub(super) fn own_drops<'p>(
    owner: Node<'p>,
    parsed: &'p ParsedFile,
    edges: &EdgeSink,
) -> Option<Drops> {
    let live = edges.live_records()?;
    let authority = Authority::new(parsed, owner)?;
    let mut keep = Vec::with_capacity(live.len());
    let mut then_ends = Vec::new();
    for r in live {
        // A record drawn for a nested callable's construct is that callable's, not this owner's.
        let owned = if r.nested {
            None
        } else {
            authority.owns(r, false)
        };
        let mut drop_it = owned.is_some();
        if let (Some((root, _)), Producer::ThenEnd, Some(from)) = (owned, r.producer, r.from) {
            // The replacement is computed before the record goes: if it cannot be, the record stays.
            if is_direct_loop(parsed, from) || !crate::cfg::is_compound(parsed, from.kind) {
                // A loop (or its label) that itself ends the arm, or a simple statement (a body completion): its
                // own exit or completion edge is the replacement, no wrapper completion is needed.
                then_ends.push((from, root));
            } else if wrapper_resolvable(parsed, owner, from, root) {
                then_ends.push((from, root));
            } else {
                debug_assert!(false, "an owned then-end whose completion does not resolve");
                drop_it = false;
            }
        }
        keep.push(!drop_it);
    }
    Some(Drops { keep, then_ends })
}

/// Drops the owner's copies of edges that an admitted tree of a nested callable owns (rule (b)).
pub(super) fn drop_nested_copies<'p>(
    owner: Node<'p>,
    parsed: &'p ParsedFile,
    edges: &mut EdgeSink,
) {
    let Some(live) = edges.live_records() else {
        return;
    };
    if !live.iter().any(|r| r.nested) {
        return;
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    let mut authority: BTreeMap<(usize, usize), Option<Authority<'p>>> = BTreeMap::new();
    let keep: Vec<bool> = live
        .iter()
        .map(|r| {
            if !r.nested {
                return true;
            }
            let Some(n) = nested_owner(parsed, owner, &boundaries, r) else {
                return true;
            };
            authority
                .entry((n.start_byte(), n.end_byte()))
                .or_insert_with(|| Authority::new(parsed, n))
                .as_ref()
                .is_none_or(|a| a.owns(r, true).is_none())
        })
        .collect();
    if keep.iter().any(|k| !k) {
        trace_drops(
            parsed,
            owner,
            edges.live_records().unwrap_or(&[]),
            &keep,
            "nested",
        );
        edges.retain_flags(&keep);
    }
}

/// The innermost callable (other than `owner`) holding the construct a nested record was drawn for.
fn nested_owner<'p>(
    parsed: &'p ParsedFile,
    owner: Node<'p>,
    boundaries: &[&'static str],
    r: &EdgeRecord,
) -> Option<Node<'p>> {
    let anchor = r.depends_on.or(r.from).or(r.to)?;
    let mut at = node_of(parsed, anchor)?.parent();
    while let Some(n) = at {
        if n.id() == owner.id() {
            return None;
        }
        if boundaries.contains(&n.kind()) {
            return Some(n);
        }
        at = n.parent();
    }
    None
}

#[cfg(test)]
#[path = "replace_tests.rs"]
mod tests;

/// `PRISM_CFG_OWNERSHIP_TRACE=1`: the standing build's drops and emissions, one line each (the accounting of
/// the corpus gate: every removed edge is a dropped record, every added edge an emission). Plan passes
/// (assumed admission) are not the standing build and print nothing.
pub(super) fn trace_drops(
    parsed: &ParsedFile,
    owner: Node<'_>,
    live: &[EdgeRecord],
    keep: &[bool],
    kind: &str,
) {
    if !crate::cfg::js_disposition::trace_enabled() || super::plan::assuming(parsed) {
        return;
    }
    for (r, _) in live.iter().zip(keep).filter(|(_, k)| !**k) {
        eprintln!(
            "own-drop {} owner=[{},{}) {kind} #{} {:?}/{:?} L{}->L{}",
            parsed.path,
            owner.start_byte(),
            owner.end_byte(),
            r.id,
            r.producer,
            r.role,
            r.from_line,
            r.to_line
        );
    }
}

/// The edges the rewrite emitted (see `trace_drops`).
pub(super) fn trace_emitted(parsed: &ParsedFile, owner: Node<'_>, edges: &EdgeSink) {
    if !crate::cfg::js_disposition::trace_enabled() || super::plan::assuming(parsed) {
        return;
    }
    for r in edges
        .live_records()
        .unwrap_or(&[])
        .iter()
        .filter(|r| matches!(r.producer, Producer::JsLoops | Producer::Jump))
    {
        eprintln!(
            "own-emit {} owner=[{},{}) #{} {:?}/{:?} L{}->L{}",
            parsed.path,
            owner.start_byte(),
            owner.end_byte(),
            r.id,
            r.producer,
            r.role,
            r.from_line,
            r.to_line
        );
    }
}

/// `PRISM_CFG_OWNERSHIP_TRACE=1` (standing build): the unmodelled-emitter records that end on a line a delegated
/// wrapper's completion is about to leave from. The class of fold 1 (a catch's first statement jumping to the
/// `finally` that the emitted completion leaves) is enumerated from these lines.
pub(super) fn trace_inbound(
    parsed: &ParsedFile,
    owner: Node<'_>,
    edges: &EdgeSink,
    w: Node<'_>,
    d: &Delegated<'_>,
) {
    if !crate::cfg::js_disposition::trace_enabled() || super::plan::assuming(parsed) {
        return;
    }
    for c in &d.completions {
        for r in edges.live_records().unwrap_or(&[]) {
            if r.role == crate::cfg::js_ownership::Role::Legacy
                && r.to_line == c.line
                && r.from_line != c.line
            {
                eprintln!(
                    "own-inbound {} owner=[{},{}) wrapper={}[{},{}) {:?} L{}->L{} completion=L{}",
                    parsed.path,
                    owner.start_byte(),
                    owner.end_byte(),
                    w.kind(),
                    w.start_byte(),
                    w.end_byte(),
                    r.producer,
                    r.from_line,
                    r.to_line,
                    c.line
                );
            }
        }
    }
}
