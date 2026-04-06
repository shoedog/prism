#[path = "../common/mod.rs"]
mod common;
use common::*;

use prism::cpg::CodePropertyGraph;
use prism::cpg_cache::{self, CacheResult};

/// Helper: extract a CPG from a CacheResult::Hit, panicking on miss.
fn expect_hit(result: CacheResult) -> CodePropertyGraph {
    match result {
        CacheResult::Hit(cpg) => cpg,
        CacheResult::PartialHit { .. } => panic!("expected Hit, got PartialHit"),
        CacheResult::Miss => panic!("expected Hit, got Miss"),
    }
}

// ---------------------------------------------------------------------------
// Round-trip tests: build CPG → save → load → verify identical results
// ---------------------------------------------------------------------------

#[test]
fn test_cache_round_trip_python() {
    let (files, sources, diff) = make_python_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    assert!(cache_dir.path().join("cpg-cache.bin").exists());
    assert!(cache_dir.path().join("cache-meta.json").exists());

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, cache_dir.path()));

    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count(),
        "node count mismatch after round-trip"
    );
    assert_eq!(
        ctx_original.cpg.graph.edge_count(),
        loaded_cpg.graph.edge_count(),
        "edge count mismatch after round-trip"
    );

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();

    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);
    let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();

    assert_eq!(
        result_original.blocks.len(),
        result_cached.blocks.len(),
        "LeftFlow block count should be identical from cache"
    );
    for (orig, cached) in result_original
        .blocks
        .iter()
        .zip(result_cached.blocks.iter())
    {
        assert_eq!(orig.file_line_map, cached.file_line_map);
    }
}

#[test]
fn test_cache_round_trip_javascript() {
    let (files, sources, _diff) = make_javascript_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, cache_dir.path()));

    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count()
    );
    assert_eq!(
        ctx_original.cpg.graph.edge_count(),
        loaded_cpg.graph.edge_count()
    );
}

#[test]
fn test_cache_round_trip_c() {
    let (files, sources, diff) = make_c_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, cache_dir.path()));
    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count()
    );

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint);
    let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();
    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);
    let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();
    assert_eq!(result_original.blocks.len(), result_cached.blocks.len());
}

// ---------------------------------------------------------------------------
// Cache invalidation tests
// ---------------------------------------------------------------------------

#[test]
fn test_cache_partial_hit_on_file_change() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Modify a source file → same file set, different hash → PartialHit.
    let mut modified_sources = sources.clone();
    if let Some(val) = modified_sources.values_mut().next() {
        val.push_str("\n# new comment\n");
    }
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit { changed_files, .. } => {
            assert!(
                !changed_files.is_empty(),
                "should have at least one changed file"
            );
        }
        other => panic!(
            "expected PartialHit on file content change, got {}",
            match other {
                CacheResult::Hit(_) => "Hit",
                CacheResult::Miss => "Miss",
                CacheResult::PartialHit { .. } => "PartialHit",
            }
        ),
    }
}

#[test]
fn test_cache_miss_when_no_cache_exists() {
    let cache_dir = TempDir::new().unwrap();
    let hashes = BTreeMap::new();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, cache_dir.path()),
            CacheResult::Miss
        ),
        "should be Miss when no cache file exists"
    );
}

#[test]
fn test_cache_miss_on_extra_file() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Add a new file → different file set → Miss (not PartialHit).
    let mut extra_sources = sources.clone();
    extra_sources.insert("extra.py".to_string(), "x = 1".to_string());
    let new_hashes = cpg_cache::compute_file_hashes(&extra_sources);

    assert!(
        matches!(
            cpg_cache::load_cache(&new_hashes, cache_dir.path()),
            CacheResult::Miss
        ),
        "should be Miss when file set changes (not just content)"
    );
}

// ---------------------------------------------------------------------------
// Multi-algorithm round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_cache_round_trip_multiple_algorithms() {
    let (files, sources, diff) = make_python_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, cache_dir.path()));
    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);

    let algos = vec![
        SlicingAlgorithm::LeftFlow,
        SlicingAlgorithm::FullFlow,
        SlicingAlgorithm::ThinSlice,
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::BarrierSlice,
    ];
    for algo in algos {
        let config = SliceConfig::default().with_algorithm(algo);
        let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();
        let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();
        assert_eq!(
            result_original.blocks.len(),
            result_cached.blocks.len(),
            "{}: block count mismatch",
            algo.name()
        );
    }
}

