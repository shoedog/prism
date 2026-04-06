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
// Cache edge cases: corruption, version, empty sets
// ---------------------------------------------------------------------------

#[test]
fn test_cache_miss_on_corrupt_binary() {
    let cache_dir = TempDir::new().unwrap();
    // Write garbage to the cache file.
    std::fs::create_dir_all(cache_dir.path()).unwrap();
    std::fs::write(cache_dir.path().join("cpg-cache.bin"), b"corrupt data").unwrap();

    let hashes = BTreeMap::new();
    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, cache_dir.path()),
            CacheResult::Miss
        ),
        "corrupt binary should produce Miss"
    );
}

#[test]
fn test_cache_miss_on_truncated_binary() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Truncate the binary file to simulate interrupted write.
    let bin_path = cache_dir.path().join("cpg-cache.bin");
    let data = std::fs::read(&bin_path).unwrap();
    std::fs::write(&bin_path, &data[..data.len() / 2]).unwrap();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, cache_dir.path()),
            CacheResult::Miss
        ),
        "truncated binary should produce Miss"
    );
}

#[test]
fn test_cache_hit_with_empty_file_set() {
    // Empty hashes → save → load with same empty hashes → Hit.
    let files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    let sources: BTreeMap<String, String> = BTreeMap::new();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, cache_dir.path()),
            CacheResult::Hit(_)
        ),
        "empty file set with matching (empty) hashes should Hit"
    );
}

#[test]
fn test_cache_miss_on_removed_file() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Remove a file from hashes → different key set → Miss.
    let fewer_hashes: BTreeMap<String, String> = BTreeMap::new();
    assert!(
        matches!(
            cpg_cache::load_cache(&fewer_hashes, cache_dir.path()),
            CacheResult::Miss
        ),
        "removed file (different key set) should produce Miss"
    );
}

#[test]
fn test_compute_file_hashes_deterministic() {
    let mut sources = BTreeMap::new();
    sources.insert("a.py".to_string(), "x = 1\n".to_string());
    sources.insert("b.py".to_string(), "y = 2\n".to_string());

    let h1 = cpg_cache::compute_file_hashes(&sources);
    let h2 = cpg_cache::compute_file_hashes(&sources);
    assert_eq!(h1, h2, "hashes should be deterministic");
}

#[test]
fn test_compute_file_hashes_content_sensitive() {
    let mut s1 = BTreeMap::new();
    s1.insert("a.py".to_string(), "x = 1".to_string());
    let mut s2 = BTreeMap::new();
    s2.insert("a.py".to_string(), "x = 2".to_string());

    let h1 = cpg_cache::compute_file_hashes(&s1);
    let h2 = cpg_cache::compute_file_hashes(&s2);
    assert_ne!(
        h1["a.py"], h2["a.py"],
        "different content should produce different hashes"
    );
}

// ---------------------------------------------------------------------------
// Direct unit tests for CallGraph incremental methods
// ---------------------------------------------------------------------------

#[test]
fn test_callgraph_remove_files() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let cg = CallGraph::build(&files);

    // Baseline: functions from both files exist.
    let all_fids: Vec<_> = cg.functions.values().flatten().collect();
    assert!(
        all_fids.iter().any(|f| f.file == "src/device.c"),
        "should have device.c functions"
    );
    assert!(
        all_fids.iter().any(|f| f.file == "src/handler.c"),
        "should have handler.c functions"
    );

    let mut cg = cg;
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    cg.remove_files(&exclude);

    // After removal: handler.c functions should be gone.
    let remaining_fids: Vec<_> = cg.functions.values().flatten().collect();
    assert!(
        remaining_fids.iter().all(|f| f.file != "src/handler.c"),
        "handler.c functions should be removed"
    );
    assert!(
        remaining_fids.iter().any(|f| f.file == "src/device.c"),
        "device.c functions should be preserved"
    );

    // Call sites from handler.c should be removed.
    for sites in cg.calls.values() {
        for site in sites {
            assert_ne!(
                site.caller.file, "src/handler.c",
                "calls from handler.c should be removed"
            );
        }
    }
    for sites in cg.callers.values() {
        for site in sites {
            assert_ne!(
                site.caller.file, "src/handler.c",
                "callers from handler.c should be removed"
            );
        }
    }

    // imports for handler.c should be removed.
    assert!(
        !cg.imports.contains_key("src/handler.c"),
        "imports for handler.c should be removed"
    );
}

