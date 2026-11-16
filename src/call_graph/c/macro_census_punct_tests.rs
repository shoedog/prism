//! #829 round-2 (terra): the census lexer reads punctuators by maximal munch
//! ([lex.pptoken]/3, C 6.4.6) BEFORE it looks for digraphs, so the second byte of a longer
//! token never starts a bracket. Compiler evidence, `clang++ -std=c++17 -fsyntax-only
//! -Xclang -dump-tokens` (and `clang -std=c17`, `-std=c2x`, which agree on every row below):
//!   `[ ::>`   -> l_square coloncolon greater     (not `[` `]`)
//!   `<<% }`   -> lessless percent r_brace        (not `<%`)
//!   `<<: ]`   -> lessless colon r_square         (not `<:`)
//!   `<<<:`    -> lessless l_square               (`<<` then the digraph `<:`)
//!   `<<=:>`   -> lesslessequal r_square          (`:>` after a whole `<<=`)
//!   `->*:>`   -> arrowstar r_square (C++), arrow star r_square (C)
//!   `x %:%>`  -> identifier hash r_brace         (`%:` then `%>`)
//!   `x %::>`  -> identifier hash r_square        (`%:` then `:>`)
//!   `x %:>`   -> identifier hash greater         (`%:` wins over `:>`; no bracket)
//!   `x %:%:`  -> identifier hashhash
//!   `x <:%:`  -> identifier l_square hash
//!   `x <::]`  -> C++: less coloncolon r_square, C: l_square colon r_square (the exception)
//! clang also reads `::` as `coloncolon` in C17 mode, so the C reading with `::` (C23) is the
//! one the census must add to the C17 one (`:` `:`): `[ ::>` is `[ :` `:>` in C17.
use super::macro_census::{tokens_for_tests, CMacroCensus, MacroFacts};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use std::sync::Arc;

fn structural(text: &str, cxx: bool) -> Vec<String> {
    tokens_for_tests(text, cxx)
        .unwrap()
        .into_iter()
        .filter(|t| t != "other")
        .collect()
}

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

/// (body, unbalanced): the C++ and the union-of-C readings agree on each row except where a
/// row says otherwise; `true` rows are red on 809e0f4a.
const ROWS: &[(&str, &str, bool)] = &[
    ("A1", "[ ::>", true),
    ("A2", "<<% }", true),
    ("A3", "<<: ]", true),
    ("A4", "<<<: ", true),
    ("A5", "{ <<% ", true),
    ("A6", "a <<= b", false),
    // Genuine digraphs stay brackets (controls).
    ("K1", "<: :>", false),
    ("K2", "<% %>", false),
    ("K3", "a <: b :>", false),
    ("K4", "<<<: :>", false),
    ("K5", "<<=:>", true),
    ("K6", "->*:>", true),
    ("K7", "%:%>", true),
    ("K8", "%::>", true),
    ("K9", "%:>", false),
    ("K10", "%:%: x", false),
    ("K11", "<:%:", true),
    ("K12", "<::>", false),
    ("K13", "<:::x]", false),
    ("K14", "x<<<%  %>", false),
];

fn defs() -> String {
    ROWS.iter()
        .map(|(n, b, _)| format!("#define {n} {b}\n"))
        .collect()
}

fn check(c: &CMacroCensus, what: &str) {
    for (n, body, want) in ROWS {
        assert_eq!(fact(c, n).unbalanced, *want, "{what}: {n} = {body:?}");
    }
}

#[test]
fn p1_facts_cxx_file_c_file_header_and_census_text() {
    let d = defs();
    for file in ["a.cc", "a.c", "a.h"] {
        check(&census(file, &d), file);
    }
    check(&text_census(&d), "census text");
}

#[test]
fn p2_token_streams_by_maximal_munch() {
    for cxx in [false, true] {
        assert_eq!(structural("[ ::>", cxx).first().unwrap(), "[");
        assert_eq!(structural("<<% }", cxx), ["}"], "cxx={cxx}");
        assert_eq!(structural("<<: ]", cxx), ["]"], "cxx={cxx}");
        assert_eq!(structural("<<<:", cxx), ["digraph:["], "cxx={cxx}");
        assert_eq!(structural("<<=:>", cxx), ["digraph:]"], "cxx={cxx}");
        assert_eq!(structural("a %:%> b", cxx), ["id:a", "digraph:}", "id:b"]);
        assert_eq!(structural("a %::> b", cxx), ["id:a", "digraph:]", "id:b"]);
        assert_eq!(structural("a %:> b", cxx), ["id:a", "id:b"]);
        assert_eq!(structural("a <:%: b", cxx), ["id:a", "digraph:[", "id:b"]);
    }
    // `::` is one token in C++ (and C23): no `:>` inside it. C17 reads `:` `:>`.
    assert_eq!(structural("[ ::>", true), ["["]);
    assert_eq!(structural("[ ::>", false), ["[", "digraph:]"]);
}

#[test]
fn p3_the_less_colon_colon_exception_still_holds() {
    assert_eq!(structural("<::value]", true), ["id:value", "]"]);
    assert_eq!(
        structural("<::value]", false),
        ["digraph:[", "id:value", "]"]
    );
}
