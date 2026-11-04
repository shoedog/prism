//! Round-2 structural successors, missing-entry kills and cyclicity.
use crate::common::*;
use prism::cpg::{FlowConfidence, FlowDoubt};

const SOURCE: &str = include_str!("../fixtures/go_jumps_round2/round2.go");

fn check_fold_cycle(source: &str, carried: usize) {
    let parsed = ParsedFile::parse("cycle.go", source, Language::Go).unwrap();
    let graph = CodePropertyGraph::build(&BTreeMap::from([("cycle.go".to_owned(), parsed)]));
    let line = |marker| source.lines().position(|l| l.contains(marker)).unwrap() + 1;
    let labels: Vec<_> = graph
        .dfg
        .labels()
        .filter(|(e, _)| e.from.line == line("// def") && e.to.line == line("// use"))
        .map(|(_, l)| l)
        .collect();
    assert_eq!(labels, vec![FlowConfidence::Exact]);
    if std::env::var_os("PRISM_EXPORT_GO_JUMPS").is_some() {
        let mut edges: Vec<_> = graph
            .dfg
            .edges()
            .iter()
            .map(|e| serde_json::json!([e.from, e.to]))
            .collect();
        edges.sort_by_key(serde_json::Value::to_string);
        println!(
            "GO_FOLD_SNAPSHOT {}",
            serde_json::json!({"source":source,"edges":edges,"stats":graph.dfg_label_stats,"availability":graph.dfg.rd_function_stats()})
        );
    }
    assert_eq!(graph.dfg_label_stats.dfg_label_loop_carried, carried);
}

#[test]
fn go_fold_public_acyclic_defer() {
    check_fold_cycle(
        include_str!("../fixtures/go_loop_cycles/acyclic_defer.go"),
        0,
    );
}

#[test]
fn go_fold_public_cyclic_goto() {
    check_fold_cycle(include_str!("../fixtures/go_loop_cycles/cyclic_goto.go"), 1);
}

#[test]
fn go_fold_public_cyclic_defer() {
    check_fold_cycle(
        include_str!("../fixtures/go_loop_cycles/cyclic_defer.go"),
        1,
    );
}

fn line(marker: &str) -> usize {
    SOURCE.lines().position(|l| l.contains(marker)).unwrap() + 1
}

fn cpg() -> CodePropertyGraph {
    let parsed = ParsedFile::parse("round2.go", SOURCE, Language::Go).unwrap();
    CodePropertyGraph::build(&BTreeMap::from([("round2.go".to_owned(), parsed)]))
}

