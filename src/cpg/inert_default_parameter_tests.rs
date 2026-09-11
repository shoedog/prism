//! JS/TS/TSX inert-default-identifier parameter occurrences used by Step 5b.
//!
//! Newly-admitted default occurrences are purely additive over the existing
//! required/optional-without-default contracts (see `src/ast_inert_default_parameter_tests.rs`
//! for the occurrence-level allowlist and whole-signature guard). This file
//! covers the Def/DFG/CPG consequences: entry-Def creation, Step-5b argument
//! binding, body-overwrite isolation, and label/parity behavior.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn build(language: Language, source: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
    let file = match language {
        Language::JavaScript => "default.js",
        Language::TypeScript => "default.ts",
        Language::Tsx => "default.tsx",
        _ => unreachable!(),
    };
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    let files = BTreeMap::from([(file.to_string(), parsed)]);
    (CodePropertyGraph::build(&files), files)
}

fn parameter_defs(cpg: &CodePropertyGraph, function: &str) -> BTreeSet<(String, usize, usize)> {
    cpg.graph
        .node_indices()
        .filter_map(|node| match cpg.node(node) {
            CpgNode::Variable {
                path,
                function: owner,
                access: VarAccess::Def,
                start_byte,
                end_byte,
                ..
            } if owner == function => Some((path.to_string(), *start_byte, *end_byte)),
            _ => None,
        })
        .collect()
}

fn argument_edges(
    cpg: &CodePropertyGraph,
    caller: &str,
    callee: &str,
) -> BTreeSet<(String, String, usize, usize)> {
    cpg.graph
        .edge_indices()
        .filter_map(|edge| {
            if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
                return None;
            }
            let (from, to) = cpg.graph.edge_endpoints(edge)?;
            match (cpg.node(from), cpg.node(to)) {
                (
                    CpgNode::Variable {
                        path,
                        function,
                        access: VarAccess::Use,
                        ..
                    },
                    CpgNode::Variable {
                        path: parameter,
                        function: owner,
                        access: VarAccess::Def,
                        start_byte,
                        end_byte,
                        ..
                    },
                ) if function == caller && owner == callee => Some((
                    path.to_string(),
                    parameter.to_string(),
                    *start_byte,
                    *end_byte,
                )),
                _ => None,
            }
        })
        .collect()
}

#[test]
fn inert_default_identifier_defs_use_the_parameter_identifier_bytes() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let source = match language {
            Language::JavaScript => {
                "function take(a = 1, b = \"clean\") { sink(a, b); }\n".to_string()
            }
            _ => "function take(a: any = 1, b: any = \"clean\") { sink(a, b); }\n".to_string(),
        };
        let (cpg, _) = build(language, &source);
        let a = source.find("(a").unwrap() + 1;
        let needle = if language == Language::JavaScript {
            "b = \"clean\""
        } else {
            "b: any = \"clean\""
        };
        let b = source.find(needle).unwrap();
        let defs = parameter_defs(&cpg, "take");
        assert!(
            defs.contains(&("a".into(), a, a + 1)),
            "{language:?}: {defs:?}"
        );
        assert!(
            defs.contains(&("b".into(), b, b + 1)),
            "{language:?}: {defs:?}"
        );
    }
}

#[test]
fn inert_default_parameter_step5b_parallel_and_serial_match() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let callee = match language {
            Language::JavaScript => "function take(a = 1, b = 2) { sink(a, b); }\n",
            _ => "function take(a: any = 1, b: any = 2) { sink(a, b); }\n",
        };
        let source = format!("{callee}function run(x, y) {{\n  take(x, y);\n}}\n");
        let (cpg, files) = build(language, &source);
        let parallel = CodePropertyGraph::collect_step5b_edges(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        let serial = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        assert!(
            !argument_edges(&cpg, "run", "take").is_empty(),
            "{language:?}"
        );
        assert_eq!(parallel, serial, "{language:?}: Step-5b par != serial");
    }
}

#[test]
fn explicit_argument_binds_to_the_inert_default_parameter_token() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let callee = match language {
            Language::JavaScript => {
                "function take(first, middle = 1, last) { sink(first, middle, last); }\n"
            }
            _ => "function take(first: any, middle: any = 1, last: any) { sink(first, middle, last); }\n",
        };
        // Distinct caller argument names (alpha/beta/gamma) deliberately avoid
        // the same-path-on-one-line var_index collision trap.
        let source = format!(
            "{callee}function run(alpha, beta, gamma) {{\n  take(alpha, beta, gamma);\n}}\n"
        );
        let (cpg, _) = build(language, &source);
        let middle = source.find("middle").unwrap();
        let edges = argument_edges(&cpg, "run", "take");
        assert!(
            edges.contains(&("beta".to_string(), "middle".to_string(), middle, middle + 6)),
            "{language:?}: {edges:?}"
        );
        assert!(
            edges
                .iter()
                .any(|(path, parameter, _, _)| path == "alpha" && parameter == "first"),
            "{language:?}: required prefix must still bind: {edges:?}"
        );
        assert!(
            edges
                .iter()
                .any(|(path, parameter, _, _)| path == "gamma" && parameter == "last"),
            "{language:?}: {edges:?}"
        );
    }
}