#[test]
fn test_callgraph_merge() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let full_cg = CallGraph::build(&files);

    // Build separate CGs: one with only device.c retained, one with only handler.c.
    let mut retained = full_cg.clone();
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    retained.remove_files(&exclude);

    let handler_only = BTreeSet::from(["src/handler.c".to_string()]);
    let subset = CallGraph::build_direct_subset(&files, &handler_only);

    // Merge and verify both files' functions exist.
    retained.merge(subset);

    let all_fids: Vec<_> = retained.functions.values().flatten().collect();
    assert!(
        all_fids.iter().any(|f| f.file == "src/device.c"),
        "device.c functions should exist after merge"
    );
    assert!(
        all_fids.iter().any(|f| f.file == "src/handler.c"),
        "handler.c functions should exist after merge"
    );
}

#[test]
fn test_callgraph_build_direct_subset() {
    let (files, _sources, _diff) = make_c_multifile_test();

    let only = BTreeSet::from(["src/handler.c".to_string()]);
    let subset = CallGraph::build_direct_subset(&files, &only);

    // Should have functions only from handler.c.
    let all_fids: Vec<_> = subset.functions.values().flatten().collect();
    assert!(
        all_fids.iter().all(|f| f.file == "src/handler.c"),
        "subset should only contain handler.c functions"
    );
    assert!(
        !all_fids.is_empty(),
        "subset should contain at least one function"
    );

    // Call sites should reference handler.c as caller.
    for (caller, _) in &subset.calls {
        assert_eq!(caller.file, "src/handler.c");
    }
}

#[test]
fn test_callgraph_remove_empty_set() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let cg = CallGraph::build(&files);
    let original_count: usize = cg.functions.values().map(|v| v.len()).sum();

    let mut cg = cg;
    let exclude = BTreeSet::new();
    cg.remove_files(&exclude);

    let after_count: usize = cg.functions.values().map(|v| v.len()).sum();
    assert_eq!(
        original_count, after_count,
        "removing empty set should not change anything"
    );
}

// ---------------------------------------------------------------------------
// Direct unit tests for DataFlowGraph incremental methods
// ---------------------------------------------------------------------------

#[test]
fn test_dfg_remove_files() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let dfg = DataFlowGraph::build(&files);

    let has_handler_edges = dfg
        .edges
        .iter()
        .any(|e| e.from.file == "src/handler.c" || e.to.file == "src/handler.c");

    let mut dfg = dfg;
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    dfg.remove_files(&exclude);

    // After removal: no edges involving handler.c.
    let still_has_handler = dfg
        .edges
        .iter()
        .any(|e| e.from.file == "src/handler.c" || e.to.file == "src/handler.c");
    assert!(
        !still_has_handler,
        "handler.c edges should be removed from DFG"
    );

    // Defs and uses for handler.c should be gone.
    for (key, _) in &dfg.defs {
        assert_ne!(
            key.0, "src/handler.c",
            "defs from handler.c should be removed"
        );
    }
    for (key, _) in &dfg.uses {
        assert_ne!(
            key.0, "src/handler.c",
            "uses from handler.c should be removed"
        );
    }

    // Device.c entries should be preserved if they existed.
    let has_device = dfg.edges.iter().any(|e| e.from.file == "src/device.c");
    let device_defs = dfg.defs.keys().any(|k| k.0 == "src/device.c");
    // At least one should be true (device.c has variables).
    assert!(
        has_device || device_defs || dfg.edges.is_empty(),
        "device.c data should be preserved (or DFG was empty)"
    );
}

