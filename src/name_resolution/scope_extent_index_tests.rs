use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::name_resolution::rust_populator::{
    enclosing_scope, enclosing_scope_reference, file_id, populate_rust, RustCrateConfig,
};
use crate::name_resolution::types::{Scope, ScopeExtent, ScopeKind, SourceLoc, Span};

fn extent(file: u32, lo: usize, hi: usize) -> ScopeExtent {
    let file = FileId(file);
    ScopeExtent {
        file,
        range: Span {
            lo: SourceLoc { file, byte: lo },
            hi: SourceLoc { file, byte: hi },
        },
        cond: None,
        occ: None,
    }
}

fn scope(id: u32, extents: Vec<ScopeExtent>) -> Scope {
    Scope {
        id: ScopeId(id),
        kind: ScopeKind::Block,
        parent: None,
        extents,
        owner_item: None,
        cond: None,
    }
}

fn compare(graph: &ScopeGraph, files: &[u32], end: usize) {
    for &file in files {
        for byte in 0..=end {
            assert_eq!(
                enclosing_scope(graph, FileId(file), byte),
                enclosing_scope_reference(graph, FileId(file), byte),
                "file {file}, byte {byte}"
            );
        }
    }
}

#[test]
fn rust_repos_match_reference_and_build_file_index() {
    let sources = [
        (
            "src/lib.rs",
            "mod outer { pub mod inner { pub fn nested() { let c = || { use crate::T; T }; { use crate::T; let _ = c(); } } } }\nmod disk;\nstruct T; impl T { fn method(&self) { let c = || { Self }; { use crate::T; let _ = c(); } } }\n",
        ),
        ("src/disk.rs", "pub mod child; pub fn on_disk() {}\n"),
        ("src/disk/child.rs", "pub fn child() { { use crate::T; let _ = T; } }\n"),
        ("other/src/lib.rs", "pub fn other_crate() {}\n"),
        ("src/unmodeled.rs", "fn not_reached_by_mod() {}\n"),
    ];
    let files: BTreeMap<_, _> = sources
        .iter()
        .map(|(path, source)| {
            (
                path.to_string(),
                ParsedFile::parse(path, source, Language::Rust).unwrap(),
            )
        })
        .collect();
    let graph = populate_rust(&files, &RustCrateConfig::from_convention(&files), None);
    assert!(graph.scope_extent_index.0.get().is_none());
    for (path, source) in sources {
        let file = file_id(&files, path).unwrap();
        for byte in 0..=source.len() + 1 {
            assert_eq!(
                enclosing_scope(&graph, file, byte),
                enclosing_scope_reference(&graph, file, byte),
                "{path}:{byte}"
            );
        }
    }
    let index = graph
        .scope_extent_index
        .0
        .get()
        .expect("lookup must build index");
    assert_eq!(index.by_file.len(), 4, "unmodeled file has no extents");
    assert_eq!(
        enclosing_scope(&graph, file_id(&files, "src/unmodeled.rs").unwrap(), 0),
        None
    );
    assert_eq!(enclosing_scope(&graph, FileId(999), 0), None);
    for kind in [
        ScopeKind::Root,
        ScopeKind::Module,
        ScopeKind::Type,
        ScopeKind::Callable,
        ScopeKind::Block,
    ] {
        assert!(
            graph.scopes.values().any(|s| s.kind == kind),
            "missing {kind:?}"
        );
    }
}

