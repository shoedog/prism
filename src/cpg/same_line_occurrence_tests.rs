//! Exact same-line occurrence identity and interprocedural binding.
use super::*;
use crate::data_flow::VarAccessKind;
use crate::languages::Language;

const ORIGIN: &str = "export function item(input){return input;}";

fn fixture(language: Language, ext: &str, app: &str) -> BTreeMap<String, ParsedFile> {
    fixture_with_origin(language, ext, app, ORIGIN)
}

fn fixture_with_origin(
    language: Language,
    ext: &str,
    app: &str,
    origin: &str,
) -> BTreeMap<String, ParsedFile> {
    [
        (format!("app.{ext}"), app),
        (format!("origin.{ext}"), origin),
    ]
    .into_iter()
    .map(|(path, source)| {
        let parsed = ParsedFile::parse(&path, source, language).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{path}: {source}");
        (path, parsed)
    })
    .collect()
}

fn value_spans(source: &str) -> Vec<(usize, usize)> {
    source
        .match_indices("value")
        .map(|(start, text)| (start, start + text.len()))
        .collect()
}

fn boundary_rows(cpg: &CodePropertyGraph, app_file: &str, callee: &str) -> Vec<String> {
    let mut rows: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(confidence) = cpg.graph[edge] else {
                return None;
            };
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            match (&cpg.graph[from], &cpg.graph[to]) {
                (
                    CpgNode::Variable {
                        file,
                        function,
                        function_start_line,
                        line,
                        path,
                        access: VarAccess::Use,
                        start_byte,
                        end_byte,
                    },
                    CpgNode::Variable {
                        file: to_file,
                        function: to_function,
                        function_start_line: to_fsl,
                        line: to_line,
                        path: to_path,
                        access: VarAccess::Def,
                        start_byte: to_start,
                        end_byte: to_end,
                    },
                ) if file == app_file && function == "outer" && to_function == callee => {
                    Some(format!(
                        "{file}:{function}:{function_start_line}:{line}:{path}:Use:{start_byte}-{end_byte}->{to_file}:{to_function}:{to_fsl}:{to_line}:{to_path}:Def:{to_start}-{to_end}:{confidence:?}"
                    ))
                }
                _ => None,
            }
        })
        .collect();
    rows.sort();
    rows
}

fn graph_path_rows(cpg: &CodePropertyGraph, file: &str, path: &str) -> Vec<String> {
    let mut rows: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(confidence) = cpg.graph[edge] else {
                return None;
            };
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            match (&cpg.graph[from], &cpg.graph[to]) {
                (
                    CpgNode::Variable {
                        file: from_file,
                        function: from_function,
                        path: from_path,
                        access: from_access,
                        start_byte: from_start,
                        end_byte: from_end,
                        ..
                    },
                    CpgNode::Variable {
                        file: to_file,
                        function: to_function,
                        path: to_path,
                        access: to_access,
                        start_byte: to_start,
                        end_byte: to_end,
                        ..
                    },
                ) if from_file == file
                    && to_file == file
                    && from_function == "flow"
                    && to_function == "flow"
                    && from_path == &AccessPath::simple(path)
                    && to_path == &AccessPath::simple(path) => Some(format!(
                    "{from_access:?}:{from_start}-{from_end}->{to_access:?}:{to_start}-{to_end}:{confidence:?}"
                )),
                _ => None,
            }
        })
        .collect();
    rows.sort();
    rows
}

fn cpg_value_uses(cpg: &CodePropertyGraph, file: &str) -> Vec<(usize, usize)> {
    let mut rows: Vec<_> = cpg
        .graph
        .node_indices()
        .filter_map(|idx| match &cpg.graph[idx] {
            CpgNode::Variable {
                file: actual_file,
                function,
                function_start_line,
                line,
                path,
                access: VarAccess::Use,
                start_byte,
                end_byte,
            } if actual_file == file
                && function == "outer"
                && *function_start_line == 2
                && *line == 2
                && path == &AccessPath::simple("value") =>
            {
                Some((*start_byte, *end_byte))
            }
            _ => None,
        })
        .collect();
    rows.sort();
    rows
}

fn dfg_value_uses(cpg: &CodePropertyGraph, file: &str) -> Vec<(usize, usize)> {
    let mut rows: Vec<_> = cpg
        .dfg
        .uses
        .values()
        .flatten()
        .filter(|loc| {
            loc.file == file
                && loc.function == "outer"
                && loc.function_start_line == 2
                && loc.line == 2
                && loc.path == AccessPath::simple("value")
                && loc.kind == VarAccessKind::Use
        })
        .map(|loc| (loc.start_byte, loc.end_byte))
        .collect();
    rows.sort();
    rows
}

fn value_flow_rows(cpg: &CodePropertyGraph) -> Vec<String> {
    let mut rows: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(confidence) = cpg.graph[edge] else {
                return None;
            };
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            let node = |idx| match &cpg.graph[idx] {
                CpgNode::Variable {
                    file,
                    function,
                    function_start_line,
                    line,
                    path,
                    access,
                    start_byte,
                    end_byte,
                } => Some(format!(
                    "{file}:{function}:{function_start_line}:{line}:{path}:{access:?}:{start_byte}-{end_byte}"
                )),
                _ => None,
            };
            let from = node(from)?;
            let to = node(to)?;
            (from.contains(":value:") || to.contains(":input:"))
                .then(|| format!("{from}->{to}:{confidence:?}"))
        })
        .collect();
    rows.sort();
    rows
}

