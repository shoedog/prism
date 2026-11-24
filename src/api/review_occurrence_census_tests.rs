use super::*;
use crate::{ast::ParsedFile, languages::Language};

fn parse(path: &str, source: &str) -> ParsedFile {
    ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
}
fn spans(source: &str, token: &str) -> Vec<Range<usize>> {
    source
        .match_indices(token)
        .map(|(i, _)| i..i + token.len())
        .collect()
}
fn query(facts: &ReviewOccurrenceFacts<'_>, line: usize, name: &str) -> ReviewOccurrenceCensus {
    facts.literal_census(
        facts.parsed.tree.root_node().byte_range(),
        line,
        &AccessPath::simple(name),
    )
}

#[test]
fn receivers_and_complete_zero_cover_supported_languages() {
    for (file, source, line, expression) in [
        (
            "a.js",
            "function h(req) {\n send(req.body);\n}",
            2,
            "req.body",
        ),
        (
            "a.ts",
            "function h(req: R) {\n send(req.query.command);\n}",
            2,
            "req.query.command",
        ),
        (
            "a.tsx",
            "function h(req: R) {\n return <C value={req.body}/>;\n}",
            2,
            "req.body",
        ),
        ("a.py", "def h(req):\n    send(req)\n", 2, "req)\n"),
        (
            "a.go",
            "package p\nfunc h(req string) {\n send(req)\n}",
            3,
            "req)\n",
        ),
    ] {
        let parsed = parse(file, source);
        assert_eq!(parsed.parse_error_count, 0, "{file}");
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let start = source.find(expression).unwrap();
        assert_eq!(
            query(&facts, line, "req"),
            ReviewOccurrenceCensus::Complete(vec![start..start + 3]),
            "{file}"
        );
        assert_eq!(
            query(&facts, line, "absent"),
            ReviewOccurrenceCensus::Complete(vec![])
        );
    }
}

#[test]
fn competitors_include_bindings_labels_types_comments_strings_and_nested_syntax() {
    for (file, source) in [
        ("a.js", "function h(arg) { arg(arg); send(arg, arg); /* arg */ const s = 'arg'; const f = arg => send(arg); }"),
        ("a.ts", "const f = ({arg}: {arg: string}) => send(arg);"),
        ("a.js", "const f = arg => ({arg: arg});"),
        ("a.py", "send(arg=arg) # arg"),
    ] {
        let parsed = parse(file, source);
        assert_eq!(parsed.parse_error_count, 0, "{file}");
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let expected = spans(source, "arg");
        assert!(expected.len() >= 3);
        assert_eq!(query(&facts, 1, "arg"), ReviewOccurrenceCensus::Complete(expected), "{file}");
    }
}

#[test]
fn exact_owner_envelopes_count_nested_tokens_but_reject_subranges() {
    let source = "function h(x) { const f = x => send(x); send(x); }\nfunction other(x) {}";
    let parsed = parse("a.js", source);
    let facts = ReviewOccurrenceFacts::new(&parsed);
    let outer = facts
        .owners()
        .iter()
        .find(|o| o.function.as_ref().is_some_and(|id| id.name == "h"))
        .unwrap();
    let inner = facts
        .owners()
        .iter()
        .find(|o| &source[o.span.clone()] == "x => send(x)")
        .unwrap();
    let first_line = source.lines().next().unwrap();
    assert_eq!(
        facts.literal_census(outer.span.clone(), 1, &AccessPath::simple("x")),
        ReviewOccurrenceCensus::Complete(spans(first_line, "x"))
    );
    let expected = spans(source, "x")
        .into_iter()
        .filter(|s| inner.span.start <= s.start && s.end <= inner.span.end)
        .collect();
    assert_eq!(
        facts.literal_census(inner.span.clone(), 1, &AccessPath::simple("x")),
        ReviewOccurrenceCensus::Complete(expected)
    );
    for (owner, line) in [
        (outer.span.start + 1..outer.span.end, 1),
        (outer.span.start..outer.span.end - 1, 1),
        (outer.span.clone(), 2),
        (0..source.len() + 1, 1),
    ] {
        assert_eq!(
            facts.literal_census(owner, line, &AccessPath::simple("x")),
            ReviewOccurrenceCensus::Unknown
        );
    }
    assert_eq!(
        query(&facts, 2, "x"),
        ReviewOccurrenceCensus::Complete(vec![
            source.rfind('x').unwrap()..source.rfind('x').unwrap() + 1
        ])
    );
}

