use super::*;

fn named_function<'a>(parsed: &'a ParsedFile, name: &str) -> Node<'a> {
    parsed
        .all_functions()
        .into_iter()
        .find(|node| {
            parsed
                .language
                .function_name(node)
                .is_some_and(|name_node| parsed.node_text(&name_node) == name)
        })
        .unwrap_or_else(|| panic!("missing function {name}"))
}

fn expected_span(source: &str, callee: &str, call_text: &str) -> (String, usize, usize) {
    let start = source.find(call_text).unwrap();
    (callee.to_string(), start, start + call_text.len())
}

fn expected_routes(source: &str, calls: &[(&str, &str)]) -> [Vec<(String, usize, usize)>; 4] {
    let mut spans: Vec<_> = calls
        .iter()
        .map(|(callee, call)| expected_span(source, callee, call))
        .collect();
    spans.sort();
    let names = spans
        .iter()
        .map(|(name, _, _)| (name.clone(), 0, 0))
        .collect();
    let qualified_names = spans
        .iter()
        .map(|(name, _, _)| (format!("None:{name}"), 0, 0))
        .collect();
    let qualified_spans = spans
        .iter()
        .map(|(name, start, end)| (format!("None:{name}"), *start, *end))
        .collect();
    [names, spans, qualified_names, qualified_spans]
}

fn find_node_by_text<'a>(
    parsed: &'a ParsedFile,
    node: Node<'a>,
    kind: &str,
    text: &str,
) -> Option<Node<'a>> {
    if node.kind() == kind && parsed.node_text(&node) == text {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node
        .named_children(&mut cursor)
        .find_map(|child| find_node_by_text(parsed, child, kind, text));
    found
}

fn route_rows(parsed: &ParsedFile, owner: Node<'_>) -> [Vec<(String, usize, usize)>; 8] {
    let lines: BTreeSet<_> = (1..=parsed.node_line_range(&owner).1).collect();
    let macro_shadow = BTreeSet::new();

    let mut query_names: Vec<_> = parsed
        .function_calls_on_lines(&owner, &lines)
        .into_iter()
        .map(|(name, _)| (name, 0, 0))
        .collect();
    let mut query_spans: Vec<_> = parsed
        .function_calls_with_spans_on_lines(&owner, &lines, &macro_shadow)
        .into_iter()
        .map(|site| (site.callee_name, site.start_byte, site.end_byte))
        .collect();
    let mut query_qualified: Vec<_> = parsed
        .function_calls_on_lines_with_qualifier(&owner, &lines)
        .into_iter()
        .map(|(name, _, qualifier)| (format!("{qualifier:?}:{name}"), 0, 0))
        .collect();
    let mut query_qualified_spans: Vec<_> = parsed
        .function_calls_with_qualifier_and_spans_on_lines(&owner, &lines, &macro_shadow)
        .0
        .into_iter()
        .map(|site| {
            (
                format!("{:?}:{}", site.qualifier, site.callee_name),
                site.start_byte,
                site.end_byte,
            )
        })
        .collect();

    let mut manual_names = Vec::new();
    parsed.collect_calls_manual(owner, &lines, &mut manual_names);
    let mut manual_names: Vec<_> = manual_names
        .into_iter()
        .map(|(name, _)| (name, 0, 0))
        .collect();
    let mut manual_spans = Vec::new();
    parsed.collect_calls_manual_with_spans(owner, &lines, &mut manual_spans);
    let mut manual_spans: Vec<_> = manual_spans
        .into_iter()
        .map(|site| (site.callee_name, site.start_byte, site.end_byte))
        .collect();
    let mut manual_qualified = Vec::new();
    parsed.collect_calls_manual_with_qualifier(owner, &lines, &mut manual_qualified);
    let mut manual_qualified: Vec<_> = manual_qualified
        .into_iter()
        .map(|(name, _, qualifier)| (format!("{qualifier:?}:{name}"), 0, 0))
        .collect();
    let mut manual_qualified_spans = Vec::new();
    parsed.collect_calls_manual_with_qualifier_and_spans(
        owner,
        &lines,
        &mut manual_qualified_spans,
    );
    let mut manual_qualified_spans: Vec<_> = manual_qualified_spans
        .into_iter()
        .map(|site| {
            (
                format!("{:?}:{}", site.qualifier, site.callee_name),
                site.start_byte,
                site.end_byte,
            )
        })
        .collect();

    for rows in [
        &mut query_names,
        &mut query_spans,
        &mut query_qualified,
        &mut query_qualified_spans,
        &mut manual_names,
        &mut manual_spans,
        &mut manual_qualified,
        &mut manual_qualified_spans,
    ] {
        rows.sort();
    }
    [
        query_names,
        query_spans,
        query_qualified,
        query_qualified_spans,
        manual_names,
        manual_spans,
        manual_qualified,
        manual_qualified_spans,
    ]
}

