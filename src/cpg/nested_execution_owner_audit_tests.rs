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
fn nested_execution_owner_source_epochs() {
    let epochs = [
        "function outer(token){return item(token);}",
        "function outer(token){const cb=function inner(){return item(token);};return cb;}",
        "function outer(token){register(token,function inner(){return item(token);});}",
        "function outer(token){const cb=function inner(token){return item(token);};return cb;}",
        "function outer(token){return item(token);}",
    ];
    let expected_resolution = [
        vec!["outer:1:item:29-40->FreeSingle:Exact:origin.js:item:1"],
        vec![
            "inner:1:item:55-66->FreeSingle:Exact:origin.js:item:1",
            "outer:1:item:55-66->FreeSingle:Exact:origin.js:item:1",
        ],
        vec![
            "inner:1:item:61-72->FreeSingle:Exact:origin.js:item:1",
            "outer:1:item:61-72->FreeSingle:Exact:origin.js:item:1",
        ],
        vec![
            "inner:1:item:60-71->FreeSingle:Exact:origin.js:item:1",
            "outer:1:item:60-71->FreeSingle:Exact:origin.js:item:1",
        ],
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
    assert!(CodePropertyGraph::argument_var_node_in_span(
        &same_site.caller,
        same_parsed,
        &AccessPath::simple("value"),
        &same_arg,
        &same_cpg.var_index,
        &same_cpg.graph,
    )
    .is_none());

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
