//! Task 2 TDD: the resolution engine + the Rust policy (the §7 recall-safety matrix).
//!
//! **Cardinal rule (spec §7):** the engine RESOLVES-OR-FALLS-THROUGH and NEVER
//! emits a wrong target. Every case here that is *meant to fall through* asserts
//! the **status** (`Unresolved`/`Ambiguous`/`Poisoned`/`ResolvedSet`) and the
//! ABSENCE of any wrong outer same-name target — never the outer item.
//!
//! These tests build `Scope`/`Binding`/`Edge`/`Target`/`MacroWildcard` graphs
//! **by hand** (there is no AST populator yet — that is Task 3). This isolates
//! the engine's recall-safety logic from population.
//!
//! ## Scope note on the "glob (engine)" tests
//! The `glob_engine_*` tests exercise the engine's *combination* over `Glob`
//! edges whose members are **already known** (`to = Resolved(Scope)` — the
//! target scope's bindings are present in the graph). They are NOT Rust
//! `use a::*` member *expansion* (that is *deferred* — the populator poisons it,
//! see the `*_poison*` tests + Task 3). The engine support for known-member
//! globs is intentional and not scope creep: it is the language-neutral
//! combination machinery that Go `import .`, Java on-demand-import, and a
//! future Phase-2 expanded Rust glob all reuse.

use prism::name_resolution::engine::{resolve, resolve_path, ScopeGraph};
use prism::name_resolution::rust_policy::{
    self, RustPolicy, NS_MACRO, NS_TYPE, NS_VALUE, VIS_PRIV, VIS_PUB, VIS_PUB_CRATE, VIS_PUB_IN,
    VIS_PUB_SUPER,
};
use prism::name_resolution::types::{
    Anchor, BindTarget, Binding, CfgCond, CfgCtx, Edge, FileId, Ident, ItemId, NamespaceId,
    PolicyQueryCtx, RawPath, ResStatus, Resolution, ResolveQuery, Scope, ScopeExtent, ScopeId,
    ScopeKind, SourceLoc, Span, Target, Vis,
};

// ── hand-graph builders ──────────────────────────────────────────────────────

const F: FileId = FileId(0);

/// A full-file span `[0, 1_000_000)` — every realistic byte falls inside it.
fn wide_span() -> Span {
    Span {
        lo: SourceLoc { file: F, byte: 0 },
        hi: SourceLoc {
            file: F,
            byte: 1_000_000,
        },
    }
}

fn span(lo: usize, hi: usize) -> Span {
    Span {
        lo: SourceLoc { file: F, byte: lo },
        hi: SourceLoc { file: F, byte: hi },
    }
}

fn extent(range: Span) -> ScopeExtent {
    ScopeExtent {
        file: F,
        range,
        cond: None,
        occ: None,
    }
}

fn scope(id: u32, kind: ScopeKind, parent: Option<u32>) -> Scope {
    Scope {
        id: ScopeId(id),
        kind,
        parent: parent.map(ScopeId),
        extents: vec![extent(wide_span())],
        owner_item: None,
        cond: None,
    }
}

fn vis(kind: u16) -> Vis {
    Vis {
        kind,
        restrict: None,
        payload: Default::default(),
    }
}

fn vis_in(kind: u16, restrict: u32) -> Vis {
    Vis {
        kind,
        restrict: Some(ScopeId(restrict)),
        payload: Default::default(),
    }
}

/// A callable `Item` binding in `scope`, named `name`, in the Value namespace.
fn item_binding(scope: u32, name: &str, item: u32, vis_kind: u16) -> Binding {
    Binding {
        scope: ScopeId(scope),
        name: name.to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::Item {
            id: ItemId(item),
            ns: NS_VALUE,
            owns: None,
            callable: true,
        }),
        vis: vis(vis_kind),
        cond: None,
        vis_extents: vec![wide_span()],
    }
}

/// A `Target::Local` binding (param/`let`/closure) in `scope`, Value namespace.
fn local_binding(scope: u32, name: &str, ordinal: u32) -> Binding {
    use prism::name_resolution::types::BindingRef;
    Binding {
        scope: ScopeId(scope),
        name: name.to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::Local(BindingRef {
            scope: ScopeId(scope),
            ordinal,
        })),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    }
}

/// Build a `RustPolicy` over a graph for a given edition (2015 | 2018).
fn policy_for(graph: &ScopeGraph, edition: u16) -> RustPolicy<'_> {
    RustPolicy::new(graph, edition)
}

/// A bare-name Value-namespace query from `from`, at byte 50 (inside wide_span).
fn bare_value_query(name: &str, from: u32) -> ResolveQuery {
    query_at(name, NS_VALUE, from, 50)
}

fn query_at(name: &str, ns: NamespaceId, from: u32, byte: usize) -> ResolveQuery {
    ResolveQuery {
        name: name.to_string(),
        ns,
        from: ScopeId(from),
        at: SourceLoc { file: F, byte },
        cfg: CfgCtx::default(),
        ctx: PolicyQueryCtx::default(),
    }
}

/// Assert that a resolution did NOT pick a particular item id (recall-safety guard).
fn assert_not_item(res: &Resolution, forbidden: u32) {
    for c in &res.candidates {
        if let Target::Item { id, .. } = &c.target {
            assert_ne!(
                id.0, forbidden,
                "RECALL-SAFETY VIOLATION: resolved to a forbidden outer item {forbidden}"
            );
        }
    }
}

/// Assert the single resolved candidate is `Item{id}`.
fn assert_resolved_item(res: &Resolution, id: u32) {
    assert_eq!(res.status, ResStatus::Resolved, "expected Resolved status");
    assert_eq!(res.candidates.len(), 1, "expected exactly one candidate");
    match &res.candidates[0].target {
        Target::Item { id: got, .. } => assert_eq!(got.0, id, "wrong resolved item"),
        other => panic!("expected Item, got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// MODULE-BOUNDARY STOP (§4 round6-B1) — the single worst-outcome guard.
// ═══════════════════════════════════════════════════════════════════════════

/// Root `fn start`; child Module `m`; a bare `start` call inside `m` does NOT
/// see the root `start` (Rust does not inherit unqualified names across a module
/// boundary). MUST be `Unresolved`, NOT the root `start`.
#[test]
fn test_module_boundary_stop_bare_name_unresolved() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    // root `fn start` (item 100)
    g.add_binding(item_binding(0, "start", 100, VIS_PUB));

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("start", 1), &pol);

    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "bare name must NOT cross the module boundary into the parent"
    );
    assert_not_item(&res, 100);
}

