//! #630 slice 6d-1: C++ raw string literals with line splices in their body. C++
//! [lex.pptoken]/3 reverts phases 1-2 (trigraphs, line splicing) between the opening quote
//! and the closing delimiter, so the closing `)delim"` is searched as written.
use super::macro_census::CMacroCensus;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;

fn cpp(src: &str) -> CMacroCensus {
    let mut m = BTreeMap::new();
    m.insert(
        "a.cpp".to_string(),
        ParsedFile::parse("a.cpp", src, Language::Cpp).unwrap(),
    );
    CMacroCensus::from_files(&m)
}

fn reads_m(c: &CMacroCensus) {
    let m = c.get("M").expect("M read");
    assert!(m.free.contains("x"), "M.free = {:?}", m.free);
}

#[test]
fn protobuf_shape_backslash_line_in_raw_string_is_complete() {
    let c = cpp("const char *k = R\"(\n#if (a) && \\\n  b\n#endif\n)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
}

#[test]
fn closing_delimiter_only_on_the_spliced_text_is_not_a_close() {
    // As written `)` newline `"` is no `)"`: the literal runs on to the real end.
    let c = cpp("const char *s = R\"(abc)\\\n\" /* )\";\n#define M x\n/* */\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
    // With no later `)"` it is unterminated: fail closed.
    assert!(cpp("const char *s = R\"(abc)\\\n\";\n#define M x\n").incomplete());
}

#[test]
fn splice_before_the_opening_quote_is_applied() {
    let c = cpp("const char *s = R\\\n\"(a)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
}

#[test]
fn splice_inside_the_delimiter_fails_closed() {
    // Reverted, the delimiter holds a backslash: ill-formed.
    assert!(cpp("const char *s = R\"ab\\\nc(x)abc\";\n#define M x\n").incomplete());
    assert!(cpp("const char *s = R\"\\\n(x)\";\n#define M x\n").incomplete());
    assert!(cpp("const char *s = R\"ab\\\n(x)ab\";\n#define M x\n").incomplete());
}

#[test]
fn lenient_blank_splice_in_the_body_is_content() {
    // Backslash, blank, newline: a splice only in the lenient mode; text either way.
    let c = cpp("const char *s = R\"(a \\ \n b)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
}

#[test]
fn unterminated_raw_string_stays_incomplete() {
    assert!(cpp("const char *s = R\"(open\n#define M x\n").incomplete());
    assert!(cpp("const char *s = R\"(open \\\n more\n#define M x\n").incomplete());
}

#[test]
fn lexing_after_the_literal_is_unchanged() {
    let c = cpp(
        "const char *s = R\"(a \\\n b)\"; int y = 1; /* c */\n#define A R\"(z)\" + q\n#define M x\n",
    );
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
    assert!(c.get("A").unwrap().free.contains("q"));
}

#[test]
fn define_inside_a_raw_string_is_counted_only_by_the_unioned_over_approximating_passes() {
    // Pinned as observed: the C-dialect and comment-kept passes (and the permissive scan)
    // have no raw-string tracking by design, so a line-leading `#define` in the body is an
    // over-approximate fact; the census stays complete.
    let c = cpp("const char *k = R\"(\n#define INNER y\n)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
    assert!(c.get("INNER").is_some());
}

// ---- Fold 1: later readers of spliced text cannot see splice points. ----

#[test]
fn spliced_raw_string_on_a_directive_line_is_incomplete() {
    for src in [
        "#define M R\"(a)\\\n\" )\" + x // \"\n",
        "#define A (R\"(x)\\\n\" /* )\", q + 1)\n",
    ] {
        let c = cpp(src);
        assert!(c.incomplete(), "{src:?} must fail closed: {:?}", c.reasons);
    }
}

#[test]
fn spliced_raw_string_outside_directives_keeps_the_census_complete() {
    let c = cpp("const char *k = R\"(a)\\\n b)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    // The directive rule is about the start line: a `#`-looking line INSIDE the literal is text.
    let c = cpp("const char *k = R\"(\n#define Z R\"(a)\\\n b\n)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
}

#[test]
fn spliced_raw_string_detector() {
    use super::macro_census::has_spliced_raw_string as h;
    assert!(h("int f(){ const char *s = R\"(a\\\nb)\"; }"));
    assert!(h("int f(){ const char *s = R\"(open; }"));
    assert!(!h("int f(){ const char *s = R\"(ab)\"; }"));
    assert!(!h("int f(){ const char *s = \"a\\\nb\"; int R = 1; }"));
    // The cheap pre-filter must still see a prefix split from its quote by line splices
    // (LF, CRLF, lone CR, blanks, the `??/` trigraph), each with a spliced body.
    for split in ["\\\n", "\\\r\n", "\\\r", "\\ \t\n", "??/\n", "\\\n\\\n"] {
        let src = format!("int f(){{ const char *s = R{split}\"(a\\\nb)\"; }}");
        assert!(h(&src), "{src:?}");
    }
    // An `R` inside an ordinary string passes the pre-filter; the lexer still finds no raw string.
    assert!(!h(
        "int f(){ int R = 1; const char *s = \"R\\\n\"; return R; }"
    ));
}

#[test]
fn match_boundaries_against_splice_points() {
    use super::macro_census::raw_string_end_for_tests as e;
    // `"(a)"`: the closing `)"` is at 3..5.
    let b = br#""(a)""#;
    assert_eq!(e(b, 0, &[]), Some(5));
    // A splice exactly at the match start (before `)`): outside the match, accepted.
    assert_eq!(e(b, 0, &[3]), Some(5));
    // A splice right after the closing quote: accepted.
    assert_eq!(e(b, 0, &[5]), Some(5));
    // A splice between `)` and `"`: inside the match, rejected.
    assert_eq!(e(b, 0, &[4]), None);
}
