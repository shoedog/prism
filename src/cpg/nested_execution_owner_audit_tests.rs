use super::*;
use crate::algorithms::full_flow;
use crate::cpg::CpgContext;
use crate::data_flow::DataFlowGraph;
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;
use crate::nested_execution_owner_audit::{source_sha256, GraphObservation};
use crate::slice::{SliceConfig, SlicingAlgorithm};

fn parsed_files(app: &str) -> BTreeMap<String, ParsedFile> {
    [
        (
            "app.js".to_string(),
            ParsedFile::parse("app.js", app, Language::JavaScript).unwrap(),
        ),
        (
            "origin.js".to_string(),
            ParsedFile::parse(
                "origin.js",
                "export function item(input) { return input; }",
                Language::JavaScript,
            )
            .unwrap(),
        ),
    ]
    .into_iter()
    .collect()
}

fn use_rows(cpg: &CodePropertyGraph, file: &str) -> Vec<GraphObservation> {
    let mut rows: Vec<_> = cpg
        .dfg
        .uses
        .values()
        .flatten()
        .filter(|loc| loc.file == file)
        .map(|loc| GraphObservation {
            file: loc.file.clone(),
            function: loc.function.clone(),
            function_start_line: loc.function_start_line,
            path: loc.path.to_string(),
            access: "Use".to_string(),
            line: loc.line,
            start_byte: loc.start_byte,
            end_byte: loc.end_byte,
            confidence_or_refusal: "occurrence".to_string(),
        })
        .collect();
    rows.sort();
    rows
}

fn semantic_dfg_rows(cpg: &CodePropertyGraph, file: &str) -> Vec<String> {
    let mut rows: Vec<_> = use_rows(cpg, file)
        .into_iter()
        .map(|r| {
            format!(
                "U|{}|{}|{}|{}|{}|{}-{}",
                r.file, r.function, r.function_start_line, r.path, r.line, r.start_byte, r.end_byte
            )
        })
        .collect();
    rows.extend(
        cpg.dfg
            .edges
            .iter()
            .filter(|edge| edge.from.file == file)
            .map(|edge| {
                let label = cpg
                    .dfg
                    .labels
                    .get(&(edge.from.clone(), edge.to.clone()))
                    .copied();
                format!(
                    "E|{}:{}:{}:{}:{}-{}|{}:{}:{}:{}:{}-{}|{label:?}",
                    edge.from.function,
                    edge.from.function_start_line,
                    edge.from.path,
                    edge.from.line,
                    edge.from.start_byte,
                    edge.from.end_byte,
                    edge.to.function,
                    edge.to.function_start_line,
                    edge.to.path,
                    edge.to.line,
                    edge.to.start_byte,
                    edge.to.end_byte,
                )
            }),
    );
    rows.sort();
    rows
}

fn resolution_rows(cpg: &CodePropertyGraph, file: &str) -> Vec<String> {
    let mut rows = Vec::new();
    for sites in cpg.call_graph.calls.values() {
        for site in sites.iter().filter(|site| site.caller.file == file) {
            for resolved in cpg.call_graph.resolve_call_site(site) {
                rows.push(format!(
                    "{}:{}:{}:{}-{}->{:?}:{:?}:{}:{}:{}",
                    site.caller.name,
                    site.caller.start_line,
                    site.callee_name,
                    site.start_byte,
                    site.end_byte,
                    resolved.kind,
                    resolved.confidence,
                    resolved.target.file,
                    resolved.target.name,
                    resolved.target.start_line
                ));
            }
        }
    }
    rows.sort();
    rows
}