/// `super::start` from `m` DOES reach the root `start` (explicit anchor crosses
/// the module boundary). Asserted via `resolve_path` with a `super` anchor.
#[test]
fn test_super_anchor_reaches_parent_start() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_binding(item_binding(0, "start", 100, VIS_PUB));

    let pol = policy_for(&g, 2018);
    // `super::start` — a single-segment path `["start"]` anchored at `super` of scope 1.
    let res = resolve_path(
        &g,
        &RawPath(vec!["start".to_string()]),
        NS_VALUE,
        &Anchor::super_n(1),
        ScopeId(1),
        NS_VALUE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&res, 100);
}

/// A bare name DOES cross `Block`/`Callable` lexical parents up to the enclosing
/// Module: Root `fn outer`; Module `m`; Callable scope inside `m`; Block inside
/// the callable. A bare `helper` (a Value item defined in module `m`) resolves
/// from the inner block — the walk crosses Block→Callable→Module(stop, found here).
#[test]
fn test_bare_name_crosses_blocks_up_to_module() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Callable, Some(1)));
    g.add_scope(scope(3, ScopeKind::Block, Some(2)));
    // `helper` defined at module scope 1
    g.add_binding(item_binding(1, "helper", 200, VIS_PUB));

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("helper", 3), &pol);
    assert_resolved_item(&res, 200);
}

// ═══════════════════════════════════════════════════════════════════════════
// LOCAL SHADOW (§3a / §7 local-binding shadow)
// ═══════════════════════════════════════════════════════════════════════════

/// A Block with a `Target::Local` `f` + an outer `Item fn f` (same module, via
/// the block's callable parent which lives in the module). Bare `f` in the block
/// resolves to the LOCAL — narrowing stops, the result is NOT a callable item.
#[test]
fn test_local_shadow_stops_at_local_not_outer_fn() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer free fn `f` (item 300) at module scope
    g.add_binding(item_binding(1, "f", 300, VIS_PUB));
    // local `f` in the block (a closure binding, ordinal 0)
    g.add_binding(local_binding(2, "f", 0));

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("f", 2), &pol);

    assert_eq!(res.status, ResStatus::Resolved, "the local shadow resolves");
    assert_eq!(res.candidates.len(), 1);
    match &res.candidates[0].target {
        Target::Local(_) => {} // correct: the local won, narrowing stops here
        other => panic!("expected the Local to shadow the outer fn, got {other:?}"),
    }
    assert_not_item(&res, 300); // MUST NOT reach the outer callable fn
}

// ═══════════════════════════════════════════════════════════════════════════
// POISON-NOT-SKIP (§3.4 step 2, §4.3b, §7): deferred glob / macro wildcard /
// pending use → Poisoned, NEVER the outer same-name.
// ═══════════════════════════════════════════════════════════════════════════

/// A scope with a *deferred* `Glob` edge (`to = Pending`, members unknown) + an
/// outer `Item x`. Bare `x` → `Poisoned`, NOT the outer `x`.
#[test]
fn test_deferred_glob_poisons_not_outer() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer `Item x` (item 400) at module scope
    g.add_binding(item_binding(1, "x", 400, VIS_PUB));
    // a DEFERRED glob edge out of the block: `use a::*` whose target is Pending
    g.add_edge(Edge {
        from: ScopeId(2),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Pending(RawPath(vec!["a".to_string()]), Anchor::bare()),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 0,
        vis_range: None,
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("x", 2), &pol);

    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "a deferred glob in scope must POISON the name, not fall through to the outer x"
    );
    assert_not_item(&res, 400);
}

/// A scope carrying an *unexpanded* name-introducing macro marker (`m!(); …`)
/// over a range + an outer `Item f`. Bare `f` inside the macro's range →
/// `Poisoned` (the wildcard could have introduced `f`). A call OUTSIDE the
/// macro's range is unaffected (resolves nowhere → Unresolved, NOT the outer f
/// because that outer f is across a module boundary).
#[test]
fn test_macro_wildcard_poisons_within_range_only() {
    use prism::name_resolution::graph::MacroWildcard;
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    // a Block inside module `m` where `m!()` was invoked
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer `Item f` at module scope (item 500)
    g.add_binding(item_binding(1, "f", 500, VIS_PUB));
    // unexpanded macro wildcard over bytes [40, 60) in the Value namespace of scope 2
    g.add_macro_wildcard(MacroWildcard {
        scope: ScopeId(2),
        ns: NS_VALUE,
        range: span(40, 60),
    });

    let pol = policy_for(&g, 2018);

    // A call at byte 50 (INSIDE the macro range) → Poisoned.
    let inside = resolve(&g, &query_at("f", NS_VALUE, 2, 50), &pol);
    assert_eq!(
        inside.status,
        ResStatus::Poisoned,
        "a call inside an unexpanded name-introducing macro range must POISON"
    );
    assert_not_item(&inside, 500);

    // A call at byte 80 (OUTSIDE the macro range) → not poisoned by the macro.
    // It finds nothing in scope 2, stops at the module boundary (scope 1 is the
    // parent Module of the block — actually scope 1 IS the module; the block's
    // walk includes the module, so `f` IS found at module scope 1).
    let outside = resolve(&g, &query_at("f", NS_VALUE, 2, 80), &pol);
    assert_eq!(
        outside.status,
        ResStatus::Resolved,
        "a call outside the macro range is unaffected and resolves the module-scope f"
    );
    assert_resolved_item(&outside, 500);
}

