use crate::cpg::{CodePropertyGraph, LossReason};
use petgraph::graph::NodeIndex;

use super::shape::{reachability_and_boundary, reachability_for_node_from};
use super::types::Reachability;

fn occurrence(cpg: &CodePropertyGraph, byte: usize) -> NodeIndex {
    cpg.graph.node_indices().find(|&node| {
        matches!(cpg.node(node), crate::cpg::CpgNode::Variable { start_byte, .. } if *start_byte == byte)
    }).expect("fixture occurrence")
}

#[test]
fn sink_past_refused_hop_keeps_parent_verdict() {
    for (source, root_byte, sink_byte, reason) in [
        (
            include_str!("../../tests/taint_parity/f01/a.js"),
            180,
            61,
            LossReason::DepthCap,
        ),
        (
            include_str!("../../tests/taint_parity/f02/a.js"),
            159,
            111,
            LossReason::NotExact,
        ),
        (
            include_str!("../../tests/taint_parity/f03/a.js"),
            209,
            111,
            LossReason::TargetNotBinding,
        ),
    ] {
        let parsed =
            crate::ast::ParsedFile::parse("a.js", source, crate::languages::Language::JavaScript)
                .unwrap();
        let cpg = CodePropertyGraph::build(&[("a.js".into(), parsed)].into());
        let root = occurrence(&cpg, root_byte);
        let sink = occurrence(&cpg, sink_byte);
        let trace = cpg.taint_trace_nodes_with_mode(&[root], None, crate::cpg::ReturnFlowMode::On);
        assert_eq!(
            reachability_for_node_from(&cpg, &trace, root, sink),
            Reachability::NotReached
        );
        let boundary = trace
            .boundary
            .iter()
            .find(|edge| edge.root == root && edge.reason == reason)
            .expect("live refused hop");
        assert_eq!(
            reachability_and_boundary(&cpg, &trace, root, boundary.to, None, &mut Vec::new()),
            (Reachability::BoundaryExited, Some(boundary))
        );
    }
}