#[test]
fn test_dfg_merge() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let full_dfg = DataFlowGraph::build(&files);
    let full_edge_count = full_dfg.edges.len();

    // Remove handler.c, then build subset and merge.
    let mut retained = full_dfg;
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    retained.remove_files(&exclude);

    let handler_only = BTreeSet::from(["src/handler.c".to_string()]);
    let subset = DataFlowGraph::build_subset(&files, &handler_only);

    retained.merge(subset);

    // Merged DFG should have edges from both files.
    let has_device = retained.edges.iter().any(|e| e.from.file == "src/device.c");
    let has_handler = retained
        .edges
        .iter()
        .any(|e| e.from.file == "src/handler.c");

    // At least handler.c should be present after merge (it has function calls).
    if full_edge_count > 0 {
        assert!(
            has_device || has_handler,
            "merged DFG should have edges from at least one file"
        );
    }

    // Forward/backward adjacency should be rebuilt.
    // Every edge should have a corresponding forward entry.
    for edge in &retained.edges {
        assert!(
            retained.forward.contains_key(&edge.from),
            "forward adjacency should be rebuilt after merge"
        );
    }
}

#[test]
fn test_dfg_build_subset() {
    let (files, _sources, _diff) = make_c_multifile_test();

    let only = BTreeSet::from(["src/device.c".to_string()]);
    let subset = DataFlowGraph::build_subset(&files, &only);

    // All edges should involve device.c only.
    for edge in &subset.edges {
        assert_eq!(
            edge.from.file, "src/device.c",
            "subset edges should only come from device.c"
        );
    }

    // All defs should be from device.c.
    for (key, _) in &subset.defs {
        assert_eq!(key.0, "src/device.c");
    }
}

#[test]
fn test_dfg_remove_empty_set() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let dfg = DataFlowGraph::build(&files);
    let original_edge_count = dfg.edges.len();

    let mut dfg = dfg;
    let exclude = BTreeSet::new();
    dfg.remove_files(&exclude);

    assert_eq!(
        dfg.edges.len(),
        original_edge_count,
        "removing empty set should not change edge count"
    );
}

// ---------------------------------------------------------------------------
// Multi-file incremental cache tests
// ---------------------------------------------------------------------------

/// Build a 3-file Python fixture for multi-file incremental tests.
fn make_python_multifile() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let utils_src = r#"
def add(x, y):
    return x + y

def multiply(x, y):
    return x * y
"#;

    let calc_src = r#"
from utils import add, multiply

def compute(a, b):
    total = add(a, b)
    scaled = multiply(total, 2)
    return scaled
"#;

    let main_src = r#"
from calc import compute

def main():
    result = compute(3, 4)
    print(result)
    return result
"#;

    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();

    for (path, src) in [
        ("src/utils.py", utils_src),
        ("src/calc.py", calc_src),
        ("src/main.py", main_src),
    ] {
        let parsed = ParsedFile::parse(path, src, Language::Python).unwrap();
        files.insert(path.to_string(), parsed);
        sources.insert(path.to_string(), src.to_string());
    }

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/calc.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    (files, sources, diff)
}

#[test]
fn test_incremental_multifile_partial_hit() {
    let (files, sources, _diff) = make_python_multifile();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, cache_dir.path()).unwrap();

    // Modify TWO files to test multi-file partial hit.
    let mut modified_sources = sources.clone();
    modified_sources
        .get_mut("src/utils.py")
        .unwrap()
        .push_str("\n# changed\n");
    modified_sources
        .get_mut("src/main.py")
        .unwrap()
        .push_str("\n# also changed\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit { changed_files, .. } => {
            assert_eq!(changed_files.len(), 2, "should detect 2 changed files");
            assert!(changed_files.contains("src/utils.py"));
            assert!(changed_files.contains("src/main.py"));
        }
        other => panic!(
            "expected PartialHit, got {}",
            match other {
                CacheResult::Hit(_) => "Hit",
                CacheResult::Miss => "Miss",
                _ => "PartialHit",
            }
        ),
    }
}

