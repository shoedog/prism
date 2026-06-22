//! The language-neutral resolution engine (Task 2, PR-1).
//!
//! `resolve` / `resolve_path` walk the scope graph and consult a
//! [`ResolutionPolicy`](crate::name_resolution::types::ResolutionPolicy) at
//! every decision point. **No Rust-ism lives here** — the Rust rules (per-rib
//! order, module-boundary stop, glob accessibility, anchors, candidate
//! combination, the `visible()` predicate) all live in
//! [`crate::name_resolution::rust_policy`]. The engine is generic over
//! `&dyn ResolutionPolicy`.
//!
//! ## The cardinal invariant (spec §7)
//! **Resolve-or-fall-through, NEVER a wrong target.** A bare name that fails to
//! resolve returns `Unresolved`/`Ambiguous`/`Poisoned` — the engine never
//! silently picks a wrong outer same-name item. Every shadow/poison/visibility
//! decision is structured so the *worst* outcome is a missed (not a wrong) edge.
//!
//! ## Engine ⇄ policy seam
//! The engine asks the policy three structural questions during the walk:
//! - `edge_order()` — which `EdgeKindId`s to follow at a rib (Rust: just `Glob`;
//!   the lexical-parent step is the engine's structural ascent, gated below).
//! - `ascend_to_parent(scope_kind)` — **policy-gated** lexical ascent (the Rust
//!   module-boundary stop lives here). Defaulted on the trait for non-Rust
//!   policies that want an unconditional walk.
//! - `combine(candidates)` / `visible(binding, q, trav)` / `anchor(anchor, from)`.

use crate::name_resolution::glob_stats::GlobExpandStats;
pub use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::types::{
    Anchor, BindTarget, Binding, Candidate, CfgCond, CfgCtx, GlobEdgeVis, NamespaceId,
    PolicyQueryCtx, ResStatus, Resolution, ResolutionPolicy, ResolveQuery, ScopeId, SourceLoc,
    Target, TraversalCtx,
};

// ── public entry points ───────────────────────────────────────────────────────

/// Resolve a **bare** `(name, ns)` lookup from `q.from` at `q.at`.
///
/// Walks scopes inner→outer under the policy's per-rib order, with the policy's
/// lexical-ascent gate (the Rust module-boundary stop). Returns the per-candidate
/// [`Resolution`].
pub fn resolve(graph: &ScopeGraph, q: &ResolveQuery, policy: &dyn ResolutionPolicy) -> Resolution {
    let mut guard = CycleGuard::with_stats(None);
    resolve_bare(graph, q, policy, &mut guard)
}

/// Resolve a multi-segment **path** anchored by `anchor` from `from`.
///
/// `anchor_ns` is the namespace used for the *prefix* (scope-bearing) segments
/// (Rust: Type/Module); `ns` is the namespace of the **final** segment. The
/// prefix segments are resolved as member lookups within each successive scope
/// (longest-prefix), the final segment within the final scope. No lexical
/// fall-out: an anchored path targets a specific scope chain and never silently
/// walks outward to a wrong same-name (§7).
#[allow(clippy::too_many_arguments)]
pub fn resolve_path(
    graph: &ScopeGraph,
    path: &crate::name_resolution::types::RawPath,
    ns: NamespaceId,
    anchor: &Anchor,
    from: ScopeId,
    anchor_ns: NamespaceId,
    at: &SourceLoc,
    policy: &dyn ResolutionPolicy,
) -> Resolution {
    let mut guard = CycleGuard::with_stats(None);
    resolve_path_guarded(
        graph, path, ns, anchor, from, anchor_ns, at, policy, &mut guard,
    )
}

#[doc(hidden)]
/// test-support: inject a local glob-expansion stats sink.
pub fn resolve_with_stats(
    graph: &ScopeGraph,
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    stats: &GlobExpandStats,
) -> Resolution {
    let mut guard = CycleGuard::with_stats(Some(stats));
    resolve_bare(graph, q, policy, &mut guard)
}

