//! The Rust [`ResolutionPolicy`] (Task 2, PR-1).
//!
//! Every Rust-specific resolution rule lives here — the engine
//! ([`crate::name_resolution::engine`]) is language-neutral and consults this
//! policy at each decision point. This struct supplies:
//!
//! - **Namespaces** `{Type, Value, Macro}` (Type is scope-bearing for prefixes).
//! - **Per-rib edge order** — Rust follows only `Glob` edges at a rib (the
//!   lexical-parent step is the engine's structural ascent, gated by
//!   `ascend_to_parent`); explicit binding ▷ this-scope glob ▷ lexical parent.
//! - **MODULE-BOUNDARY STOP** for bare names (`ascend_to_parent`): a bare name
//!   crosses `Block`/`Callable`/`Type` parents but **stops at the enclosing
//!   `Module`/`Root`** — Rust does not inherit unqualified names across a module
//!   boundary (§4 round6-B1). Only explicit `super::`/`crate::`/`self::` anchors
//!   + the extern-prelude reach module ancestors.
//! - **`visible()`** — `pub` / `pub(crate)` / `pub(super)` / `pub(in path)` /
//!   private accessibility, computed against the borrowed [`ScopeGraph`]'s
//!   ancestry. Un-enforced / unknown visibility **falls through** (never
//!   resolves a not-visible target — §7).
//! - **`combine()`** — single ⇒ `Resolved`; same-target dedup ⇒ `Resolved`;
//!   distinct-namespace set ⇒ `ResolvedSet`; cfg-exclusive worlds ⇒
//!   `ResolvedSet` (distinct worlds, never merged); conflicting compatible-cfg
//!   ⇒ `Ambiguous`.
//! - **Edition-aware anchors** (`crate`/`self`/`super`/bare/`::`, 2015 vs 2018).
//!
//! ## Recall-safety stance
//! Every hook is written so the *worst* outcome is a missed edge, never a wrong
//! one: an unknown visibility kind is **not visible**, an un-mappable anchor is
//! `None` (fall through), a non-provably-exclusive conflict is `Ambiguous`.

use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::types::{
    Anchor, AnchorKind, Binding, Candidate, Edge, EdgeKindId, VisibilityDecision, NamespaceId, ResStatus,
    Resolution, ResolutionPolicy, ResolveQuery, ScopeId, ScopeKind, TraversalCtx, Vis, VisKindId,
};

// ── Rust namespace registry (policy-owned discriminants) ──────────────────────

/// Type namespace (modules, structs, enums, traits, type aliases).
pub const NS_TYPE: NamespaceId = 0;
/// Value namespace (fns, consts, statics, locals, tuple/unit ctors).
pub const NS_VALUE: NamespaceId = 1;
/// Macro namespace.
pub const NS_MACRO: NamespaceId = 2;

// ── Rust visibility-kind registry ─────────────────────────────────────────────

/// `pub` — visible everywhere the path reaches.
pub const VIS_PUB: VisKindId = 0;
/// `pub(crate)` — visible anywhere in the defining crate.
pub const VIS_PUB_CRATE: VisKindId = 1;
/// `pub(super)` — visible in the parent module's subtree.
pub const VIS_PUB_SUPER: VisKindId = 2;
/// `pub(in path)` — visible only inside the `restrict` scope's subtree.
pub const VIS_PUB_IN: VisKindId = 3;
/// private (the default) — visible in the defining module and its descendants.
pub const VIS_PRIV: VisKindId = 4;

// ── Rust edge-kind registry ───────────────────────────────────────────────────

/// A `use a::*` / `pub use a::*` glob edge.
pub const EK_GLOB: EdgeKindId = 1;

// ── The policy struct ─────────────────────────────────────────────────────────

/// The Rust resolution policy.
///
/// Borrows the [`ScopeGraph`] so `visible()` can compute module ancestry for
/// `pub(super)`/`pub(in)`/`pub(crate)`. `edition` selects 2015 vs 2018+ anchor
/// semantics (the only edition split Phase 1 needs).
pub struct RustPolicy<'g> {
    graph: &'g ScopeGraph,
    edition: u16,
}