/// Macro-wildcard is GLOB-TIER, not before the explicit rib (§4.3b Reading B):
/// a scope with BOTH a covering `MacroWildcard` AND an explicit `fn f` resolves a
/// bare in-range `f` to the EXPLICIT `fn f` — the explicit local binding (step 1)
/// shadows the wildcard exactly as it shadows a deferred glob (step 2). The
/// wildcard only poisons when step 1 found no explicit binding for the name.
#[test]
fn test_macro_wildcard_shadowed_by_explicit_local() {
    use prism::name_resolution::graph::MacroWildcard;
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    // a Block inside module `m` where `m!()` was invoked AND an explicit `fn f` is
    // defined in the SAME scope (item 550).
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // explicit `fn f` at the same scope 2 as the wildcard (item 550)
    g.add_binding(item_binding(2, "f", 550, VIS_PUB));
    // unexpanded macro wildcard over bytes [40, 60) in the Value namespace of scope 2
    g.add_macro_wildcard(MacroWildcard {
        scope: ScopeId(2),
        ns: NS_VALUE,
        range: span(40, 60),
    });

    let pol = policy_for(&g, 2018);
    // A bare `f` at byte 50 (INSIDE the wildcard range) — the explicit `fn f` in the
    // same scope SHADOWS the wildcard → Resolved to item 550 (NOT Poisoned).
    let res = resolve(&g, &query_at("f", NS_VALUE, 2, 50), &pol);
    assert_resolved_item(&res, 550);
}

/// A still-`Pending` *local* `use x` (a named import not yet resolved) + an outer
/// `x` → `Poisoned` (an unresolved local import must not continue outward).
#[test]
fn test_pending_local_use_poisons_not_outer() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer `x` (item 600) at module scope
    g.add_binding(item_binding(1, "x", 600, VIS_PUB));
    // a Pending local `use somewhere::x` in the block that we leave UNRESOLVABLE
    // (its path target `zzz::x` is not in the graph → stays Pending → poisons).
    g.add_binding(Binding {
        scope: ScopeId(2),
        name: "x".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(
            RawPath(vec!["zzz".to_string(), "x".to_string()]),
            Anchor::bare(),
        ),
        vis: vis(VIS_PRIV),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("x", 2), &pol);

    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "a still-Pending local use must POISON, not fall through to the outer x"
    );
    assert_not_item(&res, 600);
}

// ═══════════════════════════════════════════════════════════════════════════
// VISIBILITY: enforce-or-fall-through (the `visible()` hook, hand-built).
// ═══════════════════════════════════════════════════════════════════════════

