//! Runtime path identity only: assertions confer no class or owner authority.
use super::*;
use crate::{cpg::FlowConfidence, data_flow::DataFlowGraph};
use std::collections::BTreeMap;

fn parse(source: &str, language: Language) -> ParsedFile {
    let parsed = ParsedFile::parse("fixture.ts", source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{source}");
    parsed
}

fn graph(source: &str, language: Language) -> DataFlowGraph {
    let files = BTreeMap::from([("fixture.ts".to_string(), parse(source, language))]);
    let full = DataFlowGraph::build(&files);
    let subset = DataFlowGraph::build_subset(&files, &BTreeSet::from(["fixture.ts".into()]));
    // VarLocation equality excludes bytes. Serialization explicitly checks them.
    for (a, b) in [(&full.edges, &subset.edges)] {
        assert_eq!(
            serde_json::to_value(a).unwrap(),
            serde_json::to_value(b).unwrap()
        );
    }
    assert_eq!(
        serde_json::to_value(full.defs.values().collect::<Vec<_>>()).unwrap(),
        serde_json::to_value(subset.defs.values().collect::<Vec<_>>()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(full.uses.values().collect::<Vec<_>>()).unwrap(),
        serde_json::to_value(subset.uses.values().collect::<Vec<_>>()).unwrap()
    );
    assert_eq!(full.labels, subset.labels);
    full
}

fn field_labels(graph: &DataFlowGraph) -> Vec<(usize, usize, FlowConfidence)> {
    let path = AccessPath::from_expr("runtime.X");
    graph
        .labels
        .iter()
        .filter_map(|((from, to), label)| {
            (from.path == path && to.path == path).then_some((from.line, to.line, *label))
        })
        .collect()
}

#[test]
fn asserted_rhs_argument_and_return_retain_exact_field_flow() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expression in [
            "(runtime as any).X",
            "(runtime satisfies any).X",
            "((runtime as any) satisfies any).X",
            "(/* Ω */ runtime /* value */ as /* type */ any).X",
            "(((((((runtime as any))))))).X",
        ] {
            for statement in [
                format!("result = {expression};"),
                format!("sink({expression});"),
                format!("return {expression};"),
            ] {
                let source = format!("function owner(runtime: any, value: any) {{\nruntime.X = value;\n{statement}\n}}");
                let full = graph(&source, language);
                assert!(
                    field_labels(&full).contains(&(2, 3, FlowConfidence::Exact)),
                    "{source}: {:?}",
                    field_labels(&full)
                );
            }
        }
    }
}

#[test]
fn names_paths_and_manual_routes_agree_without_expanding_return_only_api() {
    for language in [Language::TypeScript, Language::Tsx] {
        for statement in ["result = VALUE;", "sink(VALUE);", "return VALUE;"] {
            let plain = parse(
                &format!(
                    "function owner() {{ {} }}",
                    statement.replace("VALUE", "runtime.X")
                ),
                language,
            );
            let asserted = parse(
                &format!(
                    "function owner() {{ {} }}",
                    statement.replace("VALUE", "(runtime as any).X")
                ),
                language,
            );
            let lines = BTreeSet::from([1]);
            let root = asserted.tree.root_node();
            let mut expected_names =
                plain.rvalue_identifiers_on_lines(&plain.tree.root_node(), &lines);
            let mut names = asserted.rvalue_identifiers_on_lines(&root, &lines);
            expected_names.sort();
            names.sort();
            assert_eq!(names, expected_names, "{statement}");
            let mut expected_paths =
                plain.rvalue_identifier_paths_on_lines(&plain.tree.root_node(), &lines);
            let mut paths = asserted.rvalue_identifier_paths_on_lines(&root, &lines);
            expected_paths.sort();
            paths.sort();
            assert_eq!(paths, expected_paths, "{statement}");
            // Names/paths deliberately do not gain return-only observations.
            if statement.starts_with("return") {
                assert!(names.is_empty() && paths.is_empty());
            }
            let mut manual_names = vec![];
            let mut manual_paths = vec![];
            let mut manual_spans = vec![];
            asserted.collect_rvalues_manual(root, &lines, &mut manual_names);
            asserted.collect_rvalue_paths_manual(root, &lines, &mut manual_paths);
            asserted.collect_rvalue_spans_manual(root, &lines, &mut manual_spans);
            manual_names.sort();
            manual_paths.sort();
            assert_eq!(manual_names, names);
            assert_eq!(manual_paths, paths);
            let mut spans = asserted.rvalue_identifier_spans_on_lines(&root, &lines);
            spans.sort_by_key(|s| (s.start_byte, s.end_byte));
            manual_spans.sort_by_key(|s| (s.start_byte, s.end_byte));
            assert_eq!(manual_spans, spans);
        }
    }
}

