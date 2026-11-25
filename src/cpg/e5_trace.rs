use super::{CpgEdge, CpgNode};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction::Incoming;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::io::{self, Write};

pub(super) fn enabled() -> bool {
    std::env::var_os("PRISM_E5_TRACE").is_some_and(|value| value == "1")
}

/// Walk the same reverse DataFlow reachability as the guard, before it removes
/// edges. Keep one route to any seed for each discovered node.
pub(super) fn for_each_witness(
    graph: &DiGraph<CpgNode, CpgEdge>,
    seeds: Vec<NodeIndex>,
    refused: &BTreeSet<NodeIndex>,
    mut report: impl FnMut(NodeIndex, NodeIndex, &[NodeIndex]),
) {
    let mut toward_seed: BTreeMap<_, _> = seeds.iter().map(|&seed| (seed, None)).collect();
    let mut stack = seeds;
    while let Some(node) = stack.pop() {
        for predecessor in graph
            .edges_directed(node, Incoming)
            .filter(|edge| edge.weight().is_data_flow())
            .map(|edge| edge.source())
        {
            if let Entry::Vacant(entry) = toward_seed.entry(predecessor) {
                entry.insert(Some(node));
                stack.push(predecessor);
            }
        }
    }

    for &source in refused {
        let mut path = vec![source];
        let mut seed = source;
        while let Some(next) = toward_seed[&seed] {
            path.push(next);
            seed = next;
        }
        path.reverse();
        report(source, seed, &path);
    }
}

pub(super) fn report(
    graph: &DiGraph<CpgNode, CpgEdge>,
    source: NodeIndex,
    seed: NodeIndex,
    path: &[NodeIndex],
) {
    if !enabled() {
        return;
    }
    let path = path
        .iter()
        .map(|&index| site(graph, index))
        .collect::<Vec<_>>()
        .join(" <- ");
    // A stderr write failure must leave the guard free to remove the edges.
    let _ = writeln!(
        io::stderr().lock(),
        "PRISM_E5_TRACE source={} seed={} reverse_dataflow=[{}]",
        site(graph, source),
        site(graph, seed),
        path,
    );
}

fn site(graph: &DiGraph<CpgNode, CpgEdge>, index: NodeIndex) -> String {
    let node = &graph[index];
    let (start, end) = match node {
        CpgNode::Function {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::Statement {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::Variable {
            start_byte,
            end_byte,
            ..
        }
        | CpgNode::ReturnValue {
            start_byte,
            end_byte,
            ..
        } => (start_byte, end_byte),
    };
    format!("{:?}:{start}..{end}#{}", node.file(), index.index())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_path::AccessPath;
    use crate::cpg::{FlowConfidence, VarAccess};

    #[test]
    fn traces_each_refused_def_back_from_a_seed_along_dataflow() {
        let mut graph = DiGraph::new();
        let mut add_node = |start, access| {
            graph.add_node(CpgNode::variable_occurrence(
                AccessPath::simple("x"),
                "a.js".into(),
                "caller".into(),
                1,
                1,
                access,
                start,
                start + 1,
            ))
        };
        let source = add_node(0, VarAccess::Def);
        let other_source = add_node(2, VarAccess::Def);
        let via = add_node(4, VarAccess::Use);
        let seed = add_node(6, VarAccess::Use);
        let control_only_seed = add_node(8, VarAccess::Use);
        let flow = CpgEdge::DataFlow(FlowConfidence::Exact);
        graph.add_edge(source, via, flow.clone());
        graph.add_edge(via, seed, flow.clone());
        graph.add_edge(via, source, flow.clone());
        graph.add_edge(other_source, seed, flow);
        graph.add_edge(source, control_only_seed, CpgEdge::ControlFlow);

        let mut witnesses = Vec::new();
        for_each_witness(
            &graph,
            vec![seed, seed, control_only_seed],
            &BTreeSet::from([source, other_source]),
            |source, seed, path| witnesses.push((source, seed, path.to_vec())),
        );
        assert_eq!(
            witnesses,
            [
                (source, seed, vec![seed, via, source]),
                (other_source, seed, vec![seed, other_source]),
            ]
        );
    }
}