fn complete_site_rows(cg: &crate::call_graph::CallGraph, file: &str) -> Vec<String> {
    let mut rows: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|site| site.caller.file == file)
        .map(|site| {
            format!(
                "{}:{}:{}:{}:{}-{}:{:?}:src={:?}:q={:?}:bound={}:type={:?}:owner={:?}:shadow={}:recovery={:?}:materialized={}:new={}:argc={:?}:spread={}:outcome={:?}:origin={:?}:target={:?}",
                site.caller.name,
                site.caller.start_line,
                site.callee_name,
                site.line,
                site.start_byte,
                site.end_byte,
                site.kind,
                site.source_callee_name,
                site.qualifier,
                site.receiver_lexically_bound,
                site.receiver_type,
                site.receiver_owner_identity,
                site.receiver_local_type_shadowed,
                site.receiver_recovery,
                site.receiver_materialized,
                site.receiver_newly_recovered,
                site.arg_count,
                site.arg_spread,
                site.receiver_outcome,
                site.origin,
                site.pre_resolved_target,
            )
        })
        .collect();
    rows.sort();
    rows
}

fn cpg_endpoint(node: &CpgNode) -> String {
    match node {
        CpgNode::Function {
            file,
            name,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } => format!("F:{file}:{name}:{start_line}-{end_line}:{start_byte}-{end_byte}"),
        CpgNode::Statement {
            file,
            line,
            kind,
            start_byte,
            end_byte,
        } => format!("S:{file}:{line}:{kind:?}:{start_byte}-{end_byte}"),
        CpgNode::Variable {
            file,
            function,
            function_start_line,
            line,
            path,
            access,
            start_byte,
            end_byte,
        } => format!(
            "V:{file}:{function}:{function_start_line}:{line}:{path}:{access:?}:{start_byte}-{end_byte}"
        ),
        CpgNode::ReturnValue {
            file,
            function,
            function_start_line,
            line,
            return_start_byte,
            return_end_byte,
            child_slot,
            start_byte,
            end_byte,
        } => format!(
            "R:{file}:{function}:{function_start_line}:{line}:{return_start_byte}-{return_end_byte}:{child_slot}:{start_byte}-{end_byte}"
        ),
    }
}

fn ownership_edge_rows(cpg: &CodePropertyGraph) -> Vec<String> {
    let mut rows = Vec::new();
    for edge in cpg.graph.edge_indices() {
        let weight = &cpg.graph[edge];
        if !matches!(
            weight,
            CpgEdge::Call(_) | CpgEdge::Return(_) | CpgEdge::DataFlow(_)
        ) {
            continue;
        }
        let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
        rows.push(format!(
            "{weight:?}|{}|{}",
            cpg_endpoint(cpg.node(from)),
            cpg_endpoint(cpg.node(to))
        ));
    }
    rows.sort();
    rows
}