#[doc(hidden)]
/// test-support: inject a local glob-expansion stats sink.
#[allow(clippy::too_many_arguments)]
pub fn resolve_path_with_stats(
    graph: &ScopeGraph,
    path: &crate::name_resolution::types::RawPath,
    ns: NamespaceId,
    anchor: &Anchor,
    from: ScopeId,
    anchor_ns: NamespaceId,
    at: &SourceLoc,
    policy: &dyn ResolutionPolicy,
    stats: &GlobExpandStats,
) -> Resolution {
    let mut guard = CycleGuard::with_stats(Some(stats));
    resolve_path_guarded(
        graph, path, ns, anchor, from, anchor_ns, at, policy, &mut guard,
    )
}

// ── cycle guard for the Pending fixpoint ──────────────────────────────────────

/// Tracks the set of currently-resolving binding identities (by graph index) so
/// a re-export cycle terminates rather than recursing forever.
pub(crate) const MAX_GLOB_DEPTH: usize = 2;

struct CycleGuard<'s> {
    /// Indices into `graph.bindings` currently on the resolution stack.
    active: std::collections::BTreeSet<usize>,
    /// Indices into `graph.edges` currently on the glob-expansion stack.
    active_globs: std::collections::BTreeSet<usize>,
    glob_depth: usize,
    stats: Option<&'s GlobExpandStats>,
}

impl<'s> CycleGuard<'s> {
    fn with_stats(stats: Option<&'s GlobExpandStats>) -> Self {
        CycleGuard {
            active: Default::default(),
            active_globs: Default::default(),
            glob_depth: 0,
            stats,
        }
    }

    /// Returns `false` if `idx` is already active (a cycle); otherwise marks it
    /// active and returns `true`.
    fn enter(&mut self, idx: usize) -> bool {
        self.active.insert(idx)
    }
    fn leave(&mut self, idx: usize) {
        self.active.remove(&idx);
    }

    fn glob_depth(&self) -> usize {
        self.glob_depth
    }

    /// Returns `false` when this glob edge is already on the active chain.
    fn enter_glob(&mut self, edge_idx: usize) -> bool {
        if !self.active_globs.insert(edge_idx) {
            return false;
        }
        self.glob_depth += 1;
        true
    }

    fn leave_glob(&mut self, edge_idx: usize) {
        self.active_globs.remove(&edge_idx);
        self.glob_depth -= 1;
    }

    fn stats(&self) -> &GlobExpandStats {
        self.stats
            .unwrap_or(&crate::name_resolution::glob_stats::GLOBAL)
    }

    /// Enter a glob edge, run `body`, then leave on every successful entry.
    fn with_glob<R>(&mut self, edge_idx: usize, body: impl FnOnce(&mut Self, bool) -> R) -> R {
        let entered = self.enter_glob(edge_idx);
        let r = body(self, entered);
        if entered {
            self.leave_glob(edge_idx);
        }
        r
    }
}

#[cfg(test)]
mod glob_guard_tests {
    use super::*;

    #[test]
    fn enter_glob_tracks_depth_and_cycle_then_leaves() {
        let stats = crate::name_resolution::glob_stats::GlobExpandStats::default();
        let mut g = CycleGuard::with_stats(Some(&stats));
        assert_eq!(g.glob_depth(), 0);
        let entered = g.enter_glob(7);
        assert!(entered);
        assert_eq!(g.glob_depth(), 1);
        assert!(!g.enter_glob(7), "re-entering the same edge is a cycle");
        g.leave_glob(7);
        assert_eq!(g.glob_depth(), 0);
        assert!(g.enter_glob(7), "leaving clears the edge");
        g.leave_glob(7);
    }
}

// ── the bare-name inner→outer walk ────────────────────────────────────────────

