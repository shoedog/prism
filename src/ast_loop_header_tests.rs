use super::*;

fn fixture(header: &str, language: Language) -> ParsedFile {
    let source = format!("async function f(items) {{\n{header} {{}}\n}}");
    let parsed = ParsedFile::parse("loop", &source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{source}");
    parsed
}

const HEADERS: &[(&str, &[&str])] = &[
    ("for (const key in items)", &["key"]),
    ("for (let item of items)", &["item"]),
    ("for (var item of items)", &["item"]),
    ("for (item of items)", &["item"]),
    ("for await (const item of items)", &["item"]),
    ("for (const {name: alias} of items)", &["alias"]),
    ("for (const [first, ...rest] of items)", &["first", "rest"]),
    ("for (const {nested: {value}} of items)", &["value"]),
    ("for (\n const /* Ω */ item\n of items\n)", &["item"]),
];

#[test]
fn loop_header_paths_keep_left_not_iterable_in_query_and_manual_routes() {
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for &(header, expected) in HEADERS {
            let p = fixture(header, language);
            let f = p.all_functions()[0];
            let lines = (1..=20).collect();
            let query = p.assignment_lvalue_paths_on_lines(&f, &lines);
            let mut manual = Vec::new();
            p.collect_assignment_paths_manual(f, &lines, &mut manual);
            for (route, paths) in [("query", query), ("manual", manual)] {
                let names: BTreeSet<_> = paths.iter().map(|(p, _)| p.to_string()).collect();
                if names.contains("items") || expected.iter().any(|n| !names.contains(*n)) {
                    failures.push(format!("{language:?}/{route} {header}: {names:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn loop_header_spans_keep_exact_left_not_iterable_in_both_routes() {
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for &(header, expected) in HEADERS {
            let p = fixture(header, language);
            let f = p.all_functions()[0];
            let lines = (1..=20).collect();
            let query = p.assignment_lvalue_spans_on_lines(&f, &lines);
            let mut manual = Vec::new();
            p.collect_assignment_spans_manual(f, &lines, &mut manual);
            for (route, spans) in [("query", query), ("manual", manual)] {
                let names: BTreeSet<_> = spans.iter().map(|s| s.path.to_string()).collect();
                if names.contains("items") || expected.iter().any(|n| !names.contains(*n)) {
                    failures.push(format!("{language:?}/{route} {header}: {names:?}"));
                }
                for name in expected {
                    for span in spans.iter().filter(|s| s.path.to_string() == *name) {
                        assert_eq!(&p.source[span.start_byte..span.end_byte], *name);
                        assert_eq!(span.line, p.line_for_byte(span.start_byte));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn loop_header_complex_rhs_and_empty_line_selection_do_not_invent_defs() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for rhs in ["items.values", "items[index]", "load(items)", "(items)"] {
            let p = fixture(&format!("for (const item of {rhs})"), language);
            let f = p.all_functions()[0];
            let paths = p.assignment_lvalue_paths_on_lines(&f, &(1..=10).collect());
            assert!(paths.iter().any(|(p, _)| p.base == "item"));
            assert!(!paths
                .iter()
                .any(|(p, _)| p.base == "items" || p.base == "index"));
            assert!(p
                .assignment_lvalue_paths_on_lines(&f, &BTreeSet::new())
                .is_empty());
            assert!(p
                .assignment_lvalue_spans_on_lines(&f, &BTreeSet::new())
                .is_empty());
        }
    }
}

#[test]
fn loop_header_python_control_keeps_body_assignment_not_iterable() {
    let p = ParsedFile::parse(
        "loop.py",
        "def f(items):\n    for item in items:\n        value = item\n",
        Language::Python,
    )
    .unwrap();
    let f = p.all_functions()[0];
    let paths = p.assignment_lvalue_paths_on_lines(&f, &(1..=4).collect());
    let spans = p.assignment_lvalue_spans_on_lines(&f, &(1..=4).collect());
    assert!(paths.iter().any(|(p, _)| p.base == "value"));
    assert!(!paths.iter().any(|(p, _)| p.base == "items"));
    assert!(spans.iter().any(|p| p.path.base == "value"));
    assert!(!spans.iter().any(|p| p.path.base == "items"));
}

#[test]
fn loop_header_unsupported_left_does_not_fall_back_to_rhs_or_body() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let p = fixture("for (state.value of items)", language);
        let body = p.all_functions()[0].child_by_field_name("body").unwrap();
        let node = body.named_child(0).unwrap();
        assert_eq!(node.kind(), "for_in_statement");
        let mut paths = Vec::new();
        let mut spans = Vec::new();
        p.extract_for_in_lvalues(&node, 2, &mut paths);
        p.extract_for_in_lvalue_spans(&node, &mut spans);
        assert!(
            paths.is_empty(),
            "this helper must not widen member-left policy"
        );
        assert!(spans.is_empty());
        // A missing left field must not activate an all-child fallback either.
        p.extract_for_in_lvalues(&body, 2, &mut paths);
        p.extract_for_in_lvalue_spans(&body, &mut spans);
        assert!(paths.is_empty() && spans.is_empty());
    }
}

#[test]
fn loop_header_real_rhs_and_body_assignments_remain_definitions() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let source =
            "function f(items) {\n for (const item of (items = load())) {\n  output = item;\n }\n}";
        let p = ParsedFile::parse("loop", source, language).unwrap();
        assert_eq!(p.parse_error_count, 0);
        let f = p.all_functions()[0];
        let lines = (1..=6).collect();
        let paths = p.assignment_lvalue_paths_on_lines(&f, &lines);
        let spans = p.assignment_lvalue_spans_on_lines(&f, &lines);
        for name in ["item", "items", "output"] {
            assert!(paths.iter().any(|(p, _)| p.to_string() == name));
            assert!(spans.iter().any(|p| p.path.to_string() == name));
        }
    }
}

#[test]
fn loop_header_same_name_left_and_right_keep_distinct_spans() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let p = fixture("for (item of item)", language);
        let f = p.all_functions()[0];
        let left = p.source.find("item of").unwrap();
        let rhs = p.source.find("item)").unwrap();
        let lines = (1..=6).collect();
        let query = p.assignment_lvalue_spans_on_lines(&f, &lines);
        let mut manual = Vec::new();
        p.collect_assignment_spans_manual(f, &lines, &mut manual);
        for spans in [query, manual] {
            assert!(spans
                .iter()
                .any(|s| s.start_byte == left && s.end_byte == left + 4));
            assert!(!spans
                .iter()
                .any(|s| s.start_byte == rhs && s.end_byte == rhs + 4));
        }
    }
}