#[test]
fn nested_execution_owner_graph_classification() {
    let source = "function outer(seed) {\n  let cb;\n  cb=function inner(p) {\n    sink(p);\n    return seed;\n  };\n  let local=seed;\n  return local;\n}\n";
    let files = parsed_files(source);
    let cpg = CodePropertyGraph::build(&files);
    let rows = use_rows(&cpg, "app.js");
    for row in &rows {
        println!("OWNER_GRAPH {row:?}");
    }
    let parameter = source.find("p)").unwrap();
    let nested_read = source.find("p);").unwrap();
    let exact_uses: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row.function.as_str(),
                row.path.as_str(),
                row.line,
                row.start_byte,
                row.end_byte,
            )
        })
        .collect();
    assert_eq!(
        exact_uses,
        vec![
            ("inner", "p", 4, nested_read, nested_read + 1),
            (
                "inner",
                "seed",
                5,
                source.find("seed;").unwrap(),
                source.find("seed;").unwrap() + 4
            ),
            (
                "inner",
                "sink",
                4,
                source.find("sink(").unwrap(),
                source.find("sink(").unwrap() + 4
            ),
            ("outer", "cb", 2, 23, 23),
            ("outer", "cb", 3, 33, 33),
            (
                "outer",
                "local",
                8,
                source.rfind("local;").unwrap(),
                source.rfind("local;").unwrap() + 5
            ),
            ("outer", "seed", 1, 0, 0),
            (
                "outer",
                "seed",
                5,
                source.find("    return seed;").unwrap(),
                source.find("    return seed;").unwrap()
            ),
            (
                "outer",
                "seed",
                7,
                source.find("  let local=seed;").unwrap(),
                source.find("  let local=seed;").unwrap()
            ),
        ]
    );
    assert!(!rows.iter().any(|row| {
        row.function == "outer"
            && row.start_byte < row.end_byte
            && matches!(row.path.as_str(), "p" | "sink")
    }));
    assert!(!rows
        .iter()
        .any(|row| row.function == "outer" && row.start_byte == parameter));
    let zero_width: Vec<_> = rows
        .iter()
        .filter(|row| row.function == "outer" && row.start_byte == row.end_byte)
        .map(|row| (row.path.as_str(), row.line, row.start_byte))
        .collect();
    assert_eq!(
        zero_width,
        vec![
            ("cb", 2, 23),
            ("cb", 3, 33),
            ("seed", 1, 0),
            ("seed", 5, 71),
            ("seed", 7, 93)
        ]
    );
    let nested_seed_anchor = source.find("    return seed;").unwrap();
    let capture_labels: Vec<_> = cpg
        .dfg
        .labels
        .iter()
        .filter(|((from, to), _)| {
            from.file == "app.js"
                && from.function == "outer"
                && from.path.to_string() == "seed"
                && to.function == "outer"
                && to.line == 5
                && to.start_byte == nested_seed_anchor
                && to.end_byte == nested_seed_anchor
        })
        .map(|(_, label)| *label)
        .collect();
    assert_eq!(
        capture_labels,
        vec![
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
        ]
    );
    assert!(!capture_labels.contains(&FlowConfidence::Exact));
    assert!(!semantic_dfg_rows(&cpg, "app.js").is_empty());
}

#[test]
fn call_execution_owner_source_epochs() {
    let epochs = [
        "function outer(token){return item(token);}",
        "function outer(token){const cb=function inner(){return item(token);};return cb;}",
        "function outer(token){register(token,function inner(){return item(token);});}",
        "function outer(token){const cb=function inner(token){return item(token);};return cb;}",
        "function outer(token){return item(token);}",
    ];
    let expected_resolution = [
        vec!["outer:1:item:29-40->FreeSingle:Exact:origin.js:item:1"],
        vec!["inner:1:item:55-66->FreeSingle:Exact:origin.js:item:1"],
        vec!["inner:1:item:61-72->FreeSingle:Exact:origin.js:item:1"],
        vec!["inner:1:item:60-71->FreeSingle:Exact:origin.js:item:1"],
        vec!["outer:1:item:29-40->FreeSingle:Exact:origin.js:item:1"],
    ];
    let mut previous: Option<CodePropertyGraph> = None;
    let mut previous_source = String::new();
    for (epoch, source) in epochs.into_iter().enumerate() {
        let files = parsed_files(source);
        let full = CodePropertyGraph::build(&files);
        let full_rows = semantic_dfg_rows(&full, "app.js");
        let full_resolution = resolution_rows(&full, "app.js");
        assert_eq!(full_resolution, expected_resolution[epoch], "epoch {epoch}");
        println!(
            "OWNER_EPOCH epoch={epoch} sha={} rows={} resolution={:?}",
            source_sha256(source),
            full_rows.len(),
            full_resolution
        );
        let current = if let Some(old) = previous {
            assert_ne!(previous_source, source);
            let cached_cg = bincode::deserialize(
                &bincode::serialize(&old.call_graph).expect("serialize call graph"),
            )
            .expect("deserialize call graph");
            let cached_dfg =
                bincode::deserialize(&bincode::serialize(&old.dfg).expect("serialize DFG"))
                    .expect("deserialize DFG");
            let changed = BTreeSet::from(["app.js".to_string()]);
            let incremental =
                CodePropertyGraph::build_incremental(cached_cg, cached_dfg, &changed, &files, None);
            assert_eq!(
                semantic_dfg_rows(&incremental, "app.js"),
                full_rows,
                "epoch {epoch} DFG semantic rows"
            );
            assert_eq!(
                resolution_rows(&incremental, "app.js"),
                full_resolution,
                "epoch {epoch} call resolution"
            );
            incremental
        } else {
            full
        };
        previous = Some(current);
        previous_source = source.to_string();
    }
}

