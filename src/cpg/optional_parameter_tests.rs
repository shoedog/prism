//! TypeScript optional-identifier (`a?`) parameter occurrences used by Step 5b.
//!
//! Optional occurrence support is gated by a whole-signature barrier: any
//! initializer anywhere in the parameter list (a sibling default, or a
//! default nested inside a destructuring pattern) refuses every optional
//! occurrence in that list. Required-parameter occurrences are unaffected
//! and keep their existing, independent per-parameter contract.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn build(language: Language, source: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
    let file = match language {
        Language::TypeScript => "optional.ts",
        Language::Tsx => "optional.tsx",
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
                ) if function == "run" && owner == callee => Some((
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
fn optional_identifier_defs_use_the_parameter_identifier_bytes() {
    for source in [
        "function take(a?: any, b?: any) { sink(a, b); }\n",
        "function take(a?, b?) { sink(a, b); }\n",
        "function take(\n  a?: any,\n  café?: any\n) { sink(a, café); }\n",
    ] {
        for language in [Language::TypeScript, Language::Tsx] {
            let (cpg, _) = build(language, source);
            let a = source.find("a?").unwrap();
            let defs = parameter_defs(&cpg, "take");
            assert!(
                defs.contains(&("a".into(), a, a + 1)),
                "{language:?}: {source}: {defs:?}"
            );
        }
    }
}

#[test]
fn optional_parameter_step5b_parallel_and_serial_match() {
    let source = "function take(a?: any, b?: any) { sink(a, b); }\n\
                  function run(value: any, obj: any) { take(value, obj); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, files) = build(language, source);
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
        assert!(!argument_edges(&cpg, "take").is_empty(), "{language:?}");
        assert_eq!(parallel, serial, "{language:?}: Step-5b par != serial");
    }
}

#[test]
fn sibling_default_anywhere_holes_optional_slots_but_leaves_required_positions_unaffected() {
    let source = "function take(x: any, y?: any, z: any = init()) { sink(x, y, z); }\n\
                  function run(p: any, q: any, r: any) { take(p, q, r); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let defs = parameter_defs(&cpg, "take");
        assert!(
            defs.iter().any(|(name, _, _)| name == "x"),
            "{language:?}: required position must stay unaffected by the optional-only barrier: {defs:?}"
        );
        assert!(!defs.iter().any(|(name, _, _)| name == "y"));
        assert!(!defs.iter().any(|(name, _, _)| name == "z"));
        let edges = argument_edges(&cpg, "take");
        assert!(edges
            .iter()
            .any(|(path, parameter, _, _)| path == "p" && parameter == "x"));
        assert!(
            !edges
                .iter()
                .any(|(_, parameter, _, _)| parameter == "y" || parameter == "z"),
            "{language:?}: holes must not compress or fall back: {edges:?}"
        );
    }
}

#[test]
fn nested_destructuring_default_sibling_refuses_the_optional_occurrence() {
    for parameters in ["a?: any, {x = 1}: any", "a?: any, [y = 1]: any"] {
        let source = format!("function take({parameters}) {{ sink(a); }}\n");
        for language in [Language::TypeScript, Language::Tsx] {
            let (cpg, _) = build(language, &source);
            let defs = parameter_defs(&cpg, "take");
            assert!(
                !defs.iter().any(|(name, _, _)| name == "a"),
                "{language:?}: {source}: {defs:?}"
            );
        }
    }
}

#[test]
fn explicit_optional_argument_binds() {
    let source = "function take(a: any, b?: any) { sink(a, b); }\n\
                  function run(first: any, second: any) { take(first, second); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let b = source.find("b?").unwrap();
        let edges = argument_edges(&cpg, "take");
        assert!(
            edges.contains(&("second".into(), "b".into(), b, b + 1)),
            "{language:?}: explicit optional argument must bind: {edges:?}"
        );
    }
}

#[test]
fn omitted_optional_argument_yields_no_edge() {
    let source = "function take(a: any, b?: any) { sink(a, b); }\n\
                  function run(only: any) { take(only); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let edges = argument_edges(&cpg, "take");
        assert!(edges
            .iter()
            .any(|(path, parameter, _, _)| path == "only" && parameter == "a"));
        assert!(
            !edges.iter().any(|(_, parameter, _, _)| parameter == "b"),
            "{language:?}: omitted optional argument must not synthesize an edge: {edges:?}"
        );
    }
}