#[test]
fn omitted_argument_yields_no_edge_with_required_prefix_control() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let callee = match language {
            Language::JavaScript => "function take(first, extra = 1) { sink(first, extra); }\n",
            _ => "function take(first: any, extra: any = 1) { sink(first, extra); }\n",
        };
        let source = format!("{callee}function run(only) {{\n  take(only);\n}}\n");
        let (cpg, _) = build(language, &source);
        let edges = argument_edges(&cpg, "run", "take");
        assert!(
            edges
                .iter()
                .any(|(path, parameter, _, _)| path == "only" && parameter == "first"),
            "{language:?}: required-prefix control edge missing: {edges:?}"
        );
        assert!(
            !edges
                .iter()
                .any(|(_, parameter, _, _)| parameter == "extra"),
            "{language:?}: omitted default argument must not synthesize an edge: {edges:?}"
        );
    }
}

#[test]
fn explicit_literal_undefined_argument_yields_no_edge_unlike_a_variable_argument() {
    // Diagnostic record. Hypothesis (from prior review advice): an explicit
    // `undefined` argument mints a may-flow `Exact` edge to the default
    // parameter, since Step-5b grades resolution/token-binding evidence, not
    // the delivered runtime value. Falsifier: Step-5b's `argument_var_node_in_span`
    // only binds when the caller-side argument span is itself an indexed
    // `Variable` node. The paired calls below establish that a variable
    // argument produces an edge while the literal produces none:
    // `undefined` is its own leaf
    // grammar kind (tree-sitter-javascript `undefined: _ => 'undefined'`),
    // never registered as a `Use`. Alternative ruled out below: a genuine
    // variable argument that *could* hold `undefined` at runtime (`held`) DOES
    // bind, confirming the null result is the literal's non-variable shape,
    // not a blanket default-parameter refusal.
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let callee = match language {
            Language::JavaScript => "function take(value = 1) { sink(value); }\n",
            _ => "function take(value: any = 1) { sink(value); }\n",
        };
        let source =
            format!("{callee}function run(held) {{\n  take(undefined);\n  take(held);\n}}\n");
        let (cpg, _) = build(language, &source);
        let edges = argument_edges(&cpg, "run", "take");
        assert!(
            !edges.iter().any(|(path, _, _, _)| path == "undefined"),
            "{language:?}: a literal `undefined` argument must not fabricate an edge: {edges:?}"
        );
        assert!(
            edges
                .iter()
                .any(|(path, parameter, _, _)| path == "held" && parameter == "value"),
            "{language:?}: a variable argument (may hold undefined at runtime) is still a \
             static may-flow claim, not a runtime guarantee: {edges:?}"
        );
    }
}

#[test]
fn inert_default_parameter_never_falls_back_to_later_overwrite_var_redeclaration_or_shadow() {
    let cases = [
        ("overwrite", "value = clean();\n  sink(value);"),
        ("var-redeclaration", "var value = clean();\n  sink(value);"),
        (
            "block-shadow",
            "sink(value);\n  { let value = other; sink(value); }\n  sink(value);",
        ),
    ];
    for (case, body) in cases {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let callee = match language {
                Language::JavaScript => format!("function take(value = 1) {{\n  {body}\n}}\n"),
                _ => format!("function take(value: any = 1) {{\n  {body}\n}}\n"),
            };
            let source = format!("{callee}function run(input) {{\n  take(input);\n}}\n");
            let (cpg, _) = build(language, &source);
            let value = source.find("value").unwrap();
            let edges = argument_edges(&cpg, "run", "take");
            assert_eq!(
                edges,
                BTreeSet::from([("input".to_string(), "value".to_string(), value, value + 5)]),
                "{language:?}/{case}: must bind to the entry token, never the body write: {edges:?}"
            );
        }
    }
    // Field-only usage keeps the pre-existing field-isolation contract: no
    // bare-reference Def at all, hence no argument edge.
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let callee = match language {
            Language::JavaScript => {
                "function take(value = 1) {\n  sink(value.field);\n  sink(value.other);\n}\n"
            }
            _ => "function take(value: any = 1) {\n  sink(value.field);\n  sink(value.other);\n}\n",
        };
        let source = format!("{callee}function run(input) {{\n  take(input);\n}}\n");
        let (cpg, _) = build(language, &source);
        assert!(
            argument_edges(&cpg, "run", "take").is_empty(),
            "{language:?}/field-only-isolation: {:?}",
            argument_edges(&cpg, "run", "take")
        );
    }
}

