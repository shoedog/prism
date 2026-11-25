use crate::cpg::{BoundaryEdge, BoundaryKind, CodePropertyGraph, LossReason, Trace, VarAccess};
use petgraph::graph::NodeIndex;

use super::shape::{reachability_for_node_from, BoundaryAttribution, BoundaryTargets};
use super::types::Reachability;

fn occurrence(cpg: &CodePropertyGraph, byte: usize) -> NodeIndex {
    cpg.graph.node_indices().find(|&node| {
        matches!(cpg.node(node), crate::cpg::CpgNode::Variable { start_byte, .. } if *start_byte == byte)
    }).expect("fixture occurrence")
}

#[test]
fn sink_past_refused_hop_is_boundary_exited_with_its_reason() {
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
            Reachability::BoundaryExited
        );
        let mut targets = BoundaryTargets::new(&cpg, &trace, None);
        assert!(matches!(targets.find(root, sink, &mut Vec::new()),
            BoundaryAttribution::Matched(edge) if edge.reason == reason));
        assert_eq!(
            reachability_for_node_from(&cpg, &trace, sink, root),
            Reachability::NotReached
        );
    }
}

fn variable(cpg: &mut CodePropertyGraph, owner: &str, start: usize) -> NodeIndex {
    cpg.graph.add_node(crate::cpg::CpgNode::variable_occurrence(
        crate::access_path::AccessPath::simple("x"),
        "a.js".into(),
        owner.into(),
        1,
        2,
        VarAccess::Use,
        start,
        start + 1,
    ))
}

fn boundary(root: NodeIndex, to: NodeIndex, reason: LossReason) -> BoundaryEdge {
    BoundaryEdge {
        root,
        from: root,
        to,
        kind: BoundaryKind::CrossFunction,
        reason,
        relation: crate::cpg::Relation::CallDescent,
        depth: 0,
        site: None,
        edge_own_not_exact: false,
    }
}

#[test]
fn boundary_target_test_cap_warns() {
    let mut cpg = CodePropertyGraph::build(&Default::default());
    let root = variable(&mut cpg, "root", 0);
    let sink = variable(&mut cpg, "sink", 1000);
    let mut trace = Trace::default();
    let mut targets = Vec::new();
    for i in (1..=129).rev() {
        let to = variable(&mut cpg, "target", i);
        targets.push(to);
        trace
            .boundary
            .insert(boundary(root, to, LossReason::NotExact));
    }
    let target = targets[0];
    let warm_root = variable(&mut cpg, "warm_root", 200);
    trace
        .boundary
        .insert(boundary(warm_root, target, LossReason::NotExact));
    cpg.graph.add_edge(
        target,
        sink,
        crate::cpg::CpgEdge::DataFlow(crate::cpg::FlowConfidence::Exact),
    );
    // The 129th candidate is required, even when a different root warmed its answer.
    let mut target_search = super::shape::BoundaryTargets::new(&cpg, &trace, None);
    for warm in [false, true] {
        if warm {
            assert!(
                matches!(target_search.find(warm_root, sink, &mut Vec::new()),
                super::shape::BoundaryAttribution::Matched(edge) if edge.to == target)
            );
        }
        let attribution = target_search.find(root, sink, &mut Vec::new());
        assert!(matches!(
            attribution,
            super::shape::BoundaryAttribution::Incomplete
        ));
        let warning = super::boundary_warnings::attribution_warning(
            &cpg,
            &cpg.to_var_location(root).unwrap(),
            &cpg.to_var_location(sink).unwrap(),
            attribution,
        )
        .unwrap();
        assert!(warning.message.contains("incomplete (unexplained)"));
        assert_eq!(warning.location.unwrap().start_byte, 1000);
    }
    trace
        .boundary
        .remove(&boundary(root, targets[128], LossReason::NotExact));
    let mut target_search = super::shape::BoundaryTargets::new(&cpg, &trace, None);
    for _ in 0..2 {
        assert!(
            matches!(target_search.find(root, sink, &mut Vec::new()), super::shape::BoundaryAttribution::Matched(edge) if edge.to == target)
        );
    }
    // A direct match outranks the bounded continuation pass.
    assert!(
        matches!(target_search.find(root, target, &mut Vec::new()), super::shape::BoundaryAttribution::Matched(edge) if edge.to == target)
    );
}