#[test]
fn call_execution_owner_complete_site_and_edge_metadata() {
    let source =
        "function outer(token){register(eager(),function inner(){return item(token);});direct();}";
    for (language, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let app = format!("app.{ext}");
        let origin = format!("origin.{ext}");
        let files = BTreeMap::from([
            (
                app.clone(),
                ParsedFile::parse(&app, source, language).unwrap(),
            ),
            (
                origin.clone(),
                ParsedFile::parse(
                    &origin,
                    "export function item(input){return input;}",
                    language,
                )
                .unwrap(),
            ),
        ]);
        let full = crate::call_graph::CallGraph::build(&files);
        let skeleton = crate::call_graph::CallGraph::build_skeleton(&files);
        let subset = crate::call_graph::CallGraph::build_direct_subset(
            &files,
            &files.keys().cloned().collect(),
        );
        let expected_row = |caller: &str, callee: &str, start, end, argc: &str| {
            format!(
                "{caller}:1:{callee}:1:{start}-{end}:Call:src=None:q=None:bound=false:type=None:owner=None:shadow=false:recovery=None:materialized=false:new=false:argc={argc}:spread=false:outcome=None:origin=Source:target=None"
            )
        };
        let expected_sites = vec![
            expected_row("inner", "item", 63, 74, "Some(1)"),
            expected_row("outer", "direct", 78, 86, "Some(0)"),
            expected_row("outer", "eager", 31, 38, "Some(0)"),
            expected_row("outer", "register", 22, 77, "Some(2)"),
        ];
        let expected_skeleton = vec![
            expected_row("inner", "item", 63, 74, "None"),
            expected_row("outer", "direct", 78, 86, "None"),
            expected_row("outer", "eager", 31, 38, "None"),
            expected_row("outer", "register", 22, 77, "None"),
        ];
        assert_eq!(
            complete_site_rows(&full, &app),
            expected_sites,
            "{ext}/full"
        );
        assert_eq!(
            complete_site_rows(&skeleton, &app),
            expected_skeleton,
            "{ext}/skeleton"
        );
        assert_eq!(
            complete_site_rows(&subset, &app),
            expected_sites,
            "{ext}/subset"
        );
        let roundtrip: crate::call_graph::CallGraph = bincode::deserialize(
            &bincode::serialize(&full).expect("serialize complete call graph"),
        )
        .expect("deserialize complete call graph");
        assert_eq!(
            complete_site_rows(&roundtrip, &app),
            expected_sites,
            "{ext}/bincode"
        );

        let cpg = CodePropertyGraph::build(&files);
        let resolved = resolution_rows(&cpg, &app);
        assert_eq!(
            resolved,
            vec![format!(
                "inner:1:item:63-74->FreeSingle:Exact:{origin}:item:1"
            )],
            "{ext}"
        );
        assert!(!expected_sites
            .iter()
            .any(|row| row.starts_with("outer:1:item:")));
        assert_eq!(
            ownership_edge_rows(&cpg),
            vec![
                format!("Call(Exact)|F:{app}:inner:1-1:39-76|F:{origin}:item:1-1:7-42"),
                format!("DataFlow(Exact)|V:{app}:inner:1:1:token:Use:68-73|V:{origin}:item:1:1:input:Def:21-26"),
                format!("DataFlow(NameOnly(CfgIncomplete))|V:{origin}:item:1:1:input:Def:21-26|V:{origin}:item:1:1:input:Use:35-40"),
                format!("Return(Exact)|F:{origin}:item:1-1:7-42|F:{app}:inner:1-1:39-76"),
            ],
            "{ext}/CPG edges"
        );
    }
}

fn item_site(cpg: &CodePropertyGraph) -> crate::call_graph::CallSite {
    cpg.call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.caller.file == "app.js" && site.callee_name == "item")
        .unwrap()
        .clone()
}

