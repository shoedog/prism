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
//! These tests pin: the anchor-only globs now resolve (a real caller edge into
//! the parent/crate-root/self module), the sentinel still poisons, and every
//! precision guard (ambiguous/undecidable member, empty-path non-expanding
//! anchors) still fails closed.

use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::consumer::{graph_module_dep_edge, GraphImport, ResolvedImport};
use prism::name_resolution::engine::{resolve, resolve_path, ScopeGraph};
use prism::name_resolution::rust_policy::{RustPolicy, EK_GLOB, NS_TYPE, NS_VALUE};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Edge, FileId, RawPath, ResStatus, Resolution, ResolveQuery,
    Scope, ScopeExtent, ScopeId, ScopeKind, Span, SourceLoc, Target, Vis,
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
    ns: prism::name_resolution::types::NamespaceId,
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