fn route_inventory_failures(
    id: &str,
    source: &str,
    language: Language,
    outer_calls: &[(&str, &str)],
) -> Vec<String> {
    let parsed = ParsedFile::parse("owner-fixture", source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{id}/{language:?}");
    let outer = named_function(&parsed, "outer");
    let inner = named_function(&parsed, "inner");
    let outer_rows = route_rows(&parsed, outer);
    let inner_rows = route_rows(&parsed, inner);

    let mut expected_outer_spans: Vec<_> = outer_calls
        .iter()
        .map(|(callee, text)| expected_span(source, callee, text))
        .collect();
    expected_outer_spans.sort();
    let mut expected_outer_names: Vec<_> = expected_outer_spans
        .iter()
        .map(|(name, _, _)| (name.clone(), 0, 0))
        .collect();
    expected_outer_names.sort();
    let mut expected_outer_qualified_names: Vec<_> = expected_outer_names
        .iter()
        .map(|(name, _, _)| (format!("None:{name}"), 0, 0))
        .collect();
    expected_outer_qualified_names.sort();
    let mut expected_outer_qualified_spans: Vec<_> = expected_outer_spans
        .iter()
        .map(|(name, start, end)| (format!("None:{name}"), *start, *end))
        .collect();
    expected_outer_qualified_spans.sort();

    let mut expected_inner_spans = vec![
        expected_span(source, "seed", "seed()"),
        expected_span(source, "late", "late(p)"),
    ];
    expected_inner_spans.sort();
    let mut expected_inner_names: Vec<_> = expected_inner_spans
        .iter()
        .map(|(name, _, _)| (name.clone(), 0, 0))
        .collect();
    expected_inner_names.sort();
    let mut expected_inner_qualified_names: Vec<_> = expected_inner_names
        .iter()
        .map(|(name, _, _)| (format!("None:{name}"), 0, 0))
        .collect();
    expected_inner_qualified_names.sort();
    let mut expected_inner_qualified_spans: Vec<_> = expected_inner_spans
        .iter()
        .map(|(name, start, end)| (format!("None:{name}"), *start, *end))
        .collect();
    expected_inner_qualified_spans.sort();

    let expected_outer = [
        expected_outer_names,
        expected_outer_spans,
        expected_outer_qualified_names,
        expected_outer_qualified_spans,
    ];
    let expected_inner = [
        expected_inner_names,
        expected_inner_spans,
        expected_inner_qualified_names,
        expected_inner_qualified_spans,
    ];
    let mut failures = Vec::new();
    for route in 0..8 {
        let family = route % 4;
        if outer_rows[route] != expected_outer[family] {
            failures.push(format!(
                "{id}/{language:?}/outer/route-{route}: actual={:?} expected={:?}",
                outer_rows[route], expected_outer[family]
            ));
        }
        if inner_rows[route] != expected_inner[family] {
            failures.push(format!(
                "{id}/{language:?}/inner/route-{route}: actual={:?} expected={:?}",
                inner_rows[route], expected_inner[family]
            ));
        }
    }
    failures
}

#[test]
fn call_execution_owner_query_and_manual_routes() {
    let fixtures = [
        (
            "assignment",
            "function outer(){let cb;cb=function inner(p=seed()){late(p);};direct();}",
            vec![("direct", "direct()")],
        ),
        (
            "initializer",
            "function outer(){const cb=function inner(p=seed()){late(p);};direct();}",
            vec![("direct", "direct()")],
        ),
        (
            "call-argument",
            "function outer(){register(eager(),function inner(p=seed()){late(p);});direct();}",
            vec![
                (
                    "register",
                    "register(eager(),function inner(p=seed()){late(p);})",
                ),
                ("eager", "eager()"),
                ("direct", "direct()"),
            ],
        ),
        (
            "returned",
            "function outer(){return (eager(),function inner(p=seed()){late(p);});}",
            vec![("eager", "eager()")],
        ),
    ];
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for (id, source, outer_calls) in &fixtures {
            failures.extend(route_inventory_failures(id, source, language, outer_calls));
        }
    }
    for failure in &failures {
        println!("CALL_EXECUTION_OWNER_MISMATCH {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} ownership mismatches",
        failures.len()
    );
}

#[test]
fn call_execution_owner_computed_key_unindexed_and_root_controls() {
    let source =
        "function outer(){const obj={ [key()](){body();} };const g=function*(){late();};direct();}";
    let parsed = ParsedFile::parse("controls.js", source, Language::JavaScript).unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    let outer = named_function(&parsed, "outer");
    let outer_rows = route_rows(&parsed, outer);
    let outer_names: Vec<_> = outer_rows[0]
        .iter()
        .map(|(name, _, _)| name.as_str())
        .collect();
    assert_eq!(outer_names, ["direct", "key"]);

    let root_rows = route_rows(&parsed, parsed.tree.root_node());
    let root_names: Vec<_> = root_rows[0]
        .iter()
        .map(|(name, _, _)| name.as_str())
        .collect();
    assert_eq!(root_names, ["body", "direct", "key", "late"]);
}

#[test]
fn call_execution_owner_computed_key_callback_and_recovery_controls() {
    let computed = "const π='🙂';function outer(){const obj={ [wrap(function inner(p=seed()){late(p);})](){body();} };direct();}";
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("computed", computed, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let outer = named_function(&parsed, "outer");
        let inner = named_function(&parsed, "inner");
        let method = parsed
            .all_functions()
            .into_iter()
            .find(|node| node.kind() == "method_definition")
            .expect("computed method");
        let root = parsed.tree.root_node();
        let expectations = [
            (
                outer,
                expected_routes(
                    computed,
                    &[
                        ("wrap", "wrap(function inner(p=seed()){late(p);})"),
                        ("direct", "direct()"),
                    ],
                ),
                "outer",
            ),
            (
                inner,
                expected_routes(computed, &[("seed", "seed()"), ("late", "late(p)")]),
                "inner",
            ),
            (
                method,
                expected_routes(computed, &[("body", "body()")]),
                "method",
            ),
            (
                root,
                expected_routes(
                    computed,
                    &[
                        ("wrap", "wrap(function inner(p=seed()){late(p);})"),
                        ("seed", "seed()"),
                        ("late", "late(p)"),
                        ("body", "body()"),
                        ("direct", "direct()"),
                    ],
                ),
                "root",
            ),
        ];
        for (owner, expected, label) in expectations {
            for (route, actual) in route_rows(&parsed, owner).iter().enumerate() {
                if *actual != expected[route % 4] {
                    failures.push(format!(
                        "{language:?}/{label}/route-{route}: actual={actual:?} expected={:?}",
                        expected[route % 4]
                    ));
                }
            }
        }

        let wrap = find_node_by_text(
            &parsed,
            root,
            "call_expression",
            "wrap(function inner(p=seed()){late(p);})",
        )
        .unwrap();
        let method_name = method.child_by_field_name("name").unwrap();
        assert!(
            method_name.start_byte() <= wrap.start_byte()
                && wrap.end_byte() <= method_name.end_byte()
        );
        let seed = find_node_by_text(&parsed, root, "call_expression", "seed()").unwrap();
        let late = find_node_by_text(&parsed, root, "call_expression", "late(p)").unwrap();
        let body = find_node_by_text(&parsed, root, "call_expression", "body()").unwrap();
        let params = inner.child_by_field_name("parameters").unwrap();
        let inner_body = inner.child_by_field_name("body").unwrap();
        let method_body = method.child_by_field_name("body").unwrap();
        assert!(params.start_byte() <= seed.start_byte() && seed.end_byte() <= params.end_byte());
        assert!(
            inner_body.start_byte() <= late.start_byte()
                && late.end_byte() <= inner_body.end_byte()
        );
        assert!(
            method_body.start_byte() <= body.start_byte()
                && body.end_byte() <= method_body.end_byte()
        );
    }

    let recovery = "function outer(){const cb=function(){let = ; suspect();};direct();}function clean(){kept();}";
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("recovery", recovery, language).unwrap();
        assert!(parsed.parse_error_count > 0, "{language:?}");
        for (owner_name, calls) in [
            ("outer", vec![("direct", "direct()")]),
            ("clean", vec![("kept", "kept()")]),
        ] {
            let owner = named_function(&parsed, owner_name);
            let expected = expected_routes(recovery, &calls);
            for (route, actual) in route_rows(&parsed, owner).iter().enumerate() {
                assert_eq!(
                    *actual,
                    expected[route % 4],
                    "{language:?}/{owner_name}/route-{route}"
                );
            }
        }
    }
    for failure in &failures {
        println!("CALL_EXECUTION_OWNER_COMPUTED {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} computed-key mismatches",
        failures.len()
    );
}

#[test]
fn call_execution_owner_python_nested_and_root_preservation() {
    let source = "def outer():\n    def inner():\n        late()\n    direct()\n";
    let parsed = ParsedFile::parse("preserve.py", source, Language::Python).unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    for (owner, calls, label) in [
        (
            named_function(&parsed, "outer"),
            vec![("direct", "direct()"), ("late", "late()")],
            "outer",
        ),
        (
            named_function(&parsed, "inner"),
            vec![("late", "late()")],
            "inner",
        ),
        (
            parsed.tree.root_node(),
            vec![("direct", "direct()"), ("late", "late()")],
            "root",
        ),
    ] {
        let expected = expected_routes(source, &calls);
        for (route, actual) in route_rows(&parsed, owner).iter().enumerate() {
            assert_eq!(*actual, expected[route % 4], "{label}/route-{route}");
        }
    }
}

#[test]
fn call_execution_owner_class_computed_key_preserves_baseline() {
    let source = "function outer(){const C=class extends heritageExpr(){[classKey()](){methodRead();}static field=staticInit();instance=instanceInit();static{blockInit();}};use(C);}";
    let expected_method = [
        vec![
            ("classKey".to_string(), 0, 0),
            ("methodRead".to_string(), 0, 0),
        ],
        vec![
            ("classKey".to_string(), 55, 65),
            ("methodRead".to_string(), 69, 81),
        ],
        vec![
            ("None:classKey".to_string(), 0, 0),
            ("None:methodRead".to_string(), 0, 0),
        ],
        vec![
            ("None:classKey".to_string(), 55, 65),
            ("None:methodRead".to_string(), 69, 81),
        ],
    ];
    let outer_spans = [
        ("blockInit", 140, 151),
        ("classKey", 55, 65),
        ("heritageExpr", 39, 53),
        ("instanceInit", 118, 132),
        ("methodRead", 69, 81),
        ("staticInit", 96, 108),
        ("use", 155, 161),
    ];
    let expected_outer = [
        outer_spans
            .iter()
            .map(|(name, _, _)| ((*name).to_string(), 0, 0))
            .collect(),
        outer_spans
            .iter()
            .map(|(name, start, end)| ((*name).to_string(), *start, *end))
            .collect(),
        outer_spans
            .iter()
            .map(|(name, _, _)| (format!("None:{name}"), 0, 0))
            .collect(),
        outer_spans
            .iter()
            .map(|(name, start, end)| (format!("None:{name}"), *start, *end))
            .collect(),
    ];
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse("class.js", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        for (owner_name, expected) in [
            ("outer", &expected_outer),
            ("[classKey()]", &expected_method),
        ] {
            let owner = named_function(&parsed, owner_name);
            for (route, rows) in route_rows(&parsed, owner).iter().enumerate() {
                if *rows != expected[route % 4] {
                    failures.push(format!(
                        "{language:?}/{owner_name}/route-{route}: actual={rows:?} expected={:?}",
                        expected[route % 4]
                    ));
                }
            }
        }
    }
    for failure in &failures {
        println!("CALL_EXECUTION_OWNER_CLASS_BASELINE {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} class baseline changes",
        failures.len()
    );
}
