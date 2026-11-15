//! #829 chain A, round 2: census-fact rows (G2-sonnet-06, G2-opus-03). Same stubs as the other
//! bracket census tests: every `true` row is red on base. The lexical rows are the HIDE
//! direction (a literal that must not swallow a bracket or a comma, a reading that hides it);
//! the no-count direction is pinned by U10, U12 and L12.
use super::macro_census::{CMacroCensus, MacroFacts};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use std::sync::Arc;

pub(super) fn parsed_census(file: &str, src: &str) -> CMacroCensus {
    let lang = if file.ends_with(".cc") || file.ends_with(".cpp") {
        Language::Cpp
    } else {
        Language::C
    };
    let p = ParsedFile::parse(file, src, lang).unwrap();
    CMacroCensus::from_files(&BTreeMap::from([(file.to_owned(), p)]))
}

/// A census where `src` arrives as a census text (the loader's non-C include file).
pub(super) fn text_census(src: &str) -> CMacroCensus {
    let mut p = ParsedFile::parse("a.c", "int f(void) { return 0; }\n", Language::C).unwrap();
    p.set_c_census_texts(Arc::new(BTreeMap::from([(
        "defs.inc".to_owned(),
        src.to_owned(),
    )])));
    CMacroCensus::from_files(&BTreeMap::from([("a.c".to_owned(), p)]))
}

pub(super) fn fact<'a>(c: &'a CMacroCensus, name: &str) -> &'a MacroFacts {
    c.get(name).unwrap_or_else(|| panic!("{name} missing"))
}

/// `(unbalanced, loose_comma)` of each `(name, unbalanced, loose)` row in a parsed `.c`, a
/// parsed `.cpp` and a census text.
pub(super) fn expect_everywhere(src: &str, rows: &[(&str, bool, bool)]) {
    let readings = [
        ("parsed .c", parsed_census("a.c", src)),
        ("parsed .cpp", parsed_census("a.cpp", src)),
        ("census text", text_census(src)),
    ];
    for (what, c) in &readings {
        for (name, unb, loose) in rows {
            let f = fact(c, name);
            assert_eq!(f.unbalanced, *unb, "{what}: {name}.unbalanced in {src:?}");
            assert_eq!(
                f.loose_comma, *loose,
                "{what}: {name}.loose_comma in {src:?}"
            );
        }
    }
}

// ---- G2-sonnet-06 (IMMATERIAL unless the implementer writes a second scanner): a quote
// character inside the other literal kind must not turn the rest of the line into a literal ----

#[test]
fn g2_sonnet_06_double_quote_in_a_char_literal_does_not_hide_a_brace() {
    expect_everywhere("#define Q1 '\"' { '\"'\n", &[("Q1", true, false)]);
}

#[test]
fn g2_sonnet_06_single_quote_in_a_string_literal_does_not_hide_a_brace() {
    expect_everywhere("#define Q2 \"'\" { \"'\"\n", &[("Q2", true, false)]);
}

#[test]
fn g2_sonnet_06_escaped_backslash_char_literal_does_not_hide_a_brace() {
    expect_everywhere("#define Q3 '\\\\' {\n", &[("Q3", true, false)]);
}

#[test]
fn g2_sonnet_06_escaped_backslash_string_literal_does_not_hide_a_brace() {
    expect_everywhere("#define Q4 \"\\\\\" {\n", &[("Q4", true, false)]);
}

#[test]
fn g2_sonnet_06_quote_characters_do_not_hide_a_loose_comma() {
    expect_everywhere(
        "#define Q5 '\"' , '\"'\n#define Q6 \"'\" , \"'\"\n",
        &[("Q5", false, true), ("Q6", false, true)],
    );
}

// ---- G2-opus-03: the census verdict is the OR of the dialect readings ----

/// In the C reading (no digit separators) `'0) { (void)(2'` and `'0); } (void)(4'` are
/// character literals and both bodies look balanced; C23 and C++ expose `{` and `}`.
/// Compiled as C23 and C++17: `41 2`.
#[test]
fn g2_opus_03_digit_separator_pairs_do_not_hide_a_bracket() {
    expect_everywhere(
        "#define OPEN if (c != 1'0) { (void)(2'0);\n#define CLOSE (void)(3'0); } (void)(4'0);\n",
        &[("OPEN", true, false), ("CLOSE", true, false)],
    );
}

/// Member: a loose comma hidden the same way. Every loose comma sits between exactly two
/// quotes, so in the C reading each one is inside a character literal.
#[test]
fn g2_opus_03_digit_separator_pairs_do_not_hide_a_loose_comma() {
    expect_everywhere("#define L 1'0, 2'0\n", &[("L", false, true)]);
}