#[test]
fn crossing_extents_ties_boundaries_and_map_keys() {
    let mut graph = ScopeGraph::new();
    // Deliberately not ordered by start or nesting. Scope 2 has several extents
    // in different files; scope 9 wins a same-width, different-start tie at 20.
    graph.add_scope(scope(
        2,
        vec![extent(4, 10, 30), extent(7, 3, 8), extent(4, 60, 70)],
    ));
    graph.add_scope(scope(9, vec![extent(4, 20, 40)]));
    graph.add_scope(scope(3, vec![extent(4, 12, 22)]));
    graph.add_scope(scope(1, vec![extent(4, 50, 55)]));
    // Empty/reversed extents never cover, including at their endpoints.
    graph.add_scope(scope(99, vec![extent(4, 50, 50), extent(4, 70, 60)]));
    graph.add_scope(scope(100, vec![]));
    // The old scan uses the map key even if a public record's id differs.
    graph
        .scopes
        .insert(ScopeId(12), scope(200, vec![extent(4, 80, 90)]));
    for (byte, expected) in [
        (0, None),
        (9, None),
        (10, Some(2)),
        (12, Some(3)),
        (20, Some(3)),
        (21, Some(3)),
        (22, Some(9)),
        (29, Some(9)),
        (30, Some(9)),
        (39, Some(9)),
        (40, None),
        (49, None),
        (50, Some(1)),
        (54, Some(1)),
        (55, None),
        (60, Some(2)),
        (69, Some(2)),
        (70, None),
        (80, Some(12)),
        (89, Some(12)),
        (90, None),
    ] {
        assert_eq!(
            enclosing_scope(&graph, FileId(4), byte),
            expected.map(ScopeId)
        );
    }
    assert_eq!(enclosing_scope(&graph, FileId(7), 3), Some(ScopeId(2)));
    assert_eq!(enclosing_scope(&graph, FileId(7), 7), Some(ScopeId(2)));
    assert_eq!(enclosing_scope(&graph, FileId(7), 8), None);
    compare(&graph, &[0, 4, 7, 999], 100);
    assert_eq!(enclosing_scope(&graph, FileId(4), usize::MAX), None);
    assert_eq!(
        graph.scope_extent_index.0.get().unwrap().by_file[&FileId(7)].len(),
        1
    );
    // Identical-range ties must also prefer the largest ID.
    graph.add_scope(scope(10, vec![extent(4, 20, 40)]));
    assert_eq!(enclosing_scope(&graph, FileId(4), 22), Some(ScopeId(10)));
}

#[test]
fn mutation_invalidation_and_stale_count_fallback() {
    let mut graph = ScopeGraph::new();
    assert_eq!(enclosing_scope(&graph, FileId(0), 10), None);
    assert!(graph.scope_extent_index.0.get().is_some());
    graph.add_scope(scope(1, vec![extent(0, 0, 30)]));
    assert!(graph.scope_extent_index.0.get().is_none());
    assert_eq!(enclosing_scope(&graph, FileId(0), 10), Some(ScopeId(1)));
    graph.add_scope(scope(2, vec![extent(0, 10, 20)]));
    assert!(graph.scope_extent_index.0.get().is_none());
    assert_eq!(enclosing_scope(&graph, FileId(0), 10), Some(ScopeId(2)));
    // Replacement changes extents without changing scope count.
    graph.add_scope(scope(2, vec![extent(8, 40, 50), extent(0, 15, 20)]));
    assert!(graph.scope_extent_index.0.get().is_none());
    assert_eq!(enclosing_scope(&graph, FileId(0), 10), Some(ScopeId(1)));
    assert_eq!(enclosing_scope(&graph, FileId(8), 40), Some(ScopeId(2)));
    // Explicit invalidation covers direct in-place additions and edits.
    graph
        .scopes
        .get_mut(&ScopeId(1))
        .unwrap()
        .extents
        .push(extent(9, 1, 5));
    graph.scopes.get_mut(&ScopeId(2)).unwrap().extents.clear();
    graph.invalidate_lookup_indexes();
    assert!(graph.scope_extent_index.0.get().is_none());
    assert_eq!(enclosing_scope(&graph, FileId(9), 1), Some(ScopeId(1)));
    assert_eq!(enclosing_scope(&graph, FileId(8), 40), None);
    compare(&graph, &[0, 8, 9, 999], 55);
    // A direct insertion bypassing the API is detected: stale cache stays warm,
    // but lookups use the safe scan, including absent files and boundaries.
    graph
        .scopes
        .insert(ScopeId(3), scope(3, vec![extent(10, 0, 5)]));
    assert_eq!(enclosing_scope(&graph, FileId(10), 0), Some(ScopeId(3)));
    assert_ne!(
        graph.scope_extent_index.0.get().unwrap().scope_count,
        graph.scopes.len()
    );
    compare(&graph, &[0, 8, 9, 10, 999], 55);
    graph.invalidate_lookup_indexes();
    assert_eq!(enclosing_scope(&graph, FileId(10), 4), Some(ScopeId(3)));
    graph.scopes.remove(&ScopeId(3));
    assert_eq!(enclosing_scope(&graph, FileId(10), 4), None);
    compare(&graph, &[0, 9, 10], 55);
}

