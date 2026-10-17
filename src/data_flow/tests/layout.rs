//! Physical storage observations on production graphs. This is accounting, not
//! allocator live heap: BTree node/slack and private RD key heaps are excluded.
use super::super::*;
use super::identity::loc;
use serde_json::{json, Value};
use std::mem::size_of;

fn slots<T>(v: &Vec<T>) -> usize {
    v.capacity() * size_of::<T>()
}
fn path_heap(p: &AccessPath) -> usize {
    p.base.capacity() + slots(&p.fields) + p.fields.iter().map(String::capacity).sum::<usize>()
}
fn loc_heap(v: &VarLocation) -> usize {
    v.file.capacity() + v.function.capacity() + path_heap(&v.path)
}
fn key_heap(k: &BindingKey) -> usize {
    k.0.capacity() + k.1.capacity() + path_heap(&k.3)
}
fn map<K, V>(m: &BTreeMap<K, V>, kh: impl Fn(&K) -> usize, vh: impl Fn(&V) -> usize) -> usize {
    m.len() * (size_of::<K>() + size_of::<V>())
        + m.keys().map(kh).sum::<usize>()
        + m.values().map(vh).sum::<usize>()
}
fn census(g: &DataFlowGraph) -> Value {
    let g = &g.storage;
    json!({
        "arena_rows": g.arena.rows().len(), "arena_capacity": g.arena.rows().capacity(),
        "arena_bytes": slots(g.arena.rows()) + g.arena.rows().iter().map(loc_heap).sum::<usize>(),
        "rank_lengths": [g.arena.legacy_reps().len(), g.arena.legacy_by_occ().len()],
        "rank_capacities": [g.arena.legacy_reps().capacity(), g.arena.legacy_by_occ().capacity()],
        "rank_bytes": slots(g.arena.legacy_reps()) + slots(g.arena.legacy_by_occ()),
        "edges": g.edges.len(), "edge_capacity": g.edges.capacity(), "edge_id_bytes": slots(&g.edges),
        "defs": map(&g.defs, key_heap, slots), "uses": map(&g.uses, key_heap, slots),
        "forward": map(&g.forward, |_| 0, |v| slots(&v.neighbors)),
        "backward": map(&g.backward, |_| 0, |v| slots(&v.neighbors)),
        "labels": map(&g.labels, |_| 0, |_| 0), "exact_labels": map(&g.exact_labels, |_| 0, |_| 0),
        "rd": map(&g.rd_function_stats, String::capacity, |_| 0),
        "serialized_bytes": bincode::serialized_size(g).unwrap(),
        "inline_sizes": {"location": size_of::<VarLocation>(), "dfg": size_of::<DataFlowGraph>(), "occ_id": size_of::<super::super::compact::OccId>()}
    })
}

#[test]
fn repeated_empty_and_distinct_storage_invariants() {
    use super::super::compact::{CompactDfg, Parts};
    let mut rows = BTreeMap::new();
    let empty = || Parts {
        edges: vec![],
        defs: BTreeMap::new(),
        uses: BTreeMap::new(),
        labels: BTreeMap::new(),
        exact_labels: BTreeMap::new(),
        rd_function_stats: BTreeMap::new(),
        adjacency: None,
    };
    for mode in ["empty", "repeated", "all-distinct"] {
        let mut p = empty();
        if mode == "repeated" {
            p.edges = vec![
                FlowEdge {
                    from: loc("x", VarAccessKind::Def, 1),
                    to: loc("x", VarAccessKind::Use, 2)
                };
                1000
            ];
        } else if mode == "all-distinct" {
            p.defs.insert(
                ("u.py".into(), "f".into(), 1, AccessPath::simple("x")),
                (0..1000).map(|i| loc("x", VarAccessKind::Def, i)).collect(),
            );
        }
        let g = DataFlowGraph {
            storage: CompactDfg::from_parts(p).unwrap(),
        };
        let expected = match mode {
            "empty" => 0,
            "repeated" => 2,
            _ => 1000,
        };
        assert_eq!(g.storage.arena.rows().len(), expected);
        assert_eq!(size_of::<super::super::compact::OccId>(), 4);
        if mode == "repeated" {
            assert_eq!(g.edges().len(), 1000);
            assert_eq!(g.forward().next().unwrap().1.iter().count(), 1000);
            assert_eq!(g.backward().next().unwrap().1.iter().count(), 1000);
            assert!(
                census(&g)["arena_bytes"].as_u64().unwrap() < (1000 * size_of::<FlowEdge>()) as u64
            );
        }
        rows.insert(mode.to_string(), census(&g));
    }
    crate::build_pool::install(|| {
        for (path, source, language) in [
            (
                "a.py",
                "def f(x):\n y=x+x\n return y\n",
                crate::languages::Language::Python,
            ),
            (
                "a.js",
                "function f(x){let y=x;return y;}\n",
                crate::languages::Language::JavaScript,
            ),
        ] {
            let files = BTreeMap::from([(
                path.into(),
                ParsedFile::parse(path, source, language).unwrap(),
            )]);
            rows.insert(path.into(), census(&DataFlowGraph::build(&files)));
        }
    });
    if let Ok(dir) = std::env::var("DFG_CENSUS_DIR") {
        let dir = std::path::Path::new(&dir);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("physical-storage.json"),
            serde_json::to_vec_pretty(&rows).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("build-identity.txt"),
            crate::cpg_cache::current_cache_build_identity(),
        )
        .unwrap();
    }
}