impl<'g> RustPolicy<'g> {
    /// Construct a policy over `graph` for the given `edition` (e.g. 2015/2018).
    pub fn new(graph: &'g ScopeGraph, edition: u16) -> Self {
        RustPolicy { graph, edition }
    }

    fn is_2018_plus(&self) -> bool {
        self.edition >= 2018
    }

    /// The enclosing `Module`/`Root` of `scope` (itself if it is one).
    fn enclosing_module(&self, scope: ScopeId) -> Option<ScopeId> {
        let mut cur = Some(scope);
        while let Some(id) = cur {
            let s = self.graph.scope(id)?;
            if matches!(s.kind, ScopeKind::Module | ScopeKind::Root) {
                return Some(id);
            }
            cur = s.parent;
        }
        None
    }

    /// The crate `Root` ancestor of `scope`.
    ///
    /// Walks the parent chain and returns on the **first** (nearest) `Root` found;
    /// the `break` exits immediately, so the variable holds that Root, not a "last"
    /// or topmost one.
    fn crate_root(&self, scope: ScopeId) -> Option<ScopeId> {
        let mut cur = Some(scope);
        let mut root = None;
        while let Some(id) = cur {
            let s = self.graph.scope(id)?;
            if matches!(s.kind, ScopeKind::Root) {
                root = Some(id);
                break; // first Root found — exit immediately
            }
            cur = s.parent;
            if cur.is_none() {
                // Graph-without-Root safety net: no Root ancestor exists; treat
                // the topmost reachable scope as the crate root.
                root = Some(id);
            }
        }
        root
    }

    /// Walk `n` module hops up from `scope`'s enclosing module.
    fn super_n(&self, scope: ScopeId, n: u32) -> Option<ScopeId> {
        let mut cur = self.enclosing_module(scope)?;
        for _ in 0..n {
            // `super` of the crate Root is undefined → fall through.
            let parent = self.graph.parent_of(cur)?;
            cur = self.enclosing_module(parent)?;
        }
        Some(cur)
    }

    /// Is `desc` inside `ancestor`'s subtree (inclusive)?
    fn is_within(&self, desc: ScopeId, ancestor: ScopeId) -> bool {
        let mut cur = Some(desc);
        while let Some(id) = cur {
            if id == ancestor {
                return true;
            }
            cur = self.graph.parent_of(id);
        }
        false
    }

    fn vis_reaches(&self, vis: &Vis, def_scope: ScopeId, from: ScopeId) -> Option<bool> {
        let def_module = match self.enclosing_module(def_scope) {
            Some(m) => m,
            None => return Some(false),
        };
        match vis.kind {
            VIS_PUB => Some(true),
            VIS_PUB_CRATE => Some(match (self.crate_root(def_scope), self.crate_root(from)) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }),
            VIS_PUB_SUPER => Some(match self.super_n(def_scope, 1) {
                Some(parent_mod) => self.is_within(from, parent_mod),
                None => false,
            }),
            VIS_PUB_IN => vis.restrict.map(|r| self.is_within(from, r)),
            VIS_PRIV => Some(self.is_within(from, def_module)),
            _ => Some(false),
        }
    }
}