fn resolve_bare(
    graph: &ScopeGraph,
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard<'_>,
) -> Resolution {
    let mut cur = Some(q.from);
    while let Some(scope_id) = cur {
        // 1) Local explicit bindings for (name, ns) in-range + cfg-compatible.
        //    If this rib has ANY such binding, the rib is AUTHORITATIVE for the
        //    name: we resolve/visibility-filter/combine here and STOP. We never
        //    skip past a claimed name to an outer same-name (§7 decoy rule) —
        //    even an inaccessible match stops the walk (claim-then-fail-visibility
        //    ⇒ fall through, not a wrong outer target).
        //
        //    An explicit binding here ALSO SHADOWS a covering macro wildcard
        //    (§4.3b Reading B): the macro poisons "exactly like a deferred glob"
        //    (§3.4 step 2), and a deferred glob is shadowed by an explicit step-1
        //    binding — so the macro is checked at the GLOB TIER (step 2 below),
        //    AFTER this rib, not before it.
        let rib = self_rib_bindings(graph, scope_id, &q.name, q.ns, &q.at);
        if !rib.is_empty() {
            return resolve_rib(graph, &rib, q, policy, guard);
        }

        // 2) Glob tier (reached only when step 1 found NO explicit binding for
        //    the name in this scope). A covering macro wildcard poisons here
        //    FIRST — exactly like a deferred glob (§4.3b / §7 poison-not-skip): a
        //    name-introducing item-position macro could emit `name`, and we
        //    cannot know its identity pre-expansion, so we poison (→ never reach
        //    an outer same-name) rather than fall through. It sits at the same
        //    tier as (and short-circuits ahead of) a same-scope non-deferred
        //    glob, mirroring the deferred-glob short-circuit in `glob_lookup`.
        if macro_wildcard_poisons(graph, scope_id, q.ns, &q.at) {
            return poisoned();
        }
        //    Else this scope's glob edges. A deferred glob poisons; otherwise
        //    union the visible (name, ns) members of each non-deferred glob
        //    target. Globs that yield NOTHING do not claim the name → continue.
        match glob_lookup(graph, scope_id, q, policy, guard) {
            GlobOutcome::Poison => return poisoned(),
            GlobOutcome::Hit(cands) => return policy.combine(cands),
            GlobOutcome::Empty => {}
        }

        // 3) Else recurse to the lexical parent — policy-gated (Rust module
        //    boundary stop). If the policy says stop, fall through (Unresolved).
        let kind = match graph.scope(scope_id) {
            Some(s) => s.kind.clone(),
            None => break,
        };
        if !policy.ascend_to_parent(&kind) {
            break;
        }
        cur = graph.parent_of(scope_id);
    }

    // Allow the policy to inject (prelude / ADL); empty for Rust Phase 1.
    let injected = policy.inject(q);
    if injected.is_empty() {
        unresolved()
    } else {
        policy.combine(injected)
    }
}

/// Resolve the candidates claimed at a single rib (explicit bindings).
///
/// Pending targets are chased through the fixpoint (cycle-guarded). A still-
/// pending / cyclic import **poisons**. Visibility is enforced via the policy
/// hook; if every name-match fails visibility the result is `Unresolved` (fall
/// through) — we do NOT continue outward (the rib claimed the name).
fn resolve_rib(
    graph: &ScopeGraph,
    rib: &[usize],
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard<'_>,
) -> Resolution {
    let mut candidates: Vec<Candidate> = Vec::new();
    for &bidx in rib {
        let b = &graph.bindings[bidx];
        // Visibility is checked against the binding as found at this rib.
        let trav = TraversalCtx {
            lookup_scope: Some(b.scope),
            via_glob: false,
            edge_kind: None,
        };
        if !policy.visible(b, q, &trav) {
            // Claimed the name but not visible → does not contribute a candidate
            // AND does not let us fall through to an outer item. We keep scanning
            // the rib (a sibling cfg-alternative may be visible) but never ascend.
            continue;
        }
        match &b.target {
            BindTarget::Resolved(t) => {
                candidates.push(Candidate {
                    target: t.clone(),
                    cond: cond_of(&b.cond),
                    provenance: Default::default(),
                });
            }
            BindTarget::Pending(path, anchor) => {
                // Chase the re-export chain from the BINDING'S OWN scope (the
                // re-export author's perspective — a `pub use` does NOT launder
                // privacy: the chased path must be visible at the re-export site).
                if !guard.enter(bidx) {
                    // Cycle: a still-pending import in a cycle poisons.
                    return poisoned();
                }
                // Prefix segments resolve in the policy's first scope-bearing
                // namespace (Rust: Type), NOT the final-segment ns of the query.
                let prefix_ns = policy.namespaces().first().copied().unwrap_or(b.ns);
                let sub = resolve_path_guarded(
                    graph, path, b.ns, anchor, b.scope, prefix_ns, &q.at, policy, guard,
                );
                guard.leave(bidx);
                match sub.status {
                    // A resolved (or legit set) chained target is folded in,
                    // re-conditioned by this binding's own cfg.
                    ResStatus::Resolved | ResStatus::ResolvedSet => {
                        for c in sub.candidates {
                            candidates.push(Candidate {
                                target: c.target,
                                cond: conjoin(&cond_of(&b.cond), &c.cond),
                                provenance: Default::default(),
                            });
                        }
                    }
                    ResStatus::Ambiguous => return ambiguous(sub.candidates),
                    // A STILL-PENDING import (its path is unresolvable, OR its
                    // target is not visible at the re-export site) ⇒ POISON, never
                    // fall through to an outer same-name (§4 / §7 poison-not-skip).
                    // `Poisoned` and `Unresolved` both mean "this import did not
                    // yield a concrete visible target" → poison.
                    ResStatus::Poisoned | ResStatus::Unresolved => return poisoned(),
                }
            }
        }
    }
    if candidates.is_empty() {
        // Claimed but nothing visible/resolvable → fall through (NOT outward).
        unresolved()
    } else {
        policy.combine(candidates)
    }
}

