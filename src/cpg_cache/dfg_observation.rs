//! Read-only capture adapter. May be overlaid on clean main without the oracle.
use crate::cpg::CodePropertyGraph;
use crate::data_flow::DataFlowGraph;
use petgraph::visit::EdgeRef;

/// Current codec layout, retained only as a byte compatibility control.
pub fn payload(cpg: &CodePropertyGraph) -> Vec<u8> {
    let nodes: Vec<_> = cpg.graph.node_indices().map(|n| &cpg.graph[n]).collect();
    let edges: Vec<_> = cpg
        .graph
        .edge_references()
        .map(|e| {
            (
                e.source().index() as u32,
                e.target().index() as u32,
                e.weight(),
            )
        })
        .collect();
    bincode::serialize(&(
        nodes,
        edges,
        &cpg.call_graph,
        &cpg.dfg,
        &cpg.return_flow_stats,
        &cpg.dfg_label_stats,
    ))
    .unwrap()
}

/// Legacy field sequence, independent of DataFlowGraph's Serialize implementation.
pub(crate) fn dfg_storage(g: &DataFlowGraph) -> Vec<u8> {
    bincode::serialize(&(
        g.edges().iter().map(|e| e.to_owned()).collect::<Vec<_>>(),
        g.defs().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
        g.uses().map(|(k, v)| (k, v.to_vec())).collect::<Vec<_>>(),
        g.forward()
            .map(|(k, v)| (k, v.to_vec()))
            .collect::<Vec<_>>(),
        g.backward()
            .map(|(k, v)| (k, v.to_vec()))
            .collect::<Vec<_>>(),
        g.labels()
            .map(|(e, c)| ((e.from, e.to), c))
            .collect::<Vec<_>>(),
        g.exact_labels().collect::<Vec<_>>(),
        g.rd_function_stats(),
    ))
    .unwrap()
}
/// Bincode tuples and structs emit fields without framing. Splicing the field
/// sequence preserves SerializedCpg bytes after the production DFG layout changes.
pub fn fieldwise_payload(cpg: &CodePropertyGraph) -> Vec<u8> {
    let nodes: Vec<_> = cpg.graph.node_indices().map(|n| &cpg.graph[n]).collect();
    let edges: Vec<_> = cpg
        .graph
        .edge_references()
        .map(|e| {
            (
                e.source().index() as u32,
                e.target().index() as u32,
                e.weight(),
            )
        })
        .collect();
    let mut bytes = bincode::serialize(&(nodes, edges, &cpg.call_graph)).unwrap();
    bytes.extend(dfg_storage(&cpg.dfg));
    bytes.extend(bincode::serialize(&(&cpg.return_flow_stats, &cpg.dfg_label_stats)).unwrap());
    bytes
}
/// Ordered keys and node IDs, including empty rows, representatives and vector order.
pub fn lookup_indexes(cpg: &CodePropertyGraph) -> Vec<u8> {
    bincode::serialize(&(
        cpg.func_index
            .iter()
            .map(|(k, n)| (k, n.index() as u32))
            .collect::<Vec<_>>(),
        cpg.name_index
            .iter()
            .map(|(k, ns)| (k, ns.iter().map(|n| n.index() as u32).collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
        cpg.var_index
            .iter()
            .map(|(k, n)| (k, n.index() as u32))
            .collect::<Vec<_>>(),
        cpg.location_index
            .iter()
            .map(|(k, ns)| (k, ns.iter().map(|n| n.index() as u32).collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
    ))
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::ParsedFile, languages::Language};
    use std::collections::BTreeMap;

    #[test]
    fn observation_covers_all_four_lookup_indexes() {
        crate::build_pool::install(|| {
            let files = BTreeMap::from([(
                "a.py".into(),
                ParsedFile::parse(
                    "a.py",
                    "def f(x):\n    y = x\n    return y\n",
                    Language::Python,
                )
                .unwrap(),
            )]);
            for index in 0..4 {
                let mut graph = CodePropertyGraph::build(&files);
                let expected_payload = fieldwise_payload(&graph);
                let expected_indexes = lookup_indexes(&graph);
                match index {
                    0 => {
                        assert!(!graph.func_index.is_empty());
                        graph.func_index.clear();
                    }
                    1 => {
                        assert!(!graph.name_index.is_empty());
                        graph.name_index.clear();
                    }
                    2 => {
                        assert!(!graph.var_index.is_empty());
                        graph.var_index.clear();
                    }
                    _ => {
                        assert!(!graph.location_index.is_empty());
                        graph.location_index.clear();
                    }
                }
                assert_eq!(
                    expected_payload,
                    fieldwise_payload(&graph),
                    "payload alone is blind to index {index}"
                );
                assert_ne!(
                    expected_indexes,
                    lookup_indexes(&graph),
                    "lookup observation missed index {index}"
                );
            }
            let empty = CodePropertyGraph::empty();
            assert_eq!(lookup_indexes(&empty), vec![0; 32]);
        });
    }
}
