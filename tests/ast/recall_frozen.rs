//! Pre-S1 view of a DFG for receipts frozen before the field-write recall lane.
//!
//! Recall edges are `Def(p.f) -> Use(ancestor)` (a strict path prefix), which
//! no earlier producer emitted. Receipts that pin an existing population
//! compare everything else, so each pre-existing edge, label, counter and
//! availability row stays byte-identical. The recall lane itself is pinned in
//! `src/data_flow/build/field_recall*_tests.rs`.
use prism::cpg::CodePropertyGraph;
use prism::data_flow::VarLocation;
use serde_json::{json, Value};

pub(super) fn is_recall(from: &VarLocation, to: &VarLocation) -> bool {
    to.path != from.path && to.path.is_prefix_of(&from.path)
}

pub(super) fn edges(graph: &CodePropertyGraph) -> Vec<Value> {
    let mut edges: Vec<_> = graph
        .dfg
        .edges()
        .iter()
        .filter(|e| !is_recall(&e.from, &e.to))
        .map(|e| json!([e.from, e.to]))
        .collect();
    edges.sort_by_key(Value::to_string);
    edges
}

pub(super) fn labels(graph: &CodePropertyGraph) -> Vec<Value> {
    let mut labels: Vec<_> = graph
        .dfg
        .labels()
        .filter(|(e, _)| !is_recall(&e.from, &e.to))
        .map(|(e, l)| json!([e.from, e.to, l]))
        .collect();
    labels.sort_by_key(Value::to_string);
    labels
}

/// Label counters without the recall lane's labels and its own counters.
pub(super) fn stats(graph: &CodePropertyGraph) -> Value {
    let mut stats = serde_json::to_value(&graph.dfg_label_stats).unwrap();
    let object = stats.as_object_mut().unwrap();
    object.retain(|k, _| !k.starts_with("dfg_recall_"));
    let mut sub = |key: &str, n: u64| {
        let v = object.get_mut(key).unwrap();
        *v = json!(v.as_u64().unwrap() - n);
    };
    for (e, l) in graph.dfg.labels() {
        if !is_recall(&e.from, &e.to) {
            continue;
        }
        if l.is_exact() {
            sub("dfg_label_exact", 1);
            if e.to.line <= e.from.line {
                sub("dfg_label_loop_carried", 1);
            }
        } else {
            sub("dfg_label_nameonly_cfg_incomplete", 1);
        }
    }
    stats
}

/// Availability rows without the recall lane's per-owner status.
pub(super) fn availability(graph: &CodePropertyGraph) -> Value {
    let mut all = serde_json::to_value(graph.dfg.rd_function_stats()).unwrap();
    for file in all.as_object_mut().unwrap().values_mut() {
        file.as_object_mut().unwrap().remove("recall");
    }
    all
}
