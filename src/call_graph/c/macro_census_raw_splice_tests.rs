//! #630 slice 6d-1: C++ raw string literals with line splices in their body. C++
//! [lex.pptoken]/3 reverts line splicing (and trigraphs, which the census cannot revert: it fails closed) between the opening quote
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
    // As written `)` newline `"` is no `)"`, but the naive reading closes there: the two
    // readings disagree, so the literal is uncertain (fold 3). M is still read by the
    // comment-kept pass.
    let c = cpp("const char *s = R\"(abc)\\\n\" /* )\";\n#define M x\n/* */\n");
    assert!(c.incomplete(), "{:?}", c.reasons);
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

// ---- Fold 2: directive-like leaders and trigraphs. ----

#[test]
fn directive_leaders_of_any_spelling_fail_closed() {
    for lead in ["\x0b", "\x0c", " \t", "/*c*/ ", "/* c */\t"] {
        let src = format!("{lead}#define A (R\"(x)\\\n\" /* )\", q + 1)\n");
        let c = cpp(&src);
        assert!(c.incomplete(), "{src:?}: {:?}", c.reasons);
    }
    for src in [
        "%:define A (R\"(x)\\\n\" /* )\", q + 1)\n",
        "??=define A (R\"(x)\\\n\" /* )\", q + 1)\n",
    ] {
        let c = cpp(src);
        assert!(c.incomplete(), "{src:?}: {:?}", c.reasons);
    }
    let c = cpp("\x0b#define A (R\"(x)\\\n y)\", q + 1)\n");
    let r = &c.incomplete_reasons().first[0];
    assert_eq!(r.kind, "raw_string_splice_in_directive");
    assert_eq!(r.line, 1);
    assert!(r.snippet.contains("define"));
}

#[test]
fn lenient_only_splice_in_a_directive_raw_string_fails_closed() {
    let c = cpp("#define A (R\"(x)\\ \n\" /* )\", q + 1)\n");
    assert!(c.incomplete(), "{:?}", c.reasons);
}

