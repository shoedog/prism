//! Deterministic boolean, classification, source-output and operation witnesses.
use super::*;
use std::cell::Cell;

thread_local! {
    static LEGACY: Cell<bool> = const { Cell::new(false) };
    static WALKS: Cell<usize> = const { Cell::new(0) };
    static LEGACY_CALLS: Cell<usize> = const { Cell::new(0) };
}
pub(super) fn legacy_enabled() -> bool {
    let enabled = LEGACY.with(Cell::get);
    if enabled {
        LEGACY_CALLS.with(|n| n.set(n.get() + 1));
    }
    enabled
}
pub(super) fn walk() {
    WALKS.with(|n| n.set(n.get() + 1));
}
fn walks(f: impl FnOnce()) -> usize {
    WALKS.with(|n| n.set(0));
    f();
    WALKS.with(Cell::get)
}
struct LegacyMode;
impl LegacyMode {
    fn begin() -> Self {
        LEGACY.with(|m| assert!(!m.replace(true)));
        LEGACY_CALLS.with(|n| n.set(0));
        Self
    }
}
impl Drop for LegacyMode {
    fn drop(&mut self) {
        LEGACY.with(|m| m.set(false));
    }
}

#[test]
fn endpoints_filters_and_kill_domains() {
    // 0 -> 1 -> 2, with only an incomplete route to 3; 4 is isolated.
    let graph = vec![
        vec![(0, false), (1, false)],
        vec![(2, false)],
        vec![(3, true)],
        vec![],
        vec![],
    ];
    let mut paths = Reachability::new(&graph);
    for n in 0..graph.len() {
        assert!(paths.path_exists(n, n));
        assert!(paths.reaches_target(n, n));
    }
    assert!(paths.path_exists(0, 2));
    assert!(paths.reaches_target(0, 2));
    for (from, to) in [(0, 3), (0, 4), (2, 0), (4, 0)] {
        assert!(!paths.path_exists(from, to));
        assert!(!paths.reaches_target(from, to));
    }
    assert!(graph::path_exists(&graph, 0, 3, true));
    assert!(!graph::path_exists(&graph, 0, 3, false));
    let mapped = [Some(0), Some(1), None];
    let flat = vec![BitSet::new(3); graph.len()];
    let mut kill = flat.clone();
    kill[0].insert(0); // A killed source must still expand.
    kill[1].insert(0); // A killed target is accepted, but not traversed.
    let mut bound = DefinitionReachability::new(&graph, &kill, &mapped);
    let mut flat = DefinitionReachability::new(&graph, &flat, &mapped);
    assert!(bound.reaches(0, 0));
    assert!(bound.reaches(0, 1));
    assert!(!bound.reaches(0, 2));
    assert!(flat.reaches(0, 2));
    assert!(!flat.reaches(0, 3));
    assert!(bound.reaches(1, 2)); // Same node, different definition.
    assert!(!bound.reaches(2, 0)); // Unmapped definition.
    let lines = BTreeMap::from([(90, 0), (10, 1), (3, 4), (100, 2)]);
    assert_eq!(
        lowest_reachable_kill(0, Some(0), 2, &kill, &mut paths, &lines),
        Some(10)
    );
    assert_eq!(
        lowest_reachable_kill(0, Some(0), 0, &kill, &mut paths, &lines),
        Some(90)
    );
    assert_eq!(
        lowest_reachable_kill(0, None, 2, &kill, &mut paths, &lines),
        None
    );
    assert_eq!(
        lowest_reachable_kill(0, Some(0), 4, &kill, &mut paths, &lines),
        None
    );
}

fn next(seed: &mut u64) -> usize {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (*seed >> 32) as usize
}

#[test]
fn random_graph_and_classification_equivalence() {
    let mut seed = 0x437_c84f_3769;
    // Cross a machine-word boundary, plus singleton and empty graphs.
    assert_eq!(Reachability::new(&[]).row_counts(), (0, 0));
    for n in [1, 2, 7, 65] {
        for _ in 0..12 {
            let mut graph = vec![Vec::new(); n];
            for edges in &mut graph {
                for to in 0..n {
                    if next(&mut seed).is_multiple_of(5) {
                        edges.push((to, next(&mut seed).is_multiple_of(3)));
                    }
                }
            }
            let mapped = [Some(0), Some(n - 1), Some(next(&mut seed) % n), None];
            let mut kill = vec![BitSet::new(mapped.len()); n];
            let mut in_sets = kill.clone();
            for node in 0..n {
                for def in 0..mapped.len() {
                    if next(&mut seed).is_multiple_of(3) {
                        kill[node].insert(def);
                    }
                    if next(&mut seed).is_multiple_of(2) {
                        in_sets[node].insert(def);
                    }
                }
            }
            let lines = (0..n).map(|node| (n * 3 - node * 2, node)).collect();
            let mut paths = Reachability::new(&graph);
            let mut defs = DefinitionReachability::new(&graph, &kill, &mapped);
            for from in 0..n {
                for to in 0..n {
                    let old = graph::path_exists(&graph, from, to, false);
                    assert_eq!(paths.path_exists(from, to), old);
                    assert_eq!(paths.reaches_target(from, to), old);
                }
            }
            for to in 0..n {
                for (def, &from) in mapped.iter().enumerate() {
                    let old = from.is_some_and(|from| {
                        graph::definition_reaches_unflagged(def, from, to, &kill, &graph)
                    });
                    assert_eq!(defs.reaches(def, to), old, "n={n}, def={def}, to={to}");
                    assert_eq!(
                        lowest_reachable_kill(def, from, to, &kill, &mut paths, &lines),
                        legacy_classify::lowest_reachable_kill(
                            def, from, to, &kill, &graph, &lines
                        )
                    );
                }
                for candidates in [&[][..], &[0][..], &[1, 2][..], &[0, 1, 2, 3][..]] {
                    assert_eq!(
                        classify_by_reaching_sets(
                            candidates, to, &mapped, &in_sets, &kill, &mut paths, &mut defs, &lines
                        ),
                        legacy_classify::classify_by_reaching_sets(
                            candidates, to, &mapped, &in_sets, &kill, &graph, &lines
                        )
                    );
                }
            }
        }
    }
}

