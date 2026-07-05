//! Anchor-only glob expansion (`use super::*` / `crate::*` / `self::*`).
//!
//! `use super::*` (and `crate::*` / `self::*`) flattens to a glob whose path is
//! EMPTY but whose anchor is meaningful (`Super(n)` / `CrateRoot` / `SelfMod`).
//! Two engine sites used to poison/discard that anchor unconditionally:
//! - `engine.rs` glob-loop poison gate (any empty-path pending glob → poison);
//! - `resolve_path_guarded`'s empty-segs hard `unresolved()` before anchoring.
//!
//! The `poison_scope` sentinel (missing-mod / parse-failed target module)
//! constructs the IDENTICAL shape — empty path + pending — differing only in
//! `anchor.kind` (`Bare` vs `Super`/`CrateRoot`/`SelfMod`). The
//! `ResolutionPolicy::glob_anchor_expands` predicate is the ONLY thing that
//! tells the two apart; the engine never matches `AnchorKind` itself.
//!
//! This file pins the POSITIVE resolution shapes (the fixture shape, the
//! adjacent crate::*/self::*/super::super::* anchors, non-test nesting, and
//! composition with a second glob) plus the poison-sentinel guard. Precision
//! negatives (ambiguous/undecidable member, empty-path non-expanding
//! anchors, module-dep isolation) live in `anchor_glob_precision_test.rs`
//! (split to keep both files under the repo's 600-line guideline).

use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::engine::{resolve, ScopeGraph};
use prism::name_resolution::rust_policy::{RustPolicy, EK_GLOB, NS_VALUE};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, BindTarget, Binding, Edge, FileId, ItemId, NamespaceId, RawPath, ResStatus, Resolution,
    ResolveQuery, Scope, ScopeExtent, ScopeId, ScopeKind, SourceLoc, Span, Target, Vis,
};

// ── populate_rust-backed helpers (mirrors rust_populate_test.rs / glob_expand_test.rs) ──

fn rs(path: &str, src: &str) -> (String, ParsedFile) {
    (
        path.to_string(),
        ParsedFile::parse(path, src, Language::Rust).unwrap(),
    )
}

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs.iter().map(|(p, s)| rs(p, s)).collect()
}

fn byte_of(src: &str, needle: &str) -> usize {
    src.find(needle)
        .unwrap_or_else(|| panic!("needle {needle:?} not found in source"))
}

fn convention(fs: &BTreeMap<String, ParsedFile>) -> RustCrateConfig {
    RustCrateConfig::from_convention(fs)
}

fn resolve_bare_at(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    path: &str,
    byte: usize,
    name: &str,
    ns: NamespaceId,
) -> Resolution {
    let fid = file_id(fs, path).expect("file id");
    let from = enclosing_scope(g, fid, byte)
        .unwrap_or_else(|| panic!("no enclosing scope at {path}:{byte}"));
    let pol = RustPolicy::new(g, edition);
    let q = ResolveQuery {
        name: name.to_string(),
        ns,
        from,
        at: SourceLoc { file: fid, byte },
        cfg: Default::default(),
        ctx: Default::default(),
    };
    resolve(g, &q, &pol)
}