#[test]
fn serde_identity_clone_and_parallel_initialization() {
    let mut graph = ScopeGraph::new();
    graph.add_scope(scope(3, vec![extent(8, 20, 30), extent(2, 0, 10)]));
    let cold = graph.clone();
    let before = bincode::serialize(&graph).unwrap();
    // Simultaneous cold reads exercise OnceLock on a graph shared by workers.
    rayon::scope(|s| {
        for _ in 0..16 {
            let graph = &graph;
            s.spawn(move |_| compare(graph, &[2, 8, 999], 40));
        }
    });
    assert!(graph.scope_extent_index.0.get().is_some());
    assert_eq!(graph, cold, "warmth must not affect identity");
    assert_eq!(before, bincode::serialize(&graph).unwrap());
    let mut cloned = graph.clone();
    assert!(cloned.scope_extent_index.0.get().is_none());
    cloned.add_scope(scope(4, vec![extent(8, 20, 30)]));
    assert_eq!(enclosing_scope(&cloned, FileId(8), 20), Some(ScopeId(4)));
    assert_eq!(enclosing_scope(&graph, FileId(8), 20), Some(ScopeId(3)));
    let decoded: ScopeGraph = bincode::deserialize(&before).unwrap();
    assert!(decoded.scope_extent_index.0.get().is_none());
    assert_eq!(decoded, graph);
    compare(&decoded, &[2, 8, 999], 40);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "scope extent lookup equivalence")]
fn debug_check_detects_uninvalidated_extent_edit() {
    let mut graph = ScopeGraph::new();
    graph.add_scope(scope(1, vec![extent(0, 0, 10)]));
    assert_eq!(enclosing_scope(&graph, FileId(0), 0), Some(ScopeId(1)));
    graph.scopes.get_mut(&ScopeId(1)).unwrap().extents[0] = extent(7, 20, 30);
    enclosing_scope(&graph, FileId(7), 20);
}

#[test]
#[cfg(debug_assertions)]
fn debug_cross_check_is_bounded() {
    let mut graph = ScopeGraph::new();
    for id in 0..4097 {
        graph.add_scope(scope(id, vec![extent(id, 0, 10)]));
    }
    assert_eq!(enclosing_scope(&graph, FileId(0), 0), Some(ScopeId(0)));
    // At 4097 the debug scan is intentionally skipped, as for #546. This
    // deliberately invalid edit tests the cap, not a supported mutation route.
    graph.scopes.get_mut(&ScopeId(0)).unwrap().extents.clear();
    assert_eq!(enclosing_scope(&graph, FileId(0), 0), Some(ScopeId(0)));
    graph.invalidate_lookup_indexes();
    assert_eq!(enclosing_scope(&graph, FileId(0), 0), None);
    graph.scopes.remove(&ScopeId(4096));
    graph.invalidate_lookup_indexes();
    assert_eq!(enclosing_scope(&graph, FileId(1), 0), Some(ScopeId(1)));
    graph.scopes.get_mut(&ScopeId(1)).unwrap().extents.clear();
    let panic = std::panic::catch_unwind(|| enclosing_scope(&graph, FileId(1), 0));
    assert!(panic.is_err(), "at 4096 the debug check must run");
}
