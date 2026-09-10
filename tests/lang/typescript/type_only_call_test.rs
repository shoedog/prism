//! Import types retain their grammar nodes, but never become runtime calls.
use prism::{ast::ParsedFile, call_graph::CallGraph, languages::Language};
use std::collections::{BTreeMap, BTreeSet};

fn fixtures() -> Vec<(ParsedFile, Vec<(String, usize, usize)>)> {
    let mut cases = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        for source in [
            "function owner() { type T = typeof import(\"types\"); }",
            "function owner(x: typeof import(\"types\")) {}",
            "function owner() { type T = import(\"types\").X; }",
            "function owner(x: import(\"types\").X) {}",
            "function owner() { type T = typeof import(\"types\"); import(\"runtime\"); }",
            "function owner() { type T = import(\"types\").X; import(\"runtime\"); }",
            "function owner() { import(\"runtime\"); }",
            "function owner() { const x = typeof import(\"runtime\"); }",
            "function owner() { const x = value() as import(\"types\").X; }",
            "function owner() { const x = value() satisfies import(\"types\").X; }",
            "function owner() { const x = <import(\"types\").X>value(); }",
            "function owner<T extends import(\"types\").X>() {}",
            "function owner<T = import(\"types\").X>() {}",
            "function owner() { interface I { x: import(\"types\").X; } }",
            "function owner() { const x = value() /* left */ as /* right */ import(\"types\").X; }",
            "function owner() { const x = value() as typeof import(\"types\"); }",
        ] {
            if language == Language::Tsx && source.contains("<import(") {
                continue; // Angle assertions are not TSX syntax.
            }
            let parsed = ParsedFile::parse("calls.ts", source, language).unwrap();
            assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
            assert_eq!(parsed.source, source);
            let mut expected: Vec<_> = source
                .find("import(\"runtime\")")
                .map(|start| ("import".into(), start, start + "import(\"runtime\")".len()))
                .into_iter()
                .collect();
            expected.extend(
                source
                    .match_indices("value()")
                    .map(|(start, _)| ("value".into(), start, start + "value()".len())),
            );
            expected.sort_by_key(|x| x.1);
            cases.push((parsed, expected));
        }
    }
    cases
}

