//! QUARANTINED (#829 TEST-FORM section 5): every test here pins the tentative design's census facts
//! (`unbalanced`, `loose_comma`, digest, name list), not required behaviour. They are `#[ignore]`d
//! until the redesign chooses its facts; run them with `cargo test -p prism --lib -- --ignored
//! macro_census_brackets`.
//! #829: census facts `unbalanced` (commit A, rows U01..U26) and `loose_comma` (commit B, rows
//! L01..L12). Row ids are in the test names so the mutation lane can cite them. The tests were
//! written against stubs (both facts always false) and are red on base until the scan lands.
use super::macro_census::{CMacroCensus, MacroFacts};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use std::sync::Arc;

fn lang_of(f: &str) -> Language {
    if f.ends_with(".cpp") || f.ends_with(".cc") || f.ends_with(".hpp") {
        Language::Cpp
    } else {
        Language::C
    }
}

fn census(src: &[(&str, &str)]) -> CMacroCensus {
    let files: BTreeMap<String, ParsedFile> = src
        .iter()
        .map(|(f, s)| (f.to_string(), ParsedFile::parse(f, s, lang_of(f)).unwrap()))
        .collect();
    CMacroCensus::from_files(&files)
}

/// A census over one parsed `.c` file plus census texts stamped on it, as the loader does for
/// a non-C include file (`defs.inc`).
fn census_with_text(text: &str) -> CMacroCensus {
    let mut p = ParsedFile::parse("a.c", "int f(void) { return 0; }\n", Language::C).unwrap();
    p.set_c_census_texts(Arc::new(BTreeMap::from([(
        "defs.inc".to_owned(),
        text.to_owned(),
    )])));
    CMacroCensus::from_files(&BTreeMap::from([("a.c".to_owned(), p)]))
}

fn fact<'a>(c: &'a CMacroCensus, name: &str) -> &'a MacroFacts {
    c.get(name).unwrap_or_else(|| panic!("{name} missing"))
}

fn unbalanced(c: &CMacroCensus, name: &str) -> bool {
    fact(c, name).unbalanced
}

fn loose(c: &CMacroCensus, name: &str) -> bool {
    fact(c, name).loose_comma
}

/// One `.c` file of `defines`, one flag per `(name, expected)`.
fn expect_unbalanced(file: &str, defines: &str, rows: &[(&str, bool)]) {
    let c = census(&[(file, defines)]);
    for (name, want) in rows {
        assert_eq!(unbalanced(&c, name), *want, "{name} in {defines:?}");
    }
}

fn expect_loose(file: &str, defines: &str, rows: &[(&str, bool)]) {
    let c = census(&[(file, defines)]);
    for (name, want) in rows {
        assert_eq!(loose(&c, name), *want, "{name} in {defines:?}");
    }
}