// ── glob handling (non-deferred members known; deferred ⇒ poison) ─────────────

enum GlobOutcome {
    /// A deferred/unexpanded glob in scope → poison.
    Poison,
    /// Glob(s) produced candidates.
    Hit(Vec<Candidate>),
    /// No glob produced anything (do not claim the name).
    Empty,
}

fn glob_lookup(
    graph: &ScopeGraph,
    scope_id: ScopeId,
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard<'_>,
) -> GlobOutcome {
    let glob_kinds = policy.edge_order();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut saw_glob = false;
    for (edge_idx, e) in graph.edges.iter().enumerate() {
        if e.from != scope_id || !glob_kinds.contains(&e.kind) {
            continue;
        }
        if e.vis_range
            .as_ref()
            .is_some_and(|span| !span_covers(span, &q.at))
        {
            continue;
        }
        if let BindTarget::Pending(path, _) = &e.to {
            if path.0.is_empty() {
                guard.stats().record_external();
                return GlobOutcome::Poison;
            }
        }
        let trav = TraversalCtx {
            lookup_scope: Some(scope_id),
            via_glob: true,
            edge_kind: Some(e.kind),
        };
        match policy.glob_edge_visible(e, q, &trav) {
            GlobEdgeVis::Hidden => continue,
            GlobEdgeVis::Unknown => {
                guard.stats().record_vis_unknown();
                return GlobOutcome::Poison;
            }
            GlobEdgeVis::Visible => {}
        }
        saw_glob = true;
        match &e.to {
            BindTarget::Pending(path, anchor) => {
                if guard.glob_depth() >= MAX_GLOB_DEPTH {
                    guard.stats().record_depth_exceeded();
                    return GlobOutcome::Poison;
                }
                let edge_outcome = guard.with_glob(edge_idx, |guard, entered| {
                    if !entered {
                        guard.stats().record_cycle();
                        return GlobOutcome::Poison;
                    }

                    let prefix_ns = policy.namespaces().first().copied().unwrap_or(q.ns);
                    let target_res = resolve_path_guarded(
                        graph, path, prefix_ns, anchor, scope_id, prefix_ns, &q.at, policy, guard,
                    );
                    let (target_scope, target_cond) =
                        match (&target_res.status, target_res.candidates.as_slice()) {
                            (ResStatus::Resolved, [tc]) => match &tc.target {
                                Target::Scope(s) => (*s, tc.cond.clone()),
                                _ => {
                                    guard.stats().record_external();
                                    return GlobOutcome::Poison;
                                }
                            },
                            (ResStatus::Ambiguous | ResStatus::ResolvedSet, _) => {
                                guard.stats().record_multi_target();
                                return GlobOutcome::Poison;
                            }
                            (ResStatus::Resolved, _) => {
                                guard.stats().record_multi_target();
                                return GlobOutcome::Poison;
                            }
                            (ResStatus::Unresolved | ResStatus::Poisoned, _) => {
                                guard.stats().record_external();
                                return GlobOutcome::Poison;
                            }
                        };

                    let (member_res, _) =
                        scope_member_lookup_probed(graph, target_scope, q, policy, guard);
                    match member_res.status {
                        ResStatus::Resolved if member_res.candidates.len() == 1 => {
                            let mut member_candidates = member_res.candidates;
                            let mc = member_candidates.pop().expect("checked len == 1");
                            candidates.push(Candidate {
                                target: mc.target,
                                cond: conjoin(&cond_of(&e.cond), &conjoin(&target_cond, &mc.cond)),
                                provenance: Default::default(),
                            });
                            guard.stats().record_resolved(guard.glob_depth());
                            GlobOutcome::Empty
                        }
                        ResStatus::Resolved | ResStatus::ResolvedSet | ResStatus::Ambiguous => {
                            guard.stats().record_ambiguous();
                            GlobOutcome::Poison
                        }
                        ResStatus::Poisoned => GlobOutcome::Poison,
                        ResStatus::Unresolved => GlobOutcome::Empty,
                    }
                });
                if matches!(edge_outcome, GlobOutcome::Poison) {
                    return GlobOutcome::Poison;
                }
            }
            BindTarget::Resolved(Target::Scope(target_scope)) => {
                // Union the visible (name, ns) members of the target scope.
                let members = glob_member_bindings(graph, *target_scope, &q.name, q.ns);
                for bidx in members {
                    let b = &graph.bindings[bidx];
                    let trav = TraversalCtx {
                        lookup_scope: Some(*target_scope),
                        via_glob: true,
                        edge_kind: Some(e.kind),
                    };
                    if !policy.visible(b, q, &trav) {
                        continue;
                    }
                    if let BindTarget::Resolved(t) = &b.target {
                        candidates.push(Candidate {
                            target: t.clone(),
                            cond: conjoin(&cond_of(&e.cond), &cond_of(&b.cond)),
                            provenance: Default::default(),
                        });
                    } else {
                        // A glob whose member is itself a still-Pending import:
                        // conservatively poison (we cannot know its identity).
                        return GlobOutcome::Poison;
                    }
                }
            }
            // A glob resolved to a non-scope target is malformed for Rust; treat
            // conservatively as no-contribution (the policy combination decides).
            BindTarget::Resolved(_) => {}
        }
    }
    if !saw_glob || candidates.is_empty() {
        GlobOutcome::Empty
    } else {
        GlobOutcome::Hit(candidates)
    }
}

