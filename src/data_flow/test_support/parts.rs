//! Small input constructors and read-only observations, with no second engine.
use super::*;
use crate::data_flow::compact::{CompactDfg, Parts};

pub(in crate::data_flow) fn empty() -> Parts {
    Parts {
        edges: vec![],
        defs: BTreeMap::new(),
        uses: BTreeMap::new(),
        labels: BTreeMap::new(),
        exact_labels: BTreeMap::new(),
        rd_function_stats: BTreeMap::new(),
        adjacency: None,
    }
}

pub(in crate::data_flow) fn graph(parts: Parts) -> DataFlowGraph {
    DataFlowGraph {
        storage: CompactDfg::from_parts(parts).unwrap(),
    }
}

pub(in crate::data_flow) fn snapshot(g: &DataFlowGraph) -> Parts {
    Parts {
        edges: edge_snapshot(g),
        defs: g.defs().map(|(k, v)| (k.clone(), v.to_vec())).collect(),
        uses: g.uses().map(|(k, v)| (k.clone(), v.to_vec())).collect(),
        labels: label_snapshot(g),
        exact_labels: exact_label_snapshot(g),
        rd_function_stats: g.rd_function_stats().clone(),
        adjacency: Some((
            g.forward().map(|(k, v)| (k.clone(), v.to_vec())).collect(),
            g.backward().map(|(k, v)| (k.clone(), v.to_vec())).collect(),
        )),
    }
}

/// Compare directly with the authored primary inputs, including byte-bearing keys.
/// Adjacency is checked only when the fixture explicitly supplies expected rows.
pub(in crate::data_flow) fn assert_inputs(p: &Parts, g: &DataFlowGraph) {
    let actual = snapshot(g);
    assert_bytes("edges", &p.edges, &actual.edges);
    assert_bytes("defs", &p.defs, &actual.defs);
    assert_bytes("uses", &p.uses, &actual.uses);
    assert_bytes("labels", &p.labels, &actual.labels);
    assert_bytes("exact labels", &p.exact_labels, &actual.exact_labels);
    assert_bytes(
        "RD identities",
        &p.rd_function_stats,
        &actual.rd_function_stats,
    );
    if let Some(expected) = &p.adjacency {
        assert_bytes("adjacency", expected, actual.adjacency.as_ref().unwrap());
    }
}

pub(in crate::data_flow) fn roundtrip(g: &DataFlowGraph) {
    let encoded = bytes(g);
    let decoded: DataFlowGraph = bincode::deserialize(&encoded).unwrap();
    assert_eq!(encoded, bytes(&decoded));
    assert_eq!(current_storage(g), current_storage(&decoded));
    let p = snapshot(g);
    let mut locations: Vec<_> = p.edges.iter().flat_map(|e| [&e.from, &e.to]).collect();
    locations.extend(p.defs.values().chain(p.uses.values()).flatten());
    if let Some((f, b)) = &p.adjacency {
        for (key, values) in f.iter().chain(b) {
            locations.push(key);
            locations.extend(values);
        }
    }
    for at in locations {
        assert_bytes(
            "forward roundtrip",
            &g.forward_reachable(at),
            &decoded.forward_reachable(at),
        );
        assert_eq!(
            labeled_obs(&g.backward_reachable_labeled(at)),
            labeled_obs(&decoded.backward_reachable_labeled(at))
        );
        assert_bytes(
            "definition roundtrip",
            &g.all_defs_of(&at.file, at.var_name()),
            &decoded.all_defs_of(&at.file, at.var_name()),
        );
        assert_bytes(
            "line roundtrip",
            &g.all_locations_at(&at.file, at.line),
            &decoded.all_locations_at(&at.file, at.line),
        );
        let source = [(at.file.clone(), at.line)];
        assert_eq!(
            path_bytes(&g.taint_forward(&source)),
            path_bytes(&decoded.taint_forward(&source))
        );
    }
}

pub(in crate::data_flow) fn compact_roundtrip(g: &CompactDfg) {
    roundtrip(&DataFlowGraph { storage: g.clone() });
}
