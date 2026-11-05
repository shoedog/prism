//! Independent addition budgets, RD refusal, and persisted cache parity.
use super::{FileRefs, Producer};
use crate::{
    ast::ParsedFile,
    cpg::{with_recall_cap, CodePropertyGraph},
    cpg_cache::{self, CacheResult},
    data_flow::{compact::CompactDfg, DataFlowGraph},
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};

const SOURCE: &str = "package p\nfunc f() {\np := T{}\np.a.b = 1\nuse(p.a)\nuse(p)\n}\n";
fn refs(source: &str, cap: usize) -> FileRefs {
    let parsed = ParsedFile::parse("a.go", source, Language::Go).unwrap();
    with_recall_cap(cap, || Producer::file_refs(&parsed.path, &parsed))
}
fn is_recall(e: &crate::data_flow::FlowEdge) -> bool {
    e.from.path.base == e.to.path.base
        && e.from.path.fields.len() > e.to.path.fields.len()
        && e.from.path.fields.starts_with(&e.to.path.fields)
}
fn bytes(parts: FileRefs) -> Vec<u8> {
    bincode::serialize(&DataFlowGraph {
        storage: CompactDfg::from_parts(parts.into_parts()).unwrap(),
    })
    .unwrap()
}

#[test]
fn go_field_recall_addition_boundary_and_delayed_use() {
    let admitted = refs(SOURCE, usize::MAX);
    let n = admitted.rd_function_stats.recall.candidates();
    assert_eq!(n, 2);
    assert_eq!(bytes(refs(SOURCE, n)), bytes(refs(SOURCE, n + 1)));
    let capped = refs(SOURCE, n - 1);
    assert_eq!(capped.rd_function_stats.recall.functions_capped(), 1);
    assert_eq!(capped.rd_function_stats.recall.candidates_capped(), n);
    assert_eq!(capped.defs, admitted.defs);
    assert_eq!(
        capped.edges,
        admitted
            .edges
            .iter()
            .filter(|e| !is_recall(e))
            .cloned()
            .collect::<Vec<_>>()
    );
    assert!(capped.edges.iter().all(|e| !is_recall(e)));
    // Preserve every ordinary Use on refusal, including existing prefix reads.
    assert_eq!(capped.uses, refs(SOURCE, 0).uses);
    assert!(admitted
        .uses
        .values()
        .flatten()
        .any(|u| u.path.to_string() == "p.a" && u.line == 5));
    for (key, label) in &capped.labels {
        assert_eq!(admitted.labels.get(key), Some(label));
    }
    assert_eq!(capped.rd_function_stats.fallback_functions_capped, 0);
}

#[test]
fn go_field_recall_rd_boundaries() {
    for (defs, limit) in [(true, 2048), (false, 4096)] {
        for over in [false, true] {
            let mut source = String::from("package p\nfunc f() {\np := T{}\np.f = 1\nuse(p)\n");
            let count = limit + usize::from(over);
            for i in 0..count - if defs { 2 } else { 3 } {
                source += &if defs {
                    format!("v{i} := 0\n")
                } else {
                    "noop()\n".into()
                };
            }
            source += "}\n";
            let parsed = ParsedFile::parse("a.go", &source, Language::Go).unwrap();
            let result = refs(&source, usize::MAX);
            if !defs {
                assert_eq!(
                    parsed
                        .statements_in_function(&parsed.all_functions()[0])
                        .len(),
                    count
                );
            } else {
                let unique: BTreeSet<_> = result
                    .defs
                    .values()
                    .flatten()
                    .map(|d| (&d.path, d.line, d.start_byte))
                    .collect();
                assert_eq!(unique.len(), count);
            }
            assert_eq!(
                result.edges.iter().filter(|e| is_recall(e)).count(),
                usize::from(!over)
            );
            assert_eq!(
                result.rd_function_stats.recall.functions_over_cap(),
                usize::from(over)
            );
            assert_eq!(
                result.rd_function_stats.functions_over_cap,
                usize::from(over)
            );
        }
    }
}

#[test]
fn go_field_recall_no_cfg_and_zero_edge_owner() {
    let result = refs("package p\nfunc f(p T) { p.f = 1 }\n", usize::MAX);
    assert!(result.edges.iter().all(|e| !is_recall(e)));
    assert_eq!(result.rd_function_stats.recall.functions_without_cfg(), 0);
    let empty = refs("package p\nfunc empty() {}\n", usize::MAX);
    assert!(empty.edges.is_empty());
    assert_eq!(empty.rd_function_stats.recall.candidates(), 0);
}

#[test]
fn go_field_recall_threads_one_and_four() {
    let files = BTreeMap::from([(
        "a.go".into(),
        ParsedFile::parse("a.go", SOURCE, Language::Go).unwrap(),
    )]);
    let run = |threads| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| bincode::serialize(&DataFlowGraph::build_on_current_pool(&files)).unwrap())
    };
    assert_eq!(run(1), run(4));
}

fn parse(sources: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(file, source)| {
            (
                file.clone(),
                ParsedFile::parse(file, source, Language::Go).unwrap(),
            )
        })
        .collect()
}
fn fingerprint(graph: &CodePropertyGraph) -> Vec<u8> {
    bincode::serialize(&(&graph.dfg, &graph.dfg_label_stats)).unwrap()
}