/// A `private` item in module `m`, resolved FROM A SIBLING module → not visible
/// → fall through (`Unresolved`), NOT a wrong edge to the private item.
/// Structure: Root → {m (scope 1), sib (scope 2)}; private `secret` in `m`;
/// resolve `m::secret` (a path) from `sib`.
#[test]
fn test_private_item_not_visible_from_sibling() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    let mut m = scope(1, ScopeKind::Module, Some(0));
    m.owner_item = Some(ItemId(1));
    g.add_scope(m);
    g.add_scope(scope(2, ScopeKind::Module, Some(0)));
    // `m` is reachable as a Scope binding `m` in the root (so the path prefix resolves)
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "m".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // private `secret` in m (item 700)
    g.add_binding(item_binding(1, "secret", 700, VIS_PRIV));

    let pol = policy_for(&g, 2018);
    // `m::secret` from the sibling scope 2.
    let res = resolve_path(
        &g,
        &RawPath(vec!["m".to_string(), "secret".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(2),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "a private item is not visible from a sibling — fall through, not a wrong edge"
    );
    assert_not_item(&res, 700);
}

/// `pub(self)` ≡ private: the populator maps `pub(self)` to the private vis-kind
/// (`VIS_PRIV`), so a `pub(self)` item is NOT visible from a sibling — exactly
/// like a plain private item. This pins that equivalence at the policy hook.
/// Same structure as `test_private_item_not_visible_from_sibling`.
#[test]
fn test_pub_self_equiv_private_not_visible_from_sibling() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // m
    g.add_scope(scope(2, ScopeKind::Module, Some(0))); // sibling
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "m".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // `pub(self) secret` in m — `pub(self)` maps to the private vis-kind (item 750).
    g.add_binding(item_binding(1, "secret", 750, VIS_PRIV));

    let pol = policy_for(&g, 2018);
    let res = resolve_path(
        &g,
        &RawPath(vec!["m".to_string(), "secret".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(2),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "pub(self) ≡ private: not visible from a sibling — fall through, not a wrong edge"
    );
    assert_not_item(&res, 750);
}

/// `pub(super)` item is visible from the parent's subtree only.
/// Root → m (scope1) → inner (scope2); `pub(super)` item `g` in `inner`.
/// Visible from `m` (the parent) and its subtree; NOT visible from a sibling of m.
///
/// The negative path (`from_sib`) fully resolves the prefix `crate::m::inner` so
/// that the resolution reaches the `pub(super)` binding on `g` — it is the
/// visibility *denial* (not a missing prefix) that produces the non-`Resolved`
/// status. The `m` Scope binding at the root is required for this.
///
/// Sanity reasoning: if `visible()` were stubbed to always return `true`, the
/// negative assertion (`assert_not_item(&from_sib, 800)`) would fail because
/// `g` would then be returned as a `Resolved` candidate — confirming that this
/// test now genuinely exercises the denial path.
#[test]
fn test_pub_super_visible_from_parent_subtree_only() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // m
    g.add_scope(scope(2, ScopeKind::Module, Some(1))); // inner (child of m)
    g.add_scope(scope(3, ScopeKind::Module, Some(0))); // sib (sibling of m)

    // path prefix bindings to make the full crate::m::inner::g path resolvable.
    // `m` Scope binding in the root — required so `from_sib` can walk past the
    // first segment and reach the `pub(super)` binding on `g`.
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "m".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // `inner` Scope binding in m
    g.add_binding(Binding {
        scope: ScopeId(1),
        name: "inner".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(2))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // pub(super) `g` in inner — super of inner == m (scope1)
    g.add_binding(Binding {
        scope: ScopeId(2),
        name: "g".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::Item {
            id: ItemId(800),
            ns: NS_VALUE,
            owns: None,
            callable: true,
        }),
        vis: vis(VIS_PUB_SUPER),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);

    // From m (scope 1): `inner::g` → visible (m is the super-target's subtree root).
    let from_m = resolve_path(
        &g,
        &RawPath(vec!["inner".to_string(), "g".to_string()]),
        NS_VALUE,
        &Anchor::self_mod(),
        ScopeId(1),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&from_m, 800);

    // From sib (scope 3): `crate::m::inner::g` — the prefix m::inner resolves
    // fully (the `m` and `inner` Scope bindings are present), reaching `inner`
    // (scope 2). The final segment `g` IS found there, but `pub(super)` restricts
    // it to m's subtree; `sib` is NOT in m's subtree → the visibility hook denies
    // it → Unresolved (never `Resolved`, never item 800).
    let from_sib = resolve_path(
        &g,
        &RawPath(vec!["m".to_string(), "inner".to_string(), "g".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(3),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_ne!(
        from_sib.status,
        ResStatus::Resolved,
        "pub(super) g must NOT be visible from a sibling subtree — the visibility denial fires"
    );
    assert_not_item(&from_sib, 800);
}

/// `pub(in path)` visible only inside `path`.
/// Root → a (scope1) → b (scope2) → c (scope3); `pub(in a)` item `t` in c.
/// Visible from inside `a`'s subtree (e.g. from b), NOT from outside a.
#[test]
fn test_pub_in_path_visible_only_inside_path() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // a
    g.add_scope(scope(2, ScopeKind::Module, Some(1))); // b
    g.add_scope(scope(3, ScopeKind::Module, Some(2))); // c
    g.add_scope(scope(4, ScopeKind::Module, Some(0))); // outside (sibling of a)
                                                       // prefix: `c` Scope binding in b
    g.add_binding(Binding {
        scope: ScopeId(2),
        name: "c".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(3))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // pub(in a) `t` in c — restrict = scope 1 (a)
    g.add_binding(Binding {
        scope: ScopeId(3),
        name: "t".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::Item {
            id: ItemId(900),
            ns: NS_VALUE,
            owns: None,
            callable: true,
        }),
        vis: vis_in(VIS_PUB_IN, 1),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);

    // From b (scope 2, inside a's subtree): `c::t` → visible.
    let from_b = resolve_path(
        &g,
        &RawPath(vec!["c".to_string(), "t".to_string()]),
        NS_VALUE,
        &Anchor::self_mod(),
        ScopeId(2),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&from_b, 900);

    // From outside (scope 4): not inside `a` → not visible.
    let from_outside = resolve_path(
        &g,
        &RawPath(vec!["c".to_string(), "t".to_string()]),
        NS_VALUE,
        &Anchor::self_mod(),
        ScopeId(4),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    // The prefix `c` isn't even in scope 4; but even if it were, t is pub(in a).
    assert_ne!(from_outside.status, ResStatus::Resolved);
    assert_not_item(&from_outside, 900);
}

/// `pub(crate)`: visible anywhere in the defining crate, NOT through a sibling
/// workspace crate unless an actually-`pub` facade re-exports it.
/// Two crate roots (scope 0 = crate A, scope 10 = crate B). `pub(crate)` item `k`
/// in a module of crate A. Visible from elsewhere in crate A; NOT from crate B.
#[test]
fn test_pub_crate_visible_within_crate_not_across() {
    let mut g = ScopeGraph::new();
    // crate A
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // A::m
    g.add_scope(scope(2, ScopeKind::Module, Some(0))); // A::other
                                                       // crate B
    g.add_scope(scope(10, ScopeKind::Root, None));
    // prefix: `m` Scope binding in A root
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "m".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // pub(crate) `k` in A::m (item 1000)
    g.add_binding(item_binding(1, "k", 1000, VIS_PUB_CRATE));

    let pol = policy_for(&g, 2018);

    // From A::other (scope 2): visible (same crate).
    let from_a = resolve_path(
        &g,
        &RawPath(vec!["m".to_string(), "k".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(2),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&from_a, 1000);

    // From crate B root (scope 10): NOT visible (different crate, no pub facade).
    let from_b = resolve_path(
        &g,
        &RawPath(vec!["m".to_string(), "k".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(10),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_ne!(
        from_b.status,
        ResStatus::Resolved,
        "pub(crate) must not be visible across a crate boundary without a pub facade"
    );
    assert_not_item(&from_b, 1000);
}

/// Re-export privacy case (i): a `pub use` of a *private* item → still not
/// visible → fall through. A `pub use` does NOT launder privacy.
/// Root → m (scope1); private item `secret` (item 1100) in m; root re-exports it
/// `pub use m::secret as r`. Resolving `r` from a sibling must NOT reach item 1100.
#[test]
fn test_pub_use_of_private_does_not_launder() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Module, Some(0))); // sibling resolving `r`
                                                       // private `secret` in m
    g.add_binding(item_binding(1, "secret", 1100, VIS_PRIV));
    // `pub use m::secret as r` in the ROOT — a public re-export Binding (Pending).
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "r".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(
            RawPath(vec!["m".to_string(), "secret".to_string()]),
            Anchor::crate_root(),
        ),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // prefix `m`
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "m".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    // resolve `r` (bare, from the sibling scope 2 — but `r` lives in root, across
    // the module boundary; use crate::r path to even reach it).
    let res = resolve_path(
        &g,
        &RawPath(vec!["r".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(2),
        NS_VALUE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    // The re-export points at a PRIVATE item not visible to the sibling → the
    // chased target fails visibility → fall through. NEVER item 1100.
    assert_ne!(
        res.status,
        ResStatus::Resolved,
        "pub use of a private item does not launder privacy"
    );
    assert_not_item(&res, 1100);
}

/// Re-export privacy case (ii): a *public* item in a *private module*,
/// re-exported via `pub use` from an accessible scope → RESOLVED through the
/// re-export (the item is pub; only its module path is private — the facade).
/// Root → priv_mod (scope1, private); `pub fn real` (item 1200) in priv_mod;
/// root `pub use priv_mod::real as facade`. Resolving `facade` (crate path) →
/// resolves to item 1200.
#[test]
fn test_pub_use_of_public_in_private_module_resolves_through_facade() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    let mut pm = scope(1, ScopeKind::Module, Some(0));
    pm.owner_item = Some(ItemId(1));
    g.add_scope(pm);
    // `priv_mod` itself is private (the module binding is private), but `real` is pub.
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "priv_mod".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PRIV),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // pub `real` in priv_mod (item 1200)
    g.add_binding(item_binding(1, "real", 1200, VIS_PUB));
    // root `pub use priv_mod::real as facade`
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "facade".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(
            RawPath(vec!["priv_mod".to_string(), "real".to_string()]),
            Anchor::crate_root(),
        ),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    // resolve `crate::facade` from the root scope itself.
    let res = resolve_path(
        &g,
        &RawPath(vec!["facade".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(0),
        NS_VALUE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&res, 1200);
}

/// Outer same-name decoy: an INACCESSIBLE explicit binding at the resolving rib
/// plus an outer callable of the SAME name. The engine must NOT continue outward
/// to the outer callable — the inaccessible inner binding wins the name race then
/// fails visibility, so resolution falls through (`Unresolved`), NOT the outer
/// target. Structure: Module m (scope1) has a private `dec` (item 1300); an outer
/// crate fn `dec` (item 1301) exists at root. We resolve `m::dec` from a SIBLING
/// module so the private `dec` is the name-race winner yet fails visibility; the
/// engine must NOT fall back to the root `dec`.
#[test]
fn test_inaccessible_inner_decoy_does_not_fall_to_outer() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // m holds the private decoy
    g.add_scope(scope(2, ScopeKind::Module, Some(0))); // sibling resolving the path
                                                       // outer callable `dec` at ROOT (item 1301) — the WRONG target if we fall through
    g.add_binding(item_binding(0, "dec", 1301, VIS_PUB));
    // private `dec` in m (item 1300) — the inaccessible inner decoy
    g.add_binding(item_binding(1, "dec", 1300, VIS_PRIV));
    // prefix `m` Scope binding in root
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "m".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    // `m::dec` from the sibling scope 2: the name resolves to the private `dec`
    // in m (the path lands there), which fails visibility → fall through.
    // It must NOT silently fall back to the root `dec` (item 1301).
    let res = resolve_path(
        &g,
        &RawPath(vec!["m".to_string(), "dec".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(2),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_ne!(res.status, ResStatus::Resolved);
    assert_not_item(&res, 1301); // the cardinal guard: not the outer callable
    assert_not_item(&res, 1300); // not the private one either (failed visibility)
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPLICIT non-callable / External / Scope shadow decoys (§ "explicit shadow").
// ═══════════════════════════════════════════════════════════════════════════

/// Inner `use ext::x` (→ External) + an outer callable `x` (item). Bare `x` in
/// the inner block resolves to External and the engine must NOT reach the outer
/// callable. (External is a Resolved-to-External — known, not a wrong in-repo edge.)
#[test]
fn test_external_shadow_decoy_does_not_reach_outer_callable() {
    use prism::name_resolution::types::ExternRef;
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer callable `x` at module scope (item 1400)
    g.add_binding(item_binding(1, "x", 1400, VIS_PUB));
    // inner `use ext::x` → External, resolved (a known import)
    g.add_binding(Binding {
        scope: ScopeId(2),
        name: "x".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::External(ExternRef {
            name: "ext::x".to_string(),
        })),
        vis: vis(VIS_PRIV),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("x", 2), &pol);
    // Resolved to External — shadows the outer callable.
    assert_eq!(res.status, ResStatus::Resolved);
    assert_eq!(res.candidates.len(), 1);
    match &res.candidates[0].target {
        Target::External(_) => {}
        other => panic!("expected External to shadow, got {other:?}"),
    }
    assert_not_item(&res, 1400);
}

/// Inner `use crate::m as x` (→ Scope) + outer callable `x`. Bare `x` resolves to
/// the Scope and the engine must NOT reach the outer callable.
#[test]
fn test_scope_alias_shadow_decoy_does_not_reach_outer_callable() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer callable `x` (item 1500)
    g.add_binding(item_binding(1, "x", 1500, VIS_PUB));
    // inner `use crate::m as x` → Scope alias in Value... actually a Scope alias is
    // in the Type namespace. To shadow a Value lookup we need a Value binding.
    // Use a non-callable Value Item (a const) named x as the shadow instead — see
    // the next test; here keep the Scope alias in NS_VALUE to prove the engine
    // returns Scope (non-callable) and stops.
    g.add_binding(Binding {
        scope: ScopeId(2),
        name: "x".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PRIV),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("x", 2), &pol);
    assert_eq!(res.status, ResStatus::Resolved);
    match &res.candidates[0].target {
        Target::Scope(_) => {}
        other => panic!("expected Scope alias to shadow, got {other:?}"),
    }
    assert_not_item(&res, 1500);
}

/// Inner non-callable `Item x` (a const/type) + outer callable `x`. Bare `x`
/// resolves to the non-callable inner item; the consumer will fall through
/// (callable:false), but the engine must NOT reach the outer callable.
#[test]
fn test_noncallable_item_shadow_decoy_does_not_reach_outer_callable() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // outer CALLABLE `x` at module scope (item 1600)
    g.add_binding(item_binding(1, "x", 1600, VIS_PUB));
    // inner NON-callable `x` (a const, item 1601) in the block
    g.add_binding(Binding {
        scope: ScopeId(2),
        name: "x".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::Item {
            id: ItemId(1601),
            ns: NS_VALUE,
            owns: None,
            callable: false,
        }),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("x", 2), &pol);
    assert_resolved_item(&res, 1601); // the inner non-callable wins the name race
    assert_not_item(&res, 1600); // MUST NOT reach the outer callable
                                 // the consumer sees callable:false and falls through (engine's job done)
    match &res.candidates[0].target {
        Target::Item { callable, .. } => assert!(!callable, "must be the non-callable item"),
        other => panic!("expected Item, got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// GLOB (engine, NON-DEFERRED only — see module doc). Known-member globs.
// ═══════════════════════════════════════════════════════════════════════════

/// explicit-beats-glob: an explicit local binding `y` (item 1700) + a non-deferred
/// Glob edge whose target scope also has a `y` (item 1701). The explicit binding
/// wins → Resolved to item 1700.
#[test]
fn test_glob_engine_explicit_beats_glob() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // resolving scope
    g.add_scope(scope(5, ScopeKind::Module, Some(0))); // glob target scope
                                                       // explicit `y` in scope 1 (item 1700)
    g.add_binding(item_binding(1, "y", 1700, VIS_PUB));
    // `y` in the glob target scope 5 (item 1701) — public so accessible
    g.add_binding(item_binding(5, "y", 1701, VIS_PUB));
    // non-deferred glob edge scope1 -> scope5 (members known: scope 5's bindings)
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(5))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 0,
        vis_range: None,
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("y", 1), &pol);
    assert_resolved_item(&res, 1700); // explicit beats glob
}

/// two-glob conflict → Ambiguous. Two non-deferred globs to different scopes,
/// each with a `z` (items 1800 / 1801) → Ambiguous (consumer falls through).
#[test]
fn test_glob_engine_two_glob_conflict_ambiguous() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(5, ScopeKind::Module, Some(0)));
    g.add_scope(scope(6, ScopeKind::Module, Some(0)));
    g.add_binding(item_binding(5, "z", 1800, VIS_PUB));
    g.add_binding(item_binding(6, "z", 1801, VIS_PUB));
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(5))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 0,
        vis_range: None,
    });
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(6))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 1,
        vis_range: None,
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("z", 1), &pol);
    assert_eq!(
        res.status,
        ResStatus::Ambiguous,
        "two globs to different items must be Ambiguous, not a silent pick"
    );
}

/// Already-resolved glob arm soundness completion: a NON-deferred glob edge
/// (scope 1 -> resolved scope 5) whose target member `z` is `pub(in <unresolved>)`
/// (VIS_PUB_IN, restrict None) -> Unknown from scope 1's vantage. The arm must
/// POISON on the Unknown member, not blanket-skip it (representation-independent
/// member-visibility tri-state — the same Unknown-skip hole in another form).
#[test]
fn test_glob_engine_already_resolved_arm_unknown_member_poisons() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(5, ScopeKind::Module, Some(0)));
    g.add_binding(item_binding(5, "z", 1850, VIS_PUB_IN));
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(5))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 0,
        vis_range: None,
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("z", 1), &pol);
    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "an undecidable pub(in) member behind a non-deferred glob must poison, not skip"
    );
}

