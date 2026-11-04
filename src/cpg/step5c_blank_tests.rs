//! Go `_` lvalues are positional holes in return flow (#647), not missing targets.
use super::*;
use crate::languages::Language;
use petgraph::visit::EdgeRef;

const SRC: &str = "package main\n\
func src() (int, error) { x := 1; return x, nil }\n\
func one() int { y := 2; return y }\n\
func three() (int, int, int) { p := 1; q := 2; r := 3; return p, q, r }\n\
func run() {\n\
 v, _ := src()\n\
 _, err := src()\n\
 _ = one()\n\
 a, _, c := three()\n\
 sink(v, err, a, c)\n\
}\n";

fn build() -> CodePropertyGraph {
    let files = BTreeMap::from([(
        "m.go".to_string(),
        ParsedFile::parse("m.go", SRC, Language::Go).unwrap(),
    )]);
    let cg = CallGraph::build(&files);
    let dfg = DataFlowGraph::build(&files);
    CodePropertyGraph::assemble_graph(cg, dfg, &files, None)
}

/// (target variable, source variable name, suppress_shortcut) of every ReturnFlow edge.
fn flows(cpg: &CodePropertyGraph) -> Vec<(String, String, bool)> {
    let name = |n: NodeIndex| match &cpg.graph[n] {
        CpgNode::Variable { path, .. } => path.base.clone(),
        CpgNode::ReturnValue { .. } => "<ret>".to_string(),
        _ => "?".to_string(),
    };
    let mut out: Vec<_> = cpg
        .graph
        .edge_references()
        .filter_map(|e| match e.weight() {
            CpgEdge::ReturnFlow { suppress_shortcut } => {
                Some((name(e.target()), name(e.source()), *suppress_shortcut))
            }
            _ => None,
        })
        .collect();
    out.sort();
    out
}

fn check(cpg: &CodePropertyGraph) {
    let got = flows(cpg);
    let expect: Vec<(String, String, bool)> =
        [("a", "p"), ("c", "r"), ("err", "<ret>"), ("v", "x")]
            .iter()
            .map(|(t, s)| (t.to_string(), s.to_string(), true))
            .collect();
    assert_eq!(got, expect, "no flow into `_`, real slots keep theirs");
    let s = &cpg.return_flow_stats;
    assert_eq!(s.return_flow_edges, 4);
    assert_eq!(s.return_flow_skipped_blank_lhs, 1, "only `_ = one()`");
    assert_eq!(s.return_flow_skipped_non_simple_lhs, 0);
    assert_eq!(s.return_flow_suppression_certified, 3);
}

#[test]
fn go_blank_slots_keep_return_flow() {
    check(&build());
}

#[test]
fn go_blank_slots_keep_return_flow_in_reference() {
    check(&step5c_reference::with_reference(build));
}
