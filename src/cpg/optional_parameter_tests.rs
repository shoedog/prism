//! TypeScript optional-identifier (`a?`) parameter occurrences used by Step 5b.
//!
//! Optional occurrence support has two routes: an initializer-free signature,
//! or an all-simple signature with a nonempty set of inert runtime defaults.
//! Effectful, structural, type-only, and otherwise unsupported initializers
//! still refuse optional occurrences. Required occurrences keep their
//! independent per-parameter contract.

use super::build::{compute_param_names, CodePropertyGraph};
use super::{CpgEdge, CpgNode, FlowConfidence, VarAccess};
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

#[test]
fn argument_comments_do_not_occupy_js_ts_argument_positions() {
    for (arguments, expected) in [
        ("/* only */", vec![]),
        (
            "/* before */ first, /* middle */ second /* after */,",
            vec!["first", "second"],
        ),
        ("first, // line comment\n second,", vec!["first", "second"]),
        (
            "first, /* comment */ inner(second), ...rest",
            vec!["first", "inner(second)", "...rest"],
        ),
        (
            "'/* string */', /* trivia */ [first, second]",
            vec!["'/* string */'", "[first, second]"],
        ),
    ] {
        let source = format!("function run() {{\n target({arguments});\n}}");
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let parsed = ParsedFile::parse("comments", &source, language).unwrap();
            assert_eq!(parsed.parse_error_count, 0);
            let start = source.find("target(").unwrap();
            assert_eq!(
                parsed.call_argument_texts_at(start, "target"),
                expected,
                "{language:?}: {arguments}"
            );
            assert_eq!(parsed.call_argument_texts(2, "target"), expected);
            for (index, value) in expected.iter().enumerate() {
                assert_eq!(
                    parsed.call_argument_text_at(2, "target", index).as_deref(),
                    Some(*value)
                );
            }
            assert_eq!(
                parsed.call_argument_text_at(2, "target", expected.len()),
                None
            );
            let spans = parsed.call_argument_texts_and_spans_at(start, "target");
            assert_eq!(spans.len(), expected.len());
            for ((text, span), expected) in spans.iter().zip(&expected) {
                assert_eq!(text, expected);
                assert_eq!(&source[span.clone()], *expected);
            }
        }
    }
}

#[test]
fn argument_comments_preserve_required_parameter_edges_and_original_indices() {
    let source = "function take(first, second, last) { sink(first, second, last); }\nfunction run(a, b, c) {\n take(a, /* not an argument */ b, c);\n}\n";
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("comments", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let files = BTreeMap::from([("comments".into(), parsed)]);
        let cpg = CodePropertyGraph::build(&files);
        let expected = [("a", "first"), ("b", "second"), ("c", "last")]
            .map(|(a, p)| {
                let start = source.find(p).unwrap();
                (a.to_string(), p.to_string(), start, start + p.len())
            })
            .into_iter()
            .collect();
        assert_eq!(argument_edges(&cpg, "take"), expected, "{language:?}");
        assert_eq!(
            CodePropertyGraph::collect_step5b_edges(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files
            ),
            CodePropertyGraph::collect_step5b_edges_reference(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files
            )
        );
    }
}

#[test]
fn argument_comments_cannot_fill_an_omitted_optional_parameter() {
    let source = "function take(first: any, second?: any, last?: any) { sink(first, second, last); }\nfunction run(a: any, b: any) {\n take(a, /* comment */ b);\n}\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let edges = argument_edges(&cpg, "take");
        assert_eq!(edges.len(), 2);
        assert!(edges
            .iter()
            .any(|(from, to, _, _)| from == "a" && to == "first"));
        assert!(edges
            .iter()
            .any(|(from, to, _, _)| from == "b" && to == "second"));
        assert!(!edges.iter().any(|(_, to, _, _)| to == "last"));
    }
}

#[test]
fn argument_comments_preserve_member_and_base_supplementation() {
    let source = "function take(first, second) { sink(first, second); }\nfunction run(a, object) {\n take(a, /* comment */ object.field);\n}\n";
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("comments", source, language).unwrap();
        let cpg = CodePropertyGraph::build(&BTreeMap::from([("comments".into(), parsed)]));
        let edges = argument_edges(&cpg, "take");
        for from in ["object", "object.field"] {
            assert!(
                edges.iter().any(|(a, p, _, _)| a == from && p == "second"),
                "{language:?}: {edges:?}"
            );
        }
    }
}

