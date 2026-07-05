//! Precision negatives for anchor-only glob expansion (`use super::*` /
//! `crate::*` / `self::*`) — split out of `anchor_glob_test.rs` (which pins
//! the positive resolution shapes + the poison-sentinel guard) to keep both
//! files under the repo's 600-line guideline.
//!
//! Every ambiguity/undecidability behind an anchor-only glob must still fail
//! closed exactly like a named glob's does (`glob_expand_test.rs`): an
//! ambiguous or undecidable member poisons, and an empty-path glob whose
//! anchor does NOT expand (`Bare`/`UsePath`/`LeadingColon` — spec-review
//! [F1]) must stay Unresolved at BOTH the engine's `resolve_path_guarded`
//! (Site B, tested here in isolation) and the module-dep
//! (`consumer::resolve_glob_path`) layer.

use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::consumer::{graph_module_dep_edge, GraphImport, ResolvedImport};
use prism::name_resolution::engine::{resolve_path, resolve_with_stats, ScopeGraph};
use prism::name_resolution::glob_stats::{GlobExpandSnapshot, GlobExpandStats};
use prism::name_resolution::rust_policy::{RustPolicy, EK_GLOB, NS_TYPE, NS_VALUE};
use prism::name_resolution::rust_populator::{
    enclosing_scope, file_id, populate_rust, RustCrateConfig,
};
use prism::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Edge, FileId, NamespaceId, RawPath, ResStatus, Resolution,
    ResolveQuery, Scope, ScopeExtent, ScopeId, ScopeKind, SourceLoc, Span, Vis,
};

// ── helpers (mirrors anchor_glob_test.rs / rust_populate_test.rs) ──

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

/// Resolves a bare `(name, ns)` and also returns the glob-expansion telemetry
/// snapshot so a test can pin the specific precision guard that fired
/// (`member_multi` vs `member_undecidable`, etc).
fn resolve_bare_at_with_stats(
    g: &ScopeGraph,
    fs: &BTreeMap<String, ParsedFile>,
    edition: u16,
    path: &str,
    byte: usize,
    name: &str,
    ns: NamespaceId,
) -> (Resolution, GlobExpandSnapshot) {
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
    let stats = GlobExpandStats::default();
    let res = resolve_with_stats(g, &q, &pol, &stats);
    (res, stats.snapshot())
}

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
// Precision negatives — every ambiguity/undecidability still fails closed
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn super_glob_ambiguous_member_poisons() {
    // The parent module has TWO `target`s (a genuinely multiply-defined member
    // once the glob's member lookup runs) — must poison, never mint either as
    // a wrong singleton.
    let src = "mod outer {\n    pub fn target(x: i32) -> bool { x > 0 }\n    pub fn target(y: i32) -> bool { y < 0 }\n    mod tests {\n        use super::*;\n        fn calls_target() {\n            target(1);\n        }\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let (res, snap) = resolve_bare_at_with_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "an ambiguous member behind an anchor-only glob must poison, got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert_eq!(snap.member_multi, 1);
}

#[test]
fn super_glob_undecidable_member_poisons() {
    // `pub(in crate::ghost)` restricts to an unresolvable path -> `Unknown`
    // visibility (undecidable, not proved hidden) -> must poison, never fall
    // through to treat the member as absent.
    let src = "mod outer {\n    pub(in crate::ghost) fn target(x: i32) -> bool { x > 0 }\n    mod tests {\n        use super::*;\n        fn calls_target() {\n            target(1);\n        }\n    }\n}\n";
    let fs = files(&[("src/lib.rs", src)]);
    let g = populate_rust(&fs, &convention(&fs), None);

    let (res, snap) = resolve_bare_at_with_stats(
        &g,
        &fs,
        2018,
        "src/lib.rs",
        byte_of(src, "target(1)"),
        "target",
        NS_VALUE,
    );
    assert_eq!(
        res.status,
        ResStatus::Poisoned,
        "an undecidable member behind an anchor-only glob must poison, got {:?} ({:?})",
        res.status,
        res.candidates
    );
    assert_eq!(snap.member_undecidable, 1);
}