#[test]
fn nested_execution_owner_capture_and_refusal_controls() {
    let distinct = "import {item} from './origin';\nfunction outer(value){\n sink(value);\n return item(value);\n}";
    let files = parsed_files(distinct);
    let cpg = CodePropertyGraph::build(&files);
    let site = item_site(&cpg);
    let parsed = &files["app.js"];
    let arg = parsed
        .call_argument_texts_and_spans_at(site.start_byte, "item")
        .pop()
        .unwrap()
        .1;
    let selected = CodePropertyGraph::argument_var_node_in_span(
        &site.caller,
        parsed,
        &AccessPath::simple("value"),
        &arg,
        &cpg.var_index,
        &cpg.graph,
    )
    .expect("distinct-line exact argument Use");
    assert_eq!(&parsed.source[arg.clone()], "value");
    assert!(CodePropertyGraph::argument_var_node_in_span(
        &site.caller,
        parsed,
        &AccessPath::simple("wrongToken"),
        &arg,
        &cpg.var_index,
        &cpg.graph,
    )
    .is_none());
    let mut moved_graph = cpg.graph.clone();
    if let CpgNode::Variable { end_byte, .. } = &mut moved_graph[selected] {
        *end_byte = arg.end + 1;
    }
    assert!(CodePropertyGraph::argument_var_node_in_span(
        &site.caller,
        parsed,
        &AccessPath::simple("value"),
        &arg,
        &cpg.var_index,
        &moved_graph,
    )
    .is_none());

    let same_line =
        "import {item} from './origin'; function outer(value){sink(value);return item(value);}";
    let same_files = parsed_files(same_line);
    let same_cpg = CodePropertyGraph::build(&same_files);
    let same_site = item_site(&same_cpg);
    let same_parsed = &same_files["app.js"];
    let same_arg = same_parsed
        .call_argument_texts_and_spans_at(same_site.start_byte, "item")
        .pop()
        .unwrap()
        .1;
    let selected_same_line = CodePropertyGraph::argument_var_node_in_span(
        &same_site.caller,
        same_parsed,
        &AccessPath::simple("value"),
        &same_arg,
        &same_cpg.var_index,
        &same_cpg.graph,
    )
    .expect("byte-distinct same-line argument occurrence");
    assert!(matches!(
        &same_cpg.graph[selected_same_line],
        CpgNode::Variable {
            function,
            access: VarAccess::Use,
            start_byte,
            end_byte,
            ..
        } if function == "outer"
            && (*start_byte, *end_byte) == (same_arg.start, same_arg.end)
    ));

    let unsupported_files: BTreeMap<_, _> = [
        (
            "app.js".to_string(),
            ParsedFile::parse(
                "app.js",
                "function outer(value){return item(value);}",
                Language::JavaScript,
            )
            .unwrap(),
        ),
        (
            "origin.js".to_string(),
            ParsedFile::parse(
                "origin.js",
                "export function item(input=seed()) { return input; }",
                Language::JavaScript,
            )
            .unwrap(),
        ),
    ]
    .into_iter()
    .collect();
    let unsupported = CodePropertyGraph::build(&unsupported_files);
    let unsupported_site = item_site(&unsupported);
    let resolved = unsupported
        .call_graph
        .resolve_call_site(&unsupported_site)
        .pop()
        .unwrap();
    let unsupported_params = compute_param_def_nodes(
        &unsupported_files["origin.js"],
        resolved.target,
        &unsupported.var_index,
        &unsupported.graph,
    )
    .expect("the positional signature remains observable");
    assert_eq!(unsupported_params, vec![None]);
}

fn normalized_slice(result: &crate::slice::SliceResult) -> Vec<String> {
    let mut rows = Vec::new();
    for block in &result.blocks {
        for (file, lines) in &block.file_line_map {
            for (line, is_diff) in lines {
                rows.push(format!("{file}:{line}:{is_diff}"));
            }
        }
    }
    rows.sort();
    rows
}

