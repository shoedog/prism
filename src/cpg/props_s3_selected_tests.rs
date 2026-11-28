//! Binding proofs at the actual selected plain-props Use.

use super::props_generated_tests::parse;
use super::props_s2_test_support::capture;
use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
use crate::resolution::{ResolutionConfidence, ResolutionKind};
use petgraph::{graph::NodeIndex, visit::EdgeRef};
use std::collections::{BTreeMap, BTreeSet};

fn endpoint(
    graph: &petgraph::graph::DiGraph<CpgNode, CpgEdge>,
    id: NodeIndex,
    owner: &str,
    name: &str,
    mode: VarAccess,
    start: usize,
) -> bool {
    matches!(&graph[id], CpgNode::Variable { function, path, access, start_byte, end_byte, .. }
        if function == owner && path.base == name && *access == mode && *start_byte == start && *end_byte > start)
}

#[test]
fn props_s3_selected_read_scope_swaps() {
    let mut errors = Vec::new();
    let mut materialized = 0;
    let mut unmaterialized = 0;
    for (scope, params, body) in [
        ("block", "p", "{ let Q = clean(); READ; }"),
        ("loop", "p", "for (let Q of values) { READ; }"),
        (
            "switch",
            "p",
            "switch (flag) { case 1: let Q = clean(); READ; break; }",
        ),
        ("catch", "p", "try { throw err; } catch (Q) { READ; }"),
        (
            "parameter-body",
            "p, init = observe(p)",
            "var Q = clean(); READ;",
        ),
    ] {
        for (shape, read) in [
            ("bare", "observe(p)"),
            ("projected", "observe(p.a)"),
            ("mixed", "observe(p); observe(p.a)"),
        ] {
            for (binding, same) in [("p", false), ("q", true)] {
                for grammar in ["jsx", "tsx"] {
                    let name = format!("case.{grammar}");
                    let body = body.replace('Q', binding).replace("READ", read);
                    let source = format!("function P({params}) {{\n  {body}\n}}\nfunction App(x) {{\n  return <P a={{x}}/>;\n}}\n");
                    let case = format!("{scope}/{shape}/{binding}/{grammar}");
                    let start = "function P(".len();
                    let site_source = source.find("a={x}").unwrap() + "a={".len();
                    let targets: BTreeSet<_> = source
                        .lines()
                        .nth(1)
                        .unwrap()
                        .match_indices("observe(p")
                        .map(|(offset, _)| {
                            source.lines().next().unwrap().len() + 1 + offset + "observe(".len()
                        })
                        .collect();
                    let inputs = parse(&BTreeMap::from([(name.clone(), source)]));
                    let file = &inputs[&name];
                    assert_eq!(file.parse_error_count, 0, "{case}");
                    let formal_owner = file.all_functions()[0];
                    for &target in &targets {
                        assert_eq!(
                            file.js_ts_occurrence_binds_formal(&formal_owner, "p", target),
                            same,
                            "{case}/{target}"
                        );
                    }
                    let (graph, captured) = capture(&inputs, || CodePropertyGraph::build(&inputs));
                    let pre = &captured.pre_binding.unwrap().graph;
                    let defs: Vec<_> = pre
                        .node_indices()
                        .filter(|&id| endpoint(pre, id, "P", "p", VarAccess::Def, start))
                        .collect();
                    for target in targets {
                        let reached: Vec<_> = defs
                            .iter()
                            .flat_map(|&id| pre.edges(id))
                            .filter(|edge| {
                                edge.weight().is_data_flow()
                                    && endpoint(
                                        pre,
                                        edge.target(),
                                        "P",
                                        "p",
                                        VarAccess::Use,
                                        target,
                                    )
                            })
                            .collect();
                        if reached.is_empty() {
                            unmaterialized += 1;
                            println!("S3_SELECTED {case}/{target} AST_ONLY");
                            continue;
                        }
                        materialized += 1;
                        for inherited in reached {
                            let id = inherited.target();
                            let actual: Vec<_> = graph
                                .graph
                                .edges_directed(id, petgraph::Incoming)
                                .filter(|edge| {
                                    edge.weight().is_data_flow()
                                        && endpoint(
                                            &graph.graph,
                                            edge.source(),
                                            "App",
                                            "x",
                                            VarAccess::Use,
                                            site_source,
                                        )
                                })
                                .map(|edge| edge.weight().clone())
                                .collect();
                            let expected: Vec<_> = same
                                .then_some(inherited.weight().clone())
                                .into_iter()
                                .collect();
                            let verdict = if actual == expected { "PASS" } else { "RED" };
                            println!("S3_SELECTED {case}/{target} {verdict} {id:?} {actual:?}");
                            if actual != expected {
                                errors.push(format!("{case}/{target} {id:?}: expected {expected:?}, observed {actual:?}"));
                            }
                        }
                    }
                }
            }
        }
    }
    println!("S3_SELECTED_TOTAL materialized={materialized} ast_only={unmaterialized}");
    assert!(materialized > 0, "require actual reached endpoints");
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}