#[test]
fn repeated_queries_build_only_one_row_per_key() {
    let n = 80;
    let graph: Vec<_> = (0..n)
        .map(|i| {
            if i + 1 < n {
                vec![(i + 1, false)]
            } else {
                vec![]
            }
        })
        .collect();
    let mapped = [Some(0), None];
    let kill = vec![BitSet::new(2); n];
    let mut paths = Reachability::new(&graph);
    let mut defs = DefinitionReachability::new(&graph, &kill, &mapped);
    assert_eq!(paths.row_counts(), (0, 0));
    assert_eq!(defs.row_count(), 0);
    let memo_walks = walks(|| {
        for _ in 0..3 {
            for node in 0..n {
                assert!(paths.path_exists(0, node));
                assert!(paths.reaches_target(node, n - 1));
                assert!(defs.reaches(0, node));
                assert!(!defs.reaches(1, node));
            }
        }
    });
    assert_eq!(memo_walks, 3, "one row per direction and definition");
    assert_eq!(paths.row_counts(), (1, 1));
    assert_eq!(defs.row_count(), 1);
    // Same-environment old-logic control: 720 BFS queries for identical booleans.
    let old_walks = walks(|| {
        for _ in 0..3 {
            for node in 0..n {
                assert!(graph::path_exists(&graph, 0, node, false));
                assert!(graph::path_exists(&graph, node, n - 1, false));
                assert!(graph::definition_reaches_unflagged(
                    0, 0, node, &kill, &graph
                ));
            }
        }
    });
    assert_eq!(old_walks, 720);
    // A second function with different adjacency must never reuse prior rows.
    let disconnected = vec![vec![]; n];
    let mut fresh = Reachability::new(&disconnected);
    assert!(!fresh.path_exists(0, n - 1));
    assert!(!fresh.reaches_target(0, n - 1));
}

#[test]
fn synthetic_source_every_edge_matches_reference() {
    let fixtures = [
        ("memo.go", "package p\nfunc f(flag bool) int {\nx := 0\nx = 1\nif flag {\nx = 2\n}\nfor x < 4 {\nx = x + 1\n}\nreturn x\n}\n"),
        ("memo.py", "def f(flag):\n    x = 0\n    x = 1\n    if flag:\n        x = 2\n    try:\n        x = x + 1\n    except Exception:\n        x = 3\n    return x\n"),
        ("memo.js", "function f(flag) {\nlet x = 0;\nx = 1;\nif (flag) {\nx = 2;\n}\nwhile (x < 4) {\nx = x + 1;\n}\n{ let x = 9; consume(x); }\nreturn x;\n}\n"),
        ("memo.rs", "fn f(flag: bool) -> i32 {\nlet mut x = 0;\nx = 1;\nif flag {\nx = 2;\n}\nwhile x < 4 {\nx = x + 1;\n}\nx\n}\n"),
    ];
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .unwrap()
        .install(|| {
            for (path, source) in fixtures {
                let parsed =
                    ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap();
                let files = BTreeMap::from([(path.to_owned(), parsed)]);
                let mode = LegacyMode::begin();
                let old = crate::data_flow::DataFlowGraph::build_on_current_pool(&files);
                assert!(
                    LEGACY_CALLS.with(Cell::get) > 0,
                    "reference exercised: {path}"
                );
                drop(mode);
                let new = crate::data_flow::DataFlowGraph::build_on_current_pool(&files);
                assert_eq!(
                    crate::cpg::dfg_observation::dfg_storage(&old),
                    crate::cpg::dfg_observation::dfg_storage(&new),
                    "producer: {path}"
                );
                let cg = crate::call_graph::CallGraph::build(&files);
                let old =
                    crate::cpg::CodePropertyGraph::assemble_graph(cg.clone(), old, &files, None);
                let new = crate::cpg::CodePropertyGraph::assemble_graph(cg, new, &files, None);
                assert_eq!(
                    crate::cpg::dfg_observation::fieldwise_payload(&old),
                    crate::cpg::dfg_observation::fieldwise_payload(&new),
                    "all labels, doubts, kill lines: {path}"
                );
            }
        });
}