impl ResolutionPolicy for RustPolicy<'_> {
    fn namespaces(&self) -> Vec<NamespaceId> {
        // Scope-bearing namespaces for resolve_path prefixes: Type (modules,
        // type bodies). Value/Macro are final-segment-only.
        vec![NS_TYPE]
    }

    fn edge_order(&self) -> Vec<EdgeKindId> {
        // Rust follows only Glob edges at a rib. (Lexical parent ascent is the
        // engine's structural step, gated by `ascend_to_parent` below.)
        vec![EK_GLOB]
    }

    fn ascend_to_parent(&self, kind: &ScopeKind) -> bool {
        // THE MODULE-BOUNDARY STOP. A bare name crosses block/callable/type
        // lexical parents, but once we have looked in a Module/Root we STOP —
        // Rust does not inherit unqualified names from a parent module.
        matches!(
            kind,
            ScopeKind::Block | ScopeKind::Callable | ScopeKind::Type
        )
    }

    fn combine(&self, candidates: Vec<Candidate>) -> Resolution {
        if candidates.is_empty() {
            return Resolution {
                candidates,
                status: ResStatus::Unresolved,
            };
        }
        // Dedup exact-duplicate candidates (same target + same cond) — e.g. two
        // glob paths to the SAME item (a re-export diamond) is a single Resolved,
        // not Ambiguous.
        let mut deduped: Vec<Candidate> = Vec::new();
        for c in candidates {
            if !deduped
                .iter()
                .any(|d| d.target == c.target && d.cond == c.cond)
            {
                deduped.push(c);
            }
        }

        if deduped.len() == 1 {
            return Resolution {
                candidates: deduped,
                status: ResStatus::Resolved,
            };
        }

        // Multiple distinct candidates. Classify:
        //   (a) all the SAME target but distinct cfg conds → cfg-exclusive worlds
        //       (or distinct conditioned definitions) → ResolvedSet (never merged).
        //   (b) distinct targets that are pairwise cfg-EXCLUSIVE → ResolvedSet
        //       (the cfg-duplicate-mod / cfg-impl case — distinct worlds).
        //   (c) distinct targets under COMPATIBLE cfg → Ambiguous (a genuine
        //       conflict; the consumer falls through, never a silent pick).
        let all_cfg_exclusive = pairwise_all_exclusive(&deduped);
        let same_target = deduped.iter().all(|c| c.target == deduped[0].target);

        if same_target || all_cfg_exclusive {
            Resolution {
                candidates: deduped,
                status: ResStatus::ResolvedSet,
            }
        } else {
            Resolution {
                candidates: deduped,
                status: ResStatus::Ambiguous,
            }
        }
    }

    fn visible(&self, binding: &Binding, q: &ResolveQuery, _trav: &TraversalCtx) -> bool {
        self.vis_reaches(&binding.vis, binding.scope, q.from)
            .unwrap_or(false)
    }

    fn glob_edge_visible(
        &self,
        edge: &Edge,
        q: &ResolveQuery,
        _trav: &TraversalCtx,
    ) -> VisibilityDecision {
        match self.vis_reaches(&edge.vis, edge.from, q.from) {
            Some(true) => VisibilityDecision::Visible,
            Some(false) => VisibilityDecision::Hidden,
            None => VisibilityDecision::Unknown,
        }
    }

    fn member_visible(
        &self,
        binding: &Binding,
        q: &ResolveQuery,
        _trav: &TraversalCtx,
    ) -> VisibilityDecision {
        match self.vis_reaches(&binding.vis, binding.scope, q.from) {
            Some(true) => VisibilityDecision::Visible,
            Some(false) => VisibilityDecision::Hidden,
            None => VisibilityDecision::Unknown,
        }
    }

    fn anchor(&self, anchor: &Anchor, from: ScopeId) -> Option<(ScopeId, NamespaceId)> {
        match anchor.kind {
            AnchorKind::CrateRoot => self.crate_root(from).map(|s| (s, NS_TYPE)),
            AnchorKind::SelfMod => self.enclosing_module(from).map(|s| (s, NS_TYPE)),
            AnchorKind::Super(n) => self.super_n(from, n).map(|s| (s, NS_TYPE)),
            AnchorKind::UsePath => {
                // A bare leading ident in a `use` path. 2015: crate-root-relative.
                // 2018+: lexical/extern-prelude — Phase 1 anchors at the enclosing
                // module (the lexical module), which is the recall-safe start (an
                // extern-prelude ident that is NOT an in-repo item falls through).
                if self.is_2018_plus() {
                    self.enclosing_module(from).map(|s| (s, NS_TYPE))
                } else {
                    self.crate_root(from).map(|s| (s, NS_TYPE))
                }
            }
            AnchorKind::LeadingColon => {
                // `::x`. 2018+: extern-prelude-based. 2015: crate-root-based.
                if self.is_2018_plus() {
                    // The populator records the prelude scope; without it we fall
                    // through (anchor conservatively over a wrong guess).
                    anchor.prelude.map(|s| (s, NS_TYPE))
                } else {
                    self.crate_root(from).map(|s| (s, NS_TYPE))
                }
            }
            AnchorKind::Bare => {
                // A bare expression-position leading ident. In Phase 1 these are
                // handled by `resolve` (the lexical walk), not `resolve_path`; if
                // an anchored Bare reaches here, start at the enclosing module
                // (recall-safe — never a wrong cross-module guess).
                self.enclosing_module(from).map(|s| (s, NS_TYPE))
            }
        }
    }

    fn extern_crate_root(
        &self,
        graph: &ScopeGraph,
        name: &str,
        anchor: &Anchor,
        from: ScopeId,
    ) -> Option<ScopeId> {
        // Eligibility: only a 2018+ extern-prelude ROOT may name a sibling crate.
        // `crate::`/`self::`/`super::` anchor inside THIS crate; `LeadingColon`
        // (`::other::X`) is excluded in v1 (spec §8); a 2015 `use sibling::X` needs
        // an `extern crate` binding (modeled at walk/items.rs:160), so the bare
        // fallback must not invent one.
        if !self.is_2018_plus() {
            return None;
        }
        if !matches!(anchor.kind, AnchorKind::UsePath | AnchorKind::Bare) {
            return None;
        }
        // P3 (per-crate dep gate): resolve `name` ONLY through the consuming crate's
        // in-repo dependency map. A crate can name another in-repo crate iff it
        // actually depends on it; each map value is one specific target root.
        let consuming_root = crate_root_of(graph, from)?;
        graph
            .crate_deps_by_root
            .get(&consuming_root)?
            .get(&normalize_crate_ident(name))
            .copied()
    }
}