// ── path resolution (anchored member-lookup chain) ───────────────────────────

#[allow(clippy::too_many_arguments)]
fn resolve_path_guarded(
    graph: &ScopeGraph,
    path: &crate::name_resolution::types::RawPath,
    ns: NamespaceId,
    anchor: &Anchor,
    from: ScopeId,
    anchor_ns: NamespaceId,
    at: &SourceLoc,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard<'_>,
) -> Resolution {
    let segs = &path.0;
    if segs.is_empty() {
        return unresolved();
    }
    // Anchor the starting scope. `None` ⇒ conservative fall-through.
    let (mut scope, _start_ns) = match policy.anchor(anchor, from) {
        Some(x) => x,
        None => return unresolved(),
    };

    // Walk the prefix segments as scope-bearing member lookups.
    for (i, seg) in segs.iter().enumerate() {
        let is_last = i + 1 == segs.len();
        let seg_ns = if is_last { ns } else { anchor_ns };
        // Build a query whose `from` is the ORIGINAL query origin so visibility
        // is judged from the caller's vantage (sibling-private decoys fall
        // through; pub facades resolve).
        let seg_q = ResolveQuery {
            name: seg.clone(),
            ns: seg_ns,
            from,
            at: at.clone(),
            cfg: CfgCtx::default(),
            ctx: PolicyQueryCtx::default(),
        };
        // Single-scope member lookup AND a "was a rib claimed here?" probe (so a
        // claimed-but-invisible local cannot be overridden by the crate fallback).
        let (res, rib_present) = scope_member_lookup_probed(graph, scope, &seg_q, policy, guard);
        // Leading-segment crate-root fallback (strictly last; P2/P3): a 2018+
        // extern-prelude root resolves to the owning crate's Root scope ONLY on a
        // TRUE no-rib miss — no rib was claimed for the segment (so a local item,
        // even a deliberately-invisible one, always shadows — P2) — AND the policy's
        // anchor/edition gate + the consuming crate's per-crate in-repo dependency
        // gate pass (P3). Poison/empty-glob `Unresolved` WITH a claimed rib does not
        // qualify. `from` (the query origin) is threaded so the policy can identify
        // the consuming crate.
        let res = if i == 0 && !rib_present && matches!(res.status, ResStatus::Unresolved) {
            match policy.extern_crate_root(graph, seg, anchor, from) {
                Some(root) => Resolution {
                    candidates: vec![Candidate {
                        target: Target::Scope(root),
                        cond: CfgCond::True,
                        provenance: Default::default(),
                    }],
                    status: ResStatus::Resolved,
                },
                None => res,
            }
        } else {
            res
        };
        if is_last {
            return res;
        }
        // Non-final: must resolve to exactly one scope-bearing target.
        match (&res.status, res.candidates.as_slice()) {
            (ResStatus::Resolved, [c]) => match scope_of_target(&c.target) {
                Some(s) => scope = s,
                None => return unresolved(), // not scope-bearing → fall through
            },
            // Poison propagates; anything non-singular falls through (never wrong).
            (ResStatus::Poisoned, _) => return poisoned(),
            _ => return unresolved(),
        }
    }
    unreachable!("path with >=1 segment returns inside the loop")
}