#[test]
fn argument_comments_leave_non_js_argument_contract_unchanged() {
    let source = "fn run() { target(first, /* legacy */ second); }";
    let parsed = ParsedFile::parse("comments.rs", source, Language::Rust).unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    assert_eq!(
        parsed.call_argument_texts_at(source.find("target(").unwrap(), "target"),
        vec!["first", "/* legacy */", "second"]
    );
}

#[test]
fn argument_comments_do_not_inflate_call_site_metadata_count() {
    for (arguments, count) in [
        ("/* only */", 0),
        ("a, /* block */ b", 2),
        ("a, // line\n b,", 2),
    ] {
        let source = format!("function run(a,b) {{\n target({arguments});\n}}");
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let parsed = ParsedFile::parse("comments", &source, language).unwrap();
            assert_eq!(parsed.parse_error_count, 0);
            let cpg = CodePropertyGraph::build(&BTreeMap::from([("comments".into(), parsed)]));
            let sites = cpg
                .call_graph
                .calls
                .values()
                .flatten()
                .filter(|s| s.callee_name == "target")
                .collect::<Vec<_>>();
            assert_eq!(sites.len(), 1);
            assert_eq!(sites[0].arg_count, Some(count), "{language:?}: {arguments}");
        }
    }
}

#[test]
fn optional_inert_signature_adds_entry_def_and_ordinal_matched_argument_edge() {
    let source = "function take(\n  seed: any = 0,\n  value?: unknown\n) {\n  const held = value;\n  sink(held);\n}\nfunction run(input: unknown) {\n  take(1, input);\n}";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, files) = build(language, source);
        let value = source.find("value?").unwrap();
        let slots = compute_param_names(
            files.values().next().unwrap(),
            cpg.call_graph.functions["take"].first().unwrap(),
        );
        assert_eq!(
            slots,
            Some(vec!["seed".to_string(), "value".to_string()]),
            "{language:?}: occurrence admission must not alter positional slots"
        );
        let defs = parameter_defs(&cpg, "take");
        assert!(
            defs.contains(&("value".into(), value, value + "value".len())),
            "{language:?}: expected optional entry Def at source token: {defs:?}"
        );
        let edges = argument_edges(&cpg, "take");
        assert!(
            edges.contains(&("input".into(), "value".into(), value, value + "value".len())),
            "{language:?}: expected second supplied argument to bind the optional token: {edges:?}"
        );
        assert_eq!(
            CodePropertyGraph::collect_step5b_edges(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files
            ),
            CodePropertyGraph::collect_step5b_edges_reference(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files
            ),
            "{language:?}: Step-5b parallel/reference parity"
        );
    }
}

#[test]
fn optional_inert_signature_preserves_type_only_and_effectful_refusals() {
    for source in [
        "function take(value?: <T = unknown>() => void) { sink(value); }\nfunction run(bound: any) { take(bound); }\n",
        "function take(seed: any = init(), value?: unknown) { sink(value); }\nfunction run(bound: any) { take(0, bound); }\n",
    ] {
        for language in [Language::TypeScript, Language::Tsx] {
            let (cpg, _) = build(language, source);
            let defs = parameter_defs(&cpg, "take");
            let edges = argument_edges(&cpg, "take");
            assert!(
                !defs.iter().any(|(name, _, _)| name == "value"),
                "{language:?}: legacy refusal must retain no value Def: {defs:?}"
            );
            assert!(
                !edges.iter().any(|(_, parameter, _, _)| parameter == "value"),
                "{language:?}: legacy refusal must retain no value edge: {edges:?}"
            );
        }
    }
}

#[test]
fn optional_inert_signature_omission_literals_and_bound_argument_keep_ordinal_behavior() {
    let source = "function take(seed: any = 0, value?: unknown) { sink(value); }\nfunction run(bound: unknown) {\n  take(1);\n  take(1, undefined);\n  take(1, null);\n  take(1, bound);\n}\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, _) = build(language, source);
        let value = source.find("value?").unwrap();
        let edges = argument_edges(&cpg, "take");
        assert!(
            edges.contains(&("bound".into(), "value".into(), value, value + "value".len())),
            "{language:?}: bound second argument must bind the optional token: {edges:?}"
        );
        assert!(
            !edges.iter().any(|(from, parameter, _, _)| {
                parameter == "value" && (from == "undefined" || from == "null")
            }),
            "{language:?}: literal undefined/null must not invent a variable edge: {edges:?}"
        );
    }
}