#[test]
fn inert_default_label_on_a_nontrivial_branching_body_matches_required_control() {
    let body =
        "if (flag) {\n    sink(value);\n  } else {\n    value = clean();\n  }\n  sink(value);\n";
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (default_source, required_source) = match language {
            Language::JavaScript => (
                format!("function take(value = 1, flag) {{\n{body}}}\n"),
                format!("function take(value, flag) {{\n{body}}}\n"),
            ),
            _ => (
                format!("function take(value: any = 1, flag: any) {{\n{body}}}\n"),
                format!("function take(value: any, flag: any) {{\n{body}}}\n"),
            ),
        };
        let default_dfg = DataFlowGraph::build(&BTreeMap::from([(
            "default".to_string(),
            ParsedFile::parse("default", &default_source, language).unwrap(),
        )]));
        let required_dfg = DataFlowGraph::build(&BTreeMap::from([(
            "required".to_string(),
            ParsedFile::parse("required", &required_source, language).unwrap(),
        )]));
        let labels_for = |graph: &DataFlowGraph, file: &str| {
            graph
                .labels
                .iter()
                .filter(|((from, _), _)| {
                    from.path.to_string() == "value" && from.line == 1 && from.file == file
                })
                .map(|(_, confidence)| *confidence)
                .collect::<Vec<_>>()
        };
        let default_labels = labels_for(&default_dfg, "default");
        let required_labels = labels_for(&required_dfg, "required");
        assert!(!default_labels.is_empty(), "{language:?}: {default_source}");
        assert_eq!(
            default_labels, required_labels,
            "{language:?}: default-param entry label must match the required-param control on \
             an identical branching body: {default_labels:?} vs {required_labels:?}"
        );
    }
}

#[test]
fn inert_default_missing_entry_def_never_searches_the_body() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let callee = match language {
            Language::JavaScript => {
                "function take(value = 1) {\n value = clean();\n return value;\n}\n"
            }
            _ => "function take(value: any = 1) {\n value = clean();\n return value;\n}\n",
        };
        let source = format!("{callee}function run(input) {{\n take(input);\n}}\n");
        let (cpg, files) = build(language, &source);
        assert_eq!(argument_edges(&cpg, "run", "take").len(), 1, "{language:?}");
        let mut index = cpg.var_index.clone();
        index.retain(|(_, name, _, line, path, access), _| {
            !(name == "take" && *line == 1 && path.base == "value" && *access == VarAccess::Def)
        });
        assert!(
            index
                .keys()
                .any(|(_, name, _, line, path, access)| name == "take"
                    && *line > 1
                    && path.base == "value"
                    && *access == VarAccess::Def),
            "{language:?}: fixture must retain a same-name body Def to prove no substitution"
        );
        let edges =
            CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &index, &cpg.graph, &files);
        assert!(
            edges.is_empty(),
            "{language:?}: body fallback survived: {edges:?}"
        );
    }
}

#[test]
fn inert_default_full_and_subset_dfg_parity() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let source = match language {
            Language::JavaScript => "function take(\n a = 1,\n b = 2\n) {\n sink(a, b);\n}",
            _ => "function take(\n a: any = 1,\n b: any = 2\n) {\n sink(a, b);\n}",
        };
        let parsed = ParsedFile::parse("default.ts", source, language).unwrap();
        let files = BTreeMap::from([("default.ts".to_string(), parsed)]);
        let full = DataFlowGraph::build(&files);
        let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["default.ts".into()]));
        let defs = full.defs.values().flatten().collect::<Vec<_>>();
        assert_eq!(defs.len(), 2, "{language:?}: {defs:?}");
        for def in &defs {
            assert_eq!(def.line, 1);
            assert_eq!(&source[def.start_byte..def.end_byte], def.path.to_string());
        }
        assert_eq!(
            serde_json::to_value(full.defs.values().collect::<Vec<_>>()).unwrap(),
            serde_json::to_value(subset.defs.values().collect::<Vec<_>>()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&full.edges).unwrap(),
            serde_json::to_value(&subset.edges).unwrap()
        );
        assert_eq!(full.labels, subset.labels);
    }
}

#[test]
fn structurally_unsafe_default_forms_never_gain_a_def_or_edge() {
    let cases = [
        (
            "self_reference",
            "function take(value = value) { sink(value); }\n",
        ),
        (
            "tdz_forward_reference",
            "function take(a = b, b = 1) { sink(a, b); }\n",
        ),
        (
            "computed_destructuring_key",
            "function take({[key]: value = 1}) { sink(value); }\n",
        ),
    ];
    for (case, callee) in cases {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let source = format!("{callee}function run(x, y) {{\n  take(x, y);\n}}\n");
            let (cpg, _) = build(language, &source);
            let defs = parameter_defs(&cpg, "take");
            assert!(
                defs.iter()
                    .all(|(name, _, _)| name != "a" && name != "b" && name != "value"),
                "{language:?}/{case}: {defs:?}"
            );
        }
    }
}
