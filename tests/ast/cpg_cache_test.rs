#[path = "../common/mod.rs"]
mod common;
use common::*;

use prism::cpg_cache;

// ---------------------------------------------------------------------------
// Round-trip tests: build CPG → save → load → verify identical results
// ---------------------------------------------------------------------------

#[test]
fn test_cache_round_trip_python() {
    let (files, sources, diff) = make_python_test();

    // Build the CPG from scratch.
    let ctx_original = CpgContext::build(&files, None);

    // Save to a temp directory.
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    // Verify cache files exist.
    assert!(cache_dir.path().join("cpg-cache.bin").exists());
    assert!(cache_dir.path().join("cache-meta.json").exists());

    // Load from cache.
    let loaded_cpg = cpg_cache::load_cache(&hashes, cache_dir.path())
        .expect("cache load should succeed with matching hashes");

    // Verify the loaded CPG has the same node and edge counts.
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

    // Verify algorithm results are identical.
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();

    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);
    let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();

    assert_eq!(
        result_original.blocks.len(),
        result_cached.blocks.len(),
        "LeftFlow block count should be identical from cache"
    );

    // Compare block contents.
    for (orig, cached) in result_original
        .blocks
        .iter()
        .zip(result_cached.blocks.iter())
    {
        assert_eq!(
            orig.file_line_map, cached.file_line_map,
            "block file_line_maps should be identical"
        );
    }
}

#[test]
fn test_cache_round_trip_javascript() {
    let (files, sources, _diff) = make_javascript_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg =
        cpg_cache::load_cache(&hashes, cache_dir.path()).expect("cache load should succeed");

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

    let loaded_cpg =
        cpg_cache::load_cache(&hashes, cache_dir.path()).expect("cache load should succeed");

    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count()
    );

    // Verify taint algorithm works with cached CPG.
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
fn test_cache_invalidation_on_file_change() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Modify a source file → hashes differ → cache miss.
    let mut modified_sources = sources.clone();
    if let Some(val) = modified_sources.values_mut().next() {
        val.push_str("\n# new comment\n");
    }
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    let result = cpg_cache::load_cache(&new_hashes, cache_dir.path());
    assert!(
        result.is_none(),
        "cache should be invalidated on file change"
    );
}

#[test]
fn test_cache_miss_when_no_cache_exists() {
    let cache_dir = TempDir::new().unwrap();
    let hashes = BTreeMap::new();

    let result = cpg_cache::load_cache(&hashes, cache_dir.path());
    assert!(
        result.is_none(),
        "should return None when no cache file exists"
    );
}

#[test]
fn test_cache_invalidation_on_extra_file() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Add a new file → hashes differ → cache miss.
    let mut extra_sources = sources.clone();
    extra_sources.insert("extra.py".to_string(), "x = 1".to_string());
    let new_hashes = cpg_cache::compute_file_hashes(&extra_sources);

    let result = cpg_cache::load_cache(&new_hashes, cache_dir.path());
    assert!(
        result.is_none(),
        "cache should be invalidated when file set changes"
    );
}

// ---------------------------------------------------------------------------
// Multi-algorithm round-trip: verify graph-based algorithms work from cache
// ---------------------------------------------------------------------------

#[test]
fn test_cache_round_trip_multiple_algorithms() {
    let (files, sources, diff) = make_python_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = cpg_cache::load_cache(&hashes, cache_dir.path()).unwrap();
    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);

    // Test several graph-based algorithms.
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
            "{}: block count mismatch between original and cached",
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

    let loaded_cpg = cpg_cache::load_cache(&hashes, cache_dir.path()).unwrap();

    // Call graph should have the same functions.
    assert_eq!(
        ctx_original.cpg.call_graph.functions.len(),
        loaded_cpg.call_graph.functions.len(),
        "call graph function count mismatch"
    );

    // Call edges should match.
    assert_eq!(
        ctx_original.cpg.call_graph.calls.len(),
        loaded_cpg.call_graph.calls.len(),
        "call graph edge count mismatch"
    );
}

#[test]
fn test_cache_preserves_dfg() {
    let (files, sources, _diff) = make_python_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, cache_dir.path()).unwrap();

    let loaded_cpg = cpg_cache::load_cache(&hashes, cache_dir.path()).unwrap();

    assert_eq!(
        ctx_original.cpg.dfg.edges.len(),
        loaded_cpg.dfg.edges.len(),
        "DFG edge count mismatch"
    );

    assert_eq!(
        ctx_original.cpg.dfg.defs.len(),
        loaded_cpg.dfg.defs.len(),
        "DFG defs index mismatch"
    );
}

// ---------------------------------------------------------------------------
// Cache metadata file
// ---------------------------------------------------------------------------

#[test]
fn test_cache_meta_json_is_valid() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    let meta_path = cache_dir.path().join("cache-meta.json");
    let meta_text = std::fs::read_to_string(&meta_path).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&meta_text).unwrap();

    assert!(meta["prism_version"].is_string());
    assert!(meta["cache_version"].is_number());
    assert!(meta["file_count"].is_number());
    assert!(meta["node_count"].is_number());
    assert!(meta["edge_count"].is_number());
    assert!(meta["cache_size_bytes"].is_number());
}