/// True iff every pair of candidates is provably cfg-exclusive (distinct worlds).
fn pairwise_all_exclusive(cands: &[Candidate]) -> bool {
    if cands.len() < 2 {
        return false;
    }
    for i in 0..cands.len() {
        for j in (i + 1)..cands.len() {
            if !cands[i].cond.exclusive(&cands[j].cond) {
                return false;
            }
        }
    }
    true
}

/// Normalize a crate/dependency identifier to the Rust path-identifier form: a
/// Cargo dependency name may carry hyphens (`my-crate`) while a `use` path writes
/// underscores (`my_crate`). Used to key `ScopeGraph::crate_deps_by_root` and to
/// normalize the leading-segment query in `extern_crate_root` identically.
pub(crate) fn normalize_crate_ident(name: &str) -> String {
    name.replace('-', "_")
}

/// Climb `graph.scope(id).parent` from `from` to its enclosing `Root` scope and
/// return it. A free helper (the trait hook receives `graph` as a parameter, not
/// `RustPolicy`'s borrowed graph). Returns `None` only for a malformed graph with
/// no Root ancestor.
pub(crate) fn crate_root_of(graph: &ScopeGraph, from: ScopeId) -> Option<ScopeId> {
    let mut cur = Some(from);
    while let Some(id) = cur {
        let s = graph.scope(id)?;
        if matches!(s.kind, ScopeKind::Root) {
            return Some(id);
        }
        cur = s.parent;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{
        Anchor, ResolutionPolicy, Scope, ScopeExtent, ScopeId, ScopeKind, SourceLoc, Span,
    };

    fn root_scope(id: u32) -> Scope {
        Scope {
            id: ScopeId(id),
            kind: ScopeKind::Root,
            parent: None,
            extents: vec![ScopeExtent {
                file: crate::name_resolution::types::FileId(id),
                range: Span {
                    lo: SourceLoc {
                        file: crate::name_resolution::types::FileId(id),
                        byte: 0,
                    },
                    hi: SourceLoc {
                        file: crate::name_resolution::types::FileId(id),
                        byte: 10,
                    },
                },
                cond: None,
                occ: None,
            }],
            owner_item: None,
            cond: None,
        }
    }

    fn module_under(id: u32, parent: u32) -> Scope {
        let mut s = root_scope(id);
        s.kind = ScopeKind::Module;
        s.parent = Some(ScopeId(parent));
        s
    }

    /// A graph: Root(0) [crate a] with module(2); Root(1) [crate b]. a depends on b.
    fn two_crate_graph() -> ScopeGraph {
        let mut g = ScopeGraph::new();
        g.edition = 2021;
        g.add_scope(root_scope(0));
        g.add_scope(root_scope(1));
        g.add_scope(module_under(2, 0));
        let mut a_deps = std::collections::BTreeMap::new();
        a_deps.insert("b_crate".to_string(), ScopeId(1));
        g.crate_deps_by_root.insert(ScopeId(0), a_deps);
        g
    }

    #[test]
    fn extern_crate_root_resolves_declared_dep_from_use_path() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        // From module(2) inside crate a, a UsePath leading `b_crate` -> b's Root(1).
        let got = policy.extern_crate_root(&g, "b_crate", &Anchor::use_path_2015(), ScopeId(2));
        assert_eq!(got, Some(ScopeId(1)));
    }

    #[test]
    fn extern_crate_root_declines_2015() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2015); // 2015: the bare fallback must not fire.
        assert_eq!(
            policy.extern_crate_root(&g, "b_crate", &Anchor::use_path_2015(), ScopeId(2)),
            None
        );
    }

    #[test]
    fn extern_crate_root_declines_crate_self_super_anchors() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        for anchor in [Anchor::crate_root(), Anchor::self_mod(), Anchor::super_n(1)] {
            assert_eq!(
                policy.extern_crate_root(&g, "b_crate", &anchor, ScopeId(2)),
                None,
                "crate::/self::/super:: anchor inside THIS crate, not a sibling"
            );
        }
    }

    #[test]
    fn extern_crate_root_declines_leading_colon() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        assert_eq!(
            policy.extern_crate_root(&g, "b_crate", &Anchor::leading_colon_2018(99), ScopeId(2)),
            None,
            "LeadingColon is excluded in v1 (spec §8)"
        );
    }

    #[test]
    fn extern_crate_root_declines_undeclared_name() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        // `other` is not in a's dep map → decline (per-crate dep gate, P3).
        assert_eq!(
            policy.extern_crate_root(&g, "other", &Anchor::use_path_2015(), ScopeId(2)),
            None
        );
    }

    #[test]
    fn extern_crate_root_is_per_consuming_crate() {
        let g = two_crate_graph();
        let policy = RustPolicy::new(&g, 2021);
        // From crate b's Root(1) there is no dep map entry → `b_crate` declines.
        assert_eq!(
            policy.extern_crate_root(&g, "b_crate", &Anchor::use_path_2015(), ScopeId(1)),
            None,
            "the extern prelude is per-crate; b does not declare b_crate"
        );
    }

    #[test]
    fn extern_crate_root_normalizes_hyphen() {
        let mut g = two_crate_graph();
        // a depends on a hyphenated in-repo crate keyed underscore in the map.
        g.crate_deps_by_root
            .get_mut(&ScopeId(0))
            .unwrap()
            .insert("my_dep".to_string(), ScopeId(1));
        let policy = RustPolicy::new(&g, 2021);
        // A `use my-dep::X` writes `my_dep`; either spelling normalizes to the key.
        assert_eq!(
            policy.extern_crate_root(&g, "my-dep", &Anchor::use_path_2015(), ScopeId(2)),
            Some(ScopeId(1))
        );
    }

    #[test]
    fn crate_root_of_climbs_to_root() {
        let g = two_crate_graph();
        assert_eq!(crate_root_of(&g, ScopeId(2)), Some(ScopeId(0)));
        assert_eq!(crate_root_of(&g, ScopeId(0)), Some(ScopeId(0)));
    }
}

