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

use crate::name_resolution::types::{
    Anchor, AnchorKind, Binding, Candidate, EdgeKindId, NamespaceId, ResStatus, Resolution,
    ResolutionPolicy, ResolveQuery, ScopeGraph, ScopeId, ScopeKind, TraversalCtx, VisKindId,
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
    fn crate_root(&self, scope: ScopeId) -> Option<ScopeId> {
        let mut cur = Some(scope);
        let mut last_root = None;
        while let Some(id) = cur {
            let s = self.graph.scope(id)?;
            if matches!(s.kind, ScopeKind::Root) {
                last_root = Some(id);
                break;
            }
            cur = s.parent;
            if cur.is_none() {
                // A scope with no Root ancestor is malformed; the topmost is it.
                last_root = Some(id);
            }
        }
        last_root
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
        // The query origin and the binding's defining scope drive Rust visibility.
        let from = q.from;
        let def_scope = binding.scope;
        let def_module = match self.enclosing_module(def_scope) {
            Some(m) => m,
            None => return false, // malformed → not visible → fall through
        };
        match binding.vis.kind {
            VIS_PUB => {
                // `pub` is visible from anywhere THE PATH REACHES. For local
                // bare-name lookup the engine only ever consults bindings it
                // reached lexically (already in scope), so `pub` ⇒ visible. For
                // a path member lookup, reaching the scope at all means it is on
                // an accessible path. Either way, `pub` ⇒ visible.
                true
            }
            VIS_PUB_CRATE => {
                // Visible anywhere in the DEFINING crate; not across a crate
                // boundary (no pub facade is modeled at this hook — a facade is a
                // separate pub re-export Binding the engine resolves explicitly).
                match (self.crate_root(def_scope), self.crate_root(from)) {
                    (Some(a), Some(b)) => a == b,
                    _ => false,
                }
            }
            VIS_PUB_SUPER => {
                // Visible in the PARENT MODULE's subtree. The parent module is
                // `super` of the defining module.
                match self.super_n(def_scope, 1) {
                    Some(parent_mod) => self.is_within(from, parent_mod),
                    None => false,
                }
            }
            VIS_PUB_IN => {
                // Visible only inside the `restrict` scope's subtree.
                match binding.vis.restrict {
                    Some(r) => self.is_within(from, r),
                    None => false, // pub(in) with no recorded path → fall through
                }
            }
            VIS_PRIV => {
                // Private: visible in the defining module and its descendants.
                self.is_within(from, def_module)
            }
            // Unknown visibility kind → NOT visible (recall-safe fall-through).
            _ => false,
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
