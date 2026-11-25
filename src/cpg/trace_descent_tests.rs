use petgraph::graph::NodeIndex;

use super::object_literal_arg_tests::build;
use super::trace::{BoundaryEdge, BoundaryKind, Relation, Trace};
use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use crate::languages::Language;

const L01: &str = include_str!("../../tests/taint_parity/l01/a.js");
const NAMEONLY: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CallNameOnly);

fn occurrence(cpg: &CodePropertyGraph, start: usize, access: VarAccess) -> NodeIndex {
    let mut nodes = cpg.graph.node_indices().filter(|&id| {
        matches!(cpg.node(id), CpgNode::Variable { start_byte, access: actual, .. }
            if *start_byte == start && *actual == access)
    });
    let node = nodes.next().expect("missing occurrence");
    assert!(nodes.next().is_none(), "duplicate occurrence at {start}");
    node
}

fn add_binding(cpg: &mut CodePropertyGraph, from: NodeIndex, to: NodeIndex, label: FlowConfidence) {
    assert!(cpg
        .graph
        .edges_connecting(from, to)
        .all(|edge| !edge.weight().is_data_flow()));
    cpg.graph.add_edge(from, to, CpgEdge::DataFlow(label));
}

fn assert_boundary(trace: &Trace, from: NodeIndex, to: NodeIndex, reason: &str) {
    assert!(
        !trace.frontier_by_root[&from].contains(&to),
        "{reason}: crossed {from:?} → {to:?}"
    );
    assert!(
        trace.boundary.contains(&BoundaryEdge {
            root: from,
            from,
            to,
            kind: BoundaryKind::CrossFunction,
        }),
        "{reason}: missing refused crossing"
    );
}

#[test]
fn nameonly_callback_edge_not_crossed_through_same_line_exact_site() {
    let source = include_str!("../../tests/taint_parity/n01/a.js");
    let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
    let from = occurrence(&cpg, source.find("cb(x)").unwrap() + 3, VarAccess::Use);
    let to = occurrence(&cpg, source.find("handler(p)").unwrap() + 8, VarAccess::Def);
    add_binding(&mut cpg, from, to, NAMEONLY);
    assert_boundary(&cpg.taint_trace_nodes(&[from], None), from, to, "not_exact");
}

#[test]
fn byte_carrying_occurrence_matches_by_containment_only() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let from = occurrence(&cpg, L01.find("log(t)").unwrap() + 4, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    add_binding(&mut cpg, from, to, FlowConfidence::Exact);
    assert_boundary(
        &cpg.taint_trace_nodes(&[from], None),
        from,
        to,
        "no_matching_site",
    );
}

#[test]
fn log_then_g_still_descends_from_inner_occurrence() {
    let cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let from = occurrence(&cpg, 166, VarAccess::Use);
    assert!(matches!(
        cpg.node(from),
        CpgNode::Variable {
            start_byte: 166,
            end_byte: 167,
            ..
        }
    ));
    assert_eq!(&L01[164..168], "g(t)");
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    let sink = occurrence(&cpg, 97, VarAccess::Use);
    let trace = cpg.taint_trace_nodes(&[from], None);
    assert_eq!(
        trace.parents_by_root[&(from, to)],
        (from, Relation::CallDescent)
    );
    assert!(trace.frontier_by_root[&from].contains(&sink));
}

#[test]
fn exact_sibling_cache_cannot_admit_not_exact_binding() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    // Identical site-proof inputs, separate binding edges with their own labels.
    let nameonly = cpg.graph.add_node(cpg.node(exact).clone());
    add_binding(&mut cpg, nameonly, to, NAMEONLY);
    let trace = cpg.taint_trace_nodes(&[exact, nameonly], None);
    assert_eq!(
        trace.parents_by_root[&(exact, to)],
        (exact, Relation::CallDescent)
    );
    assert_boundary(&trace, nameonly, to, "not_exact");
}