// ---------------------------------------------------------------------------
// Call graph and DFG preservation
// ---------------------------------------------------------------------------

#[test]
fn test_cache_preserves_call_graph() {
    let (files, sources, _diff) = make_python_test();
    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, cache_dir.path()));
    assert_eq!(
        ctx_original.cpg.call_graph.functions.len(),
        loaded_cpg.call_graph.functions.len()
    );
    assert_eq!(
        ctx_original.cpg.call_graph.calls.len(),
        loaded_cpg.call_graph.calls.len()
    );
}

#[test]
fn test_cache_preserves_dfg() {
    let (files, sources, _diff) = make_python_test();
    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, cache_dir.path()));
    assert_eq!(ctx_original.cpg.dfg.edges.len(), loaded_cpg.dfg.edges.len());
    assert_eq!(ctx_original.cpg.dfg.defs.len(), loaded_cpg.dfg.defs.len());
}

// ---------------------------------------------------------------------------
// Cache metadata
// ---------------------------------------------------------------------------

#[test]
fn test_cache_meta_json_is_valid() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    let meta_text = std::fs::read_to_string(cache_dir.path().join("cache-meta.json")).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&meta_text).unwrap();
    assert!(meta["prism_version"].is_string());
    assert!(meta["cache_version"].is_number());
    assert!(meta["file_count"].is_number());
    assert!(meta["node_count"].is_number());
    assert!(meta["edge_count"].is_number());
    assert!(meta["cache_size_bytes"].is_number());
}

// ---------------------------------------------------------------------------
// Phase 2: Incremental cache update tests
// ---------------------------------------------------------------------------

#[test]
fn test_incremental_rebuild_produces_correct_results() {
    let (files, sources, diff) = make_python_test();

    // Build and cache.
    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    // Modify a file to trigger partial hit.
    let mut modified_sources = sources.clone();
    let changed_file = modified_sources.keys().next().unwrap().clone();
    modified_sources
        .get_mut(&changed_file)
        .unwrap()
        .push_str("\n# comment\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    // Load returns PartialHit.
    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert!(changed_files.contains(&changed_file));

            // Rebuild incrementally using the original parsed files.
            // (In real usage, the changed file would be re-parsed; here we test the
            // merge logic with the same parsed files.)
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            let ctx_incremental = CpgContext::build_with_cached_cpg(&files, cpg, None);

            // The incremental CPG should produce the same algorithm results as a
            // full build (since the actual ParsedFiles haven't changed — only hashes differ).
            let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
            let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();
            let result_incremental =
                algorithms::run_slicing(&ctx_incremental, &diff, &config).unwrap();

            assert_eq!(
                result_original.blocks.len(),
                result_incremental.blocks.len(),
                "incremental rebuild should produce same block count"
            );
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_rebuild_node_count_matches_full_build() {
    let (files, sources, _diff) = make_python_test();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, cache_dir.path()).unwrap();

    // Trigger partial hit by changing one file hash.
    let mut modified_sources = sources.clone();
    let changed_file = modified_sources.keys().next().unwrap().clone();
    modified_sources
        .get_mut(&changed_file)
        .unwrap()
        .push_str("\nz = 99\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
            ..
        } => {
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );

            // Node and edge counts should match the full build (using same ParsedFiles).
            assert_eq!(
                ctx_full.cpg.graph.node_count(),
                cpg.graph.node_count(),
                "incremental node count should match full build"
            );
            assert_eq!(
                ctx_full.cpg.graph.edge_count(),
                cpg.graph.edge_count(),
                "incremental edge count should match full build"
            );
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_saves_updated_cache() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // After incremental rebuild, saving the new CPG with updated hashes
    // should produce a valid cache that hits on the next load.
    let mut modified_sources = sources.clone();
    let changed_file = modified_sources.keys().next().unwrap().clone();
    modified_sources
        .get_mut(&changed_file)
        .unwrap()
        .push_str("\n# v2\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
            ..
        } => {
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );

            // Save with the new hashes.
            cpg_cache::save_cache(&cpg, &new_hashes, cache_dir.path()).unwrap();

            // The next load with the same new hashes should be a full Hit.
            assert!(
                matches!(
                    cpg_cache::load_cache(&new_hashes, cache_dir.path()),
                    CacheResult::Hit(_)
                ),
                "saved incremental cache should hit on next load"
            );
        }
        _ => panic!("expected PartialHit"),
    }
}