#[test]
fn boundary_target_tie_break_prefers_causal_length_then_reason_then_bytes() {
    let mut cpg = CodePropertyGraph::build(&Default::default());
    let root = variable(&mut cpg, "root", 0);
    let wrong_root = variable(&mut cpg, "root", 1);
    let farther = variable(&mut cpg, "target", 2);
    let via = variable(&mut cpg, "target", 3);
    let later = variable(&mut cpg, "target", 8);
    let earlier = variable(&mut cpg, "target", 7);
    let weak = variable(&mut cpg, "target", 6);
    let sink = variable(&mut cpg, "target", 11);
    let unrelated = variable(&mut cpg, "target", 12);
    let flow = crate::cpg::CpgEdge::DataFlow(crate::cpg::FlowConfidence::Exact);
    for (from, to) in [
        (farther, via),
        (via, sink),
        (earlier, sink),
        (later, sink),
        (weak, sink),
    ] {
        cpg.graph.add_edge(from, to, flow.clone());
    }
    let mut trace = Trace::default();
    trace.boundary.extend([
        boundary(root, farther, LossReason::DepthCap),
        boundary(root, later, LossReason::NotExact),
        boundary(root, weak, LossReason::PolicyCeiling),
        boundary(root, earlier, LossReason::NotExact),
        boundary(wrong_root, sink, LossReason::SelfParam),
    ]);
    let mut search = super::shape::BoundaryTargets::new(&cpg, &trace, None);
    assert!(
        matches!(search.find(root, sink, &mut Vec::new()), super::shape::BoundaryAttribution::Matched(edge) if edge.to == earlier)
    );
    assert!(matches!(
        search.find(root, unrelated, &mut Vec::new()),
        super::shape::BoundaryAttribution::None
    ));
    assert!(
        matches!(search.find(wrong_root, sink, &mut Vec::new()), super::shape::BoundaryAttribution::Matched(edge) if edge.root == wrong_root && edge.to == sink)
    );
}

#[test]
fn recursive_detour_needs_an_extra_relaxation_so_the_direct_depth_cap_wins() {
    let source = include_str!("../../tests/taint_parity/s3-js-recursive-return-distal/a.js");
    let parsed =
        crate::ast::ParsedFile::parse("a.js", source, crate::languages::Language::JavaScript)
            .unwrap();
    let files = [("a.js".into(), parsed)].into();
    let cpg = CodePropertyGraph::build(&files);
    let order = super::order::AstOrderView::new(&cpg, &files);
    let root = occurrence(&cpg, 64);
    let sink = occurrence(&cpg, 248);
    let trace =
        cpg.taint_trace_nodes_with_mode(&[root], Some(&order), crate::cpg::ReturnFlowMode::On);
    assert!(trace
        .boundary
        .iter()
        .any(|edge| edge.root == root && edge.reason == LossReason::SelfParam));
    assert!(
        matches!(BoundaryTargets::new(&cpg, &trace, Some(&order)).find(root, sink, &mut Vec::new()),
        BoundaryAttribution::Matched(edge) if edge.reason == LossReason::DepthCap)
    );
}

#[test]
fn recursive_boundary_cannot_claim_an_earlier_read_even_as_a_direct_target() {
    let source = include_str!("../../tests/taint_parity/b04b/a.js");
    let parsed =
        crate::ast::ParsedFile::parse("a.js", source, crate::languages::Language::JavaScript)
            .unwrap();
    let cpg = CodePropertyGraph::build(&[("a.js".into(), parsed)].into());
    let root = occurrence(&cpg, 49);
    let earlier = occurrence(&cpg, 26);
    let later = occurrence(&cpg, source.find("a:x").unwrap() + 2);
    let mut trace = Trace::default();
    for target in [earlier, later] {
        trace.boundary.insert(BoundaryEdge {
            kind: BoundaryKind::SelfFunctionParam,
            ..boundary(root, target, LossReason::SelfParam)
        });
    }
    let mut targets = BoundaryTargets::new(&cpg, &trace, None);
    assert!(matches!(
        targets.find(root, earlier, &mut Vec::new()),
        BoundaryAttribution::None
    ));
    assert!(
        matches!(targets.find(root, later, &mut Vec::new()), BoundaryAttribution::Matched(edge) if edge.to == later)
    );
}