#[test]
fn trigraph_in_a_file_with_a_raw_string_candidate_fails_closed() {
    // sol W2: `??)` closes the literal in the compiler's text; every census pass misses the
    // real define after the `/* real */`.
    let src = "const char *s = R??/\n\"(a\\\nb\nx /* fake\n??)\";\n/* real\n*/ #define M hidden\nconst char *t = \")\";\n";
    let c = cpp(src);
    assert!(c.incomplete(), "{:?}", c.reasons);
    assert_eq!(c.incomplete_reasons().first[0].kind, "raw_string_trigraph");
    // Without a raw-string candidate a trigraph is no census concern here.
    let c = cpp("const char *s = \"??)\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    // A trigraph-free protobuf shape stays complete.
    let c =
        cpp("void f() {\n  p->Emit(R\"cc(\n#if (a) && \\\n  b\n#endif\n)cc\");\n}\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
}

#[test]
fn owner_detector_refuses_trigraphs_beside_a_raw_string_candidate() {
    use super::macro_census::has_spliced_raw_string as h;
    assert!(h("int f(){ const char *s = R\"(a??)\"; }"));
    assert!(!h("int f(){ const char *s = R\"(a)\"; int t = 0; }"));
    assert!(!h("int f(){ const char *s = \"??)\"; }"));
}

// ---- Fold 3: both readings must agree; trigraph rule needs a recognised raw string. ----

fn parsed(name: &str, src: &str) -> CMacroCensus {
    let lang = if name.ends_with(".c") || name.ends_with(".h") {
        Language::C
    } else {
        Language::Cpp
    };
    let mut m = BTreeMap::new();
    m.insert(
        name.to_string(),
        ParsedFile::parse(name, src, lang).unwrap(),
    );
    CMacroCensus::from_files(&m)
}

fn safe(c: &CMacroCensus) -> bool {
    c.incomplete() || c.get("M").is_some_and(|m| m.free.contains("hidden"))
}

#[test]
fn readings_that_disagree_fail_closed() {
    // sol R3-W1: in C an ordinary string, in C++ a raw string whose as-written end is
    // inside `t`; every C pass misses the define after `*/`.
    let body = "#define R\nconst char *s = R\"(a)\\\n\"; /* real\n*/ #define M hidden\nconst char *t = \")\";\nint ordinary(int q) { q = 1; return q + 1; }\n";
    // Opus S11: the compiler might not revert the splice (MSVC unverified).
    let s11 = "const char *s = R\"x(\"/*)x\\\n\";\n/* )x\";\n*/ #define M hidden\nint ordinary(int q) { q = 1; return q + 1; }\n";
    for src in [body, s11] {
        for name in ["a.c", "a.cpp"] {
            for variant in [src.to_string(), src.replace('\n', "\r\n")] {
                let c = parsed(name, &variant);
                assert!(safe(&c), "{name} {variant:?}: {:?}", c.reasons);
            }
        }
    }
}

#[test]
fn agreeing_readings_with_an_interior_splice_stay_complete() {
    // The splice sits in the body; the first `)"` is the same place either way.
    let c = cpp("const char *k = R\"(a \\\n b\n)\";\nint x;\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
}

#[test]
fn trigraph_text_without_a_raw_string_stays_complete() {
    let c = cpp("const char *e = \"ERROR\";\nconst char *d = \"????-??-??\";\n#define M x\n");
    assert!(!c.incomplete(), "{:?}", c.reasons);
    reads_m(&c);
    use super::macro_census::has_spliced_raw_string as h;
    assert!(!h(
        "int f(){ const char *e = \"ERROR\"; const char *d = \"??-\"; }"
    ));
    // With a recognised raw string the trigraph still refuses.
    assert!(h(
        "int f(){ const char *e = R\"(x)\"; const char *d = \"??-\"; }"
    ));
}

// ---- Fold 4: the as-written acceptance is for files parsed as C++ only. ----

#[test]
fn c_parsed_files_keep_the_base_behaviour_for_spliced_raw_candidates() {
    // sol W1 / Opus R4-1: in C `R` expands away and `M` is real; both C++ readings close
    // inside `t`, but every C pass loses `M` behind `*/ #define`.
    let src = "#define R\nconst char *s = R\"(a\\\n\"; /* real\n*/ #define M hidden\nconst char *t = \")\";\nint f(int hidden) { hidden = M; return hidden + 1; }\n";
    for name in ["a.c", "a.h"] {
        for text in [src.to_string(), src.replace('\n', "\r\n")] {
            let c = parsed(name, &text);
            assert!(c.incomplete(), "{name} {text:?}: {:?}", c.reasons);
        }
    }
    // A plain spliced candidate in a C file, no comment trick: incomplete as on base.
    assert!(parsed("a.c", "const char *s = R\"(a\\\nb)\";\n#define M x\n").incomplete());
    // The same text as C++: complete, `M` is real and read (the literal is raw content).
    for text in [src.to_string(), src.replace('\n', "\r\n")] {
        let c = parsed("a.cpp", &text);
        assert!(!c.incomplete(), "{:?}", c.reasons);
    }
    // The protobuf shape as `.cc` stays complete.
    let c = parsed(
        "a.cc",
        "void f() {\n  p->Emit(R\"cc(\n#if (a) && \\\n  b\n#endif\n)cc\");\n}\n#define M x\n",
    );
    assert!(!c.incomplete(), "{:?}", c.reasons);
}

// ---- Fold 5: a block comment keeps its newlines outside directive lines (#812 item 3). ----

fn m_hidden(c: &CMacroCensus) {
    let m = c.get("M").expect("M read");
    assert!(m.free.contains("hidden"), "M.free = {:?}", m.free);
}

#[test]
fn directive_after_a_multi_line_block_comment_is_read() {
    let src = "int a; /* x\n*/ #define M hidden\n";
    for name in ["a.c", "a.cpp"] {
        m_hidden(&parsed(name, src));
        m_hidden(&parsed(name, &src.replace('\n', "\r\n")));
    }
    // A comment inside a directive keeps the body whole.
    let c = parsed("a.c", "#define N a /* x\ny */ b\n");
    let n = c.get("N").unwrap();
    assert!(n.free.contains("a") && n.free.contains("b"), "{:?}", n.free);
}

#[test]
fn earlier_round_inputs_read_m_without_relying_on_incomplete() {
    // r3 (sol R3-W1), r4 (R4-1) and the Opus S11 shapes, as `.c`.
    let r3 = "#define R\nconst char *s = R\"(a)\\\n\"; /* real\n*/ #define M hidden\nconst char *t = \")\";\n";
    let s11 = "const char *s = R\"x(\"/*)x\\\n\";\n/* )x\";\n*/ #define M hidden\n";
    for src in [r3, s11] {
        m_hidden(&parsed("a.c", src));
    }
    // r5: a C++ header holds the define, `a.c` uses `M`; one census.
    let mut m = BTreeMap::new();
    m.insert(
        "shared.hpp".to_string(),
        ParsedFile::parse("shared.hpp", r3, Language::Cpp).unwrap(),
    );
    m.insert(
        "a.c".to_string(),
        ParsedFile::parse(
            "a.c",
            "int f(int hidden) { hidden = M; return hidden + 1; }\n",
            Language::C,
        )
        .unwrap(),
    );
    m_hidden(&CMacroCensus::from_files(&m));
}