#[test]
fn props_s3_target_identity_alias_and_unread_controls() {
    let mut errors = Vec::new();
    for grammar in ["jsx", "tsx"] {
        for (case, import, formal, read, refusal) in [
            ("foreign", "", "{a}", "a", Some("refused_target_identity")),
            ("imported", "import {D} from './ui';\n", "{a}", "a", None),
            (
                "alias",
                "import {D} from './ui';\n",
                "{a} = defaults",
                "a",
                Some("refused_alias_pattern"),
            ),
            ("inert", "import {D} from './ui';\n", "{a} = {}", "a", None),
            (
                "unread",
                "import {D} from './ui';\n",
                "{b}",
                "b",
                Some("refused_key_not_read"),
            ),
        ] {
            let caller = format!("{import}function App(x) {{\n  return <D a={{x}}/>;\n}}\n");
            let target = format!("const defaults = read();\nexport function D({formal}) {{\n  return use({read});\n}}\n");
            let files = parse(&BTreeMap::from([
                (format!("app.{grammar}"), caller),
                (format!("ui.{grammar}"), target),
            ]));
            assert!(files.values().all(|file| file.parse_error_count == 0));
            let graph = CodePropertyGraph::build(&files);
            let sites: Vec<_> = graph
                .call_graph
                .calls
                .values()
                .flatten()
                .filter(|site| site.jsx_element && site.callee_name == "D")
                .collect();
            assert_eq!(sites.len(), 1, "{case}/{grammar}");
            let target = graph.call_graph.resolve_call_site(sites[0]);
            assert_eq!(target.len(), 1, "{case}/{grammar} {target:?}");
            assert_eq!(
                target[0].kind,
                if case == "foreign" {
                    ResolutionKind::FreeSingle
                } else {
                    ResolutionKind::ImportMember
                }
            );
            assert_eq!(target[0].confidence, ResolutionConfidence::Exact);
            let expected = BTreeMap::from([
                ("sites_jsx".to_owned(), 1),
                (refusal.unwrap_or("edges_to_def").to_owned(), 1),
            ]);
            let hops: Vec<_> = graph.graph.edge_references().filter(|edge| {
                edge.weight().is_data_flow()
                    && matches!(graph.node(edge.source()), CpgNode::Variable { function, .. } if function == "App")
                    && matches!(graph.node(edge.target()), CpgNode::Variable { function, .. } if function == "D")
            }).collect();
            if graph.dfg_label_stats.dfg_props == expected
                && hops.len() == usize::from(refusal.is_none())
            {
                println!("S3_ADMISSION {case}/{grammar} PASS");
            } else {
                let error = format!(
                    "counts {:?}, expected {expected:?}; {} hops",
                    graph.dfg_label_stats.dfg_props,
                    hops.len()
                );
                println!("S3_ADMISSION {case}/{grammar} RED {error}");
                errors.push(format!("{case}/{grammar}: {error}"));
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
