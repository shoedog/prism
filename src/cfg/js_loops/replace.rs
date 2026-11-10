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
        if let (Some((root, _)), Producer::ThenEnd, Some(from)) = (owned, r.producer, r.from) {
            then_ends.push((from, root));
        }
        keep.push(owned.is_none());
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