/// same-target glob dedup → Resolved. Two globs that both reach the SAME item
/// (e.g. two paths to scope 5 which has one `w`) → a single Resolved, not Ambiguous.
#[test]
fn test_glob_engine_same_target_dedup_resolved() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(5, ScopeKind::Module, Some(0)));
    g.add_binding(item_binding(5, "w", 1900, VIS_PUB));
    // two glob edges both pointing at scope 5 (e.g. re-export diamonds)
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(5))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 0,
        vis_range: None,
    });
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(5))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 1,
        vis_range: None,
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("w", 1), &pol);
    assert_resolved_item(&res, 1900); // dedup → single Resolved
}

/// A non-deferred `Glob` edge whose target scope contains a still-`Pending`
/// binding for the looked-up `(name, ns)` → `GlobOutcome::Poison` → engine
/// returns `Poisoned` (§3.4 step 2 / engine.rs `glob_lookup` ~:289-291).
///
/// The glob target scope (scope 5) has `h` as a `Pending` (unresolved) import.
/// A decoy outer `h` (item 3000) at root proves the engine never falls through
/// to it — `Poisoned` is returned instead.
#[test]
fn test_glob_member_pending_poisons_not_outer_decoy() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // resolving scope
    g.add_scope(scope(5, ScopeKind::Module, Some(0))); // glob target scope
                                                       // Outer decoy `h` at root (item 3000) — must NOT be reached.
    g.add_binding(item_binding(0, "h", 3000, VIS_PUB));
    // A still-Pending `h` in the glob target scope 5 (path unresolvable in graph).
    g.add_binding(Binding {
        scope: ScopeId(5),
        name: "h".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(
            RawPath(vec!["nowhere".to_string(), "h".to_string()]),
            Anchor::bare(),
        ),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // Non-deferred Glob edge from scope 1 → scope 5 (target is Resolved(Scope)).
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: rust_policy::EK_GLOB,
        to: BindTarget::Resolved(Target::Scope(ScopeId(5))),
        vis: vis(VIS_PRIV),
        cond: None,
        order: 0,
        vis_range: None,
    });

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("h", 1), &pol);

    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "a glob whose member is still-Pending must POISON, not fall through to the outer h"
    );
    assert_not_item(&res, 3000); // RECALL-SAFETY: must not reach the outer decoy
}

