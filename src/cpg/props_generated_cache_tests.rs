//! Cold, full and incremental cache comparisons on pinned S1 pattern formals.

use super::{endpoint, parse, Access, CodePropertyGraph, CpgEdge, Endpoint, FlowConfidence};
use crate::cpg_cache::{
    compute_file_hashes, dfg_codec_tests::assert_stale_version_miss, load_cache, save_cache,
    CacheResult, SaveOutcome,
};
use crate::data_flow::{test_support::label_snapshot, DataFlowGraph};
use petgraph::visit::EdgeRef;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const ROWS: &str = include_str!("../../tests/fixtures/props_matrix/rows.jsonl");
const ANCHOR: &str = "function retained(x) {\n  const y = x;\n  return y;\n}\n";

#[derive(Deserialize)]
struct Row {
    id: String,
    langs: Vec<String>,
    files: BTreeMap<String, String>,
}

fn snapshot(
    graph: &CodePropertyGraph,
) -> (Vec<Endpoint>, Vec<(Endpoint, Endpoint, FlowConfidence)>) {
    let mut defs: Vec<_> = graph
        .graph
        .node_weights()
        .filter_map(endpoint)
        .filter(|at| at.access == Access::Def)
        .collect();
    let mut edges: Vec<_> = graph
        .graph
        .edge_references()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(label) = *edge.weight() else {
                return None;
            };
            Some((
                endpoint(graph.node(edge.source()))?,
                endpoint(graph.node(edge.target()))?,
                label,
            ))
        })
        .collect();
    defs.sort();
    edges.sort();
    (defs, edges)
}

#[test]
fn pattern_definitions_edges_and_labels_survive_cache_round_trips() {
    let rows: BTreeMap<_, _> = ROWS
        .lines()
        .map(|line| {
            let row: Row = serde_json::from_str(line).unwrap();
            (row.id.clone(), row)
        })
        .collect();
    let mut compared = 0;
    let mut ledger_digest = Sha256::new();
    for (id, required, absent) in [
        ("p05", &["a", "rest"][..], &[][..]),
        ("p07", &[], &["first"]),
        ("p13", &["a", "other.a"], &[]),
        ("p15", &["a"], &[]),
        ("p27", &["a"], &[]),
        ("p31", &["a"], &[]),
        ("p34", &["a"], &[]),
    ] {
        let row = &rows[id];
        for ext in &row.langs {
            let fixture = format!("m.{ext}");
            let mut sources = BTreeMap::from([
                (fixture.clone(), row.files["m"].clone()),
                ("retained.js".to_owned(), ANCHOR.to_owned()),
            ]);
            let files = parse(&sources);
            assert!(
                files
                    .values()
                    .all(|file| !file.tree.root_node().has_error()),
                "{id}/{ext}"
            );
            let hashes = compute_file_hashes(&sources);
            let dir = tempfile::tempdir().unwrap();
            assert!(matches!(
                load_cache(&hashes, false, dir.path()),
                CacheResult::Miss
            ));
            let cold = CodePropertyGraph::build(&files);
            let expected = snapshot(&cold);
            let exact_labels =
                |graph: &CodePropertyGraph| graph.dfg.exact_labels().collect::<BTreeMap<_, _>>();
            let revision = format!("cache-fixture/{id}/{ext}");
            let ledger = DataFlowGraph::collect_s1_evidence(&files, &revision);
            ledger.counters();
            ledger_digest.update(serde_json::to_vec(&ledger).unwrap());
            for key in required {
                assert!(
                    expected
                        .0
                        .iter()
                        .any(|def| def.owner == "C" && def.name == *key && def.line == 1),
                    "{id}/{ext}: no pattern witness {key}"
                );
            }
            for key in absent {
                assert!(
                    !expected
                        .0
                        .iter()
                        .any(|def| def.owner == "C" && def.name == *key),
                    "{id}/{ext}: unexpected definition {key}"
                );
            }
            assert_eq!(
                save_cache(&cold, &hashes, false, dir.path()).unwrap(),
                SaveOutcome::Saved
            );
            let CacheResult::Hit(full) = load_cache(&hashes, false, dir.path()) else {
                panic!("{id}/{ext}: expected a full hit");
            };
            assert_eq!(snapshot(&full), expected, "{id}/{ext} full");
            assert_eq!(
                exact_labels(&full),
                exact_labels(&cold),
                "{id}/{ext} full exact DFG labels"
            );
            assert_eq!(
                DataFlowGraph::collect_s1_evidence(&files, &revision),
                ledger,
                "{id}/{ext} full input evidence"
            );
            assert_eq!(
                label_snapshot(&full.dfg),
                label_snapshot(&cold.dfg),
                "{id}/{ext} full DFG labels"
            );
            assert_eq!(
                full.dfg_label_stats, cold.dfg_label_stats,
                "{id}/{ext} full statistics"
            );
            assert_stale_version_miss(&hashes, dir.path());

            sources
                .get_mut(&fixture)
                .unwrap()
                .push_str("\nfunction removed(old) {\n  const gone = old;\n  return gone;\n}\n");
            let previous = CodePropertyGraph::build(&parse(&sources));
            assert!(
                snapshot(&previous)
                    .0
                    .iter()
                    .any(|def| def.owner == "removed"),
                "{id}/{ext}: old cache lacks the deleted Def"
            );
            assert_eq!(
                save_cache(&previous, &compute_file_hashes(&sources), false, dir.path()).unwrap(),
                SaveOutcome::Saved,
            );
            let CacheResult::PartialHit {
                cached_call_graph,
                cached_dfg,
                changed_files,
            } = load_cache(&hashes, false, dir.path())
            else {
                panic!("{id}/{ext}: expected a partial hit");
            };
            assert_eq!(changed_files, BTreeSet::from([fixture]));
            assert!(label_snapshot(&cached_dfg)
                .into_keys()
                .any(|(from, _)| from.function == "retained"));
            let partial = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            assert_eq!(snapshot(&partial), expected, "{id}/{ext} partial");
            assert_eq!(
                exact_labels(&partial),
                exact_labels(&cold),
                "{id}/{ext} partial exact DFG labels"
            );
            assert_eq!(
                DataFlowGraph::collect_s1_evidence(&files, &revision),
                ledger,
                "{id}/{ext} partial input evidence"
            );
            assert_eq!(
                label_snapshot(&partial.dfg),
                label_snapshot(&cold.dfg),
                "{id}/{ext} partial DFG labels"
            );
            assert_eq!(
                partial.dfg_label_stats, cold.dfg_label_stats,
                "{id}/{ext} partial statistics"
            );
            compared += 1;
        }
    }
    assert_eq!(compared, 20);
    eprintln!(
        "cache parity and stale-version misses: {compared} row/grammar pairs, {} Rayon workers, ledger SHA-256 {:x}",
        crate::build_pool::build_pool().current_num_threads(), ledger_digest.finalize()
    );
}

