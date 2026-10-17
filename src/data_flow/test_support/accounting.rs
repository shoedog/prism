//! Fixture accounting follows c3162f94 census capacity/owned-heap conventions.
//! Excludes BTree node/slack, allocator rounding, top-level graph headers and,
//! like that census, private nested RD function-key heaps in both backends.
use super::*;
use crate::data_flow::compact::{Adj, CompactDfg, Label, LegacyId, OccId};
use crate::data_flow::{BindingKey, ExactFlowEdge, FlowEdge};
use serde_json::{json, Value};
use std::mem::size_of;

fn path_heap(p: &AccessPath) -> usize {
    p.base.capacity()
        + p.fields.capacity() * size_of::<String>()
        + p.fields.iter().map(String::capacity).sum::<usize>()
}
fn loc_heap(v: &VarLocation) -> usize {
    v.file.capacity() + v.function.capacity() + path_heap(&v.path)
}
fn exact_heap(v: &ExactVarLocation) -> usize {
    v.file.capacity() + v.function.capacity() + path_heap(&v.path)
}
fn key_heap(k: &BindingKey) -> usize {
    k.0.capacity() + k.1.capacity() + path_heap(&k.3)
}
fn slots<T>(v: &Vec<T>) -> usize {
    v.capacity() * size_of::<T>()
}
fn map<K, V>(m: &BTreeMap<K, V>, kh: impl Fn(&K) -> usize, vh: impl Fn(&V) -> usize) -> usize {
    m.len() * (size_of::<K>() + size_of::<V>())
        + m.keys().map(kh).sum::<usize>()
        + m.values().map(vh).sum::<usize>()
}
fn legacy_counts(g: &LegacyDataFlowGraph) -> Vec<usize> {
    vec![
        g.edges.len(),
        g.defs.len(),
        g.defs.values().map(Vec::len).sum(),
        g.uses.len(),
        g.uses.values().map(Vec::len).sum(),
        g.forward.len(),
        g.forward.values().map(Vec::len).sum(),
        g.backward.len(),
        g.backward.values().map(Vec::len).sum(),
        g.labels.len(),
        g.exact_labels.len(),
    ]
}
fn compact_counts(g: &CompactDfg) -> Vec<usize> {
    vec![
        g.edges().len(),
        g.defs().count(),
        g.defs().map(|(_, v)| v.iter().count()).sum(),
        g.uses().count(),
        g.uses().map(|(_, v)| v.iter().count()).sum(),
        g.forward().count(),
        g.forward().map(|(_, v)| v.iter().count()).sum(),
        g.backward().count(),
        g.backward().map(|(_, v)| v.iter().count()).sum(),
        g.labels().count(),
        g.exact_labels().count(),
    ]
}
fn report(old: &LegacyDataFlowGraph, new: &CompactDfg) -> Value {
    assert_eq!(old_storage(old), compact::storage(new));
    assert_eq!(legacy_counts(old), compact_counts(new));
    let old_fields = json!({
        "edges": slots(&old.edges)+old.edges.iter().map(|e|loc_heap(&e.from)+loc_heap(&e.to)).sum::<usize>(),
        "defs": map(&old.defs,key_heap,|v|slots(v)+v.iter().map(loc_heap).sum::<usize>()),
        "uses": map(&old.uses,key_heap,|v|slots(v)+v.iter().map(loc_heap).sum::<usize>()),
        "forward": map(&old.forward,loc_heap,|v|slots(v)+v.iter().map(loc_heap).sum::<usize>()),
        "backward": map(&old.backward,loc_heap,|v|slots(v)+v.iter().map(loc_heap).sum::<usize>()),
        "labels": map(&old.labels,|k|loc_heap(&k.0)+loc_heap(&k.1),|_|0),
        "exact_labels": map(&old.exact_labels,|e|exact_heap(&e.from)+exact_heap(&e.to),|_|0),
        "rd": map(&old.rd_function_stats,String::capacity,|_|0)
    });
    let rank_bytes = slots(&new.arena.legacy_reps()) + slots(&new.arena.legacy_by_occ());
    let new_fields = json!({
        "arena": slots(&new.arena.rows())+new.arena.rows().iter().map(loc_heap).sum::<usize>(),
        "ranks":rank_bytes,
        "edges":slots(&new.edges),
        "defs": map(&new.defs,key_heap,slots),"uses":map(&new.uses,key_heap,slots),
        "forward":map(&new.forward,|_|0,|v|slots(&v.neighbors)),
        "backward":map(&new.backward,|_|0,|v|slots(&v.neighbors)),
        "labels":map(&new.labels,|_|0,|_|0),"exact_labels":map(&new.exact_labels,|_|0,|_|0),
        "rd":map(&new.rd_function_stats,String::capacity,|_|0)
    });
    let mut core = BTreeSet::new();
    for e in &old.edges {
        core.insert(ExactVarLocation::from_legacy(&e.from));
        core.insert(ExactVarLocation::from_legacy(&e.to));
    }
    for (k, vs) in old.forward.iter().chain(&old.backward) {
        core.insert(ExactVarLocation::from_legacy(k));
        for v in vs {
            core.insert(ExactVarLocation::from_legacy(v));
        }
    }
    let total = |v: &Value| {
        v.as_object()
            .unwrap()
            .values()
            .map(|n| n.as_u64().unwrap())
            .sum::<u64>()
    };
    json!({"legacy_accounted_bytes":total(&old_fields),"compact_accounted_bytes":total(&new_fields),
        "legacy_fields":old_fields,"compact_fields":new_fields,
        "arena_rows":new.arena.rows().len(),"core_exact_u":core.len(),
        "arena_rows_per_core_u":if core.is_empty(){None}else{Some(new.arena.rows().len() as f64/core.len() as f64)},
        "rank_bytes":rank_bytes,"rank_lengths":[new.arena.legacy_reps().len(),new.arena.legacy_by_occ().len()],
        "rank_capacities":[new.arena.legacy_reps().capacity(),new.arena.legacy_by_occ().capacity()],
        "semantic_counts":legacy_counts(old),
        "inline_sizes":{"location":size_of::<VarLocation>(),"edge":size_of::<FlowEdge>(),"occ_id":size_of::<OccId>(),
            "legacy_id":size_of::<LegacyId>(),"adj":size_of::<Adj>(),"label":size_of::<Label>(),"exact_edge":size_of::<ExactFlowEdge>()}})
}
/// Fixture/generator checkpoint, not allocator measurement or corpus acceptance.
#[cfg(feature = "dfg-oracle")]
pub fn fixture(files: &BTreeMap<String, ParsedFile>) -> Value {
    report(&LegacyDataFlowGraph::build(files), &compact::build(files))
}
#[cfg(feature = "dfg-oracle")]
pub fn synthetic() -> Vec<(String, Value)> {
    let mut values = Vec::new();
    for mode in ["empty", "repeated", "all-distinct"] {
        let mut old = LegacyDataFlowGraph::empty();
        let loc = |start| VarLocation {
            file: "synthetic.py".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath::simple("x"),
            start_byte: start,
            end_byte: start + 1,
            kind: crate::data_flow::VarAccessKind::Def,
        };
        if mode == "repeated" {
            for _ in 0..1000 {
                old.edges.push(FlowEdge {
                    from: loc(1),
                    to: loc(2),
                });
            }
            for e in &old.edges {
                old.forward
                    .entry(e.from.clone())
                    .or_default()
                    .push(e.to.clone());
                old.backward
                    .entry(e.to.clone())
                    .or_default()
                    .push(e.from.clone());
            }
        } else if mode == "all-distinct" {
            old.defs.insert(
                (
                    "synthetic.py".into(),
                    "f".into(),
                    1,
                    AccessPath::simple("x"),
                ),
                (0..1000).map(loc).collect(),
            );
        }
        let new = CompactDfg::from_parts(compact::parts(old.clone())).unwrap();
        let r = report(&old, &new);
        if mode == "repeated" {
            assert!(r["compact_accounted_bytes"].as_u64() < r["legacy_accounted_bytes"].as_u64());
        }
        values.push((mode.into(), r));
    }
    values
}
