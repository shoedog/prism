use crate::common::*;

#[test]
fn go_member_lvalue_span_is_real_extent() {
    let source = "package main\nfunc f(cfg Config) {\n    cfg.Timeout = 1\n}\n";
    let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();
    let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);
    let span = spans.iter().find(|span| span.path.has_fields()).unwrap();

    assert_eq!(&source[span.start_byte..span.end_byte], "cfg.Timeout");
}

#[test]
fn go_multi_target_lvalue_spans_are_per_target() {
    let source = "package main\nfunc f() {\n    a, b := pair()\n}\n";
    let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();
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
fn go_multiline_parameter_occurrence_uses_parameter_token() {
    let source = "package main\nfunc f(\n    x int,\n) int {\n    return x\n}\n";
    let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let params = parsed.function_parameter_occurrences(&func);
    let (start_byte, end_byte) = params
        .iter()
        .find(|(name, _, _)| name == "x")
        .map(|(_, start_byte, end_byte)| (*start_byte, *end_byte))
        .unwrap();

    assert_eq!(&source[start_byte..end_byte], "x");
    assert_eq!(parsed.line_for_byte(start_byte), 3);
}