/// Look up `(name, ns)` **within a single scope** (a path segment): explicit
/// bindings first (visibility-enforced, Pending chased), then this scope's
/// non-deferred globs. NO lexical fall-out (an anchored member lookup never
/// walks to a parent — that is what keeps a sibling-private decoy from reaching
/// an outer same-name).
///
/// ALSO reports whether an explicit rib binding for `(name, ns)` was CLAIMED in
/// this scope (regardless of visibility/outcome). The boolean lets
/// `resolve_path_guarded` distinguish a TRUE no-rib miss (where the crate-root
/// fallback may fire) from a claimed-but-invisible local rib (which surfaces as
/// `Unresolved` but must shadow the crate name — P2/BLOCKER 1). It does NOT change
/// the `Resolution` returned. (`resolve_path_guarded` is the only caller and needs
/// the flag, so there is no thin non-probed wrapper.)
fn scope_member_lookup_probed(
    graph: &ScopeGraph,
    scope: ScopeId,
    q: &ResolveQuery,
    policy: &dyn ResolutionPolicy,
    guard: &mut CycleGuard<'_>,
) -> (Resolution, bool) {
    // Explicit bindings in this scope for (name, ns), cfg-compatible. For path
    // member lookup we do NOT gate on vis_extents byte-range (a module member is
    // visible across the whole module to a path); visibility is the `visible()`
    // policy hook. An explicit member binding SHADOWS a covering macro wildcard
    // (the wildcard is glob-tier — §4.3b Reading B), so it is checked AFTER this
    // rib, not before.
    let rib: Vec<usize> = graph
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.scope == scope && b.name == q.name && b.ns == q.ns)
        .filter(|(_, b)| cfg_compatible(&b.cond))
        .map(|(i, _)| i)
        .collect();
    let rib_present = !rib.is_empty();
    if rib_present {
        return (resolve_rib(graph, &rib, q, policy, guard), true);
    }
    // Glob tier: a covering macro wildcard poisons here (exactly like a deferred
    // glob), reached only when the rib above found no explicit member binding.
    if macro_wildcard_poisons(graph, scope, q.ns, &q.at) {
        return (poisoned(), false);
    }
    // Else this scope's globs.
    let res = match glob_lookup(graph, scope, q, policy, guard) {
        GlobOutcome::Poison => poisoned(),
        GlobOutcome::Hit(cands) => policy.combine(cands),
        GlobOutcome::Empty => unresolved(),
    };
    (res, false)
}

