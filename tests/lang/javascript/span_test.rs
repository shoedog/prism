use crate::common::*;

#[test]
fn javascript_member_lvalue_span_is_real_extent() {
    let source = "function f(o) {\n    o.config.timeout = 1;\n}\n";
    let parsed = ParsedFile::parse("test.js", source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=3).collect();
    let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);
    let span = spans.iter().find(|span| span.path.has_fields()).unwrap();

    assert_eq!(&source[span.start_byte..span.end_byte], "o.config.timeout");
}

#[test]
fn javascript_destructuring_lvalue_spans_are_per_target() {
    let source = "function f(o) {\n    const { a, b } = o;\n}\n";
    let parsed = ParsedFile::parse("test.js", source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=3).collect();
    let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);

    for name in ["a", "b"] {
        let span = spans
            .iter()
            .find(|span| span.path.is_simple() && span.path.base == name)
            .unwrap();
        assert_eq!(&source[span.start_byte..span.end_byte], name);
    }
}

#[test]
fn javascript_multiline_parameter_occurrence_uses_parameter_token() {
    let source = "function f(\n    o\n) {\n    return o.name;\n}\n";
    let parsed = ParsedFile::parse("test.js", source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let params = parsed.function_parameter_occurrences(&func);
    let (start_byte, end_byte) = params
        .iter()
        .find(|(name, _, _)| name == "o")
        .map(|(_, start_byte, end_byte)| (*start_byte, *end_byte))
        .unwrap();

    assert_eq!(&source[start_byte..end_byte], "o");
    assert_eq!(parsed.line_for_byte(start_byte), 2);
}