// ═══════════════════════════════════════════════════════════════════════════
// Empty-path fail-closed negatives [F1]: UsePath / LeadingColon / Bare must
// NEVER expand, at BOTH engine sites (in isolation) AND the module-dep layer.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn empty_path_non_expanding_anchors_stay_unresolved_at_site_b() {
    // Direct Site-B-in-isolation probe (calling `resolve_path` straight, i.e.
    // bypassing the engine's Site-A glob poison gate entirely): proves
    // `resolve_path_guarded`'s own empty-segs branch is fail-closed for a
    // non-expanding anchor, not merely "Site A never lets it reach here in
    // production". Each anchor is constructed so `policy.anchor(...)` WOULD
    // succeed (crate-root / a known prelude scope) if the predicate gate were
    // wrongly bypassed — an adversarial regression net, not a vacuous check.
    let main = FileId(0);
    let mut g = ScopeGraph::new();
    g.add_scope(scope(0, ScopeKind::Root, None, main));
    let policy = RustPolicy::new(&g, 2021);
    let at = loc(main, 1);

    let non_expanding = [
        Anchor::bare(),
        Anchor::use_path_2015(), // 2015-style: would anchor to crate root if wrongly expanded
        Anchor {
            kind: AnchorKind::LeadingColon,
            prelude: Some(ScopeId(0)), // a KNOWN prelude scope — would resolve if wrongly expanded
        },
    ];
    for anchor in non_expanding {
        let res = resolve_path(
            &g,
            &RawPath(vec![]),
            NS_TYPE,
            &anchor,
            ScopeId(0),
            NS_TYPE,
            &at,
            &policy,
        );
        assert_eq!(
            res.status,
            ResStatus::Unresolved,
            "empty-path {:?} must stay Unresolved (glob_anchor_expands == false), got {:?}",
            anchor.kind,
            res.status
        );
    }
}

#[test]
fn empty_path_non_expanding_anchors_stay_unresolved_at_module_dep_layer() {
    let main = FileId(0);
    let mut g = ScopeGraph::new();
    g.complete = true;
    g.file_paths = BTreeMap::from([("src/lib.rs".to_string(), main)]);
    g.add_scope(scope(0, ScopeKind::Root, None, main));

    let non_expanding = [
        Anchor::bare(),
        Anchor::use_path_2015(),
        Anchor {
            kind: AnchorKind::LeadingColon,
            prelude: Some(ScopeId(0)),
        },
    ];
    for anchor in non_expanding {
        let edge = Edge {
            from: ScopeId(0),
            kind: EK_GLOB,
            to: BindTarget::Pending(RawPath(vec![]), anchor),
            vis: pub_vis(),
            cond: None,
            order: 0,
            vis_range: None,
        };
        assert_eq!(
            graph_module_dep_edge(&g, GraphImport::Glob(&edge)),
            ResolvedImport::Unresolved,
            "empty-path {:?} must stay Unresolved at the module-dep layer too [F1]",
            anchor.kind
        );
    }
}

#[test]
fn graph_module_dep_edge_resolves_anchor_only_glob_to_target_module_file() {
    // Module-dep view: an anchor-only glob is TARGET-MODULE-FILE resolution
    // (a `*` dependency on the anchor's own module file), not member
    // expansion — pinned directly (in isolation from the slower end-to-end
    // `nav module-deps` cross-file test in `module_graph_test.rs`).
    let main = FileId(0); // src/lib.rs — the crate root's own file
    let other = FileId(1); // src/inner.rs — a nested module's own file
    let mut g = ScopeGraph::new();
    g.complete = true;
    g.file_paths = BTreeMap::from([
        ("src/lib.rs".to_string(), main),
        ("src/inner.rs".to_string(), other),
    ]);
    g.add_scope(scope(0, ScopeKind::Root, None, main));
    g.add_scope(scope(1, ScopeKind::Module, Some(0), other));

    let edge = Edge {
        from: ScopeId(1),
        kind: EK_GLOB,
        to: BindTarget::Pending(RawPath(vec![]), Anchor::crate_root()),
        vis: pub_vis(),
        cond: None,
        order: 0,
        vis_range: None,
    };
    assert_eq!(
        graph_module_dep_edge(&g, GraphImport::Glob(&edge)),
        ResolvedImport::File("src/lib.rs".to_string()),
        "an anchor-only glob resolves to its TARGET MODULE FILE, not member expansion"
    );
}
