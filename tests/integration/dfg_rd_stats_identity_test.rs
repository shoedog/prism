//! Task 3 RD unavailable-function identity and cache lifecycle contracts.

use prism::cpg::CodePropertyGraph;
use prism::cpg_cache::{self, CacheResult};
use prism::data_flow::VarLocation;
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn parse_python_sources(
    sources: &BTreeMap<String, String>,
) -> BTreeMap<String, prism::ast::ParsedFile> {
    sources
        .iter()
        .map(|(path, source)| {
            (
                path.clone(),
                prism::ast::ParsedFile::parse(path, source, Language::Python).unwrap(),
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
    cpg.dfg
        .labels
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
        cached.dfg.rd_function_stats["a.py"].functions_without_cfg,
        2
    );
    let u_key = function_label_key(&cached, "u");
    let f_key = function_label_key(&cached, "f");
    assert!(cached.dfg.labels.remove(&u_key).is_some());

    let cache_dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cached, &hashes, false, cache_dir.path()).unwrap();

    let mut first_sources = sources.clone();
    first_sources.insert(
        "b.py".to_string(),
        "def edited():\n    one = source()\n    sink(one)\n".to_string(),
    );
    let mut first = partial_rebuild(cache_dir.path(), &first_sources);
    let first_stats = first.dfg.rd_function_stats["a.py"].clone();
    assert!(!first.dfg.labels.contains_key(&u_key));
    assert!(first.dfg.labels.remove(&f_key).is_some());
    let first_hashes = cpg_cache::compute_file_hashes(&first_sources);
    cpg_cache::save_cache(&first, &first_hashes, false, cache_dir.path()).unwrap();

    let mut second_sources = first_sources.clone();
    second_sources.insert(
        "b.py".to_string(),
        "def edited():\n    two = source()\n    sink(two)\n".to_string(),
    );
    let second = partial_rebuild(cache_dir.path(), &second_sources);
    let second_stats = second.dfg.rd_function_stats["a.py"].clone();
    let second_hashes = cpg_cache::compute_file_hashes(&second_sources);
    cpg_cache::save_cache(&second, &second_hashes, false, cache_dir.path()).unwrap();

    let mut third_sources = second_sources.clone();
    third_sources.insert(
        "b.py".to_string(),
        "def edited():\n    three = source()\n    sink(three)\n".to_string(),
    );
    let third = partial_rebuild(cache_dir.path(), &third_sources);
    let third_stats = third.dfg.rd_function_stats["a.py"].clone();

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