#[test]
fn optional_inert_signature_full_and_subset_dfg_parity() {
    let source = "function take(\n  seed: any = 0,\n  value?: unknown\n) {\n  const held = value;\n  sink(held);\n}\nfunction run(input: unknown) {\n  take(1, input);\n}";
    for language in [Language::TypeScript, Language::Tsx] {
        let file = match language {
            Language::TypeScript => "optional.ts",
            Language::Tsx => "optional.tsx",
            _ => unreachable!(),
        };
        let parsed = ParsedFile::parse(file, source, language).unwrap();
        let files = BTreeMap::from([(file.to_string(), parsed)]);
        let full = DataFlowGraph::build(&files);
        let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from([file.into()]));
        let parameter_defs = full
            .defs
            .values()
            .flatten()
            .filter(|definition| definition.function == "take")
            .map(|definition| definition.path.to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            parameter_defs,
            BTreeSet::from(["held".to_string(), "value".to_string()]),
            "{language:?}: a defaulted sibling is not an entry Def"
        );
        assert_eq!(
            serde_json::to_value(full.defs.values().collect::<Vec<_>>()).unwrap(),
            serde_json::to_value(subset.defs.values().collect::<Vec<_>>()).unwrap(),
            "{language:?}: full/subset definitions"
        );
        assert_eq!(
            serde_json::to_value(&full.edges).unwrap(),
            serde_json::to_value(&subset.edges).unwrap(),
            "{language:?}: full/subset edges"
        );
        assert_eq!(
            full.labels, subset.labels,
            "{language:?}: full/subset labels"
        );
    }
}