#[test]
fn nested_execution_owner_full_flow_controls() {
    let source = "function outer(seed) {\n  let before=seed;\n  register(eager, function inner(p) {\n    sink(nestedRead);\n  });\n  return before;\n}\n";
    let files = BTreeMap::from([(
        "app.js".to_string(),
        ParsedFile::parse("app.js", source, Language::JavaScript).unwrap(),
    )]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "app.js".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let with = CpgContext::build(&files, None);
    let with_rows = normalized_slice(&full_flow::slice(&with, &diff, &config).unwrap());
    let mut without = CpgContext::build(&files, None);
    without.cpg.dfg = DataFlowGraph::empty();
    let without_rows = normalized_slice(&full_flow::slice(&without, &diff, &config).unwrap());
    println!("OWNER_FULL_FLOW with={with_rows:?} without={without_rows:?}");
    assert_eq!(with_rows, without_rows);
    assert_eq!(
        with_rows,
        vec!["app.js:3:true".to_string(), "app.js:5:false".to_string()]
    );
}

fn call_names_for(cg: &crate::call_graph::CallGraph, caller: &str) -> Vec<String> {
    let mut names: Vec<_> = cg
        .calls
        .iter()
        .filter(|(id, _)| id.name == caller)
        .flat_map(|(_, sites)| sites.iter().map(|site| site.callee_name.clone()))
        .collect();
    names.sort();
    names
}

#[test]
fn call_execution_owner_same_line_identity_collision_refuses_graph_calls() {
    let source =
        "const π='🙂';function outer(){function same(){one();}function same(){two();}direct();}";
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("collision", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let same_nodes: Vec<_> = parsed
            .all_functions()
            .into_iter()
            .filter(|node| {
                parsed.language.function_name(node).is_some_and(|name| {
                    parsed.node_text(&name) == "same" && parsed.node_line_range(node) == (1, 1)
                })
            })
            .collect();
        assert_eq!(
            same_nodes.len(),
            2,
            "{language:?}: raw owners must stay distinct"
        );
        let raw: Vec<Vec<_>> = same_nodes
            .iter()
            .map(|node| {
                parsed
                    .function_calls_with_spans_on_lines(
                        node,
                        &BTreeSet::from([1]),
                        &BTreeSet::new(),
                    )
                    .into_iter()
                    .map(|site| (site.callee_name, site.start_byte, site.end_byte))
                    .collect()
            })
            .collect();
        assert_eq!(
            raw,
            [
                vec![(
                    "one".to_string(),
                    source.find("one()").unwrap(),
                    source.find("one()").unwrap() + 5
                )],
                vec![(
                    "two".to_string(),
                    source.find("two()").unwrap(),
                    source.find("two()").unwrap() + 5
                )],
            ]
        );

        let files = BTreeMap::from([("collision".to_string(), parsed)]);
        let full = crate::call_graph::CallGraph::build(&files);
        let skeleton = crate::call_graph::CallGraph::build_skeleton(&files);
        let subset = crate::call_graph::CallGraph::build_direct_subset(
            &files,
            &BTreeSet::from(["collision".to_string()]),
        );
        for (route, cg) in [("full", full), ("skeleton", skeleton), ("subset", subset)] {
            let same = call_names_for(&cg, "same");
            let outer = call_names_for(&cg, "outer");
            if !same.is_empty() {
                failures.push(format!(
                    "{language:?}/{route}: colliding graph owner calls={same:?}"
                ));
            }
            if outer != ["direct".to_string()] {
                failures.push(format!(
                    "{language:?}/{route}: unrelated outer calls={outer:?}"
                ));
            }
            let multiplicity = cg
                .calls
                .values()
                .flatten()
                .filter(|site| site.caller.name == "outer" && site.callee_name == "direct")
                .count();
            if multiplicity != 1 {
                failures.push(format!(
                    "{language:?}/{route}: direct multiplicity={multiplicity}"
                ));
            }
        }
    }
    for failure in &failures {
        println!("CALL_EXECUTION_OWNER_COLLISION {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} collision mismatches",
        failures.len()
    );
}