#[test]
fn type_only_call_names_on_lines() {
    for (parsed, expected) in fixtures() {
        let names = expected.iter().map(|x| x.0.clone()).collect::<Vec<_>>();
        let expected = if names.is_empty() {
            BTreeMap::new()
        } else {
            BTreeMap::from([(1, names)])
        };
        assert_eq!(
            parsed.call_names_on_lines(&[1]),
            expected,
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_function_calls_on_lines() {
    for (parsed, expected) in fixtures() {
        let expected = expected.into_iter().map(|x| (x.0, 1)).collect::<Vec<_>>();
        assert_eq!(
            parsed.function_calls_on_lines(&parsed.tree.root_node(), &BTreeSet::from([1])),
            expected,
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_function_calls_with_qualifier() {
    for (parsed, expected) in fixtures() {
        let expected = expected
            .into_iter()
            .map(|x| (x.0, 1, None))
            .collect::<Vec<_>>();
        assert_eq!(
            parsed.function_calls_on_lines_with_qualifier(
                &parsed.tree.root_node(),
                &BTreeSet::from([1])
            ),
            expected,
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_function_calls_with_spans() {
    for (parsed, expected) in fixtures() {
        let calls = parsed.function_calls_with_spans_on_lines(
            &parsed.tree.root_node(),
            &BTreeSet::from([1]),
            &BTreeSet::new(),
        );
        assert_eq!(
            calls
                .into_iter()
                .map(|x| (x.callee_name, x.start_byte, x.end_byte))
                .collect::<Vec<_>>(),
            expected,
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_function_calls_with_qualifier_and_spans() {
    for (parsed, expected) in fixtures() {
        let (calls, _) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &parsed.tree.root_node(),
            &BTreeSet::from([1]),
            &BTreeSet::new(),
        );
        assert!(calls.iter().all(|x| x.qualifier.is_none()
            && x.arg_count == Some(if x.callee_name == "import" { 1 } else { 0 })));
        assert_eq!(
            calls
                .into_iter()
                .map(|x| (x.callee_name, x.start_byte, x.end_byte))
                .collect::<Vec<_>>(),
            expected,
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_callees_in_function() {
    for (parsed, expected) in fixtures() {
        assert_eq!(
            parsed.callees_in_function(&parsed.tree.root_node()),
            expected.into_iter().map(|x| x.0).collect::<Vec<_>>(),
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_line_argument_lookup() {
    for (parsed, expected) in fixtures() {
        let expected = expected
            .iter()
            .any(|x| x.0 == "import")
            .then(|| "\"runtime\"".to_string());
        assert_eq!(
            parsed.call_argument_text_at(1, "import", 0),
            expected,
            "{}",
            parsed.source
        );
        assert_eq!(
            parsed.call_argument_texts(1, "import"),
            expected.into_iter().collect::<Vec<_>>(),
            "{}",
            parsed.source
        );
    }
}

#[test]
fn type_only_exact_argument_index() {
    for (parsed, expected) in fixtures() {
        for (start, _) in parsed.source.match_indices("import(") {
            let expected = expected
                .iter()
                .any(|x| x.1 == start)
                .then(|| "\"runtime\"".to_string())
                .into_iter()
                .collect::<Vec<_>>();
            assert_eq!(
                parsed.call_argument_texts_at(start, "import"),
                expected,
                "{} at {start}",
                parsed.source
            );
        }
    }
}

#[test]
fn type_only_graph_sites() {
    for (parsed, expected) in fixtures() {
        let source = parsed.source.clone();
        let graph = CallGraph::build(&BTreeMap::from([("calls.ts".into(), parsed)]));
        let sites = graph
            .calls
            .iter()
            .flat_map(|(owner, calls)| {
                assert_eq!(owner.name, "owner");
                calls
                    .iter()
                    .map(|call| (call.callee_name.clone(), call.start_byte, call.end_byte))
            })
            .collect::<Vec<_>>();
        assert_eq!(sites, expected, "{source}");
    }
}

#[test]
fn type_only_direct_subset_graph_matches_full_graph() {
    for (parsed, expected) in fixtures() {
        let source = parsed.source.clone();
        let files = BTreeMap::from([("calls.ts".into(), parsed)]);
        let subset = CallGraph::build_direct_subset(&files, &BTreeSet::from(["calls.ts".into()]));
        let sites = subset
            .calls
            .values()
            .flatten()
            .map(|call| (call.callee_name.clone(), call.start_byte, call.end_byte))
            .collect::<Vec<_>>();
        assert_eq!(sites, expected, "{source}");
        assert_eq!(subset.calls, CallGraph::build(&files).calls, "{source}");
    }
}

#[test]
fn type_only_mixed_line_preserves_qualified_runtime_call_and_argument() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source =
            "function owner() { type T = typeof import(\"types\"); svc.run(import(\"runtime\")); }";
        let parsed = ParsedFile::parse("calls.ts", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let (calls, _) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &parsed.tree.root_node(),
            &BTreeSet::from([1]),
            &BTreeSet::new(),
        );
        assert_eq!(
            calls
                .iter()
                .map(|call| (
                    call.callee_name.as_str(),
                    call.qualifier.as_deref(),
                    &source[call.start_byte..call.end_byte],
                ))
                .collect::<Vec<_>>(),
            [
                ("run", Some("svc"), "svc.run(import(\"runtime\"))"),
                ("import", None, "import(\"runtime\")"),
            ]
        );
        let run_start = source.find("svc.run").unwrap();
        assert_eq!(
            parsed.call_argument_texts_at(run_start, "run"),
            ["import(\"runtime\")"]
        );
        let files = BTreeMap::from([("calls.ts".into(), parsed)]);
        let graph = CallGraph::build(&files);
        let subset = CallGraph::build_direct_subset(&files, &BTreeSet::from(["calls.ts".into()]));
        assert_eq!(subset.calls, graph.calls);
        let sites = graph.calls.values().flatten().collect::<Vec<_>>();
        assert_eq!(sites.len(), 2);
        assert!(sites
            .iter()
            .any(|call| call.callee_name == "run" && call.qualifier.as_deref() == Some("svc")));
    }
}

#[test]
fn type_only_direct_generic_preserves_outer_runtime_call() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function owner() { get<typeof import(\"types\")>(); import(\"runtime\"); }";
        let parsed = ParsedFile::parse("calls.ts", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let (calls, _) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &parsed.tree.root_node(),
            &BTreeSet::from([1]),
            &BTreeSet::new(),
        );
        assert_eq!(
            calls
                .iter()
                .map(|call| source[call.start_byte..call.end_byte].to_string())
                .collect::<Vec<_>>(),
            ["get<typeof import(\"types\")>()", "import(\"runtime\")"]
        );
        assert_eq!(
            calls
                .iter()
                .map(|call| call.callee_name.as_str())
                .collect::<Vec<_>>(),
            ["get", "import"]
        );
        let type_start = source.find("import(\"types\")").unwrap();
        assert!(parsed
            .call_argument_texts_at(type_start, "import")
            .is_empty());
    }
}

#[test]
fn type_only_guard_preserves_javascript_dynamic_imports() {
    let parsed = ParsedFile::parse(
        "calls.js",
        "function owner() { const x = typeof import(\"runtime\"); }",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    assert_eq!(
        parsed.call_names_on_lines(&[1]),
        BTreeMap::from([(1, vec!["import".into()])])
    );
}
