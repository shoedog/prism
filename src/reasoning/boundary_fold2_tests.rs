use super::*;
use crate::reasoning::{order::AstOrderView, shape::reachability_and_boundary};

#[test]
fn continuation_cap_keeps_late_legacy_matches() {
    use std::fmt::Write;

    let mut results = Vec::new();
    for definitions in [(0..130).collect::<Vec<_>>(), (1..130).chain([0]).collect()] {
        let mut source = String::from(
            "function main(req) { const t = req.body; h1(t); }\n\
             function h1(x) { h2(x); }\nfunction h2(y) {\n",
        );
        for index in 0..130 {
            writeln!(source, "  g{index}(y);").unwrap();
        }
        source.push_str("}\n");
        for index in &definitions {
            writeln!(
                source,
                "function g{index}(p) {{\n  const q = p;\n  eval(q);\n}}"
            )
            .unwrap();
        }
        let parsed =
            crate::ast::ParsedFile::parse("a.js", &source, crate::languages::Language::JavaScript)
                .unwrap();
        let files = [("a.js".into(), parsed)].into();
        let cpg = CodePropertyGraph::build(&files);
        let order = AstOrderView::new(&cpg, &files);
        let root = occurrence(&cpg, source.find("t = req").unwrap());
        let sink = occurrence(&cpg, source.rfind("eval(q)").unwrap() + "eval(".len());
        let trace =
            cpg.taint_trace_nodes_with_mode(&[root], Some(&order), crate::cpg::ReturnFlowMode::On);
        assert_eq!(trace.boundary.len(), 130);
        let mut targets = BoundaryTargets::new(&cpg, &trace, Some(&order));
        for _ in 0..2 {
            let (verdict, attribution) =
                reachability_and_boundary(&trace, &mut targets, root, sink, &mut Vec::new());
            assert!(matches!(
                attribution,
                BoundaryAttribution::Incomplete { .. }
            ));
            results.push((*definitions.last().unwrap(), verdict));
        }
    }
    assert_eq!(
        results,
        [
            (129, Reachability::BoundaryExited),
            (129, Reachability::BoundaryExited),
            (0, Reachability::BoundaryExited),
            (0, Reachability::BoundaryExited)
        ]
    );
}

#[test]
fn depth_capped_exact_literal_binding_loses_to_a_longer_single_relaxation() {
    use crate::cpg::{CpgEdge, FlowConfidence, ReturnFlowMode};

    let source = "function main() { const t = input(); a(t); }
function a(x) { b(x); }
function b(y) {
  pick({ cmd: y });
  pick(y);
}
function pick(p) { exec(p.cmd); }";
    let parsed =
        crate::ast::ParsedFile::parse("a.js", source, crate::languages::Language::JavaScript)
            .unwrap();
    let files = [("a.js".into(), parsed)].into();
    let mut cpg = CodePropertyGraph::build(&files);
    let root = occurrence(&cpg, source.find("t = input").unwrap());
    use petgraph::visit::EdgeRef;
    let literal = occurrence(&cpg, source.find("cmd: y").unwrap() + "cmd: ".len());
    let (binding, short) = cpg.graph.edges(literal)
        .find(|edge|
            matches!(edge.weight(), CpgEdge::DataFlow(_))
                && matches!(cpg.node(edge.target()), crate::cpg::CpgNode::Variable { function, access: VarAccess::Use, .. } if function == "pick"))
        .map(|edge| (edge.id(), edge.target()))
        .expect("literal-to-Use binding");
    *cpg.graph.edge_weight_mut(binding).unwrap() = CpgEdge::DataFlow(FlowConfidence::Exact);
    let long = occurrence(&cpg, source.find("p) {").unwrap());
    let mid = variable(&mut cpg, "pick", source.len());
    let sink = variable(&mut cpg, "pick", source.len() + 1);
    for (from, to) in [(short, sink), (long, mid), (mid, sink)] {
        cpg.graph
            .add_edge(from, to, CpgEdge::DataFlow(FlowConfidence::Exact));
    }
    // Match the synthetic suffix to the parsed owner's start line.
    for node in [mid, sink] {
        let crate::cpg::CpgNode::Variable {
            function_start_line,
            line,
            ..
        } = &mut cpg.graph[node]
        else {
            unreachable!()
        };
        *function_start_line = 7;
        *line = 7;
    }
    let trace = cpg.taint_trace_nodes_with_mode(&[root], None, ReturnFlowMode::On);
    for target in [short, long] {
        assert!(
            trace.boundary.iter().any(|edge| edge.to == target
                && edge.reason == LossReason::DepthCap
                && !edge.edge_own_not_exact
                && cpg
                    .graph
                    .edges_connecting(edge.from, edge.to)
                    .any(|edge| edge.weight() == &CpgEdge::DataFlow(FlowConfidence::Exact))),
            "missing exact depth refusal for {:?}: {:?}",
            cpg.node(target),
            trace.boundary
        );
    }
    let mut targets = BoundaryTargets::new(&cpg, &trace, None);
    assert!(matches!(targets.find(root, sink, &mut Vec::new()),
        BoundaryAttribution::Matched(edge) if edge.to == long));
}