#[test]
fn proof_cache_distinguishes_occurrence_end() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    let mut wider = cpg.node(exact).clone();
    let CpgNode::Variable { end_byte, .. } = &mut wider else {
        unreachable!()
    };
    *end_byte = 169;
    let wider = cpg.graph.add_node(wider);
    add_binding(&mut cpg, wider, to, FlowConfidence::Exact);
    let trace = cpg.taint_trace_nodes(&[exact, wider], None);
    assert_eq!(
        trace.parents_by_root[&(exact, to)],
        (exact, Relation::CallDescent)
    );
    assert_boundary(&trace, wider, to, "no_matching_site");
}

#[test]
fn two_same_spelling_sites_still_refuse_as_ambiguous_site() {
    for (language, ext, source, formal) in [
        (
            Language::Python,
            "py",
            include_str!("../../tests/taint_parity/l02/a.py"),
            "g(c)",
        ),
        (
            Language::Go,
            "go",
            include_str!("../../tests/taint_parity/l03/main.go"),
            "g(c string)",
        ),
    ] {
        let cpg = build(language, &[("a", source)], ext);
        let from = occurrence(&cpg, source.find("g(t)").unwrap() + 2, VarAccess::Use);
        let to = occurrence(&cpg, source.find(formal).unwrap() + 2, VarAccess::Def);
        assert_boundary(
            &cpg.taint_trace_nodes(&[from], None),
            from,
            to,
            "ambiguous_site",
        );
    }
}

#[test]
fn empty_occurrence_at_the_exclusive_end_of_a_call_is_not_inside_it() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    assert_eq!(&L01[164..168], "g(t)");
    let mut empty = cpg.node(exact).clone();
    let CpgNode::Variable {
        start_byte,
        end_byte,
        ..
    } = &mut empty
    else {
        unreachable!()
    };
    *start_byte = 168;
    *end_byte = 168;
    let empty = cpg.graph.add_node(empty);
    add_binding(&mut cpg, empty, to, FlowConfidence::Exact);
    assert_boundary(
        &cpg.taint_trace_nodes(&[empty], None),
        empty,
        to,
        "no_matching_site",
    );
}

#[test]
fn nameonly_binding_edge_into_the_walked_function_is_a_boundary_not_local_flow() {
    let source = include_str!("../../tests/taint_parity/n01/a.js");
    let mut cpg = build(Language::JavaScript, &[("a", source)], "js");
    let from = occurrence(&cpg, source.find("cb(x)").unwrap() + 3, VarAccess::Use);
    // A name-only edge from a use to another node of the SAME function (a callback that resolves
    // back into its own caller): it must not enter the frontier as local DataFlow.
    let local = cpg
        .graph
        .node_indices()
        .find(|&id| {
            id != from
                && matches!(cpg.node(id), CpgNode::Variable { access: VarAccess::Use, function, .. }
                    if matches!(cpg.node(from), CpgNode::Variable { function: f, .. } if f == function))
        })
        .expect("a second use in the same function");
    add_binding(&mut cpg, from, local, NAMEONLY);
    let trace = cpg.taint_trace_nodes(&[from], None);
    assert!(
        !trace.frontier_by_root[&from].contains(&local),
        "name-only edge became local flow"
    );
    assert!(trace.boundary.contains(&BoundaryEdge {
        root: from,
        from,
        to: local,
        kind: BoundaryKind::SelfFunctionParam,
    }));
}

#[test]
fn nameonly_binding_edge_with_a_containing_site_is_refused_without_any_exact_sibling() {
    let mut cpg = build(Language::JavaScript, &[("a", L01)], "js");
    let exact = occurrence(&cpg, 166, VarAccess::Use);
    let to = occurrence(&cpg, L01.find("g(x)").unwrap() + 2, VarAccess::Def);
    // The occurrence sits inside g(t): only the label can refuse it (cold cache, single root).
    let nameonly = cpg.graph.add_node(cpg.node(exact).clone());
    add_binding(&mut cpg, nameonly, to, NAMEONLY);
    assert_boundary(
        &cpg.taint_trace_nodes(&[nameonly], None),
        nameonly,
        to,
        "not_exact",
    );
}