#[cfg(test)]
mod vis_reaches_tests {
    use super::*;
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{
        BindTarget, Binding, FileId, ItemId, PolicyBlob, PolicyQueryCtx, ResolveQuery, Scope,
        ScopeExtent, SourceLoc, Span, Target, Vis,
    };

    fn loc(byte: usize) -> SourceLoc {
        SourceLoc {
            file: FileId(0),
            byte,
        }
    }

    fn scope(id: u32, kind: ScopeKind, parent: Option<u32>) -> Scope {
        Scope {
            id: ScopeId(id),
            kind,
            parent: parent.map(ScopeId),
            extents: vec![ScopeExtent {
                file: FileId(0),
                range: Span {
                    lo: loc(0),
                    hi: loc(100),
                },
                cond: None,
                occ: None,
            }],
            owner_item: None,
            cond: None,
        }
    }

    fn vis(kind: VisKindId, restrict: Option<ScopeId>) -> Vis {
        Vis {
            kind,
            restrict,
            payload: PolicyBlob::default(),
        }
    }

    fn binding(scope: ScopeId, name: &str, vis: Vis) -> Binding {
        Binding {
            scope,
            name: name.to_string(),
            ns: NS_TYPE,
            target: BindTarget::Resolved(Target::Item {
                id: ItemId(1),
                ns: NS_TYPE,
                owns: None,
                callable: false,
            }),
            vis,
            cond: None,
            vis_extents: vec![],
        }
    }

