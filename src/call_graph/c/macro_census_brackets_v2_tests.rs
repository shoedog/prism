//! #829 DESIGN-v2 5.1 / PLAN-v2 tasks 3 to 5: the structural tokens, the body facts and potential
//! groups, the closure and the digest. Positive and negative rows sit side by side. The facts of
//! one census are read through `MacroFacts` (`unbalanced`, `loose_comma`, `arg_split`,
//! `paren_open`, closed over the macros a body names).
use super::macro_census::{
    macro_call_arguments, texts_digest, tokens_for_tests, CMacroCensus, MacroFacts,
};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use std::sync::Arc;

fn census(file: &str, src: &str) -> CMacroCensus {
    let lang = if file.ends_with(".cc") {
        Language::Cpp
    } else {
        Language::C
    };
    let p = ParsedFile::parse(file, src, lang).unwrap();
    CMacroCensus::from_files(&BTreeMap::from([(file.to_owned(), p)]))
}

fn text_census(text: &str) -> CMacroCensus {
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

/// `arg_split` of every `(name, want)` in a parsed `.c` and a parsed `.cc`.
fn expect_split(src: &str, rows: &[(&str, bool)]) {
    for file in ["a.c", "a.cc"] {
        let c = census(file, src);
        for (name, want) in rows {
            assert_eq!(fact(&c, name).arg_split, *want, "{file}: {name} in {src:?}");
        }
    }
}

// ---- task 3: tokens ----

/// The structural tokens of `text` (the `other` tokens of numbers, literals and punctuation
/// left out, see `t3_other_tokens_keep_a_paren_after_punctuation_plain`).
fn toks(text: &str) -> Vec<String> {
    structural(tokens_for_tests(text, false).unwrap())
}

fn structural(v: Vec<String>) -> Vec<String> {
    v.into_iter().filter(|t| t != "other").collect()
}

#[test]
fn t3_structural_tokens_in_code() {
    assert_eq!(
        toks("a { b , [ c ] } ( d )"),
        ["id:a", "{", "id:b", ",", "[", "id:c", "]", "}", "(", "id:d", ")"]
    );
}

#[test]
fn t3_literals_and_comments_carry_no_structural_token() {
    for text in ["\"{ , [ ) (\"", "'{' ','", "/* { , ] */", "// { , ]"] {
        for cxx in [false, true] {
            assert_eq!(
                structural(tokens_for_tests(text, cxx).unwrap_or_default()),
                Vec::<String>::new(),
                "{text:?} cxx={cxx}"
            );
        }
    }
    // A C++ raw string is one literal; in C `R` is an identifier and the quote opens a string.
    let raw = "R\"x({ , [)x\" { ,";
    assert_eq!(structural(tokens_for_tests(raw, true).unwrap()), ["{", ","]);
    assert_eq!(
        structural(tokens_for_tests(raw, false).unwrap()),
        ["id:R", "{", ","]
    );
}

/// Numbers, literals and other punctuation are `other` tokens: they stand between an
/// identifier and a `(` and make that paren plain.
#[test]
fn t3_other_tokens_keep_a_paren_after_punctuation_plain() {
    assert_eq!(
        tokens_for_tests("a + ( 1 ) \"s\" ( ; (", false).unwrap(),
        ["id:a", "other", "(", "other", ")", "other", "(", "other", "("]
    );
    let c = census(
        "a.c",
        "#define X(a) a + ({ b, c })\n#define Y(a) a ({ b, c })\n",
    );
    assert!(!fact(&c, "X").arg_split, "after punctuation: plain");
    assert!(fact(&c, "Y").arg_split, "after an identifier: potential");
}

/// Digit separators: `1'000` is one number in C++ (no character literal), and in C a `'`
/// opens a character literal that hides the comma and brace after it. The census reads both.
#[test]
fn t3_digit_separators_in_both_dialects() {
    let text = "1'000 , {";
    assert_eq!(
        structural(tokens_for_tests(text, true).unwrap()),
        [",", "{"]
    );
    // C dialect: `'000 , {` is an unterminated character literal.
    assert!(tokens_for_tests(text, false).is_none());
}

#[test]
fn t3_each_digraph_is_one_token() {
    for (d, kind) in [("<%", '{'), ("%>", '}'), ("<:", '['), (":>", ']')] {
        assert_eq!(
            toks(&format!("a {d} b")),
            [
                "id:a".to_owned(),
                format!("digraph:{kind}"),
                "id:b".to_owned()
            ],
            "{d}"
        );
    }
    // `%:` is `#` and `%:%:` is `##`: no structural token, `##` still pastes.
    assert_eq!(toks("a %: b"), ["id:a", "id:b"]);
    let c = census("a.c", "#define P(a, b) a %:%: b\n");
    assert!(fact(&c, "P").poisoned);
}

/// A digraph is the bracket it spells: a balanced pair nests, a crossing pair or a lone one
/// does not, and a comma held by a digraph brace is held.
#[test]
fn t3_digraphs_are_read_as_their_brackets() {
    let c = census(
        "a.c",
        "#define B1 <% a %>\n#define B2 <: a :>\n#define X1 <% a )\n#define X2 <: a %>\n\
         #define X3 <%\n#define H1 f(<% a, b %>)\n#define H2 f(<: a, b :>)\n#define T1 f((a, b))\n",
    );
    for (n, unb, split) in [
        ("B1", false, false),
        ("B2", false, false),
        ("X1", true, false),
        ("X2", true, false),
        ("X3", true, false),
        ("H1", false, true),
        ("H2", false, true),
        ("T1", false, false),
    ] {
        assert_eq!(fact(&c, n).unbalanced, unb, "{n}.unbalanced");
        assert_eq!(fact(&c, n).arg_split, split, "{n}.arg_split");
    }
}

/// [lex.pptoken]/3.2 (checked with `clang++ -std=c++17 -Xclang -dump-tokens` against `clang -std=gnu11`):
/// in C++ a `<` followed by `::` and then neither `:` nor `>` is its own token, not the digraph
/// `<:`. `<::value]` is `less coloncolon identifier r_square` there (an unmatched `]`) and
/// `l_square colon identifier r_square` in C, so the facts are dialect-dependent and the census
/// ORs both readings. `<:::` and `<::>` are the digraph in both.
#[test]
fn t3_cxx_less_colon_colon_exception() {
    let cxx = tokens_for_tests("<::value]", true).unwrap();
    assert_eq!(
        structural(cxx.clone()),
        ["id:value", "]"],
        "C++: a plain `<`"
    );
    assert_eq!(cxx[0], "other");
    let c = tokens_for_tests("<::value]", false).unwrap();
    assert_eq!(
        structural(c),
        ["digraph:[", "id:value", "]"],
        "C: the digraph"
    );
    for text in ["<:::x]", "<::>"] {
        for dialect in [false, true] {
            assert_eq!(
                tokens_for_tests(text, dialect).unwrap()[0],
                "digraph:[",
                "{text} cxx={dialect}"
            );
        }
    }
}

#[test]
fn t4_cxx_less_colon_colon_exception_sets_the_facts_everywhere() {
    let defs = "#define BAD <::value]\n#define OK1 <::>\n#define OK2 <:::x]\n#define OK3 <: a :>\n";
    let rows = [
        ("BAD", true),
        ("OK1", false),
        ("OK2", false),
        ("OK3", false),
    ];
    // A C++ file, a `.c` file, a `.h` header (parsed as C): every parsed file is read in both
    // dialects, and so is a census text.
    for file in ["a.cc", "a.c", "a.h"] {
        let c = census(file, defs);
        for (n, unb) in rows {
            assert_eq!(fact(&c, n).unbalanced, unb, "{file}: {n}");
        }
    }
    let c = text_census(defs);
    for (n, unb) in rows {
        assert_eq!(fact(&c, n).unbalanced, unb, "census text: {n}");
    }
    // The hazard travels through a wrapper.
    let c = census("a.cc", "#define BAD <::value]\n#define W BAD\n");
    assert!(fact(&c, "W").unbalanced);
}

/// `macro_call_arguments` sees identifiers and parentheses only: the structural tokens must not
/// change which names are adjacent (`F , (x)` still reads `x` as the group of `F`).
#[test]
fn t3_macro_call_arguments_is_unchanged() {
    let is_macro = |n: &str| matches!(n, "F" | "G");
    let got = |src: &str| macro_call_arguments(src, &is_macro).unwrap();
    let set = |v: &[&str]| v.iter().map(|s| s.to_string()).collect();
    assert_eq!(got("F(a, b) { c }"), set(&["a", "b"]));
    assert_eq!(got("F , (x)"), set(&["x"]));
    assert_eq!(got("G{(y)}"), set(&["y"]));
    assert_eq!(got("F(a)(b)[ (c) ]"), set(&["a", "b", "c"]));
    assert_eq!(got("x { F ( <% z %> ) }"), set(&["z"]));
    assert_eq!(got("plain(a, b)"), set(&[]));
}

// ---- task 4: potential groups, held commas, forwarded names ----

#[test]
fn t4_held_commas_in_a_potential_group_set_arg_split() {
    expect_split(
        "#define G1 K({ a, b })\n\
         #define G2 f((T){a, b})\n\
         #define G3(...) foo_impl((struct o){__VA_ARGS__})\n\
         #define G4(p) p({ a, b })\n\
         #define G5 K(x)({ a, b })\n\
         #define G6(...) __VA_OPT__(K)(a, { b, c })\n\
         #define G7 K(a, [ b, c ])\n",
        &[
            ("G1", true),
            ("G2", true),
            ("G3", true),
            ("G4", true),
            ("G5", true),
            ("G6", true),
            ("G7", true),
        ],
    );
}

#[test]
fn t4_tight_commas_and_commas_outside_every_potential_group_do_not() {
    expect_split(
        "#define N1 K(a, b)\n\
         #define N2 K((a, b))\n\
         #define N3 f((a, b))\n\
         #define N4 { 1, 2 }\n\
         #define N5 (T){ a, b }\n\
         #define N6 K(a)\n\
         #define N7 + ({ a, b })\n",
        &[
            ("N1", false),
            ("N2", false),
            ("N3", false),
            ("N4", false),
            ("N5", false),
            ("N6", false),
            ("N7", false),
        ],
    );
}

/// A group after punctuation or at the start of the body is plain: `, ({ a, b })` and
/// `({ a, b })` hold a comma that no callee can split here.
#[test]
fn t4_a_paren_after_punctuation_is_plain() {
    expect_split(
        "#define P1 x , ({ a, b })\n#define P2 ({ a, b })\n#define P3 ; ({ a, b })\n",
        &[("P1", false), ("P2", false), ("P3", false)],
    );
}

#[test]
fn t4_forwarded_names_inside_a_potential_group() {
    expect_split(
        "#define INIT { 1, 2 }\n\
         #define HI H(INIT)\n\
         #define HE ext(INIT)\n\
         #define KEEP { INIT }\n\
         #define PL(INIT2) f(INIT2)\n\
         #define INIT2 { 1, 2 }\n\
         #define TIGHT H((INIT))\n",
        &[
            ("HI", true),
            ("HE", true),
            ("KEEP", false),
            ("PL", false),
            ("TIGHT", true),
        ],
    );
}

/// Sol's identity wrapper in a body: external `EXT` and empty `E`, an in-repo `sizeof(p, q, r)`.
#[test]
fn t4_identity_wrapper_body_with_a_nested_group() {
    expect_split(
        "#define sizeof(p, q, r) q if (r) p\n\
         #define HI EXT(sizeof E({ int a, b; }, c))\n\
         #define OK EXT(sizeof E((int a, b), c))\n",
        &[("HI", true), ("OK", false)],
    );
}

#[test]
fn t4_stringizing_is_no_structural_token() {
    // `#p` replaces the argument by one literal; `"}"` is a literal: no bracket, no comma.
    let c = census(
        "a.c",
        "#define S(p) #p\n#define L \"}\"\n#define B(p) { #p }\n#define C(p) f({ #p })\n",
    );
    for n in ["S", "L", "B", "C"] {
        assert!(!fact(&c, n).unbalanced, "{n}");
        assert!(!fact(&c, n).loose_comma, "{n}");
        assert!(!fact(&c, n).arg_split, "{n}");
    }
}

#[test]
fn t4_named_variadic_fails_closed() {
    for file in ["a.c", "a.cc"] {
        let c = census(
            file,
            "#define NV(args...) f(args)\n#define OV(...) f(__VA_ARGS__)\n",
        );
        assert!(
            fact(&c, "NV").loose_comma && fact(&c, "NV").arg_split,
            "{file}"
        );
        // The standard variadic keeps its commas inside f's own parentheses.
        assert!(
            !fact(&c, "OV").loose_comma && !fact(&c, "OV").arg_split,
            "{file}"
        );
    }
}

#[test]
fn t4_va_opt_parentheses_are_transparent() {
    let c = census(
        "a.c",
        "#define V1(...) __VA_OPT__(,) x\n#define V2(...) f(__VA_OPT__(a, b))\n\
         #define V3(...) __VA_OPT__(f)({ a, b })\n",
    );
    // Its parentheses are not emitted, so the comma is not tight in V1 ...
    assert!(fact(&c, "V1").loose_comma);
    // ... and is tight inside the parentheses of f in V2.
    assert!(!fact(&c, "V2").loose_comma);
    // `__VA_OPT__(f)` may emit a callee name: the group after its close is potential.
    assert!(fact(&c, "V3").arg_split);
    assert!(!fact(&c, "V1").paren_open && !fact(&c, "V3").paren_open);
}

fn expect_open(src: &str, rows: &[(&str, bool)]) {
    for file in ["a.c", "a.cc"] {
        let c = census(file, src);
        for (name, want) in rows {
            assert_eq!(
                fact(&c, name).paren_open,
                *want,
                "{file}: {name} in {src:?}"
            );
        }
    }
}

#[test]
fn t4_paren_open_positives_and_negatives() {
    expect_open(
        "#define W3(p, q, r) p; r\n#define O1 W3(\n#define O2 for (\n#define O3 ) (\n\
         #define O4 f((T[]) {\n#define C1 if(c){\n#define C2 }\n#define C3 )\n\
         #define C4 g(a)(b)\n#define C5(...) __VA_OPT__(,)\n#define C6 { (x) }\n",
        &[
            ("O1", true),
            ("O2", true),
            ("O3", true),
            ("O4", true),
            ("C1", false),
            ("C2", false),
            ("C3", false),
            ("C4", false),
            ("C5", false),
            ("C6", false),
        ],
    );
}

/// A brace-only block macro opens no parenthesis.
#[test]
fn t4_braces_do_not_take_part_in_the_parenthesis_count() {
    expect_open(
        "#define BEGIN if (c) {\n#define BEGIN2 {\n#define END }\n",
        &[("BEGIN", false), ("BEGIN2", false), ("END", false)],
    );
}

/// A census text that no dialect reads sets every raw hazard; a text that one dialect reads
/// is read; every identifier of a group is forwarded and a leading parameter list is plain.
#[test]
fn t4_permissive_texts() {
    let t = text_census("#define BAD(a) \"unterminated { ,\n");
    let f = fact(&t, "BAD");
    assert!(f.unbalanced && f.loose_comma && f.arg_split && f.paren_open);
    // Every identifier after the name is a possible parameter: `INIT` is forwarded even though
    // it may be the parameter (over-approximation), and the root `ext` is unknown.
    let t = text_census("#define INIT { 1, 2 }\n#define HE ext(INIT)\n#define OUT { INIT }\n");
    assert!(fact(&t, "HE").arg_split);
    assert!(!fact(&t, "OUT").arg_split);
    // A leading parameter list `(a, b)` is a plain group: it forwards nothing.
    let t = text_census("#define INIT { 1, 2 }\n#define PL(INIT) x\n");
    assert!(!fact(&t, "PL").arg_split);
    // A named variadic in a text fails closed.
    let t = text_census("#define NV(args...) f(args)\n");
    assert!(fact(&t, "NV").arg_split && fact(&t, "NV").loose_comma);
}

// ---- task 5: closure ----

#[test]
fn t5_arg_split_is_closed_over_reach() {
    expect_split(
        "#define A1 K({ a, b })\n#define A2 A1\n#define A3(p) A2 p\n#define OTHER 1\n",
        &[("A1", true), ("A2", true), ("A3", true), ("OTHER", false)],
    );
}

#[test]
fn t5_forwarding_a_loose_macro_is_closed_through_two_hops() {
    expect_split(
        "#define L1 { 1, 2 }\n#define L2 L1\n#define L3 L2\n\
         #define H3 ext(L3)\n#define H4 ext((L3))\n#define N5 { L3 }\n",
        &[("H3", true), ("H4", true), ("N5", false)],
    );
    // `loose_comma` itself is closed over reach too.
    let c = census("a.c", "#define L1 { 1, 2 }\n#define L2 L1\n");
    assert!(fact(&c, "L2").loose_comma);
}

/// The result does not depend on whether the unknown root of a group is later defined: a new
/// unrelated definition of `ext` changes no fact of `HE`.
#[test]
fn t5_an_unknown_root_does_not_depend_on_its_own_definition() {
    let a = census("a.c", "#define INIT { 1, 2 }\n#define HE ext(INIT)\n");
    let b = census(
        "a.c",
        "#define INIT { 1, 2 }\n#define HE ext(INIT)\n#define ext(v) v\n",
    );
    assert!(fact(&a, "HE").arg_split && fact(&b, "HE").arg_split);
}

#[test]
fn t5_definitions_are_unioned_and_undef_clears_nothing() {
    expect_split(
        "#define U1 f(a)\n#define U1 f({ a, b })\n#define U2 f({ a, b })\n#undef U2\n\
         #define U3 f({ a, b })\n#define U3 f(a)\n",
        &[("U1", true), ("U2", true), ("U3", true)],
    );
    let c = census("a.c", "#define P1 (\n#define P1 1\n#define P2 )\n");
    assert!(fact(&c, "P1").paren_open && !fact(&c, "P2").paren_open);
}

#[test]
fn t5_paren_open_is_closed_over_reach() {
    expect_open(
        "#define W3(p, q, r) p; r\n#define O1 W3(\n#define O2 O1\n#define O3(x) O2 x\n\
         #define O4 CLOSED\n#define CLOSED ()\n",
        &[("O1", true), ("O2", true), ("O3", true), ("O4", false)],
    );
}

// ---- task 5: digest ----

/// The twins differ in ONE byte of the digest layout: only `arg_split` (a comma held in a
/// potential group against a comma after punctuation: unbalanced, loose, free names, params and
/// paren_open are equal), or only `paren_open` (`)` against `(`: both unbalanced).
#[test]
fn t5_digest_twins_move_one_flag_each() {
    let a = census("a.c", "#define G ext({ a, b })\n");
    let b = census("a.c", "#define G ext , ({ a, b })\n");
    let (fa, fb) = (fact(&a, "G"), fact(&b, "G"));
    assert_eq!(
        (fa.unbalanced, fa.loose_comma, fa.paren_open, &fa.free),
        (fb.unbalanced, fb.loose_comma, fb.paren_open, &fb.free)
    );
    assert!(fa.arg_split && !fb.arg_split);
    assert_ne!(a.digest(), b.digest(), "arg_split is in the digest");

    let a = census("a.c", "#define G )\n");
    let b = census("a.c", "#define G (\n");
    let (fa, fb) = (fact(&a, "G"), fact(&b, "G"));
    assert_eq!(
        (fa.unbalanced, fa.loose_comma),
        (fb.unbalanced, fb.loose_comma)
    );
    assert!(!fa.paren_open && fb.paren_open);
    assert_ne!(a.digest(), b.digest(), "paren_open is in the digest");
    assert_eq!(a.digest(), census("a.c", "#define G )\n").digest());
}

/// The aggregate `c-census:` identity of the text inputs is computed from the same digest.
#[test]
fn t5_text_digest_moves_with_each_new_flag() {
    let one =
        |text: &str| texts_digest(&BTreeMap::from([("defs.inc".to_owned(), text.to_owned())]));
    assert_ne!(
        one("#define G ext({ a, b })\n"),
        one("#define G ext , ({ a, b })\n")
    );
    assert_ne!(one("#define G )\n"), one("#define G (\n"));
    assert_eq!(one("#define G )\n"), one("#define G )\n"));
}