#[test]
fn field_and_nonliteral_requests_stay_unknown() {
    let parsed = parse("a.js", "function h(req) { send(req.body); }");
    let facts = ReviewOccurrenceFacts::new(&parsed);
    for path in [
        AccessPath::from_expr("req.body"),
        AccessPath::simple(""),
        AccessPath::simple("1req"),
        AccessPath::simple("req-name"),
        AccessPath::simple("réq"),
        AccessPath::simple("r\\u0065q"),
    ] {
        assert_eq!(
            facts.literal_census(parsed.tree.root_node().byte_range(), 1, &path),
            ReviewOccurrenceCensus::Unknown
        );
    }
}

#[test]
fn uncertain_lines_refuse_even_zero_matches_without_poisoning_other_lines() {
    let source = "function h(x) {\n send(x); // café\n send(x); // \\u0078\n send(x);\n}";
    let parsed = parse("a.js", source);
    assert_eq!(parsed.parse_error_count, 0);
    let facts = ReviewOccurrenceFacts::new(&parsed);
    for line in [0, 2, 3, 6] {
        assert_eq!(
            query(&facts, line, "absent"),
            ReviewOccurrenceCensus::Unknown,
            "line {line}"
        );
    }
    assert_eq!(
        query(&facts, 4, "absent"),
        ReviewOccurrenceCensus::Complete(vec![])
    );
    let start = source.rfind('x').unwrap();
    assert_eq!(
        query(&facts, 4, "x"),
        ReviewOccurrenceCensus::Complete(vec![start..start + 1])
    );
}

#[test]
fn any_file_error_or_missing_syntax_refuses_clean_lines() {
    for source in [
        "function h(x) {\n send(x);\n}
@ @ @",
        "function h(x) {\n send(x);\n",
    ] {
        let parsed = parse("a.js", source);
        assert!(parsed.parse_error_count > 0);
        if !source.contains('@') {
            let mut pending = vec![parsed.tree.root_node()];
            let mut missing = false;
            while let Some(node) = pending.pop() {
                missing |= node.is_missing();
                pending.extend(node.children(&mut node.walk()));
            }
            assert!(missing, "fixture must include an actual missing node");
        }
        let facts = ReviewOccurrenceFacts::new(&parsed);
        for name in ["x", "absent"] {
            assert_eq!(query(&facts, 2, name), ReviewOccurrenceCensus::Unknown);
        }
    }
}

#[test]
fn maximal_tokens_and_crlf_keep_actual_multiline_bytes() {
    let source = "function h($x2, _x2) {\r\n send(\r\n $x2, _x2\r\n );\r\n}\r\n";
    let parsed = parse("a.js", source);
    assert_eq!(parsed.parse_error_count, 0);
    let facts = ReviewOccurrenceFacts::new(&parsed);
    for name in ["$x2", "_x2"] {
        let start = source.rfind(name).unwrap();
        assert_eq!(
            query(&facts, 3, name),
            ReviewOccurrenceCensus::Complete(vec![start..start + name.len()])
        );
    }
    for name in ["x2", "$x", "x", "_x"] {
        assert_eq!(
            query(&facts, 3, name),
            ReviewOccurrenceCensus::Complete(vec![])
        );
    }
    assert_eq!(
        query(&facts, 2, "$x2"),
        ReviewOccurrenceCensus::Complete(vec![])
    );
}

#[test]
fn unsupported_language_has_no_complete_census() {
    let parsed = parse("a.rs", "fn h(x: i32) { send(x); }");
    assert_eq!(parsed.parse_error_count, 0);
    assert_eq!(
        query(&ReviewOccurrenceFacts::new(&parsed), 1, "x"),
        ReviewOccurrenceCensus::Unknown
    );
}

#[test]
fn whole_tokens_crossing_bounds_refuse_without_becoming_suffixes() {
    // Low-level artificial ranges exercise the guard independently of owner admission.
    let index = LiteralIndex::new("left target right");
    assert_eq!(
        index.census(5..11, 1, "target"),
        ReviewOccurrenceCensus::Complete(vec![5..11])
    );
    for owner in [6..11, 5..10, 1..17] {
        assert_eq!(
            index.census(owner, 1, "absent"),
            ReviewOccurrenceCensus::Unknown
        );
    }
}