fn assert_resolved_item(res: &Resolution) {
    assert_eq!(
        res.status,
        ResStatus::Resolved,
        "expected Resolved, got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert_eq!(res.candidates.len(), 1, "expected one candidate");
    assert!(
        matches!(res.candidates[0].target, Target::Item { .. }),
        "expected Item target, got {:?}",
        res.candidates[0].target
    );
}

// ── hand-built-graph helpers (mirrors resolve_test.rs / consumer_test.rs) ──

fn loc(file: FileId, byte: usize) -> SourceLoc {
    SourceLoc { file, byte }
}

fn span(file: FileId, lo: usize, hi: usize) -> Span {
    Span {
        lo: loc(file, lo),
        hi: loc(file, hi),
    }
}

fn scope(id: u32, kind: ScopeKind, parent: Option<u32>, file: FileId) -> Scope {
    Scope {
        id: ScopeId(id),
        kind,
        parent: parent.map(ScopeId),
        extents: vec![ScopeExtent {
            file,
            range: span(file, 0, 1000),
            cond: None,
            occ: None,
        }],
        owner_item: None,
        cond: None,
    }
}

fn pub_vis() -> Vis {
    Vis {
        kind: prism::name_resolution::rust_policy::VIS_PUB,
        restrict: None,
        payload: Default::default(),
    }
}

fn item_target(id: u32) -> Target {
    Target::Item {
        id: ItemId(id),
        ns: NS_VALUE,
        owns: None,
        callable: true,
    }
}

fn binding(scope: u32, name: &str, ns: NamespaceId, target: BindTarget) -> Binding {
    Binding {
        scope: ScopeId(scope),
        name: name.to_string(),
        ns,
        target,
        vis: pub_vis(),
        cond: None,
        // Empty `vis_extents` ⇒ `vis_extent_covers` treats the binding as
        // scope-wide (conservative: always covers `at`) — no span bookkeeping
        // needed for these hand-built graphs.
        vis_extents: vec![],
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// TDD ANCHOR 1 — the fixture shape (failing-first against unmodified HEAD)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn super_glob_resolves_bare_call_into_nested_module() {
    // The eval fixture shape (eval/fixtures/rust/nested_test_module_glob_gap):
    // a bare call from inside a nested module that relies on `use super::*` to
    // bring the callee into scope. `target()`'s ONLY route into `tests` is the
    // glob (the module-boundary stop means a bare lookup never lexically
    // ascends past `tests` on its own).
    let src = "pub fn target(x: i32) -> bool { x > 0 }\n\
#[cfg(test)]\nmod tests {\n    use super::*;\n\n    #[test]\n    fn calls_target() {\n        target(1);\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// TDD ANCHOR 2 — the poison-sentinel guard (already green pre-fix; must STAY green)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn poison_sentinel_still_poisons() {
    // `rust_populator::builder::poison_scope` constructs a Bare-anchored
    // EMPTY-path pending glob edge for a missing-mod / parse-failed target
    // module — the IDENTICAL shape as a `super::*`/`crate::*`/`self::*` glob
    // (empty path + pending), differing ONLY in `anchor.kind` (Bare here, vs
    // Super/CrateRoot/SelfMod for a real anchor-only glob).
    // `glob_anchor_expands(Bare) == false`, so a bare lookup routed through
    // this scope's glob tier must still poison — the fix must not
    // over-resolve the sentinel into its enclosing scope.
    let main = FileId(0);
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None, main));
    g.add_scope(scope(1, ScopeKind::Module, Some(0), main)); // the "missing" shell
    g.add_edge(Edge {
        from: ScopeId(1),
        kind: EK_GLOB,
        to: BindTarget::Pending(RawPath(vec![]), Anchor::default()), // Bare — poison_scope's exact shape
        vis: pub_vis(),
        cond: None,
        order: 0,
        vis_range: None,
    });
    let policy = RustPolicy::new(&g, 2021);
    let q = ResolveQuery {
        name: "anything".to_string(),
        ns: NS_VALUE,
        from: ScopeId(1),
        at: loc(main, 1),
        cfg: Default::default(),
        ctx: Default::default(),
    };
    let res = resolve(&g, &q, &policy);
    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "the Bare-anchored empty-path sentinel must still poison a bare lookup routed \
         through its scope, got {:?}",
        res.status
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// The adjacent anchor-only shapes: crate::* / self::* / super::super::*
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn crate_glob_resolves_bare_call() {
    let src = "pub fn target(x: i32) -> bool { x > 0 }\nmod tests {\n    use crate::*;\n    fn calls_target() {\n        target(1);\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

#[test]
fn self_glob_resolves_bare_call() {
    // `self::*` denotes the CURRENT module itself. To exercise it as the ONLY
    // route into `target` (rather than a same-scope rib hit trivially
    // shadowing the glob tier at Step 1), the glob is declared in a NESTED
    // callable scope so its anchor resolves OUTWARD to the enclosing module
    // that actually defines `target` — mirroring how `use self::*;` written
    // inside a function body anchors to that function's enclosing module.
    let main = FileId(0);
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None, main));
    g.add_scope(scope(1, ScopeKind::Module, Some(0), main)); // `tests`
    g.add_scope(scope(2, ScopeKind::Callable, Some(1), main)); // a fn body inside `tests`
    g.add_binding(binding(
        1,
        "target",
        NS_VALUE,
        BindTarget::Resolved(item_target(10)),
    ));
    g.add_edge(Edge {
        from: ScopeId(2),
        kind: EK_GLOB,
        to: BindTarget::Pending(RawPath(vec![]), Anchor::self_mod()),
        vis: pub_vis(),
        cond: None,
        order: 0,
        vis_range: None,
    });
    let policy = RustPolicy::new(&g, 2021);
    let q = ResolveQuery {
        name: "target".to_string(),
        ns: NS_VALUE,
        from: ScopeId(2),
        at: loc(main, 1),
        cfg: Default::default(),
        ctx: Default::default(),
    };
    let res = resolve(&g, &q, &policy);
    assert_resolved_item(&res);
}