#[test]
fn go_round2_structural_targets() {
    let parsed = ParsedFile::parse("round2.go", SOURCE, Language::Go).unwrap();
    let edges = prism::cfg::build_cfg_edges(&parsed);
    let mut failures = Vec::new();
    for (i, text) in SOURCE.lines().enumerate() {
        let outgoing: Vec<_> = edges
            .iter()
            .filter(|e| e.from_line == i + 1)
            .map(|e| e.to_line)
            .collect();
        if let Some((_, annotation)) = text.split_once("// jump:") {
            let (name, target) = annotation.split_once(':').unwrap();
            let expected = vec![line(&format!("// target:{target}"))];
            if outgoing != expected {
                failures.push(format!("{name}: {outgoing:?} != {expected:?}"));
            }
        } else if text.contains("// noedge:") && !outgoing.is_empty() {
            failures.push(format!("exit {}: {outgoing:?}", i + 1));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn check_labels(cases: &[(&str, &str, FlowConfidence)]) {
    let graph = cpg();
    let mut failures = Vec::new();
    for &(function, marker, expected) in cases {
        let from = line(&format!("// def:{marker}"));
        // Some use markers share the target annotation.
        let to = line(&format!("use:{marker}"));
        let actual = graph.dfg.labels().find_map(|(e, l)| {
            (e.from.function == function && e.from.line == from && e.to.line == to).then_some(l)
        });
        if actual != Some(expected) {
            failures.push(format!(
                "{function} {from}->{to}: {actual:?} != {expected:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn go_round2_break_labels() {
    check_labels(&[
        ("switchTail", "st", FlowConfidence::Exact),
        ("thenTail", "then", FlowConfidence::Exact),
        ("nestedTail", "nested", FlowConfidence::Exact),
        ("printer", "printer", FlowConfidence::Exact),
        (
            "finalPlain",
            "final",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        ),
        (
            "finalDefer",
            "defer",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        ),
        (
            "elseRead",
            "else-read",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        ),
    ]);
}

#[test]
fn go_round2_missing_entries_are_now_reachable() {
    check_labels(&[
        ("missingCase", "case", FlowConfidence::Exact),
        ("elseIf", "elseif", FlowConfidence::Exact),
    ]);
}

#[test]
fn go_round2_backward_goto_requires_cycle() {
    for (name, expected) in [("acyclic", 0), ("cyclic", 1)] {
        let parsed = ParsedFile::parse("round2.go", SOURCE, Language::Go).unwrap();
        // Build one owner so the public aggregate is also a per-edge witness.
        let owner = parsed
            .all_functions()
            .into_iter()
            .find(|f| {
                f.child_by_field_name("name")
                    .is_some_and(|n| parsed.node_text(&n) == name)
            })
            .unwrap();
        let source = format!("package p\n{}", parsed.node_text(&owner));
        let parsed = ParsedFile::parse("cycle.go", &source, Language::Go).unwrap();
        let graph = CodePropertyGraph::build(&BTreeMap::from([("cycle.go".to_owned(), parsed)]));
        assert!(
            graph
                .dfg
                .labels()
                .any(|(e, l)| e.from.line > e.to.line && l.is_exact()),
            "{name}: backward Exact retained"
        );
        assert_eq!(
            graph.dfg_label_stats.dfg_label_loop_carried, expected,
            "{name}"
        );
    }
}

#[test]
fn go_round2_edge_receipt() {
    let graph = cpg();
    use super::recall_frozen as frozen;
    let actual = serde_json::json!({"edges":frozen::edges(&graph),"labels":frozen::labels(&graph),"stats":frozen::stats(&graph),"availability":frozen::availability(&graph)});
    if std::env::var_os("PRISM_EXPORT_GO_JUMPS").is_some() {
        println!("GO_ROUND2_SNAPSHOT {actual}");
        return;
    }
    let base: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/go_jumps_round2/head4797.json")).unwrap();
    assert_eq!(actual["edges"], base["edges"], "round-1 endpoint multiset");
    let changes: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../fixtures/go_jumps_round2/label_changes.json"
    ))
    .unwrap();
    assert_eq!(changes.len(), 13);
    let mut wanted = base;
    let mut failures = Vec::new();
    for row in changes {
        let matches = |label: &serde_json::Value| {
            label[0]["function"] == row["function"]
                && label[0]["path"]["base"] == row["path"]
                && label[0]["line"] == row["d"]
                && label[1]["line"] == row["u"]
        };
        let entries: Vec<_> = wanted["labels"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter(|l| matches(l))
            .collect();
        assert_eq!(entries.len(), 1, "unique obligation: {row}");
        let entry = entries.into_iter().next().unwrap();
        assert_eq!(entry[2], row["old"]);
        assert_ne!(row["old"], row["new"]);
        entry[2] = row["new"].clone();
        let label = actual["labels"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| matches(l))
            .unwrap();
        if label[2] != row["new"] {
            failures.push(format!("{row}: actual {}", label[2]));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    wanted["stats"]["dfg_label_exact"] = serde_json::json!(56);
    wanted["stats"]["dfg_label_loop_carried"] = serde_json::json!(11);
    wanted["stats"]["dfg_label_nameonly_cfg_incomplete"] = serde_json::json!(19);
    wanted["stats"]["dfg_label_nameonly_killed"] = serde_json::json!(2);
    assert!(
        actual == wanted,
        "unexpected label, counter or availability change in round2.go"
    );
}