#[test]
fn test_incremental_multifile_rebuild_matches_full() {
    let (files, sources, diff) = make_python_multifile();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, cache_dir.path()).unwrap();

    // Change two files.
    let mut modified_sources = sources.clone();
    modified_sources
        .get_mut("src/utils.py")
        .unwrap()
        .push_str("\n# v2\n");
    modified_sources
        .get_mut("src/calc.py")
        .unwrap()
        .push_str("\n# v2\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );

            // Node/edge count should match full build (same parsed files).
            assert_eq!(
                ctx_full.cpg.graph.node_count(),
                cpg.graph.node_count(),
                "multi-file incremental node count should match full build"
            );
            assert_eq!(
                ctx_full.cpg.graph.edge_count(),
                cpg.graph.edge_count(),
                "multi-file incremental edge count should match full build"
            );

            // Algorithm results should match.
            let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
            let ctx_inc = CpgContext::build_with_cached_cpg(&files, cpg, None);
            let res_full = algorithms::run_slicing(&ctx_full, &diff, &config).unwrap();
            let res_inc = algorithms::run_slicing(&ctx_inc, &diff, &config).unwrap();
            assert_eq!(res_full.blocks.len(), res_inc.blocks.len());
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_multifile_c_cross_file_calls() {
    let (files, sources, diff) = make_c_multifile_test();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, cache_dir.path()).unwrap();

    // Change handler.c (the caller) to trigger incremental rebuild.
    let mut modified_sources = sources.clone();
    modified_sources
        .get_mut("src/handler.c")
        .unwrap()
        .push_str("\n// changed\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert!(changed_files.contains("src/handler.c"));
            assert!(!changed_files.contains("src/device.c"));

            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            let ctx_inc = CpgContext::build_with_cached_cpg(&files, cpg, None);

            // Taint and BarrierSlice use the call graph — verify they produce
            // the same results from incremental as from full build.
            for algo in [SlicingAlgorithm::Taint, SlicingAlgorithm::BarrierSlice] {
                let config = SliceConfig::default().with_algorithm(algo);
                let res_full = algorithms::run_slicing(&ctx_full, &diff, &config).unwrap();
                let res_inc = algorithms::run_slicing(&ctx_inc, &diff, &config).unwrap();
                assert_eq!(
                    res_full.blocks.len(),
                    res_inc.blocks.len(),
                    "{}: block count mismatch in cross-file incremental",
                    algo.name()
                );
            }
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_all_files_changed_matches_full() {
    let (files, sources, diff) = make_python_multifile();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, cache_dir.path()).unwrap();

    // Change ALL files — still a PartialHit (same keys, all different values).
    let mut modified_sources = sources.clone();
    for val in modified_sources.values_mut() {
        val.push_str("\n# changed\n");
    }
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(
                changed_files.len(),
                sources.len(),
                "all files should be marked changed"
            );

            // Incremental with all changed should still produce valid CPG.
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            assert_eq!(
                ctx_full.cpg.graph.node_count(),
                cpg.graph.node_count(),
                "all-changed incremental should match full build"
            );

            let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
            let ctx_inc = CpgContext::build_with_cached_cpg(&files, cpg, None);
            let res_full = algorithms::run_slicing(&ctx_full, &diff, &config).unwrap();
            let res_inc = algorithms::run_slicing(&ctx_inc, &diff, &config).unwrap();
            assert_eq!(res_full.blocks.len(), res_inc.blocks.len());
        }
        _ => panic!("expected PartialHit"),
    }
}

// ---------------------------------------------------------------------------
// Cache overwrite / re-save tests
// ---------------------------------------------------------------------------

#[test]
fn test_cache_overwrite_with_different_files() {
    let (files_py, sources_py, _diff) = make_python_test();
    let (files_c, sources_c, _diff) = make_c_multifile_test();

    let cache_dir = TempDir::new().unwrap();

    // Save Python cache.
    let ctx_py = CpgContext::build(&files_py, None);
    let hashes_py = cpg_cache::compute_file_hashes(&sources_py);
    cpg_cache::save_cache(&ctx_py.cpg, &hashes_py, cache_dir.path()).unwrap();
    assert!(matches!(
        cpg_cache::load_cache(&hashes_py, cache_dir.path()),
        CacheResult::Hit(_)
    ));

    // Overwrite with C cache.
    let ctx_c = CpgContext::build(&files_c, None);
    let hashes_c = cpg_cache::compute_file_hashes(&sources_c);
    cpg_cache::save_cache(&ctx_c.cpg, &hashes_c, cache_dir.path()).unwrap();

    // Old Python hashes should Miss (different file set).
    assert!(matches!(
        cpg_cache::load_cache(&hashes_py, cache_dir.path()),
        CacheResult::Miss
    ));

    // New C hashes should Hit.
    assert!(matches!(
        cpg_cache::load_cache(&hashes_c, cache_dir.path()),
        CacheResult::Hit(_)
    ));
}

// ---------------------------------------------------------------------------
// Phase 2: Incremental cache update tests (original)
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
