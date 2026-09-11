//! Consumer-level argument/parameter flow controls for the module-binding audit.
use prism::{
    ast::ParsedFile,
    cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess},
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};

fn flows(origin: &str, bridge: &str, app: &str, expected: &[(&str, &str)]) {
    let sources = [
        ("origin.ts", origin),
        ("bridge.ts", bridge),
        ("app.ts", app),
        ("decoy.ts", "function invoke(input) { return input; }"),
    ];
    let files: BTreeMap<_, _> = sources
        .iter()
        .map(|(file, source)| {
            let parsed = ParsedFile::parse(file, source, Language::TypeScript).unwrap();
            assert!(!parsed.tree.root_node().has_error());
            (file.to_string(), parsed)
        })
        .collect();
    let cpg = CodePropertyGraph::build(&files);
    let seeds: BTreeMap<_, _> = sources
        .iter()
        .map(|(file, source)| {
            let source = source
                .replace("export type {", "export {")
                .replace("run(value, invoke)", "run(value)");
            (
                file.to_string(),
                ParsedFile::parse(file, &source, Language::TypeScript).unwrap(),
            )
        })
        .collect();
    let cached = CodePropertyGraph::build(&seeds);
    let changed = BTreeSet::from(["app.ts".to_string(), "bridge.ts".to_string()]);
    let incremental =
        CodePropertyGraph::build_incremental(cached.call_graph, cached.dfg, &changed, &files, None);
    for cpg in [cpg, incremental] {
        let mut endpoints = BTreeSet::new();
        for edge in cpg.graph.edge_indices() {
            if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
                continue;
            }
            let (from, to) = cpg.graph.edge_endpoints(edge).unwrap();
            if let (
                CpgNode::Variable {
                    file: source_file,
                    function: caller,
                    path: arg,
                    access: VarAccess::Use,
                    ..
                },
                CpgNode::Variable {
                    file: target_file,
                    function: callee,
                    path: param,
                    access: VarAccess::Def,
                    start_byte,
                    end_byte,
                    ..
                },
            ) = (cpg.node(from), cpg.node(to))
            {
                if source_file == "app.ts"
                    && caller == "run"
                    && arg.to_string() == "value"
                    && target_file != "app.ts"
                {
                    assert_eq!(
                        &files[target_file].source[*start_byte..*end_byte],
                        param.to_string()
                    );
                    assert_eq!(param.to_string(), "input");
                    endpoints.insert((target_file.as_str(), callee.as_str()));
                }
            }
        }
        assert_eq!(
            endpoints,
            expected.iter().copied().collect(),
            "{app} via {bridge}"
        );
    }
}

#[test]
fn commented_require_preserves_origin_parameter_flow() {
    for trivia in ["", "/*before*/ ", "//before\n"] {
        flows("function origin(input) { return input; }\nexports.item = origin;", "",
            &format!("const {{ item: invoke }} = require({trivia}'./origin');\nfunction run(value) {{ return invoke(value); }}"),
            &[("origin.ts", "origin")]);
    }
}

#[test]
fn type_only_reexport_has_no_runtime_parameter_flow() {
    flows("function origin(input) { return input; }\nexport { origin as item };",
        "export type { item as publicName } from './origin';",
        "import { publicName as invoke } from './bridge';\nfunction run(value) { return invoke(value); }", &[]);
}

#[test]
fn shadowed_import_has_no_decoy_parameter_flow() {
    flows("function origin(input) { return input; }\nexport { origin as item };", "",
        "import { item as invoke } from './origin';\nfunction run(value, invoke) { return invoke(value); }", &[]);
}

#[test]
fn forwarded_import_does_not_flow_into_nested_decoy() {
    flows("function origin(input) { return input; }\nexport { origin as item };",
        "import { item as local } from './origin';\nfunction wrapper() { function local(input) { return input; } }\nexport { local as publicName };",
        "import { publicName as invoke } from './bridge';\nfunction run(value) { return invoke(value); }", &[]);
}