#[test]
fn reviewer_optional_inert_exact_repeated_argument_vectors() {
    let source = "function take(\n  required: any,\n  seed: any = 0,\n  value?: unknown\n) {\n  sink(value);\n  value = clean();\n  sink(value);\n}\nfunction run(input: unknown) {\n  take(1, 2, input); take(1, 2, input);\n}\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, files) = build(language, source);
        let value = source.find("value?").unwrap();
        let input_spans = source
            .match_indices("input")
            .map(|(start, text)| (start, start + text.len()))
            .collect::<Vec<_>>();
        assert_eq!(input_spans.len(), 3);
        let mut rows = cpg
            .graph
            .edge_indices()
            .filter_map(|edge| {
                let CpgEdge::DataFlow(confidence) = cpg.graph[edge] else {
                    return None;
                };
                let (from, to) = cpg.graph.edge_endpoints(edge)?;
                match (cpg.node(from), cpg.node(to)) {
                    (
                        CpgNode::Variable {
                            file: from_file,
                            function: from_owner,
                            function_start_line: from_owner_line,
                            line: from_line,
                            path: from_path,
                            access: VarAccess::Use,
                            start_byte: from_start,
                            end_byte: from_end,
                        },
                        CpgNode::Variable {
                            file: to_file,
                            function: to_owner,
                            function_start_line: to_owner_line,
                            line: to_line,
                            path: to_path,
                            access: VarAccess::Def,
                            start_byte: to_start,
                            end_byte: to_end,
                        },
                    ) if from_owner == "run"
                        && to_owner == "take"
                        && from_path.to_string() == "input"
                        && to_path.to_string() == "value" =>
                    {
                        Some((
                            (
                                from_file.clone(),
                                from_owner.clone(),
                                *from_owner_line,
                                *from_line,
                                *from_start,
                                *from_end,
                            ),
                            (
                                to_file.clone(),
                                to_owner.clone(),
                                *to_owner_line,
                                *to_line,
                                *to_start,
                                *to_end,
                            ),
                            confidence,
                        ))
                    }
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.0 .4);
        let file = match language {
            Language::TypeScript => "optional.ts",
            Language::Tsx => "optional.tsx",
            _ => unreachable!(),
        };
        assert_eq!(
            rows,
            vec![
                (
                    (
                        file.into(),
                        "run".into(),
                        10,
                        11,
                        input_spans[1].0,
                        input_spans[1].1,
                    ),
                    (file.into(), "take".into(), 1, 1, value, value + 5),
                    FlowConfidence::Exact,
                ),
                (
                    (
                        file.into(),
                        "run".into(),
                        10,
                        11,
                        input_spans[2].0,
                        input_spans[2].1,
                    ),
                    (file.into(), "take".into(), 1, 1, value, value + 5),
                    FlowConfidence::Exact,
                ),
            ],
            "{language:?}: exact endpoint/multiplicity rows"
        );
        // Pin both source sites independently of the variable occurrence index.
        let mut sites = cpg
            .call_graph
            .calls
            .values()
            .flatten()
            .filter(|site| site.callee_name == "take")
            .collect::<Vec<_>>();
        sites.sort_by_key(|site| site.start_byte);
        let starts = source
            .match_indices("take(1, 2, input)")
            .map(|(start, _)| start)
            .collect::<Vec<_>>();
        assert_eq!(sites.len(), 2);
        for (site, start) in sites.into_iter().zip(starts) {
            assert_eq!(
                (
                    site.caller.file.as_str(),
                    site.caller.name.as_str(),
                    site.caller.start_line,
                    site.line,
                    site.start_byte,
                    site.end_byte,
                    site.arg_count
                ),
                (
                    file,
                    "run",
                    10,
                    11,
                    start,
                    start + "take(1, 2, input)".len(),
                    Some(3)
                )
            );
            let resolved = cpg.call_graph.resolve_call_site(site);
            assert_eq!(resolved.len(), 1);
            assert_eq!(
                (
                    resolved[0].target.file.as_str(),
                    resolved[0].target.name.as_str(),
                    resolved[0].target.start_line,
                    resolved[0].confidence
                ),
                (
                    file,
                    "take",
                    1,
                    crate::resolution::ResolutionConfidence::Exact
                )
            );
        }
        assert_eq!(
            optional_boundary_rows(&cpg).len(),
            2,
            "no extra cross-boundary rows"
        );
        let take = cpg.call_graph.functions["take"].first().unwrap();
        assert_eq!(
            compute_param_names(files.values().next().unwrap(), take),
            Some(vec!["required".into(), "seed".into(), "value".into()]),
            "{language:?}: original slot ordinal 2"
        );
        assert_eq!(
            CodePropertyGraph::collect_step5b_edges(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files
            ),
            CodePropertyGraph::collect_step5b_edges_reference(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files
            ),
            "{language:?}: full parallel/reference Step-5b parity"
        );
    }
}

#[test]
fn reviewer_optional_inert_revokes_and_restores_without_body_fallback() {
    let sources = [
        ("inert-1", "seed: any = 0, value?: unknown", true),
        ("effectful", "seed: any = init(), value?: unknown", false),
        ("initializer-free", "seed: any, value?: unknown", true),
        ("inert-2", "seed: any = 0, value?: unknown", true),
    ];
    for language in [Language::TypeScript, Language::Tsx] {
        let mut first_inert = None;
        for (epoch, params, admitted) in sources {
            let source = format!(
                "function take({params}) {{\n  value = clean();\n  sink(value);\n}}\nfunction run(input: unknown) {{\n  take(1, input);\n}}\n"
            );
            let (cpg, files) = build(language, &source);
            let entry = source.find("value?").unwrap();
            let exact_entry =
                parameter_defs(&cpg, "take").contains(&("value".into(), entry, entry + 5));
            assert_eq!(exact_entry, admitted, "{language:?}/{epoch}");
            let rows = argument_edges(&cpg, "take");
            assert_eq!(
                rows.iter().any(|(from, to, start, end)| {
                    from == "input" && to == "value" && (*start, *end) == (entry, entry + 5)
                }),
                admitted,
                "{language:?}/{epoch}: exact entry binding"
            );
            if admitted {
                let normalized = (parameter_defs(&cpg, "take"), argument_edges(&cpg, "take"));
                if epoch == "inert-1" {
                    first_inert = Some(normalized);
                } else if epoch == "inert-2" {
                    assert_eq!(first_inert.as_ref(), Some(&normalized));
                }
            } else {
                let mut index = cpg.var_index.clone();
                index.retain(|(_, name, _, line, path, access), _| {
                    !(name == "take"
                        && *line == 1
                        && path.base == "value"
                        && *access == VarAccess::Def)
                });
                assert!(index.keys().any(|(_, name, _, line, path, access)| {
                    name == "take" && *line > 1 && path.base == "value" && *access == VarAccess::Def
                }));
                assert!(
                    CodePropertyGraph::collect_step5b_edges(
                        &cpg.call_graph,
                        &index,
                        &cpg.graph,
                        &files
                    )
                    .is_empty(),
                    "{language:?}/{epoch}: body Def must not substitute for absent entry"
                );
            }
        }
    }
}

#[test]
fn optional_inert_incremental_caller_and_callee_edits_match_fresh_rows() {
    let rows = |cpg: &CodePropertyGraph| {
        let mut nodes = cpg
            .graph
            .node_indices()
            .map(|i| format!("{:?}", cpg.graph[i]))
            .collect::<Vec<_>>();
        nodes.sort();
        let mut edges = cpg
            .graph
            .edge_indices()
            .map(|e| {
                let (from, to) = cpg.graph.edge_endpoints(e).unwrap();
                format!(
                    "{:?}->{:?}:{:?}",
                    cpg.graph[from], cpg.graph[to], cpg.graph[e]
                )
            })
            .collect::<Vec<_>>();
        edges.sort();
        (nodes, edges)
    };
    for (language, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        let files = |caller: &str, callee: &str| {
            BTreeMap::from([
                (
                    format!("caller.{ext}"),
                    ParsedFile::parse(&format!("caller.{ext}"), caller, language).unwrap(),
                ),
                (
                    format!("callee.{ext}"),
                    ParsedFile::parse(&format!("callee.{ext}"), callee, language).unwrap(),
                ),
            ])
        };
        let inert = "export function take(seed: any = 0, value?: unknown) { sink(value); }";
        let effectful =
            "export function take(seed: any = init(), value?: unknown) { sink(value); }";
        let omitted = "import {take} from './callee'; function run(input: unknown) { take(1); }";
        let bound =
            "import {take} from './callee'; function run(input: unknown) { take(1, input); }";

        let before = files(omitted, inert);
        let prior = CodePropertyGraph::build(&before);
        let caller_after = files(bound, inert);
        let caller_incremental = CodePropertyGraph::build_incremental(
            prior.call_graph.clone(),
            prior.dfg.clone(),
            &BTreeSet::from([format!("caller.{ext}")]),
            &caller_after,
            None,
        );
        let caller_fresh = CodePropertyGraph::build(&caller_after);
        assert_eq!(
            rows(&caller_incremental),
            rows(&caller_fresh),
            "{language:?}: caller-only incremental"
        );

        let effectful_files = files(bound, effectful);
        let callee_incremental = CodePropertyGraph::build_incremental(
            caller_incremental.call_graph.clone(),
            caller_incremental.dfg.clone(),
            &BTreeSet::from([format!("callee.{ext}")]),
            &effectful_files,
            None,
        );
        let effectful_fresh = CodePropertyGraph::build(&effectful_files);
        assert_eq!(
            rows(&callee_incremental),
            rows(&effectful_fresh),
            "{language:?}: callee-only revoke"
        );
        assert!(!parameter_defs(&callee_incremental, "take")
            .iter()
            .any(|(name, _, _)| name == "value"));

        let restored = CodePropertyGraph::build_incremental(
            callee_incremental.call_graph.clone(),
            callee_incremental.dfg.clone(),
            &BTreeSet::from([format!("callee.{ext}")]),
            &caller_after,
            None,
        );
        assert_eq!(
            rows(&restored),
            rows(&caller_fresh),
            "{language:?}: callee-only restore"
        );
        assert!(parameter_defs(&restored, "take")
            .iter()
            .any(|(name, _, _)| name == "value"));
    }
}

// Equal-width signatures make the required control's byte coordinates independent
// and directly comparable; no production occurrence helper constructs the oracle.
#[test]
fn optional_inert_rd_labels_match_required_and_preserve_shadow_controls() {
    for (case, body) in [
        ("straight", "sink(value);\nsink(value);"),
        ("overwrite", "sink(value);\nvalue = other;\nsink(value);"),
        (
            "branch",
            "sink(value);\nif (flag) { value = other; }\nsink(value);",
        ),
        (
            "shadow",
            "sink(value);\n{ let value = other;\nsink(value); }\nsink(value);",
        ),
    ] {
        for language in [Language::TypeScript, Language::Tsx] {
            let mixed = format!(
                "function take(seed: any = 0, value?: any, flag: any, other: any) {{\n{body}\n}}"
            );
            let required = mixed
                .replacen("= 0", "   ", 1)
                .replacen("value?", "value ", 1);
            let graph = |source: &str| {
                let parsed = ParsedFile::parse("p", source, language).unwrap();
                assert_eq!(parsed.parse_error_count, 0);
                DataFlowGraph::build(&BTreeMap::from([("p".into(), parsed)]))
            };
            let rows = |dfg: &DataFlowGraph| {
                dfg.labels
                    .iter()
                    .filter(|((from, _), _)| from.path.to_string() == "value")
                    .map(|((from, to), label)| (format!("{from:?}"), format!("{to:?}"), *label))
                    .collect::<Vec<_>>()
            };
            let candidate = graph(&mixed);
            let control = graph(&required);
            assert!(!rows(&candidate).is_empty(), "{language:?}/{case}");
            assert_eq!(
                rows(&candidate),
                rows(&control),
                "{language:?}/{case}: full endpoint/label vectors"
            );
            let labels = candidate
                .labels
                .iter()
                .filter(|((from, _), _)| from.path.to_string() == "value")
                .map(|((from, to), label)| (from.line, to.line, *label))
                .collect::<Vec<_>>();
            use super::FlowDoubt::{CfgIncomplete, Killed};
            use FlowConfidence::{Exact, NameOnly};
            let expected = match case {
                "straight" => vec![(1, 2, Exact), (1, 3, Exact)],
                "overwrite" | "branch" => vec![
                    (1, 2, Exact),
                    (1, 3, Exact),
                    (1, 4, NameOnly(Killed { kill_line: 3 })),
                    (3, 1, NameOnly(CfgIncomplete)),
                    (3, 2, NameOnly(CfgIncomplete)),
                    (3, 4, Exact),
                ],
                "shadow" => vec![
                    (1, 2, Exact),
                    (1, 5, Exact),
                    (3, 1, NameOnly(CfgIncomplete)),
                    (3, 2, NameOnly(CfgIncomplete)),
                    (3, 4, Exact),
                    (3, 5, NameOnly(Killed { kill_line: 4 })),
                ],
                _ => unreachable!(),
            };
            assert_eq!(
                labels, expected,
                "{language:?}/{case}: retained confidence and doubt, not a precision claim"
            );
        }
    }
}

type OptionalEndpoint = (String, String, usize, usize, String, String, usize, usize);
fn optional_endpoint(node: &CpgNode) -> OptionalEndpoint {
    match node {
        CpgNode::Variable {
            file,
            function,
            function_start_line,
            line,
            path,
            access,
            start_byte,
            end_byte,
        } => (
            file.clone(),
            function.clone(),
            *function_start_line,
            *line,
            path.to_string(),
            format!("{access:?}"),
            *start_byte,
            *end_byte,
        ),
        _ => panic!("expected variable endpoint: {node:?}"),
    }
}
fn optional_boundary_rows(
    cpg: &CodePropertyGraph,
) -> Vec<(OptionalEndpoint, OptionalEndpoint, FlowConfidence)> {
    let mut rows = cpg
        .graph
        .edge_indices()
        .filter_map(|e| {
            let CpgEdge::DataFlow(label) = cpg.graph[e] else {
                return None;
            };
            let (from, to) = cpg.graph.edge_endpoints(e)?;
            match (cpg.node(from), cpg.node(to)) {
                (
                    CpgNode::Variable {
                        function: caller, ..
                    },
                    CpgNode::Variable {
                        function: callee, ..
                    },
                ) if caller == "run" && callee == "take" => Some((
                    optional_endpoint(cpg.node(from)),
                    optional_endpoint(cpg.node(to)),
                    label,
                )),
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

#[test]
fn optional_inert_exact_omitted_literal_and_bound_population() {
    let source = "function take(seed: any = 0, value?: unknown) { sink(value); }\nfunction run(bound: unknown) {\n  take(1);\n  take(1, undefined);\n  take(1, null);\n  take(1, bound);\n}\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, files) = build(language, source);
        let file = files.keys().next().unwrap();
        let start = source.rfind("bound").unwrap();
        assert_eq!(
            optional_boundary_rows(&cpg),
            vec![(
                (
                    file.clone(),
                    "run".into(),
                    2,
                    6,
                    "bound".into(),
                    "Use".into(),
                    start,
                    start + 5
                ),
                (
                    file.clone(),
                    "take".into(),
                    1,
                    1,
                    "value".into(),
                    "Def".into(),
                    29,
                    34
                ),
                FlowConfidence::Exact,
            )],
            "{language:?}: full population, no literal or omission edges"
        );
        let mut sites = cpg
            .call_graph
            .calls
            .values()
            .flatten()
            .filter(|s| s.callee_name == "take")
            .collect::<Vec<_>>();
        sites.sort_by_key(|s| s.start_byte);
        let expected = [
            (3, "take(1)", 1),
            (4, "take(1, undefined)", 2),
            (5, "take(1, null)", 2),
            (6, "take(1, bound)", 2),
        ];
        assert_eq!(sites.len(), expected.len());
        for (site, (line, text, arity)) in sites.into_iter().zip(expected) {
            assert_eq!(
                (
                    site.caller.file.as_str(),
                    site.caller.name.as_str(),
                    site.caller.start_line,
                    site.line,
                    site.start_byte,
                    site.end_byte,
                    site.arg_count
                ),
                (
                    file.as_str(),
                    "run",
                    2,
                    line,
                    source.find(text).unwrap(),
                    source.find(text).unwrap() + text.len(),
                    Some(arity)
                )
            );
            let resolved = cpg.call_graph.resolve_call_site(site);
            assert_eq!(resolved.len(), 1);
            assert_eq!(
                (
                    resolved[0].target.file.as_str(),
                    resolved[0].target.name.as_str(),
                    resolved[0].target.start_line,
                    resolved[0].confidence
                ),
                (
                    file.as_str(),
                    "take",
                    1,
                    crate::resolution::ResolutionConfidence::Exact
                )
            );
        }
        assert_eq!(
            compute_param_names(
                files.values().next().unwrap(),
                &cpg.call_graph.functions["take"][0]
            ),
            Some(vec!["seed".into(), "value".into()])
        );
    }
}

#[test]
fn optional_inert_exact_body_and_wrong_owner_never_replace_entry() {
    for (case, body) in [
        ("overwrite", "value = clean();\nsink(value);"),
        ("redeclare", "var value = clean();\nsink(value);"),
        (
            "nested-shadow",
            "function inner(value: unknown) { sink(value); }\nsink(value);",
        ),
    ] {
        let source = format!("function take(seed: any = 0, value?: unknown) {{\n{body}\n}}\nfunction other(value: unknown) {{ sink(value); }}\nfunction run(input: unknown) {{ take(1, input); }}\n");
        for language in [Language::TypeScript, Language::Tsx] {
            let (cpg, files) = build(language, &source);
            let file = files.keys().next().unwrap();
            let start = source.rfind("input").unwrap();
            let line = source[..start].bytes().filter(|b| *b == b'\n').count() + 1;
            assert_eq!(
                optional_boundary_rows(&cpg),
                vec![(
                    (
                        file.clone(),
                        "run".into(),
                        line,
                        line,
                        "input".into(),
                        "Use".into(),
                        start,
                        start + 5
                    ),
                    (
                        file.clone(),
                        "take".into(),
                        1,
                        1,
                        "value".into(),
                        "Def".into(),
                        29,
                        34
                    ),
                    FlowConfidence::Exact,
                )],
                "{language:?}/{case}"
            );
            let mut index = cpg.var_index.clone();
            let before = index.len();
            index.retain(|(_, owner, start_line, line, path, access), _| {
                !(owner == "take"
                    && *start_line == 1
                    && *line == 1
                    && path.base == "value"
                    && *access == VarAccess::Def)
            });
            assert_eq!(index.len() + 1, before, "remove exactly the entry key");
            assert!(index
                .keys()
                .any(|(_, owner, _, _, path, access)| owner == "other"
                    && path.base == "value"
                    && *access == VarAccess::Def));
            if case != "nested-shadow" {
                assert!(index
                    .keys()
                    .any(|(_, owner, _, line, path, access)| owner == "take"
                        && *line > 1
                        && path.base == "value"
                        && *access == VarAccess::Def));
            } else {
                assert!(index
                    .keys()
                    .any(|(_, owner, _, _, path, access)| owner == "inner"
                        && path.base == "value"
                        && *access == VarAccess::Def));
            }
            for rows in [
                CodePropertyGraph::collect_step5b_edges(
                    &cpg.call_graph,
                    &index,
                    &cpg.graph,
                    &files,
                ),
                CodePropertyGraph::collect_step5b_edges_reference(
                    &cpg.call_graph,
                    &index,
                    &cpg.graph,
                    &files,
                ),
            ] {
                assert!(
                    rows.is_empty(),
                    "{language:?}/{case}: no body, nested or other-owner fallback: {rows:?}"
                );
            }
        }
    }
}