#[test]
fn go_field_recall_cold_full_partial_cache() {
    let mut sources = BTreeMap::from([
        ("a.go".to_string(), SOURCE.to_string()),
        ("b.go".into(), SOURCE.replace("func f", "func g")),
    ]);
    let files = parse(&sources);
    let graph = CodePropertyGraph::build(&files);
    assert_eq!(graph.dfg_label_stats.dfg_recall_candidates, 4);
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&graph, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("miss");
    };
    assert_eq!(fingerprint(&graph), fingerprint(&hit));
    // Keep one owner cached; change the other's surviving candidate population.
    sources.insert(
        "b.go".into(),
        SOURCE.replace("func f", "func g").replace("use(p.a)\n", ""),
    );
    let files = parse(&sources);
    let fresh = CodePropertyGraph::build(&files);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&cpg_cache::compute_file_hashes(&sources), false, dir.path())
    else {
        panic!("not partial");
    };
    assert_eq!(changed_files, BTreeSet::from(["b.go".into()]));
    let partial = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &files,
        None,
    );
    assert_eq!(fingerprint(&fresh), fingerprint(&partial));
    assert_eq!(partial.dfg_label_stats.dfg_recall_candidates, 3);
    assert_eq!(partial.dfg_label_stats.dfg_recall_max_candidates, 2);
}

#[test]
fn go_field_recall_capped_codec_stats() {
    let encoded = bytes(refs(SOURCE, 1));
    let decoded: DataFlowGraph = bincode::deserialize(&encoded).unwrap();
    assert_eq!(encoded, bincode::serialize(&decoded).unwrap());
    let stats = &decoded.rd_function_stats()["a.go"].recall;
    assert_eq!(stats.functions_capped(), 1);
    assert_eq!(stats.candidates_capped(), 2);
    let mut stats = refs(SOURCE, 1).rd_function_stats;
    stats.merge(stats.clone());
    assert_eq!(stats.recall.functions_capped(), 1);
    assert_eq!(stats.recall.candidates_capped(), 2);
}

#[test]
fn go_field_recall_loop_carried_telemetry() {
    let source = include_str!("../../../tests/fixtures/go_field_write/21_loop_carried/main.go");
    let files = BTreeMap::from([(
        "a.go".into(),
        ParsedFile::parse("a.go", source, Language::Go).unwrap(),
    )]);
    let graph = CodePropertyGraph::build(&files);
    let capped = DataFlowGraph {
        storage: CompactDfg::from_parts(refs(source, 0).into_parts()).unwrap(),
    };
    let without = CodePropertyGraph::build_incremental(
        graph.call_graph.clone(),
        capped,
        &BTreeSet::new(),
        &files,
        None,
    );
    assert_eq!(
        graph.dfg_label_stats.dfg_label_loop_carried,
        without.dfg_label_stats.dfg_label_loop_carried + 1
    );
}

fn cold_capped(files: &BTreeMap<String, ParsedFile>) -> CodePropertyGraph {
    let mut dfg = DataFlowGraph::empty();
    for (file, parsed) in files {
        let parts = with_recall_cap(1, || Producer::file_refs(file, parsed));
        dfg.merge(DataFlowGraph {
            storage: CompactDfg::from_parts(parts.into_parts()).unwrap(),
        });
    }
    let graph = CodePropertyGraph::build(files);
    CodePropertyGraph::build_incremental(graph.call_graph, dfg, &BTreeSet::new(), files, None)
}

#[test]
fn go_field_recall_capped_cold_full_partial_cache() {
    let mut sources = BTreeMap::from([
        ("a.go".to_string(), SOURCE.to_string()),
        ("b.go".into(), SOURCE.replace("func f", "func g")),
    ]);
    let files = parse(&sources);
    let graph = cold_capped(&files);
    assert_eq!(graph.dfg_label_stats.dfg_recall_functions_capped, 2);
    assert_eq!(graph.dfg_label_stats.dfg_recall_candidates_capped, 4);
    assert!(!graph
        .dfg
        .edges()
        .iter()
        .any(|e| e.from.path.base == e.to.path.base
            && e.from.path.fields.len() > e.to.path.fields.len()
            && e.from.path.fields.starts_with(&e.to.path.fields)));
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&graph, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("miss");
    };
    assert_eq!(fingerprint(&graph), fingerprint(&hit));
    sources.insert(
        "b.go".into(),
        SOURCE.replace("func f", "func g").replace("use(p.a)\n", ""),
    );
    let files = parse(&sources);
    let fresh = cold_capped(&files);
    let CacheResult::PartialHit {
        cached_call_graph,
        cached_dfg,
        changed_files,
    } = cpg_cache::load_cache(&cpg_cache::compute_file_hashes(&sources), false, dir.path())
    else {
        panic!("not partial");
    };
    let partial = CodePropertyGraph::build_incremental(
        cached_call_graph,
        cached_dfg,
        &changed_files,
        &files,
        None,
    );
    assert_eq!(fingerprint(&fresh), fingerprint(&partial));
    assert_eq!(partial.dfg_label_stats.dfg_recall_functions_capped, 1);
    assert_eq!(partial.dfg_label_stats.dfg_recall_candidates_capped, 2);
    assert_eq!(partial.dfg_label_stats.dfg_recall_candidates, 3);
}