// ---- M1: `unbalanced` ----

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u01_open_brace() {
    expect_unbalanced("a.c", "#define BEGIN {\n", &[("BEGIN", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u02_close_brace() {
    expect_unbalanced("a.c", "#define END }\n", &[("END", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u03_open_paren() {
    expect_unbalanced("a.c", "#define OPEN for (\n", &[("OPEN", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u04_close_paren() {
    expect_unbalanced("a.c", "#define CLOSE )\n", &[("CLOSE", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u05_square_brackets() {
    expect_unbalanced(
        "a.c",
        "#define LB [\n#define RB ]\n",
        &[("LB", true), ("RB", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u06_crossing_kinds_with_equal_counts() {
    expect_unbalanced("a.c", "#define CROSS ( { ) }\n", &[("CROSS", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u07_closer_first_with_equal_counts() {
    expect_unbalanced("a.c", "#define FLIP } {\n", &[("FLIP", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u08_function_like_opens_a_block() {
    expect_unbalanced("a.c", "#define OPEN_IF(c) if (c) {\n", &[("OPEN_IF", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u09_well_nested_bodies_are_balanced() {
    expect_unbalanced(
        "a.c",
        "#define WRAP(v) { f(v); }\n#define AT(a, i) ((a)[(i)])\n#define STMT do { g(); } while (0)\n#define EMPTY\n",
        &[
            ("WRAP", false),
            ("AT", false),
            ("STMT", false),
            ("EMPTY", false),
        ],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u10_brackets_in_literals_do_not_count() {
    expect_unbalanced(
        "a.c",
        "#define MSG \"{ ( [\"\n#define CH '}'\n#define QUOTE '\\''\n#define ESC \"\\\"{\"\n",
        &[
            ("MSG", false),
            ("CH", false),
            ("QUOTE", false),
            ("ESC", false),
        ],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u11_brackets_in_comments_do_not_count() {
    expect_unbalanced(
        "a.c",
        "#define CMT1 1 /* { */ + 2\n#define CMT2 3 // (\n",
        &[("CMT1", false), ("CMT2", false)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u12_cpp_raw_string_and_digit_separator() {
    expect_unbalanced(
        "a.cpp",
        "#define RS R\"x({)x\"\n#define SEP 1'000 + (2)\n",
        &[("RS", false), ("SEP", false)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u13_digraph_brackets() {
    expect_unbalanced(
        "a.c",
        "#define DLB <%\n#define DRB %>\n#define DLS <:\n#define DRS :>\n",
        &[("DLB", true), ("DRB", true), ("DLS", true), ("DRS", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u14_closure_over_referenced_macros() {
    expect_unbalanced(
        "a.c",
        "#define BEGIN {\n#define END }\n#define ENTER BEGIN\n#define ENTER2 ENTER\n#define BOTH(s) BEGIN s; END\n",
        &[
            ("BEGIN", true),
            ("END", true),
            ("ENTER", true),
            ("ENTER2", true),
            ("BOTH", true),
        ],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u15_any_definition_taints_the_spelling() {
    // Same file, either order.
    expect_unbalanced("a.c", "#define DUAL 0\n#define DUAL {\n", &[("DUAL", true)]);
    expect_unbalanced("a.c", "#define DUAL {\n#define DUAL 0\n", &[("DUAL", true)]);
    // Two files.
    let c = census(&[("x.c", "#define DUAL 0\n"), ("y.c", "#define DUAL {\n")]);
    assert!(unbalanced(&c, "DUAL"));
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u16_line_continuation_in_a_body() {
    expect_unbalanced(
        "a.c",
        "#define CONT { \\\n}\n#define CONT2 f( \\\n g(\n",
        &[("CONT", false), ("CONT2", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u17_multi_line_comment_in_a_body() {
    expect_unbalanced(
        "a.c",
        "#define MC ( /* a\n b */ )\n#define MC2 { /* a\n b */\n",
        &[("MC", false), ("MC2", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u18_va_opt_parentheses() {
    expect_unbalanced(
        "a.c",
        "#define VO(...) __VA_OPT__({)\n#define VO2(...) f(__VA_OPT__(1))\n",
        &[("VO", true), ("VO2", false)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u19_unterminated_literal_is_poisoned() {
    let c = census(&[("a.c", "#define BADQ \"abc {\n")]);
    assert!(fact(&c, "BADQ").poisoned, "no assertion on `unbalanced`");
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u20_define_inside_a_block_comment_counts_pinned() {
    // Accepted over-approximation: a readable `#define` in a comment still creates facts.
    expect_unbalanced(
        "a.c",
        "/*\n#define CBEGIN {\n*/\nint f(void) { return 0; }\n",
        &[("CBEGIN", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u21_trigraph_body_is_the_existing_incomplete_belt() {
    let c = census(&[("a.c", "#define TB ??<\n")]);
    assert!(c.incomplete());
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u22_permissive_census_text() {
    let c = census_with_text(
        "#define PBEGIN {\n#define PMSG \"(\"\n#define PQ \" {\n#define PF(a, b) g(a, b)\n",
    );
    assert!(unbalanced(&c, "PBEGIN"));
    assert!(!unbalanced(&c, "PMSG"), "a readable literal stays clean");
    assert!(
        unbalanced(&c, "PQ"),
        "an unreadable line is counted as written"
    );
    assert!(!unbalanced(&c, "PF"));
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u23_undef_does_not_clear() {
    expect_unbalanced("a.c", "#define BEGIN {\n#undef BEGIN\n", &[("BEGIN", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u24_digest_covers_the_bracket_fact() {
    let a = census(&[("a.c", "#define X {}\n")]);
    let b = census(&[("a.c", "#define X {\n")]);
    assert_ne!(a.digest(), b.digest());
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u25_stringizing_holds_no_bracket() {
    expect_unbalanced(
        "a.c",
        "#define S(x) #x\n#define H(x) (#x \"(\")\n",
        &[("S", false), ("H", false)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn u26_function_like_body_only() {
    expect_unbalanced("a.c", "#define FN(a) (a\n", &[("FN", true)]);
}

// ---- M4: `loose_comma` (commit B) ----

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l01_top_level_comma() {
    expect_loose("a.c", "#define PAIR 1, 2\n", &[("PAIR", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l02_brace_held_comma() {
    expect_loose("a.c", "#define INIT {0, 0}\n", &[("INIT", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l03_commas_directly_inside_parentheses() {
    expect_loose(
        "a.c",
        "#define F2(a, b) f(a, b)\n#define FORH for (i = 0, j = 0; i < n; i++)\n",
        &[("F2", false), ("FORH", false)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l04_va_args_position() {
    expect_loose(
        "a.c",
        "#define FWD(...) __VA_ARGS__\n#define CALL(...) f(__VA_ARGS__)\n#define BR(...) {__VA_ARGS__}\n",
        &[("FWD", true), ("CALL", false), ("BR", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l05_va_opt_parenthesis_is_transparent() {
    expect_loose(
        "a.c",
        "#define TOP(...) __VA_OPT__(,) __VA_ARGS__\n#define LOG(fmt, ...) printf(fmt __VA_OPT__(,) __VA_ARGS__)\n",
        &[("TOP", true), ("LOG", false)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l06_gnu_named_variadic_fails_closed() {
    expect_loose(
        "a.c",
        "#define E(fmt, args...) printf(fmt, args)\n",
        &[("E", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l07_statement_expression_declarators() {
    expect_loose(
        "a.c",
        "#define SE(x) ({ int a = (x), b = 0; a + b; })\n",
        &[("SE", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l08_comma_inside_square_brackets() {
    expect_loose("a.c", "#define SUB(a, i, j) a[i, j]\n", &[("SUB", true)]);
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l09_closure_is_blanket_pinned() {
    expect_loose(
        "a.c",
        "#define PAIR 1, 2\n#define W2 PAIR\n#define CALLP f(PAIR)\n",
        &[("PAIR", true), ("W2", true), ("CALLP", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l10_cpp_template_arguments() {
    expect_loose(
        "a.cpp",
        "#define MAP std::map<int, int>\n",
        &[("MAP", true)],
    );
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l11_permissive_census_text() {
    let c = census_with_text(
        "#define PPAIR 1, 2\n#define PG(a, b) g(a, b)\n#define PE(args...) g(args)\n",
    );
    assert!(loose(&c, "PPAIR"));
    assert!(!loose(&c, "PG"));
    assert!(loose(&c, "PE"));
}

#[test]
#[ignore = "tentative design (#829 TEST-FORM §5)"]
fn l12_commas_in_literals_do_not_count() {
    expect_loose(
        "a.c",
        "#define SC \"a, b\"\n#define CC ','\n",
        &[("SC", false), ("CC", false)],
    );
}