/// `resolve_path` on `a::b::c` where non-final segment `b` resolves to a
/// non-scope-bearing target (a callable `Item fn b`, not a module/type) →
/// `Unresolved` (the engine falls through, never picks a wrong target).
///
/// This pins the `scope_of_target → None → unresolved` branch in
/// `resolve_path_guarded` (engine.rs ~:354).
///
/// Decoy: a separate `c` (item 3100) at the outer scope that must never be reached.
#[test]
fn test_resolve_path_non_scope_bearing_segment_falls_through() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // scope `a`
                                                       // `a` Scope binding at root (prefix segment 1 — scope-bearing, OK).
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "a".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // `b` in scope `a` as a plain callable fn (Item, NOT scope-bearing: no `owns`).
    // This is the non-final segment that cannot be walked into.
    g.add_binding(Binding {
        scope: ScopeId(1),
        name: "b".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Item {
            id: ItemId(3099),
            ns: NS_TYPE,
            owns: None, // NOT scope-bearing
            callable: true,
        }),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // Outer decoy `c` at root (item 3100) — must never be reached.
    g.add_binding(item_binding(0, "c", 3100, VIS_PUB));

    let pol = policy_for(&g, 2018);
    // `a::b::c` — prefix `a` resolves to scope 1; `b` resolves to a non-scope
    // Item (callable fn, no `owns`); `scope_of_target` returns None → Unresolved.
    let res = resolve_path(
        &g,
        &RawPath(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(0),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "a non-scope-bearing non-final path segment must fall through (Unresolved)"
    );
    assert_not_item(&res, 3100); // must not silently reach the outer decoy
    assert_not_item(&res, 3099); // and not the fn b itself
}

// ═══════════════════════════════════════════════════════════════════════════
// RE-EXPORT CHAIN + CYCLE (fixpoint, cycle-guarded).
// ═══════════════════════════════════════════════════════════════════════════

/// `Pending` chain a→b→c resolves to c (item 2000).
/// scope1 has `a` = Pending(b path); `b` = Resolved(Item 2000); the `a` chases
/// `b` which is resolved → a resolves to item 2000.
#[test]
fn test_reexport_chain_resolves_to_terminal() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    // `c` = the terminal item 2000 (Value)
    g.add_binding(item_binding(1, "c", 2000, VIS_PUB));
    // `b` = pub use self::c  (Pending → c)
    g.add_binding(Binding {
        scope: ScopeId(1),
        name: "b".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(RawPath(vec!["c".to_string()]), Anchor::self_mod()),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // `a` = pub use self::b  (Pending → b → c)
    g.add_binding(Binding {
        scope: ScopeId(1),
        name: "a".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(RawPath(vec!["b".to_string()]), Anchor::self_mod()),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    // resolve `a` from within scope 1 (bare, same scope — no module crossing).
    let res = resolve(&g, &bare_value_query("a", 1), &pol);
    assert_resolved_item(&res, 2000);
}

/// `a→b→a` cycle WITH an outer `a` decoy → terminal `Poisoned` (the still-pending
/// import poisons), NOT a hang and NOT the outer `a`.
#[test]
fn test_reexport_cycle_poisons_not_outer_decoy() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    // OUTER decoy `a` callable at ROOT (item 2100) — must NOT be the result.
    g.add_binding(item_binding(0, "a", 2100, VIS_PUB));
    // cycle in scope 1: `a` = use self::b ; `b` = use self::a
    g.add_binding(Binding {
        scope: ScopeId(1),
        name: "a".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(RawPath(vec!["b".to_string()]), Anchor::self_mod()),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    g.add_binding(Binding {
        scope: ScopeId(1),
        name: "b".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Pending(RawPath(vec!["a".to_string()]), Anchor::self_mod()),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });

    let pol = policy_for(&g, 2018);
    // resolve `a` from within scope 1.
    let res = resolve(&g, &bare_value_query("a", 1), &pol);
    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "a re-export cycle must terminate as Poisoned (cycle-guarded), not hang or reach the outer a"
    );
    assert_not_item(&res, 2100); // the cardinal guard: not the outer decoy
}

// ═══════════════════════════════════════════════════════════════════════════
// CFG (a formula; never merge exclusive worlds).
// ═══════════════════════════════════════════════════════════════════════════

/// Two cfg-EXCLUSIVE bindings (unix vs windows) → a multi-candidate ResolvedSet
/// with DISTINCT `cond` (not merged, not collapsed to one). The consumer falls
/// through on a multi-candidate result — never silently picks one world.
#[test]
fn test_cfg_exclusive_bindings_yield_resolvedset_distinct_conds() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    let unix = CfgCond::Atom("target_os".to_string(), Some("linux".to_string()));
    let windows = CfgCond::Atom("target_os".to_string(), Some("windows".to_string()));
    // `imp` defined twice, cfg-exclusive (items 2200 / 2201)
    let mut b1 = item_binding(1, "imp", 2200, VIS_PUB);
    b1.cond = Some(unix.clone());
    let mut b2 = item_binding(1, "imp", 2201, VIS_PUB);
    b2.cond = Some(windows.clone());
    g.add_binding(b1);
    g.add_binding(b2);

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("imp", 1), &pol);
    assert_eq!(
        res.status,
        ResStatus::ResolvedSet,
        "cfg-exclusive duplicates are a ResolvedSet (distinct worlds), not collapsed"
    );
    assert_eq!(res.candidates.len(), 2, "both worlds preserved, not merged");
    // distinct conds
    let conds: std::collections::BTreeSet<_> =
        res.candidates.iter().map(|c| c.cond.clone()).collect();
    assert_eq!(
        conds.len(),
        2,
        "the two candidates must carry distinct cfg conds"
    );
}

/// Two bindings whose conds are NOT provably exclusive and differ → Ambiguous.
/// (feature="a" and an unconditional one — not provably exclusive.)
#[test]
fn test_cfg_non_exclusive_differing_ambiguous() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    // one unconditional, one cfg(feature="a") — these CAN coexist → conflict.
    let mut b1 = item_binding(1, "q", 2300, VIS_PUB);
    b1.cond = None;
    let mut b2 = item_binding(1, "q", 2301, VIS_PUB);
    b2.cond = Some(CfgCond::Atom("feature".to_string(), Some("a".to_string())));
    g.add_binding(b1);
    g.add_binding(b2);

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("q", 1), &pol);
    assert_eq!(
        res.status,
        ResStatus::Ambiguous,
        "two non-cfg-exclusive differing definitions are Ambiguous → fall through"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// EDITION ANCHORS (policy `anchor` hook; 2015 vs 2018).
// ═══════════════════════════════════════════════════════════════════════════

/// 2015: a bare `use foo::…` anchors at the CRATE ROOT. We model the prefix
/// resolution: `use foo::bar` (a use-path) from a nested module resolves `foo`
/// at the crate root, NOT lexically. Build Root with module `foo` (scope1) that
/// has `bar` (item 2400); resolve the use-path `foo::bar` from a DEEP module
/// (scope2, child of foo) — under 2015 use-path anchoring it must hit crate-root
/// `foo`, finding item 2400.
#[test]
fn test_edition_2015_use_path_anchors_at_crate_root() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // foo
    g.add_scope(scope(2, ScopeKind::Module, Some(1))); // foo::deep
                                                       // crate-root `foo` scope binding
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "foo".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // `bar` in foo (item 2400)
    g.add_binding(item_binding(1, "bar", 2400, VIS_PUB));

    let pol = policy_for(&g, 2015);
    // a USE path `foo::bar` → 2015 anchors at crate root.
    let res = resolve_path(
        &g,
        &RawPath(vec!["foo".to_string(), "bar".to_string()]),
        NS_VALUE,
        &Anchor::use_path_2015(),
        ScopeId(2),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&res, 2400);
}

/// 2018: `::x` anchors the extern-prelude; a bare `x` with no in-scope binding
/// falls through (the extern-prelude is not an in-repo Item). We assert via the
/// anchor hook: a leading-`::` anchor in 2018 maps to the ExternPrelude scope
/// (which holds no in-repo item `x`) → Unresolved, NOT an in-repo guess.
#[test]
fn test_edition_2018_leading_colon_extern_prelude_falls_through() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    // an ExternPrelude scope (scope 9) with NO in-repo `x`
    g.add_scope(scope(9, ScopeKind::ExternPrelude, None));
    // an in-repo `x` at the crate root — the DECOY that 2018 `::x` must NOT hit
    g.add_binding(item_binding(0, "x", 2500, VIS_PUB));

    let pol = policy_for(&g, 2018);
    // `::x` in 2018 anchors at the extern-prelude (scope 9), which has no `x`.
    let res = resolve_path(
        &g,
        &RawPath(vec!["x".to_string()]),
        NS_VALUE,
        &Anchor::leading_colon_2018(9),
        ScopeId(0),
        NS_VALUE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_eq!(
        res.status,
        ResStatus::Unresolved,
        "2018 ::x anchors the extern-prelude (no in-repo x) → fall through"
    );
    assert_not_item(&res, 2500); // MUST NOT hit the crate-root in-repo x
}

/// A `let`/item `x` shadowing an extern-prelude name resolves to the LOCAL, not
/// the prelude. (2018 bare leading ident = lexical with local shadowing prelude.)
/// scope2 (block) has a local `x`; the extern-prelude (scope9, reachable as a
/// glob from root) also "has" x conceptually — but the local shadows. We model
/// the local binding + a non-deferred prelude glob and assert the local wins.
#[test]
fn test_edition_2018_local_shadows_extern_prelude() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0)));
    g.add_scope(scope(2, ScopeKind::Block, Some(1)));
    // extern-prelude scope 9 with an `x` External (item-less) — the prelude name
    g.add_scope(scope(9, ScopeKind::ExternPrelude, None));
    g.add_binding(Binding {
        scope: ScopeId(9),
        name: "x".to_string(),
        ns: NS_VALUE,
        target: BindTarget::Resolved(Target::External(prism::name_resolution::types::ExternRef {
            name: "std::x".to_string(),
        })),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // local `x` in the block (a let binding) — must SHADOW the prelude
    g.add_binding(local_binding(2, "x", 0));

    let pol = policy_for(&g, 2018);
    let res = resolve(&g, &bare_value_query("x", 2), &pol);
    assert_eq!(res.status, ResStatus::Resolved);
    match &res.candidates[0].target {
        Target::Local(_) => {} // the local shadows the prelude
        other => panic!("expected the local to shadow the extern-prelude, got {other:?}"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// resolve_path longest-prefix.
// ═══════════════════════════════════════════════════════════════════════════

/// `crate::engine::start` resolves `crate::engine` → scope, then `start` within.
/// Root → engine (scope1); `start` (item 2600) in engine; root has `engine` Scope
/// binding. resolve_path(["engine","start"]) anchored at crate root → item 2600.
#[test]
fn test_resolve_path_longest_prefix() {
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None));
    g.add_scope(scope(1, ScopeKind::Module, Some(0))); // engine
                                                       // root `engine` scope binding (Type ns — scope-bearing)
    g.add_binding(Binding {
        scope: ScopeId(0),
        name: "engine".to_string(),
        ns: NS_TYPE,
        target: BindTarget::Resolved(Target::Scope(ScopeId(1))),
        vis: vis(VIS_PUB),
        cond: None,
        vis_extents: vec![wide_span()],
    });
    // `start` in engine (item 2600)
    g.add_binding(item_binding(1, "start", 2600, VIS_PUB));

    let pol = policy_for(&g, 2018);
    let res = resolve_path(
        &g,
        &RawPath(vec!["engine".to_string(), "start".to_string()]),
        NS_VALUE,
        &Anchor::crate_root(),
        ScopeId(0),
        NS_TYPE,
        &SourceLoc { file: F, byte: 50 },
        &pol,
    );
    assert_resolved_item(&res, 2600);
}

// ═══════════════════════════════════════════════════════════════════════════
// Sanity: the macro module-path / unused-import lints don't break the suite.
// ═══════════════════════════════════════════════════════════════════════════

/// The three Rust namespaces must be distinct discriminants.
#[test]
fn test_namespace_constants_distinct() {
    let set: std::collections::BTreeSet<u16> = [NS_TYPE, NS_VALUE, NS_MACRO].into_iter().collect();
    assert_eq!(set.len(), 3, "the three Rust namespaces must be distinct");
    // `Ident` is the name-segment type the engine keys on.
    let id: Ident = "start".to_string();
    assert_eq!(id, "start");
}
