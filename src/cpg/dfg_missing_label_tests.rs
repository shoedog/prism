//! Task 3 RD unavailable-function identity and cache lifecycle contracts.
use crate::data_flow::test_support::edit_legacy_parts;

use crate::cpg::CodePropertyGraph;
use crate::cpg_cache::{self, CacheResult};
use crate::data_flow::test_support::label_snapshot;
use crate::data_flow::VarLocation;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn parse_python_sources(
    sources: &BTreeMap<String, String>,
) -> BTreeMap<String, crate::ast::ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                path.clone(),
                crate::ast::ParsedFile::parse(path, source, Language::Python).unwrap(),
            )
        })
        .collect()
}

fn partial_rebuild(cache_dir: &Path, sources: &BTreeMap<String, String>) -> CodePropertyGraph {
    let hashes = cpg_cache::compute_file_hashes(sources);
    let files = parse_python_sources(sources);
    match cpg_cache::load_cache(&hashes, false, cache_dir) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("one-file edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("one-file edit unexpectedly produced a miss"),
    }
}

fn function_label_key(cpg: &CodePropertyGraph, function: &str) -> (VarLocation, VarLocation) {
    label_snapshot(&cpg.dfg)
        .keys()
        .find(|(from, to)| {
            from.file == "a.py"
                && to.file == "a.py"
                && from.function == function
                && to.function == function
        })
        .cloned()
        .unwrap_or_else(|| panic!("missing retained label for {function}"))
}

#[test]
fn partial_hit_fallback_unions_persisted_and_new_function_identities() {
    let sources = BTreeMap::from([
        (
            "a.py".to_string(),
            concat!(
                "def u(value):\n    return value\n\n",
                "def v(value):\n    return value\n\n",
                "def f():\n    value = source()\n    sink(value)\n",
            )
            .to_string(),
        ),
        (
            "b.py".to_string(),
            "def edited():\n    zero = source()\n    sink(zero)\n".to_string(),
        ),
    ]);
    let files = parse_python_sources(&sources);
    let mut cached = CodePropertyGraph::build(&files);
    assert_eq!(
        cached.dfg.rd_function_stats()["a.py"].functions_without_cfg,
        2
    );
    let u_key = function_label_key(&cached, "u");
    let f_key = function_label_key(&cached, "f");
    edit_legacy_parts(&mut cached.dfg, |parts| {
        assert!(parts.labels.remove(&u_key).is_some());
    });

    let cache_dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cached, &hashes, false, cache_dir.path()).unwrap();

    let mut first_sources = sources.clone();
    first_sources.insert(
        "b.py".to_string(),
        "def edited():\n    one = source()\n    sink(one)\n".to_string(),
    );
    let mut first = partial_rebuild(cache_dir.path(), &first_sources);
    let first_stats = first.dfg.rd_function_stats()["a.py"].clone();
    assert!(!label_snapshot(&first.dfg).contains_key(&u_key));
    edit_legacy_parts(&mut first.dfg, |parts| {
        assert!(parts.labels.remove(&f_key).is_some());
    });
    let first_hashes = cpg_cache::compute_file_hashes(&first_sources);
    cpg_cache::save_cache(&first, &first_hashes, false, cache_dir.path()).unwrap();

    let mut second_sources = first_sources.clone();
    second_sources.insert(
        "b.py".to_string(),
        "def edited():\n    two = source()\n    sink(two)\n".to_string(),
    );
    let second = partial_rebuild(cache_dir.path(), &second_sources);
    let second_stats = second.dfg.rd_function_stats()["a.py"].clone();
    let second_hashes = cpg_cache::compute_file_hashes(&second_sources);
    cpg_cache::save_cache(&second, &second_hashes, false, cache_dir.path()).unwrap();

    let mut third_sources = second_sources.clone();
    third_sources.insert(
        "b.py".to_string(),
        "def edited():\n    three = source()\n    sink(three)\n".to_string(),
    );
    let third = partial_rebuild(cache_dir.path(), &third_sources);
    let third_stats = third.dfg.rd_function_stats()["a.py"].clone();

    assert_eq!(first_stats.functions_without_cfg, 2);
    assert_eq!(
        second_stats.functions_without_cfg, 3,
        "missing U and newly affected F must union with persisted U/V"
    );
    assert_eq!(
        third_stats.functions_without_cfg, 3,
        "the same missing U/F keys must be idempotent on another PartialHit"
    );
}

#[test]
fn repeated_partial_hits_do_not_accumulate_retained_missing_label_stats() {
    let sources = BTreeMap::from([
        (
            "a.py".to_string(),
            "def retained(value):\n    return value\n".to_string(),
        ),
        (
            "b.py".to_string(),
            "def edited():\n    value = source()\n    sink(value)\n".to_string(),
        ),
    ]);
    let files = parse_python_sources(&sources);
    let mut cached = CodePropertyGraph::build(&files);
    let cold_stats = cached.dfg.rd_function_stats()["a.py"].clone();
    assert_eq!(
        cold_stats.functions_without_cfg, 1,
        "the retained fixture must begin with exactly one unavailable function"
    );
    let retained_key = label_snapshot(&cached.dfg)
        .keys()
        .find(|(from, to)| from.file == "a.py" && to.file == "a.py")
        .cloned()
        .expect("the retained function must have a DFG label to remove");
    edit_legacy_parts(&mut cached.dfg, |parts| {
        assert!(parts.labels.remove(&retained_key).is_some());
    });

    let cache_dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cached, &hashes, false, cache_dir.path()).unwrap();

    let mut first_sources = sources.clone();
    first_sources.insert(
        "b.py".to_string(),
        "def edited():\n    first = source()\n    sink(first)\n".to_string(),
    );
    let first_hashes = cpg_cache::compute_file_hashes(&first_sources);
    let first_files = parse_python_sources(&first_sources);
    let first = match cpg_cache::load_cache(&first_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &first_files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("first edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("first edit unexpectedly produced a miss"),
    };
    let first_stats = first.dfg.rd_function_stats()["a.py"].clone();
    assert!(!label_snapshot(&first.dfg).contains_key(&retained_key));
    cpg_cache::save_cache(&first, &first_hashes, false, cache_dir.path()).unwrap();

    let mut second_sources = first_sources.clone();
    second_sources.insert(
        "b.py".to_string(),
        "def edited():\n    second = source()\n    sink(second)\n".to_string(),
    );
    let second_hashes = cpg_cache::compute_file_hashes(&second_sources);
    let second_files = parse_python_sources(&second_sources);
    let second = match cpg_cache::load_cache(&second_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(changed_files, BTreeSet::from(["b.py".to_string()]));
            CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &second_files,
                None,
            )
        }
        CacheResult::Hit(_) => panic!("second edit unexpectedly produced a full hit"),
        CacheResult::Miss => panic!("second edit unexpectedly produced a miss"),
    };
    let second_stats = second.dfg.rd_function_stats()["a.py"].clone();
    assert!(!label_snapshot(&second.dfg).contains_key(&retained_key));

    assert_eq!(
        first_stats, cold_stats,
        "the first PartialHit accumulated the retained fallback function"
    );
    assert_eq!(
        second_stats, cold_stats,
        "the second PartialHit accumulated the retained fallback function again"
    );
}
