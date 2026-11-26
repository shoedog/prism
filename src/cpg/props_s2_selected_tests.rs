//! Byte-selected destinations and inherited #594 labels.

use super::props_s2_test_support::capture;
use super::var_index::VarIndex;
use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use petgraph::{graph::NodeIndex, visit::EdgeRef};
use std::collections::BTreeMap;

fn parse(source: &str) -> BTreeMap<String, ParsedFile> {
    let path = "selected.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "{source}");
    BTreeMap::from([(path.to_owned(), parsed)])
}

fn root(node: &CpgNode, owner: &str, name: &str, mode: VarAccess) -> bool {
    matches!(node, CpgNode::Variable { function, path, access, .. }
        if function == owner && path.base == name && *access == mode)
}

fn starts_at(node: &CpgNode, offset: usize) -> bool {
    matches!(node, CpgNode::Variable { start_byte, end_byte, .. } if *start_byte == offset && *end_byte > offset)
}

#[test]
fn props_s2_selected_read_does_not_borrow_a_formal_witness() {
    let mut failures = Vec::new();
    let mut reached = 0;
    for (scope, declaration, body) in [
        ("block", "p", "{ let p = clean(); SHADOW; } FORMAL;"),
        ("for-of", "p", "for (let p of values) { SHADOW; } FORMAL;"),
        (
            "switch",
            "p",
            "switch (flag) { case 1: let p = clean(); SHADOW; break; } FORMAL;",
        ),
        (
            "catch",
            "p",
            "try { throw err; } catch (p) { SHADOW; } FORMAL;",
        ),
        (
            "parameter-body",
            "p, init = FORMAL",
            "var p = clean(); SHADOW;",
        ),
    ] {
        for suffix in ["", ".a", "['a']"] {
            let shadow = format!("decoy(p{suffix})");
            let formal = format!("formal(p{suffix})");
            let source = format!("function D({declaration}) {{\n  {body}\n}}\nfunction App(x) {{\n  D({{a: x}});\n}}\n")
                .replace("SHADOW", &shadow).replace("FORMAL", &formal);
            let decoy_at = source.find(&shadow).unwrap() + "decoy(".len();
            let formal_at = source.find(&formal).unwrap() + "formal(".len();
            let inputs = parse(&source);
            let parsed = &inputs["selected.js"];
            let owner = parsed.all_functions()[0];
            assert!(
                !parsed.js_ts_occurrence_binds_formal(&owner, "p", decoy_at),
                "{scope}: {decoy_at}"
            );
            assert!(
                parsed.js_ts_occurrence_binds_formal(&owner, "p", formal_at),
                "{scope}: {formal_at}"
            );
            let (cpg, observed) = capture(|| CodePropertyGraph::build(&inputs));
            let pre = observed.pre_binding.unwrap().graph;
            let formal_defs: Vec<_> = pre
                .node_indices()
                .filter(|&id| {
                    root(&pre[id], "D", "p", VarAccess::Def)
                        && starts_at(&pre[id], "function D(".len())
                })
                .collect();
            for (position, bound) in [(decoy_at, false), (formal_at, true)] {
                let reached_uses: Vec<_> = formal_defs
                    .iter()
                    .flat_map(|&id| pre.edges(id))
                    .filter(|edge| {
                        edge.weight().is_data_flow()
                            && root(&pre[edge.target()], "D", "p", VarAccess::Use)
                            && starts_at(&pre[edge.target()], position)
                    })
                    .map(|edge| edge.target())
                    .collect();
                if reached_uses.is_empty() {
                    eprintln!("PROPS_SELECTED {scope} {suffix:?} {position} PRECONDITION_MISS");
                    continue;
                }
                reached += 1;
                for use_node in reached_uses {
                    let incoming =
                        cpg.graph
                            .edges_directed(use_node, petgraph::Incoming)
                            .any(|edge| {
                                edge.weight().is_data_flow()
                                    && root(&cpg.graph[edge.source()], "App", "x", VarAccess::Use)
                            });
                    if incoming != bound {
                        failures.push((scope, suffix, position, bound, incoming));
                    }
                }
            }
        }
    }
    assert!(reached > 0, "at least one graph witness is required");
    assert!(
        failures.is_empty(),
        "byte-specific selected reads: {failures:?}"
    );
}

#[test]
fn props_s2_literal_preserves_the_inherited_callee_kill_line() {
    let source = "function D(p) {\n  use(p);\n  use(p.a);\n}\nfunction App(x) {\n  D({a: x});\n}\n";
    let inputs = parse(source);
    let (cpg, observed) = capture(|| CodePropertyGraph::build(&inputs));
    let mut pre = observed.pre_binding.unwrap();
    let use_at = source.find("use(p.a)").unwrap() + "use(".len();
    let inherited: Vec<_> = pre
        .graph
        .edge_references()
        .filter(|edge| {
            edge.weight().is_data_flow()
                && root(&pre.graph[edge.source()], "D", "p", VarAccess::Def)
                && root(&pre.graph[edge.target()], "D", "p", VarAccess::Use)
                && starts_at(&pre.graph[edge.target()], use_at)
        })
        .map(|edge| (edge.id(), edge.target()))
        .collect();
    let [(inherited, read)] = inherited.as_slice() else {
        panic!("inherited member edge {inherited:?}");
    };
    let lookup = VarIndex::from_map(pre.lookup);
    let collect =
        |graph: &petgraph::graph::DiGraph<CpgNode, CpgEdge>| -> Vec<(NodeIndex, CpgEdge)> {
            CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &lookup, graph, &inputs)
                .into_iter()
                .filter_map(|(from, to, label)| {
                    (to == *read && root(&graph[from], "App", "x", VarAccess::Use))
                        .then_some((from, label))
                })
                .collect()
        };
    let exact = collect(&pre.graph);
    assert_eq!(exact.len(), 1, "unchanged admitted endpoint");
    assert_eq!(exact[0].1, CpgEdge::DataFlow(FlowConfidence::Exact));
    let killed = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 2 });
    pre.graph[*inherited] = CpgEdge::DataFlow(killed);
    assert_eq!(
        collect(&pre.graph),
        [(exact[0].0, CpgEdge::DataFlow(killed))]
    );
}