#[test]
fn s1_cache_snapshot_detects_kill_line_only_changes() {
    let sources = BTreeMap::from([(
        "m.js".to_owned(),
        "function C({ a }) {\n  run(() => { a = 2; });\n  return use(a);\n}\n".to_owned(),
    )]);
    let mut graph = CodePropertyGraph::build(&parse(&sources));
    let before = snapshot(&graph);
    let edge = graph
        .graph
        .edge_indices()
        .find(|&edge| {
            matches!(
                graph.graph[edge],
                CpgEdge::DataFlow(super::FlowConfidence::NameOnly(
                    super::FlowDoubt::Killed { .. }
                ))
            )
        })
        .expect("p27 killed witness");
    let CpgEdge::DataFlow(super::FlowConfidence::NameOnly(super::FlowDoubt::Killed { kill_line })) =
        graph.graph[edge]
    else {
        unreachable!()
    };
    graph.graph[edge] =
        CpgEdge::DataFlow(super::FlowConfidence::NameOnly(super::FlowDoubt::Killed {
            kill_line: kill_line + 1,
        }));
    assert_ne!(snapshot(&graph), before);
}

#[test]
fn s1_signature_label_at_operand_does_not_relabel_the_body_on_the_same_line() {
    let source = "function C(\n  {x,\n   y=x}) { use(x);\n  return 0;\n}\n";
    let binding = source.find("{x").unwrap() + 1;
    let signature = source.find("=x").unwrap() + 1;
    let body = source.find("use(x").unwrap() + 4;
    let build = |text: String| {
        let sources = BTreeMap::from([("m.js".to_owned(), text)]);
        snapshot(&CodePropertyGraph::build(&parse(&sources))).1
    };
    let body_only = build(source.replace("=x", "=0"));
    let labels = |edges: &[(Endpoint, Endpoint, FlowConfidence)], at| {
        edges
            .iter()
            .filter(|(from, to, _)| {
                from.bytes == [binding, binding + 1] && to.bytes == [at, at + 1]
            })
            .map(|(_, _, label)| *label)
            .collect::<Vec<_>>()
    };
    let with_signature = build(source.to_owned());
    assert_eq!(labels(&body_only, body), [FlowConfidence::Exact]);
    assert_eq!(
        labels(&with_signature, signature),
        [FlowConfidence::NameOnly(super::FlowDoubt::CfgIncomplete)]
    );
    assert_eq!(labels(&with_signature, body), [FlowConfidence::Exact]);
}