#[test]
fn member_envelopes_and_receiver_tokens_keep_real_duplicate_byte_spans() {
    for language in [Language::TypeScript, Language::Tsx] {
        let expression = "(/* Ω */ runtime /* value */ as any).X";
        let source = format!("function owner() {{ sink({expression}, {expression}); }}");
        let parsed = parse(&source, language);
        let spans =
            parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1]));
        let fields = spans
            .iter()
            .filter(|s| s.path == AccessPath::from_expr("runtime.X"))
            .collect::<Vec<_>>();
        let bases = spans
            .iter()
            .filter(|s| s.path == AccessPath::simple("runtime"))
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 2, "{spans:?}");
        assert_eq!(bases.len(), 2, "{spans:?}");
        assert_ne!(fields[0].start_byte, fields[1].start_byte);
        assert_ne!(bases[0].start_byte, bases[1].start_byte);
        for (field, base) in fields.into_iter().zip(bases) {
            assert_eq!(&source[field.start_byte..field.end_byte], expression);
            assert_eq!(&source[base.start_byte..base.end_byte], "runtime");
            assert!(base.start_byte > field.start_byte && base.end_byte < field.end_byte);
        }
        assert!(parsed
            .rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([2]))
            .is_empty());
    }
}

#[test]
fn scoped_reference_matching_tracks_identity_and_keeps_after_definition_filter() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function owner(runtime: any, value: any) {\nruntime.X = value;\nsink((runtime as any).X);\nsink((value as any).X);\nsink((runtime as any).Y);\nreturn (runtime satisfies any).X;\n}";
        let parsed = parse(source, language);
        let refs = parsed.find_path_references_scoped(
            &parsed.tree.root_node(),
            &AccessPath::from_expr("runtime.X"),
            2,
        );
        assert_eq!(refs, BTreeSet::from([3, 6]));
        assert_eq!(
            parsed.find_path_references_scoped(
                &parsed.tree.root_node(),
                &AccessPath::from_expr("runtime.X"),
                3
            ),
            BTreeSet::from([6])
        );
        let full = graph(source, language);
        assert!(field_labels(&full)
            .iter()
            .all(|(_, to, _)| ![4, 5].contains(to)));
    }
}

#[test]
fn shadow_write_and_same_line_confidence_match_plain_characterization() {
    // Plain behavior is the compatibility oracle, not a new shadow/kill policy.
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "function owner(runtime: any, value: any) {\nruntime.X = value;\nruntime.X = other;\nreturn READ;\n}",
            "function owner(runtime: any, value: any) {\nruntime.X = value;\n{ let runtime = value;\nsink(READ); }\nreturn READ;\n}",
            "function owner(runtime: any, value: any) {\nruntime.X = value; runtime.X = other;\nreturn READ;\n}",
        ] {
            let plain = graph(&source.replace("READ", "runtime.X"), language);
            let asserted = graph(&source.replace("READ", "(runtime as any).X"), language);
            assert!(!field_labels(&plain).is_empty(), "vacuous control: {source}");
            assert_eq!(field_labels(&asserted), field_labels(&plain), "{source}");
        }
    }
}

#[test]
fn erased_outer_contexts_do_not_gain_runtime_member_observations() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function owner(runtime: any) {\ntype T = typeof runtime.X;\ntype U = typeof import(\"./m\", { with: { \"resolution-mode\": \"import\" } });\n}";
        let parsed = parse(source, language);
        let root = parsed.tree.root_node();
        let lines = BTreeSet::from([2, 3]);
        assert!(parsed.rvalue_identifiers_on_lines(&root, &lines).is_empty());
        assert!(parsed
            .rvalue_identifier_paths_on_lines(&root, &lines)
            .is_empty());
        assert!(parsed
            .rvalue_identifier_spans_on_lines(&root, &lines)
            .is_empty());
    }
}

