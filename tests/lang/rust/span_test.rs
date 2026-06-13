use crate::common::*;

#[test]
fn rust_member_lvalue_span_is_real_extent() {
    let source =
        "struct Config { timeout: i32 }\nfn f(mut cfg: Config) {\n    cfg.timeout = 1;\n}\n";
    let parsed = ParsedFile::parse("test.rs", source, Language::Rust).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();
    let spans = parsed.assignment_lvalue_spans_on_lines(&func, &lines);
    let span = spans.iter().find(|span| span.path.has_fields()).unwrap();

    assert_eq!(&source[span.start_byte..span.end_byte], "cfg.timeout");
}

#[test]
fn rust_destructuring_lvalue_spans_are_per_target() {
    let source = "fn f(pair: (i32, i32)) {\n    let (a, b) = pair;\n}\n";
    let parsed = ParsedFile::parse("test.rs", source, Language::Rust).unwrap();
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
fn rust_multiline_parameter_occurrence_uses_parameter_token() {
    let source = "fn f(\n    x: i32,\n) -> i32 {\n    x\n}\n";
    let parsed = ParsedFile::parse("test.rs", source, Language::Rust).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let params = parsed.function_parameter_occurrences(&func);
    let (start_byte, end_byte) = params
        .iter()
        .find(|(name, _, _)| name == "x")
        .map(|(_, start_byte, end_byte)| (*start_byte, *end_byte))
        .unwrap();

    assert_eq!(&source[start_byte..end_byte], "x");
    assert_eq!(parsed.line_for_byte(start_byte), 2);
}