    fn query(from: ScopeId) -> ResolveQuery {
        ResolveQuery {
            name: "P".to_string(),
            ns: NS_TYPE,
            from,
            at: loc(1),
            cfg: Default::default(),
            ctx: PolicyQueryCtx::default(),
        }
    }

    #[test]
    fn vis_reaches_matches_visible_and_preserves_unknown_pub_in() {
        let mut graph = ScopeGraph::new();
        graph.add_scope(scope(0, ScopeKind::Root, None));
        graph.add_scope(scope(1, ScopeKind::Module, Some(0)));
        let policy = RustPolicy::new(&graph, 2021);

        let pub_vis = vis(VIS_PUB, None);
        let priv_vis = vis(VIS_PRIV, None);
        let unknown_pub_in = vis(VIS_PUB_IN, None);

        assert_eq!(
            policy.vis_reaches(&pub_vis, ScopeId(1), ScopeId(0)),
            Some(true)
        );
        let pub_binding = binding(ScopeId(1), "P", pub_vis);
        assert_eq!(
            policy.visible(&pub_binding, &query(ScopeId(0)), &TraversalCtx::default()),
            policy
                .vis_reaches(&pub_binding.vis, pub_binding.scope, ScopeId(0))
                .unwrap_or(false)
        );

        assert_eq!(
            policy.vis_reaches(&priv_vis, ScopeId(1), ScopeId(0)),
            Some(false)
        );
        assert_eq!(
            policy.vis_reaches(&priv_vis, ScopeId(1), ScopeId(1)),
            Some(true)
        );
        let priv_binding = binding(ScopeId(1), "H", priv_vis);
        assert_eq!(
            policy.visible(&priv_binding, &query(ScopeId(0)), &TraversalCtx::default()),
            policy
                .vis_reaches(&priv_binding.vis, priv_binding.scope, ScopeId(0))
                .unwrap_or(false)
        );

        assert_eq!(
            policy.vis_reaches(&unknown_pub_in, ScopeId(1), ScopeId(0)),
            None
        );
    }

    #[test]
    fn member_visible_maps_vis_reaches_tristate() {
        let mut graph = ScopeGraph::new();
        graph.add_scope(scope(0, ScopeKind::Root, None));
        graph.add_scope(scope(1, ScopeKind::Module, Some(0)));
        let policy = RustPolicy::new(&graph, 2021);
        let trav = TraversalCtx::default();

        // pub member, viewed from outside its module -> Visible.
        let pub_binding = binding(ScopeId(1), "P", vis(VIS_PUB, None));
        assert_eq!(
            policy.member_visible(&pub_binding, &query(ScopeId(0)), &trav),
            VisibilityDecision::Visible
        );
        // private member, viewed from OUTSIDE its module -> Hidden (vis_reaches Some(false)).
        let priv_binding = binding(ScopeId(1), "H", vis(VIS_PRIV, None));
        assert_eq!(
            policy.member_visible(&priv_binding, &query(ScopeId(0)), &trav),
            VisibilityDecision::Hidden
        );
        // ...the same private member viewed from INSIDE -> Visible.
        assert_eq!(
            policy.member_visible(&priv_binding, &query(ScopeId(1)), &trav),
            VisibilityDecision::Visible
        );
        // pub(in <unresolved>) member -> Unknown (vis_reaches None -> must fail closed).
        let pub_in_binding = binding(ScopeId(1), "U", vis(VIS_PUB_IN, None));
        assert_eq!(
            policy.member_visible(&pub_in_binding, &query(ScopeId(0)), &trav),
            VisibilityDecision::Unknown
        );
    }
}