#[test]
fn missing_optional_entry_def_never_searches_the_body() {
    let source = "function take(value?: unknown) {\n value = clean();\n return value;\n}\nfunction run(input: unknown) {\n take(input);\n}\n";
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, files) = build(language, source);
        assert_eq!(argument_edges(&cpg, "take").len(), 1);
        let mut index = cpg.var_index.clone();
        index.retain(|(_, name, _, line, path, access), _| {
            !(name == "take" && *line == 1 && path.base == "value" && *access == VarAccess::Def)
        });
        assert!(index
            .keys()
            .any(|(_, name, _, line, path, access)| name == "take"
                && *line > 1
                && path.base == "value"
                && *access == VarAccess::Def));
        let edges =
            CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &index, &cpg.graph, &files);
        if !edges.is_empty() {
            failures.push(format!("{language:?}: body fallback survived: {edges:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn duplicate_recovery_and_escaped_optional_bindings_refuse_occurrence_authority() {
    let cases = [
        (
            "duplicate",
            "function take(value?: any, value: any) { sink(value); }\n",
        ),
        (
            "escaped",
            "function take(\\u0076alue?: any, other: any) { sink(other); }\n",
        ),
    ];
    for (case, source) in cases {
        for language in [Language::TypeScript, Language::Tsx] {
            let (cpg, _) = build(language, source);
            assert!(
                parameter_defs(&cpg, "take")
                    .iter()
                    .all(|(name, _, _)| name != "value"),
                "{language:?}/{case}: {source}"
            );
        }
    }
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function take(value?: any, b: ) { sink(value); }\n";
        let parsed = ParsedFile::parse("recovery.ts", source, language).unwrap();
        assert!(parsed.parse_error_count > 0);
        let function = parsed.all_functions()[0];
        assert!(parsed.function_parameter_occurrences(&function).is_empty());
    }
}

#[test]
fn optional_parameter_write_shadow_and_field_isolation_match_javascript() {
    for body in [
        "sink(a);\na = other;\nsink(a);",
        "sink(a);\n{ let a = other;\nsink(a); }\nsink(a);",
        "sink(a.field);\nsink(a.other);",
    ] {
        let build_dfg = |language, parameter| {
            let source = format!("function take({parameter}) {{\n{body}\n}}");
            let parsed = ParsedFile::parse("optional", &source, language).unwrap();
            DataFlowGraph::build(&BTreeMap::from([("optional".into(), parsed)]))
        };
        let control = build_dfg(Language::JavaScript, "a");
        let labels = |graph: &DataFlowGraph| {
            graph
                .labels
                .iter()
                .map(|((from, to), confidence)| {
                    (
                        from.path.to_string(),
                        from.line,
                        to.path.to_string(),
                        to.line,
                        *confidence,
                    )
                })
                .collect::<Vec<_>>()
        };
        let defs = |graph: &DataFlowGraph| {
            graph
                .defs
                .values()
                .flatten()
                .map(|def| (def.path.to_string(), def.line))
                .collect::<Vec<_>>()
        };
        for language in [Language::TypeScript, Language::Tsx] {
            let candidate = build_dfg(language, "a?: any");
            assert_eq!(labels(&candidate), labels(&control), "{language:?}: {body}");
            assert_eq!(defs(&candidate), defs(&control), "{language:?}: {body}");
            if body.starts_with("sink(a.field)") {
                assert!(!defs(&candidate).iter().any(|(path, _)| path == "a"));
            }
        }
    }
}

#[test]
fn optional_full_and_subset_dfg_parity() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function take(\n a?: any,\n b?: any\n) {\n sink(a, b);\n}";
        let parsed = ParsedFile::parse("optional.ts", source, language).unwrap();
        let files = BTreeMap::from([("optional.ts".to_string(), parsed)]);
        let full = DataFlowGraph::build(&files);
        let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["optional.ts".into()]));
        let defs = full.defs.values().flatten().collect::<Vec<_>>();
        assert_eq!(defs.len(), 2, "{language:?}: {defs:?}");
        for def in defs {
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