#[test]
fn super_super_glob_resolves() {
    // `super::super::*` from a doubly-nested module flattens to
    // `Anchor::super_n(2)` + an empty path — same mechanism, one more hop.
    let src = "pub fn target(x: i32) -> bool { x > 0 }\nmod outer {\n    pub mod tests {\n        use super::super::*;\n        fn calls_target() {\n            target(1);\n        }\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// Not test-specific
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn non_test_nested_module_super_glob_resolves() {
    // No `#[cfg(test)]`/`#[test]`/macro wrapping at all — the gap (and the fix)
    // is pure name resolution, orthogonal to test-module conventions.
    let src = "pub fn target(x: i32) -> bool { x > 0 }\nmod util {\n    use super::*;\n    fn calls_target() {\n        target(1);\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_resolved_item(&res);
}

// ═══════════════════════════════════════════════════════════════════════════
// Composition with a second (named) glob — [F3]: same-name overlap is
// Ambiguous (no first-wins edge), NOT Poisoned. Matches
// `glob_expand_test.rs::glob_expand_distinct_targets_two_globs`'s contract.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn multi_glob_scope_with_super_resolves_singleton_and_ambiguous_overlap() {
    let src = "pub fn target(x: i32) -> bool { x > 0 }\n\
pub fn dup() -> i32 { 1 }\n\
mod other {\n    pub fn helper() -> i32 { 2 }\n    pub fn dup() -> i32 { 2 }\n}\n\
mod tests {\n    use super::*;\n    use crate::other::*;\n    fn f() {\n        target(1);\n        helper();\n        dup();\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    // `target` is resolvable via `super::*` ALONE (not present in `other`) —
    // composing a second glob at the same scope must not break it.
    let target_res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_resolved_item(&target_res);

    // `helper` is resolvable via the NAMED glob alone — the anchor-only glob's
    // fix must not regress an ordinary named glob sharing the scope.
    let helper_res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "helper();"),
        "helper",
        NS_VALUE,
    );
    assert_resolved_item(&helper_res);

    // `dup` is defined in BOTH glob targets (crate root via `super::*`, and
    // `other` via `crate::other::*`) — two distinct singleton candidates ⇒
    // `Ambiguous`, NOT `Poisoned` (F3: "no first-wins edge", the union of
    // per-edge Hit candidates is decided by `RustPolicy::combine`, not by
    // whichever glob's member lookup ran first).
    let dup_res = resolve_bare_at(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "dup();"),
        "dup",
        NS_VALUE,
    );
    assert_eq!(
        dup_res.status,
        ResStatus::Ambiguous,
        "same-name overlap across two globs (one anchor-only) must be Ambiguous, \
         not a first-wins singleton or a Poison, got {:?} ({:?})",
        dup_res.status,
        dup_res.candidates
    );
    assert_eq!(dup_res.candidates.len(), 2);
    assert_ne!(
        dup_res.candidates[0].target, dup_res.candidates[1].target,
        "the two `dup` candidates must be the two DISTINCT definitions"
    );
}