// ── binding selection helpers ─────────────────────────────────────────────────

/// Indices of explicit `(name, ns)` bindings in `scope` whose `vis_extents`
/// cover `at.byte` and whose cfg is compatible with the query.
fn self_rib_bindings(
    graph: &ScopeGraph,
    scope: ScopeId,
    name: &str,
    ns: NamespaceId,
    at: &SourceLoc,
) -> Vec<usize> {
    graph
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.scope == scope && b.name == name && b.ns == ns)
        .filter(|(_, b)| vis_extent_covers(b, at))
        .filter(|(_, b)| cfg_compatible(&b.cond))
        .map(|(i, _)| i)
        .collect()
}

/// Visible (name, ns) member indices in a glob target scope (no byte gate — a
/// glob brings a module's members regardless of the use site's byte).
fn glob_member_bindings(
    graph: &ScopeGraph,
    scope: ScopeId,
    name: &str,
    ns: NamespaceId,
) -> Vec<usize> {
    graph
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, b)| b.scope == scope && b.name == name && b.ns == ns)
        .map(|(i, _)| i)
        .collect()
}

/// Does any unexpanded-macro wildcard in `scope` for `ns` cover `at.byte`?
fn macro_wildcard_poisons(
    graph: &ScopeGraph,
    scope: ScopeId,
    ns: NamespaceId,
    at: &SourceLoc,
) -> bool {
    graph
        .macro_wildcards
        .iter()
        .any(|m| m.scope == scope && m.ns == ns && span_covers(&m.range, at))
}

// ── small predicates / combinators ────────────────────────────────────────────

fn vis_extent_covers(b: &Binding, at: &SourceLoc) -> bool {
    if b.vis_extents.is_empty() {
        // No recorded extent ⇒ treat as scope-wide (conservative: visible).
        return true;
    }
    b.vis_extents.iter().any(|s| span_covers(s, at))
}

fn span_covers(s: &crate::name_resolution::types::Span, at: &SourceLoc) -> bool {
    // Half-open [lo, hi); same-file comparison (callers build same-file spans).
    s.lo.file == at.file && at.byte >= s.lo.byte && at.byte < s.hi.byte
}

/// Whether a binding's cfg is compatible with the query's active cfg.
///
/// Phase 1 has no cfg *evaluation* (the `CfgCtx` is empty), so this is the
/// conservative `true` — every conditioned binding is *kept* (never silently
/// dropped); exclusivity between SIBLING candidates is handled later by the
/// policy's `combine` (cfg-exclusive ⇒ distinct worlds, not a conflict).
fn cfg_compatible(_cond: &Option<CfgCond>) -> bool {
    true
}

fn cond_of(c: &Option<CfgCond>) -> CfgCond {
    c.clone().unwrap_or(CfgCond::True)
}

/// Conjoin two cfg conditions (path accumulation), collapsing `True`.
fn conjoin(a: &CfgCond, b: &CfgCond) -> CfgCond {
    match (a, b) {
        (CfgCond::True, x) | (x, CfgCond::True) => x.clone(),
        _ => CfgCond::And(vec![a.clone(), b.clone()]),
    }
}

fn scope_of_target(t: &Target) -> Option<ScopeId> {
    match t {
        Target::Scope(s) => Some(*s),
        Target::Item { owns: Some(s), .. } => Some(*s),
        _ => None,
    }
}

// ── result constructors ───────────────────────────────────────────────────────

fn unresolved() -> Resolution {
    Resolution {
        candidates: vec![],
        status: ResStatus::Unresolved,
    }
}

fn poisoned() -> Resolution {
    Resolution {
        candidates: vec![],
        status: ResStatus::Poisoned,
    }
}

fn ambiguous(candidates: Vec<Candidate>) -> Resolution {
    Resolution {
        candidates,
        status: ResStatus::Ambiguous,
    }
}
