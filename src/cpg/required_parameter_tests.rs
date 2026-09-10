//! TypeScript required-identifier parameter occurrences used by Step 5b.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn build(language: Language, source: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
    let file = match language {
        Language::TypeScript => "required.ts",
        Language::Tsx => "required.tsx",
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
fn required_identifier_defs_use_the_parameter_identifier_bytes() {
    let source = "function take(a: any, b: any) { sink(a, b); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let a = source.find("a: any").unwrap();
        let b = source.find("b: any").unwrap();
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
fn plain_and_asserted_member_arguments_bind_to_real_ts_parameters() {
    let source = "function take(a: any, b: any) { sink(a, b); }\n\
                  function run(value: any, obj: any) { obj.other = source(); take(value, (obj as any).field); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let a = source.find("a: any").unwrap();
        let b = source.find("b: any").unwrap();
        let edges = argument_edges(&cpg, "take");
        assert!(
            edges.contains(&("value".into(), "a".into(), a, a + 1)),
            "{language:?}: {edges:?}"
        );
        assert!(
            edges.contains(&("obj.field".into(), "b".into(), b, b + 1)),
            "{language:?}: {edges:?}"
        );
        assert!(
            edges.contains(&("obj".into(), "b".into(), b, b + 1)),
            "{language:?}: {edges:?}"
        );
        assert!(!edges
            .iter()
            .any(|(path, parameter, _, _)| path == "value" && parameter == "b"));
        assert!(!edges
            .iter()
            .any(|(path, parameter, _, _)| path == "obj.other" && parameter == "b"));
    }
}

#[test]
fn required_parameter_step5b_parallel_and_serial_match() {
    let source = "function take(a: any, b: any) { sink(a, b); }\n\
                  function run(value: any, obj: any) { take(value, (obj as any).field); }\n";
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
fn duplicate_required_parameter_names_refuse_occurrence_authority() {
    let source = "function duplicate(a: any, a: any) { sink(a); }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        assert!(
            parameter_defs(&cpg, "duplicate")
                .iter()
                .all(|(name, _, _)| name != "a"),
            "{language:?}: duplicate names must not mint an authoritative Def"
        );
    }
}

#[test]
fn unsupported_parameters_are_not_introduced_and_do_not_compress_slots() {
    let source = "function mixed(a: any, { optional }: any, c: any) { sink(a, optional, c); }\n\
                  function default_middle(a: any, middle: any = clean(), c: any) { sink(a, middle, c); }\n\
                  function defaults(value = 1) { sink(value); }\n\
                  function rests(...items: any[]) { sink(items); }\n\
                  function destructured({ field }: any) { sink(field); }\n\
                  class Box { constructor(public property: any) {} }\n\
                  function run(first: any, second: any, third: any) {\n\
                    mixed(first, second, third);\n\
                    default_middle(first, second, third);\n\
                  }\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let mixed = parameter_defs(&cpg, "mixed");
        assert!(
            mixed.iter().any(|(name, _, _)| name == "a"),
            "{language:?}: {mixed:?}"
        );
        assert!(
            mixed.iter().any(|(name, _, _)| name == "c"),
            "{language:?}: {mixed:?}"
        );
        assert!(!mixed.iter().any(|(name, _, _)| name == "optional"));
        let default_middle = parameter_defs(&cpg, "default_middle");
        assert!(default_middle.iter().any(|(name, _, _)| name == "a"));
        assert!(default_middle.iter().any(|(name, _, _)| name == "c"));
        assert!(!default_middle.iter().any(|(name, _, _)| name == "middle"));
        for (function, forbidden) in [
            ("defaults", "value"),
            ("rests", "items"),
            ("destructured", "field"),
            ("constructor", "property"),
        ] {
            assert!(
                parameter_defs(&cpg, function)
                    .iter()
                    .all(|(name, _, _)| name != forbidden),
                "{language:?}: unsupported {function} parameter gained a Def"
            );
        }
        let edges = argument_edges(&cpg, "mixed");
        assert!(edges
            .iter()
            .any(|(path, parameter, _, _)| path == "first" && parameter == "a"));
        assert!(
            !edges
                .iter()
                .any(|(path, parameter, _, _)| path == "third" && parameter == "c"),
            "{language:?}: unsupported middle slot compressed the third argument: {edges:?}"
        );
        let default_edges = argument_edges(&cpg, "default_middle");
        assert!(
            default_edges
                .iter()
                .any(|(path, parameter, _, _)| path == "third" && parameter == "c"),
            "{language:?}: defaulted middle slot changed c's original index: {default_edges:?}"
        );
    }
}