#[test]
fn refused_member_shapes_do_not_gain_the_supported_field_identity() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expression in [
            "(runtime as any)?.X",
            "(runtime as any)[key]",
            "(runtime as any)[\"X\"]",
            "(factory() as any).X",
            "((flag ? runtime : value) as any).X",
            "((sideEffect(), runtime) as any).X",
            "((runtime = value) as any).X",
            "(runtime! as any).X",
            "((runtime as any).child as any).X",
            "(r\\u0075ntime as any).X",
            "(runtime as any).\\u0058",
            "((((((((runtime as any)))))))).X",
        ] {
            let source = format!("function owner() {{ return {expression}; }}");
            let parsed = parse(&source, language);
            let root = parsed.tree.root_node();
            let spans = parsed.rvalue_identifier_spans_on_lines(&root, &BTreeSet::from([1]));
            assert!(
                !spans.is_empty(),
                "refusal deleted runtime observations: {expression}"
            );
            assert!(
                spans
                    .iter()
                    .all(|s| s.path != AccessPath::from_expr("runtime.X")),
                "{expression}: {spans:?}"
            );
            if expression.contains("factory()") {
                assert!(parsed
                    .call_names_on_lines(&[1])
                    .values()
                    .flatten()
                    .any(|name| name == "factory"));
            }
            if expression.contains("sideEffect()") {
                assert!(parsed
                    .call_names_on_lines(&[1])
                    .values()
                    .flatten()
                    .any(|name| name == "sideEffect"));
            }
        }
    }
}

#[test]
fn javascript_plain_members_keep_existing_paths_and_real_spans() {
    let source = "function owner(runtime, value) {\nruntime.X = value;\nreturn runtime.X;\n}";
    let parsed = parse(source, Language::JavaScript);
    let spans =
        parsed.rvalue_identifier_spans_on_lines(&parsed.tree.root_node(), &BTreeSet::from([3]));
    assert_eq!(
        spans.iter().map(|s| s.path.clone()).collect::<Vec<_>>(),
        vec![
            AccessPath::simple("runtime"),
            AccessPath::from_expr("runtime.X")
        ]
    );
    for span in spans {
        assert_eq!(
            AccessPath::from_expr(&source[span.start_byte..span.end_byte]),
            span.path
        );
    }
    assert!(
        field_labels(&graph(source, Language::JavaScript)).contains(&(2, 3, FlowConfidence::Exact))
    );
}

#[test]
fn asserted_lhs_writes_share_plain_rhs_field_identity() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function owner(runtime: any, value: any) {\n(runtime as any).X = value;\nreturn runtime.X;\n}";
        let full = graph(source, language);
        assert!(field_labels(&full).contains(&(2, 3, FlowConfidence::Exact)));
        let def = full
            .defs
            .values()
            .flatten()
            .find(|d| d.line == 2 && d.path == AccessPath::from_expr("runtime.X"))
            .unwrap();
        assert_eq!(&source[def.start_byte..def.end_byte], "(runtime as any).X");
    }
}

#[test]
fn descriptor_budget_error_and_language_barriers_are_explicit() {
    for language in [Language::TypeScript, Language::Tsx] {
        for (expression, accepted) in [
            ("(((((((runtime as any))))))).X", true),
            ("((((((((runtime as any)))))))).X", false),
            ("(runtime as ).X", false),
        ] {
            let source = format!("function owner() {{ return {expression}; }}");
            let parsed = ParsedFile::parse("fixture.ts", &source, language).unwrap();
            let start = source.find(expression).unwrap();
            assert_eq!(
                parsed
                    .bounded_member_path_at(start, start + expression.len())
                    .is_some(),
                accepted,
                "{expression}"
            );
        }
        let source = "type T = typeof runtime.X;";
        let parsed = parse(source, language);
        let start = source.find("runtime.X").unwrap();
        assert!(parsed.bounded_member_path_at(start, start + 9).is_none());
    }
    let source = "function owner() { return runtime.X; }";
    let parsed = parse(source, Language::JavaScript);
    let start = source.find("runtime.X").unwrap();
    assert!(parsed.bounded_member_path_at(start, start + 9).is_none());
}