fn call_argument_context<'a>(
    cpg: &'a CodePropertyGraph,
    files: &'a BTreeMap<String, ParsedFile>,
    file: &str,
    callee: &str,
) -> (
    crate::call_graph::FunctionId,
    &'a ParsedFile,
    std::ops::Range<usize>,
) {
    let site = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.file == file && site.callee_name == callee)
        .unwrap();
    let parsed = &files[file];
    let span = parsed
        .call_argument_texts_and_spans_at(site.start_byte, callee)
        .pop()
        .unwrap()
        .1;
    (site.caller.clone(), parsed, span)
}

fn complete_graph_rows(cpg: &CodePropertyGraph) -> (Vec<String>, Vec<String>) {
    let mut nodes: Vec<_> = cpg
        .graph
        .node_indices()
        .map(|idx| format!("{:?}", cpg.graph[idx]))
        .collect();
    nodes.sort();
    let mut edges: Vec<_> = cpg
        .graph
        .edge_indices()
        .map(|edge| {
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            format!(
                "{:?}->{:?}:{:?}",
                cpg.graph[from], cpg.graph[to], cpg.graph[edge]
            )
        })
        .collect();
    edges.sort();
    (nodes, edges)
}

#[test]
fn same_line_occurrence_o01_recovers_later_argument_and_boundary() {
    let app =
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}";
    let anchors = value_spans(app);
    assert_eq!(anchors.len(), 3);
    assert_eq!(&app[anchors[0].0..anchors[0].1], "value");
    assert_eq!(&app[anchors[1].0..anchors[1].1], "value");
    assert_eq!(&app[anchors[2].0..anchors[2].1], "value");
    let expected_uses = vec![anchors[1], anchors[2]];
    let mut failures = Vec::new();

    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let files = fixture(language, ext, app);
        let file = format!("app.{ext}");
        let cpg = CodePropertyGraph::build(&files);
        let dfg_uses = dfg_value_uses(&cpg, &file);
        let cpg_uses = cpg_value_uses(&cpg, &file);
        let flows = value_flow_rows(&cpg);
        println!(
            "O01/{ext} anchors={anchors:?} dfg_uses={dfg_uses:?} cpg_uses={cpg_uses:?} flows={flows:?}"
        );

        let unique_dfg_uses: BTreeSet<_> = dfg_uses.iter().copied().collect();
        let expected_unique: BTreeSet<_> = expected_uses.iter().copied().collect();
        if unique_dfg_uses != expected_unique {
            failures.push(format!(
                "O01/{ext}: unique DFG uses expected {expected_unique:?}, got {unique_dfg_uses:?} from raw {dfg_uses:?}"
            ));
        }
        if cpg_uses != expected_uses {
            failures.push(format!(
                "O01/{ext}: CPG uses expected {expected_uses:?}, got {cpg_uses:?}"
            ));
        }
        let raw_caller_edges: Vec<_> = cpg
            .dfg
            .edges
            .iter()
            .filter(|edge| {
                edge.from.file == file
                    && edge.from.function == "outer"
                    && edge.from.path == AccessPath::simple("value")
                    && edge.to.file == file
                    && edge.to.function == "outer"
                    && edge.to.path == AccessPath::simple("value")
            })
            .map(|edge| {
                (
                    edge.from.start_byte,
                    edge.from.end_byte,
                    edge.to.start_byte,
                    edge.to.end_byte,
                    cpg.dfg
                        .labels
                        .get(&(edge.from.clone(), edge.to.clone()))
                        .copied(),
                )
            })
            .collect();
        if raw_caller_edges
            != [(
                anchors[0].0,
                anchors[0].1,
                anchors[1].0,
                anchors[1].1,
                Some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)),
            )]
        {
            failures.push(format!(
                "O01/{ext}: raw caller edge population {raw_caller_edges:?}"
            ));
        }
        if raw_caller_edges
            .iter()
            .any(|(_, _, to_start, to_end, _)| (*to_start, *to_end) == anchors[2])
        {
            failures.push(format!(
                "O01/{ext}: producer unexpectedly targets later bytes {raw_caller_edges:?}"
            ));
        }
        let boundary_prefix = format!(
            "{file}:outer:2:2:value:Use:{}-{}->origin.{ext}:item:1:1:input:Def:",
            anchors[2].0, anchors[2].1
        );
        if !flows
            .iter()
            .any(|row| row.starts_with(&boundary_prefix) && row.ends_with(":Exact"))
        {
            failures.push(format!(
                "O01/{ext}: missing later exact boundary {boundary_prefix}*; got {flows:?}"
            ));
        }

        let find_var = |owner: &str, path: &str, access: VarAccess, span: (usize, usize)| {
            let matches: Vec<_> = cpg
                .graph
                .node_indices()
                .filter(|&idx| {
                    matches!(
                        &cpg.graph[idx],
                        CpgNode::Variable {
                            function,
                            path: actual_path,
                            access: actual_access,
                            start_byte,
                            end_byte,
                            ..
                        } if function == owner && actual_path == &AccessPath::simple(path)
                            && *actual_access == access && (*start_byte, *end_byte) == span
                    )
                })
                .collect();
            assert_eq!(matches.len(), 1, "O01/{ext}/{owner}/{path}/{span:?}");
            matches[0]
        };
        let caller_def = find_var("outer", "value", VarAccess::Def, anchors[0]);
        let earlier = find_var("outer", "value", VarAccess::Use, anchors[1]);
        let later = find_var("outer", "value", VarAccess::Use, anchors[2]);
        let param = find_var("item", "input", VarAccess::Def, (21, 26));
        let body = find_var("item", "input", VarAccess::Use, (35, 40));
        let edge_labels = |from, to| {
            cpg.graph
                .edges_connecting(from, to)
                .filter_map(|edge| match *edge.weight() {
                    CpgEdge::DataFlow(label) => Some(label),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            edge_labels(caller_def, earlier),
            [FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)],
            "O01/{ext}/earlier producer edge"
        );
        assert!(
            cpg.graph
                .edges_directed(later, petgraph::Direction::Incoming)
                .all(|edge| !matches!(edge.weight(), CpgEdge::DataFlow(_))),
            "O01/{ext}: no synthetic incoming caller edge"
        );
        assert_eq!(edge_labels(later, param), [FlowConfidence::Exact]);
        assert!(edge_labels(earlier, param).is_empty());
        assert_eq!(
            edge_labels(param, body),
            [FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)]
        );
        let reachable = cpg.reachable_forward(later, &|edge| matches!(edge, CpgEdge::DataFlow(_)));
        assert!(reachable.contains(&param) && reachable.contains(&body));

        let origin = &files[&format!("origin.{ext}")];
        let item = origin
            .all_functions()
            .into_iter()
            .find(|node| {
                origin
                    .language
                    .function_name(node)
                    .is_some_and(|name| origin.node_text(&name) == "item")
            })
            .unwrap();
        assert_eq!(
            origin.function_parameter_slot_occurrences(&item),
            Some(vec![("input".to_string(), 21, 26)]),
            "O01/{ext}: parameter ordinal 0/token"
        );
        assert_eq!(&origin.source[21..26], "input");
        assert_eq!(&files[&file].source[anchors[2].0..anchors[2].1], "value");

        let contains_count = cpg
            .graph
            .edges_directed(later, petgraph::Direction::Incoming)
            .filter(|edge| matches!(edge.weight(), CpgEdge::Contains))
            .count();
        assert_eq!(contains_count, 1, "O01/{ext}: later Contains multiplicity");
        assert_eq!(
            cpg.location_index
                .get(&(file.clone(), 2))
                .into_iter()
                .flatten()
                .filter(|&&idx| idx == later)
                .count(),
            1,
            "O01/{ext}: later location registration multiplicity"
        );
    }

    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o02_two_calls_keep_distinct_argument_nodes() {
    let app =
        "import {item} from './origin';\nfunction outer(value){item(value);return item(value);}";
    let anchors = value_spans(app);
    assert_eq!(anchors.len(), 3);
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let file = format!("app.{ext}");
        let cpg = CodePropertyGraph::build(&fixture(language, ext, app));
        let rows = boundary_rows(&cpg, &file, "item");
        let expected: Vec<_> = [anchors[1], anchors[2]]
            .into_iter()
            .map(|(start, end)| {
                format!(
                    "{file}:outer:2:2:value:Use:{start}-{end}->origin.{ext}:item:1:1:input:Def:21-26:Exact"
                )
            })
            .collect();
        println!("O02/{ext} anchors={anchors:?} expected={expected:?} actual={rows:?}");
        if rows != expected {
            failures.push(format!("O02/{ext}: expected {expected:?}, got {rows:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o03_repeated_argument_slots_do_not_cross() {
    const PAIR: &str = "export function pair(left,right){sink(left);return right;}";
    let app = "import {pair} from './origin';\nfunction outer(value){return pair(value,value);}";
    let anchors = value_spans(app);
    assert_eq!(anchors.len(), 3);
    let left = PAIR.find("left").unwrap();
    let right = PAIR.find("right").unwrap();
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let file = format!("app.{ext}");
        let cpg = CodePropertyGraph::build(&fixture_with_origin(language, ext, app, PAIR));
        let rows = boundary_rows(&cpg, &file, "pair");
        let expected = vec![
            format!(
                "{file}:outer:2:2:value:Use:{}-{}->origin.{ext}:pair:1:1:left:Def:{left}-{}:Exact",
                anchors[1].0,
                anchors[1].1,
                left + "left".len()
            ),
            format!(
                "{file}:outer:2:2:value:Use:{}-{}->origin.{ext}:pair:1:1:right:Def:{right}-{}:Exact",
                anchors[2].0,
                anchors[2].1,
                right + "right".len()
            ),
        ];
        println!("O03/{ext} anchors={anchors:?} expected={expected:?} actual={rows:?}");
        if rows != expected {
            failures.push(format!("O03/{ext}: expected {expected:?}, got {rows:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o04_multiline_unicode_comment_preserves_slots() {
    const PAIR: &str = "export function pair(left,right){sink(left);return right;}";
    let app = "import {pair} from './origin';\nfunction outer(value){const 雪='λ';sink(value);return pair(value,\n/*π*/ value);}";
    let anchors = value_spans(app);
    assert_eq!(anchors.len(), 4);
    let left = PAIR.find("left").unwrap();
    let right = PAIR.find("right").unwrap();
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let file = format!("app.{ext}");
        let cpg = CodePropertyGraph::build(&fixture_with_origin(language, ext, app, PAIR));
        let rows = boundary_rows(&cpg, &file, "pair");
        let expected = vec![
            format!(
                "{file}:outer:2:2:value:Use:{}-{}->origin.{ext}:pair:1:1:left:Def:{left}-{}:Exact",
                anchors[2].0,
                anchors[2].1,
                left + "left".len()
            ),
            format!(
                "{file}:outer:2:3:value:Use:{}-{}->origin.{ext}:pair:1:1:right:Def:{right}-{}:Exact",
                anchors[3].0,
                anchors[3].1,
                right + "right".len()
            ),
        ];
        println!("O04/{ext} anchors={anchors:?} expected={expected:?} actual={rows:?}");
        if rows != expected {
            failures.push(format!("O04/{ext}: expected {expected:?}, got {rows:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o05_distinct_defs_keep_actual_step4_endpoints() {
    let source = "function flow(a){\n let x=a; x=source();\n sink(x);\n}";
    let x_spans: Vec<_> = source
        .match_indices('x')
        .map(|(start, text)| (start, start + text.len()))
        .collect();
    assert_eq!(x_spans.len(), 3);
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let file = format!("flow.{ext}");
        let parsed = ParsedFile::parse(&file, source, language).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "O05/{ext}");
        let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.clone(), parsed)]));
        let mut dfg_defs: Vec<_> = cpg
            .dfg
            .defs
            .values()
            .flatten()
            .filter(|loc| loc.file == file && loc.path == AccessPath::simple("x"))
            .map(|loc| (loc.start_byte, loc.end_byte))
            .collect();
        dfg_defs.sort();
        let mut dfg_uses: Vec<_> = cpg
            .dfg
            .uses
            .values()
            .flatten()
            .filter(|loc| loc.file == file && loc.path == AccessPath::simple("x"))
            .map(|loc| (loc.start_byte, loc.end_byte))
            .collect();
        dfg_uses.sort();
        let mut dfg_edges: Vec<_> = cpg
            .dfg
            .edges
            .iter()
            .filter(|edge| {
                edge.from.file == file
                    && edge.to.file == file
                    && edge.from.path == AccessPath::simple("x")
                    && edge.to.path == AccessPath::simple("x")
            })
            .map(|edge| {
                let label = cpg
                    .dfg
                    .labels
                    .get(&(edge.from.clone(), edge.to.clone()))
                    .copied();
                format!(
                    "{}-{}->{}-{}:{label:?}",
                    edge.from.start_byte, edge.from.end_byte, edge.to.start_byte, edge.to.end_byte
                )
            })
            .collect();
        dfg_edges.sort();
        let mut graph_defs = Vec::new();
        let mut graph_uses = Vec::new();
        for idx in cpg.graph.node_indices() {
            if let CpgNode::Variable {
                file: actual_file,
                function,
                path,
                access,
                start_byte,
                end_byte,
                ..
            } = &cpg.graph[idx]
            {
                if actual_file == &file && function == "flow" && path == &AccessPath::simple("x") {
                    match access {
                        VarAccess::Def => graph_defs.push((*start_byte, *end_byte)),
                        VarAccess::Use => graph_uses.push((*start_byte, *end_byte)),
                    }
                }
            }
        }
        graph_defs.sort();
        graph_uses.sort();
        let graph_edges = graph_path_rows(&cpg, &file, "x");
        println!("O05/{ext} anchors={x_spans:?} dfg_defs={dfg_defs:?} dfg_uses={dfg_uses:?} dfg_edges={dfg_edges:?} graph_defs={graph_defs:?} graph_uses={graph_uses:?} graph_edges={graph_edges:?}");
        let expected_defs = vec![x_spans[0], x_spans[1]];
        let expected_uses = vec![x_spans[2]];
        if dfg_defs != expected_defs || dfg_uses != expected_uses {
            failures.push(format!(
                "O05/{ext}: producer defs/uses {dfg_defs:?}/{dfg_uses:?}"
            ));
        }
        if graph_defs != expected_defs || graph_uses != expected_uses {
            failures.push(format!("O05/{ext}: CPG defs/uses expected {expected_defs:?}/{expected_uses:?}, got {graph_defs:?}/{graph_uses:?}"));
        }
        let expected_edge = vec![format!(
            "Def:{}-{}->Use:{}-{}:NameOnly(SameLine)",
            x_spans[0].0, x_spans[0].1, x_spans[2].0, x_spans[2].1
        )];
        if graph_edges != expected_edge {
            failures.push(format!(
                "O05/{ext}: actual Step4 endpoint/label expected {expected_edge:?}, got {graph_edges:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o06_repeated_member_keeps_field_and_base_per_slot() {
    const TAKE: &str = "export function take(left,right){sink(left);return right;}";
    let app = "import {take} from './origin';\nfunction outer(runtime){return take(runtime.X,runtime.X);}";
    let member_spans: Vec<_> = app
        .match_indices("runtime.X")
        .map(|(start, text)| (start, start + text.len()))
        .collect();
    assert_eq!(member_spans.len(), 2);
    let left = TAKE.find("left").unwrap();
    let right = TAKE.find("right").unwrap();
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let file = format!("app.{ext}");
        let cpg = CodePropertyGraph::build(&fixture_with_origin(language, ext, app, TAKE));
        let rows = boundary_rows(&cpg, &file, "take");
        let mut expected = vec![
            format!("{file}:outer:2:2:runtime:Use:{}-{}->origin.{ext}:take:1:1:left:Def:{left}-{}:Exact", member_spans[0].0, member_spans[0].0 + "runtime".len(), left + "left".len()),
            format!("{file}:outer:2:2:runtime.X:Use:{}-{}->origin.{ext}:take:1:1:left:Def:{left}-{}:Exact", member_spans[0].0, member_spans[0].1, left + "left".len()),
            format!("{file}:outer:2:2:runtime:Use:{}-{}->origin.{ext}:take:1:1:right:Def:{right}-{}:Exact", member_spans[1].0, member_spans[1].0 + "runtime".len(), right + "right".len()),
            format!("{file}:outer:2:2:runtime.X:Use:{}-{}->origin.{ext}:take:1:1:right:Def:{right}-{}:Exact", member_spans[1].0, member_spans[1].1, right + "right".len()),
        ];
        expected.sort();
        println!("O06/{ext} spans={member_spans:?} expected={expected:?} actual={rows:?}");
        if rows != expected {
            failures.push(format!("O06/{ext}: expected {expected:?}, got {rows:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o07_exact_lookup_rejects_every_forged_field() {
    let app =
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}";
    let files = fixture(Language::JavaScript, "js", app);
    let cpg = CodePropertyGraph::build(&files);
    let (caller, parsed, arg) = call_argument_context(&cpg, &files, "app.js", "item");
    let path = AccessPath::simple("value");
    let selected = CodePropertyGraph::argument_var_node_in_span(
        &caller,
        parsed,
        &path,
        &arg,
        &cpg.var_index,
        &cpg.graph,
    )
    .expect("control exact later occurrence");

    let original = cpg.graph[selected].clone();
    let original_exact = CodePropertyGraph::exact_var_index_from_graph(&cpg.graph);
    let mut failures = Vec::new();
    for field in [
        "file",
        "function",
        "function_start_line",
        "line",
        "path",
        "access",
        "start_byte",
        "end_byte",
        "zero_range",
        "inverted_range",
        "out_of_span",
    ] {
        let mut graph = cpg.graph.clone();
        let CpgNode::Variable {
            file,
            function,
            function_start_line,
            line,
            path,
            access,
            start_byte,
            end_byte,
        } = &mut graph[selected]
        else {
            panic!("control is not a variable")
        };
        match field {
            "file" => file.push_str(".wrong"),
            "function" => function.push_str("_wrong"),
            "function_start_line" => *function_start_line += 1,
            "line" => *line += 1,
            "path" => *path = AccessPath::simple("wrong"),
            "access" => *access = VarAccess::Def,
            "start_byte" => *start_byte += 1,
            "end_byte" => *end_byte -= 1,
            "zero_range" => *end_byte = *start_byte,
            "inverted_range" => *start_byte = *end_byte + 1,
            "out_of_span" => {
                *start_byte = arg.end + 1;
                *end_byte = arg.end + 6;
            }
            _ => unreachable!(),
        }
        let result = CodePropertyGraph::argument_var_node_in_span_with_exact(
            &caller,
            parsed,
            &AccessPath::simple("value"),
            &arg,
            &cpg.var_index,
            &original_exact,
            &graph,
        );
        if result.is_some() {
            failures.push(format!("{field}: selected {result:?}"));
        }
    }

    let CpgNode::Variable {
        file,
        function,
        function_start_line,
        line,
        path,
        access,
        start_byte,
        end_byte,
    } = original
    else {
        unreachable!()
    };
    let key = (file, function, function_start_line, line, path, access);
    let mut graph = cpg.graph.clone();
    if let CpgNode::Variable { function, .. } = &mut graph[selected] {
        function.push_str("_forged");
    }
    let forged = BTreeMap::from([(
        key,
        BTreeMap::from([((start_byte, end_byte), vec![selected])]),
    )]);
    let forged_result = CodePropertyGraph::argument_var_node_in_span_with_exact(
        &caller,
        parsed,
        &AccessPath::simple("value"),
        &arg,
        &cpg.var_index,
        &forged,
        &graph,
    );
    if forged_result.is_some() {
        failures.push(format!("forged exact key selected {forged_result:?}"));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o08_multiple_uses_refuse_without_def_fallback() {
    let app = "import {item} from './origin';\nfunction outer(value){return item(value + value);}";
    let files = fixture(Language::JavaScript, "js", app);
    let cpg = CodePropertyGraph::build(&files);
    let (caller, parsed, arg) = call_argument_context(&cpg, &files, "app.js", "item");
    let path = AccessPath::simple("value");
    let uses: Vec<_> = cpg
        .graph
        .node_indices()
        .filter(|&idx| {
            matches!(
                &cpg.graph[idx],
                CpgNode::Variable {
                    file,
                    function,
                    path: actual_path,
                    access: VarAccess::Use,
                    start_byte,
                    end_byte,
                    ..
                } if file == "app.js" && function == "outer" && actual_path == &path
                    && arg.start <= *start_byte && *end_byte <= arg.end
            )
        })
        .collect();
    assert_eq!(uses.len(), 2, "real syntax must produce two eligible Uses");
    assert!(CodePropertyGraph::argument_var_node_in_span(
        &caller,
        parsed,
        &path,
        &arg,
        &cpg.var_index,
        &cpg.graph,
    )
    .is_none());

    let mut graph = cpg.graph.clone();
    let def = graph.add_node(CpgNode::Variable {
        path: path.clone(),
        file: "app.js".into(),
        function: "outer".into(),
        function_start_line: caller.start_line,
        line: parsed.line_for_byte(arg.start),
        access: VarAccess::Def,
        start_byte: arg.start,
        end_byte: arg.end,
    });
    let mut exact = CodePropertyGraph::exact_var_index_from_graph(&graph);
    assert!(
        CodePropertyGraph::argument_var_node_in_span_with_exact(
            &caller,
            parsed,
            &path,
            &arg,
            &cpg.var_index,
            &exact,
            &graph,
        )
        .is_none(),
        "ambiguous Uses must not fall back to Def {def:?}"
    );

    let first = uses[0];
    let CpgNode::Variable {
        file,
        function,
        function_start_line,
        line,
        path,
        access,
        start_byte,
        end_byte,
    } = &graph[first]
    else {
        unreachable!()
    };
    exact = BTreeMap::from([(
        (
            file.clone(),
            function.clone(),
            *function_start_line,
            *line,
            path.clone(),
            *access,
        ),
        BTreeMap::from([((*start_byte, *end_byte), vec![first, first])]),
    )]);
    let one_span = *start_byte..*end_byte;
    assert_eq!(
        CodePropertyGraph::argument_var_node_in_span_with_exact(
            &caller,
            parsed,
            &AccessPath::simple("value"),
            &one_span,
            &cpg.var_index,
            &exact,
            &graph,
        ),
        Some(first),
        "duplicate identical NodeIndex entries deduplicate"
    );
}

#[test]
fn same_line_occurrence_o09_existing_exact_step4_label_is_preserved() {
    let source = "function f() {\n  var value = source();\n  {\n    let value = clean();\n  }\n  sink(value);\n}\n";
    let file = "labels.js".to_string();
    let parsed = ParsedFile::parse(&file, source, Language::JavaScript).unwrap();
    assert!(!parsed.tree.root_node().has_error());
    let files = BTreeMap::from([(file.clone(), parsed)]);
    let cpg = CodePropertyGraph::build(&files);
    let edge = cpg
        .dfg
        .edges
        .iter()
        .find(|edge| {
            edge.from.file == file
                && edge.from.function == "f"
                && edge.from.path == AccessPath::simple("value")
                && edge.from.line == 2
                && edge.to.path == AccessPath::simple("value")
                && edge.to.line == 6
        })
        .expect("authenticated existing-Exact producer edge");
    let label = cpg
        .dfg
        .labels
        .get(&(edge.from.clone(), edge.to.clone()))
        .copied();
    assert_eq!(label, Some(FlowConfidence::Exact));
    let exact = CodePropertyGraph::exact_var_index_from_graph(&cpg.graph);
    let from = CodePropertyGraph::dfg_endpoint_node(
        &edge.from,
        VarAccess::Def,
        &files,
        &cpg.var_index,
        &exact,
        &cpg.graph,
    )
    .unwrap();
    let to = CodePropertyGraph::dfg_endpoint_node(
        &edge.to,
        VarAccess::Use,
        &files,
        &cpg.var_index,
        &exact,
        &cpg.graph,
    )
    .unwrap();
    assert_eq!(
        cpg.graph
            .edges_connecting(from, to)
            .filter_map(|e| match *e.weight() {
                CpgEdge::DataFlow(label) => Some(label),
                _ => None,
            })
            .collect::<Vec<_>>(),
        [FlowConfidence::Exact]
    );
}

#[test]
fn same_line_occurrence_o10_slot_barriers_do_not_compress_arguments() {
    let cases = [
        (
            "default-hole",
            "export function take(first=seed(),second){return second;}",
            "import {take} from './origin';\nfunction outer(value){return take(value,value);}",
            1usize,
        ),
        (
            "destructuring-barrier",
            "export function take({first},second){return second;}",
            "import {take} from './origin';\nfunction outer(value){return take(value,value);}",
            0,
        ),
        (
            "rest-barrier",
            "export function take(...items){return items;}",
            "import {take} from './origin';\nfunction outer(value){return take(value);}",
            0,
        ),
        (
            "unsupported-computed-member",
            "export function take(input){return input;}",
            "import {take} from './origin';\nfunction outer(value,key){return take(value[key]);}",
            1,
        ),
    ];
    let mut failures = Vec::new();
    for (case, origin, app, expected_edges) in cases {
        for (language, ext) in [
            (Language::JavaScript, "js"),
            (Language::TypeScript, "ts"),
            (Language::Tsx, "tsx"),
        ] {
            let cpg = CodePropertyGraph::build(&fixture_with_origin(language, ext, app, origin));
            let rows = boundary_rows(&cpg, &format!("app.{ext}"), "take");
            if rows.len() != expected_edges {
                failures.push(format!(
                    "O10/{ext}/{case}: expected {expected_edges}, got {rows:?}"
                ));
            }
            if case == "default-hole" && rows.iter().any(|row| row.contains(":first:Def:")) {
                failures.push(format!(
                    "O10/{ext}/{case}: compressed into first slot {rows:?}"
                ));
            }
            if case == "unsupported-computed-member"
                && (rows.len() != 1 || !rows[0].contains(":value:Use:"))
            {
                failures.push(format!(
                    "O10/{ext}/{case}: expected established base-only supplementation, got {rows:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o12_same_name_same_line_owner_collision_refuses() {
    let app = "import {item} from './origin';\nfunction outer(value){function inner(value){return item(value);} return item(value);}";
    let mut failures = Vec::new();
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let cpg = CodePropertyGraph::build(&fixture(language, ext, app));
        let rows = boundary_rows(&cpg, &format!("app.{ext}"), "item");
        let later = value_spans(app).last().copied().unwrap();
        if rows.len() != 1
            || !rows[0].contains(":outer:2:2:value:Use:")
            || !rows[0].contains(&format!(":{}-{}->", later.0, later.1))
        {
            failures.push(format!(
                "O12/{ext}: expected only outer call's own occurrence, got {rows:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_occurrence_o11_zero_width_rows_never_become_exact_candidates() {
    let app = "import {item} from './origin';\nfunction outer(value){return item(value);}";
    let files = fixture(Language::JavaScript, "js", app);
    let cpg = CodePropertyGraph::build(&files);
    let (caller, parsed, arg) = call_argument_context(&cpg, &files, "app.js", "item");
    let path = AccessPath::simple("value");
    let mut graph = DiGraph::new();
    let zero_use = graph.add_node(CpgNode::Variable {
        path: path.clone(),
        file: "app.js".into(),
        function: "outer".into(),
        function_start_line: caller.start_line,
        line: parsed.line_for_byte(arg.start),
        access: VarAccess::Use,
        start_byte: 0,
        end_byte: 0,
    });
    let legacy = BTreeMap::from([(
        (
            "app.js".to_string(),
            "outer".to_string(),
            caller.start_line,
            parsed.line_for_byte(arg.start),
            path.clone(),
            VarAccess::Use,
        ),
        zero_use,
    )]);
    let exact = CodePropertyGraph::exact_var_index_from_graph(&graph);
    assert!(
        exact.is_empty(),
        "zero-width occurrence entered exact index"
    );
    assert!(CodePropertyGraph::argument_var_node_in_span_with_exact(
        &caller, parsed, &path, &arg, &legacy, &exact, &graph,
    )
    .is_none());
}

#[test]
fn same_line_occurrence_o13_full_parallel_incremental_and_warm_cache_match() {
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};

    let app =
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}";
    let files = fixture(Language::JavaScript, "js", app);
    let full = CodePropertyGraph::build(&files);
    let expected = complete_graph_rows(&full);
    let started = std::time::Instant::now();
    let mut exact = BTreeMap::new();
    for _ in 0..1_000 {
        exact = std::hint::black_box(CodePropertyGraph::exact_var_index_from_graph(&full.graph));
    }
    let exact_spans: usize = exact.values().map(BTreeMap::len).sum();
    let exact_nodes: usize = exact
        .values()
        .flat_map(BTreeMap::values)
        .map(Vec::len)
        .sum();
    println!(
        "O13 index_micro_1000={:?} keys={} spans={} node_refs={} graph_nodes={} graph_edges={}",
        started.elapsed(),
        exact.len(),
        exact_spans,
        exact_nodes,
        full.node_count(),
        full.edge_count()
    );

    let parallel = CodePropertyGraph::collect_step5b_edges(
        &full.call_graph,
        &full.var_index,
        &full.graph,
        &files,
    );
    let serial = CodePropertyGraph::collect_step5b_edges_reference(
        &full.call_graph,
        &full.var_index,
        &full.graph,
        &files,
    );
    assert_eq!(parallel, serial, "O13 Step5b parallel/reference");

    let one = fixture(
        Language::JavaScript,
        "js",
        "import {item} from './origin';\nfunction outer(value){return item(value);}",
    );
    let before = CodePropertyGraph::build(&one);
    let incremental = CodePropertyGraph::build_incremental(
        before.call_graph.clone(),
        before.dfg.clone(),
        &BTreeSet::from(["app.js".to_string()]),
        &files,
        None,
    );
    assert_eq!(
        complete_graph_rows(&incremental),
        expected,
        "O13 incremental/full"
    );

    let sources: BTreeMap<_, _> = files
        .iter()
        .map(|(path, parsed)| (path.clone(), parsed.source.clone()))
        .collect();
    let hashes = compute_file_hashes(&sources);
    let cache = tempfile::tempdir().unwrap();
    save_cache(&full, &hashes, false, cache.path()).unwrap();
    println!(
        "O13 cache_meta={}",
        std::fs::read_to_string(cache.path().join("cache-meta.json")).unwrap()
    );
    let warm = match load_cache(&hashes, false, cache.path()) {
        CacheResult::Hit(cpg) => cpg,
        other => panic!(
            "O13 expected warm Hit, got {}",
            match other {
                CacheResult::Hit(_) => unreachable!(),
                CacheResult::PartialHit { .. } => "PartialHit",
                CacheResult::Miss => "Miss",
            }
        ),
    };
    assert_eq!(complete_graph_rows(&warm), expected, "O13 warm/full");

    for (label, graph) in [
        ("full", &full),
        ("incremental", &incremental),
        ("warm", &warm),
    ] {
        let legacy = graph
            .var_node(
                "app.js",
                "outer",
                2,
                2,
                &AccessPath::simple("value"),
                VarAccess::Use,
            )
            .unwrap();
        assert!(
            matches!(
                graph.node(legacy),
                CpgNode::Variable {
                    start_byte: 58,
                    end_byte: 63,
                    ..
                }
            ),
            "O13/{label}: legacy representative is not first-wins"
        );
        let later: Vec<_> = graph
            .nodes_at("app.js", 2)
            .into_iter()
            .filter(|&idx| {
                matches!(
                    graph.node(idx),
                    CpgNode::Variable {
                        function,
                        path,
                        access: VarAccess::Use,
                        start_byte: 77,
                        end_byte: 82,
                        ..
                    } if function == "outer" && path == &AccessPath::simple("value")
                )
            })
            .collect();
        assert_eq!(later.len(), 1, "O13/{label}: later location multiplicity");
        assert_eq!(
            graph
                .graph
                .edges_directed(later[0], petgraph::Direction::Incoming)
                .filter(|edge| matches!(edge.weight(), CpgEdge::Contains))
                .count(),
            1,
            "O13/{label}: later Contains multiplicity"
        );
    }
}

#[test]
fn same_line_occurrence_o14_caller_source_epochs_match_fresh_builds() {
    let variants = [
        "import {item} from './origin';\nfunction outer(value){return item(value);}",
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}",
        "import {item} from './origin';\nfunction outer(value){return item(value)+sink(value);}",
        "import {item} from './origin';\nfunction outer(value){sink(value);\nreturn item(value);}",
        "import {item} from './origin';\nfunction outer(value){sink(value);return item(value);}",
    ];
    let mut files = fixture(Language::JavaScript, "js", variants[0]);
    let mut prior = CodePropertyGraph::build(&files);
    for (epoch, app) in variants.iter().enumerate().skip(1) {
        files = fixture(Language::JavaScript, "js", app);
        let incremental = CodePropertyGraph::build_incremental(
            prior.call_graph.clone(),
            prior.dfg.clone(),
            &BTreeSet::from(["app.js".to_string()]),
            &files,
            None,
        );
        let fresh = CodePropertyGraph::build(&files);
        assert_eq!(
            complete_graph_rows(&incremental),
            complete_graph_rows(&fresh),
            "O14 epoch {epoch}: incremental/fresh"
        );
        let (_caller, parsed, arg) = call_argument_context(&incremental, &files, "app.js", "item");
        let rows = boundary_rows(&incremental, "app.js", "item");
        assert_eq!(rows.len(), 1, "O14 epoch {epoch}: boundary rows {rows:?}");
        assert!(
            rows[0].contains(&format!(":{}-{}->", arg.start, arg.end)),
            "O14 epoch {epoch}: stale argument boundary {rows:?} vs {arg:?}"
        );
        let mut expected_uses: Vec<_> = incremental
            .dfg
            .uses
            .values()
            .flatten()
            .filter(|loc| {
                loc.file == "app.js"
                    && loc.function == "outer"
                    && loc.path == AccessPath::simple("value")
                    && loc.kind == VarAccessKind::Use
            })
            .map(|loc| (loc.start_byte, loc.end_byte))
            .filter(|(start, end)| start < end)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        expected_uses.sort();
        let mut actual_uses: Vec<_> = incremental
            .graph
            .node_indices()
            .filter_map(|idx| match incremental.node(idx) {
                CpgNode::Variable {
                    file,
                    function,
                    path,
                    access: VarAccess::Use,
                    start_byte,
                    end_byte,
                    ..
                } if file == "app.js"
                    && function == "outer"
                    && path == &AccessPath::simple("value") =>
                {
                    Some((*start_byte, *end_byte))
                }
                _ => None,
            })
            .collect();
        actual_uses.sort();
        assert_eq!(
            actual_uses, expected_uses,
            "O14 epoch {epoch}: stale/missing occurrence"
        );
        assert_eq!(&parsed.source[arg.clone()], "value");
        prior = incremental;
    }
}
