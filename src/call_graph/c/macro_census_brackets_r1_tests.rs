//! #829 census facts of the amended design (DESIGN-v2 section 5.1): un-quarantined by commit C2.
//! #829 round-1 enumeration (terra + sol): census-fact rows added after the first matrix.
//! Ids are `G-terra-NN` / `G-sol-NN` of the two reports (see TESTS.md for the dedupe map).
//! Same stubs as `macro_census_brackets_tests.rs`: every `true` row is red on base, every
//! `false` row and every free-set row is a green guard.
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

/// A census over one parsed `.c` file plus census texts stamped on it, as the loader does.
fn census_with_texts(texts: &[(&str, &str)]) -> CMacroCensus {
    let mut p = ParsedFile::parse("a.c", "int f(void) { return 0; }\n", Language::C).unwrap();
    p.set_c_census_texts(Arc::new(
        texts
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    ));
    CMacroCensus::from_files(&BTreeMap::from([("a.c".to_owned(), p)]))
}

fn fact<'a>(c: &'a CMacroCensus, name: &str) -> &'a MacroFacts {
    c.get(name).unwrap_or_else(|| panic!("{name} missing"))
}

/// `(unbalanced, loose_comma)` of every `(name, want)` row.
fn expect(c: &CMacroCensus, rows: &[(&str, bool, bool)], what: &str) {
    for (name, unb, loose) in rows {
        let f = fact(c, name);
        assert_eq!(f.unbalanced, *unb, "{what}: {name}.unbalanced");
        assert_eq!(f.loose_comma, *loose, "{what}: {name}.loose_comma");
    }
}

const BLOCK_NAMES: [&str; 16] = [
    "pthread_cleanup_push",
    "pthread_cleanup_pop",
    "pthread_cleanup_push_defer_np",
    "pthread_cleanup_pop_restore_np",
    "Py_BEGIN_ALLOW_THREADS",
    "Py_END_ALLOW_THREADS",
    "Py_BEGIN_CRITICAL_SECTION",
    "Py_END_CRITICAL_SECTION",
    "Py_BEGIN_CRITICAL_SECTION2",
    "Py_END_CRITICAL_SECTION2",
    "Py_BEGIN_CRITICAL_SECTION_MUTEX",
    "Py_BEGIN_CRITICAL_SECTION2_MUTEX",
    "Py_TRASHCAN_BEGIN",
    "Py_TRASHCAN_END",
    "Py_TRASHCAN_SAFE_BEGIN",
    "Py_TRASHCAN_SAFE_END",
];

// ---- G-sol-01 / G-terra-01: the free-name closure the owner gate reads (K: green on base) ----

/// The design reads `MacroFacts::free` for the wrapper rule; this pins that it holds every
/// listed name through a direct wrapper, an object-like alias and several hops.
#[test]
fn g_sol_01_free_set_holds_the_listed_name_through_every_wrapper_shape() {
    for n in BLOCK_NAMES {
        let src = format!(
            "#define W1(a) {n}(a)\n#define W2 W1\n#define W3(x) W2(x)\n#define AL {n}\n#define AL2 AL\n"
        );
        for file in ["a.c", "a.cc"] {
            let c = census(&[(file, &src)]);
            for w in ["W1", "W2", "W3", "AL", "AL2"] {
                assert!(
                    fact(&c, w).free.contains(n),
                    "{file}: {w}.free must contain {n}: {:?}",
                    fact(&c, w).free
                );
            }
        }
        let t = census_with_texts(&[("defs.inc", &src)]);
        for w in ["W1", "W3", "AL2"] {
            assert!(fact(&t, w).free.contains(n), "text: {w}.free must hold {n}");
        }
    }
}

/// G-sol-08: a parsed definition's own parameter is not a free name, whatever it is spelled.
#[test]
fn g_sol_08_a_parameter_spelled_like_a_hazard_is_not_free() {
    let c = census(&[(
        "a.c",
        "#define BEGIN {\n#define ID(BEGIN) (BEGIN)\n#define PASS(pthread_cleanup_push) (pthread_cleanup_push)\n",
    )]);
    assert!(!fact(&c, "ID").free.contains("BEGIN"));
    assert!(!fact(&c, "ID").unbalanced, "ID reaches nothing");
    assert!(!fact(&c, "PASS").free.contains("pthread_cleanup_push"));
}

// ---- G-terra-09 / G-terra-08 / G-sol-10: lexical readings ----

/// `V<::S>`: a `<:` digraph in C before C++11, a template argument list in C++. The body
/// reader does not model the exception, so the spelling counts (accepted over-refusal).
#[test]
fn g_terra_09_cpp_digraph_exception_fails_closed() {
    let c = census(&[("a.cpp", "#define T V<::S>\n")]);
    expect(&c, &[("T", true, false)], "<::");
}

/// A C++ raw string whose C reading exposes a closer: the verdict is the union of the readable
/// passes, so the C reading counts (conservative, pinned).
#[test]
fn g_terra_08_raw_string_c_reading_exposes_a_closer() {
    let c = census(&[("a.cpp", "#define RAW R\"q(\" } \")q\"\n")]);
    assert!(fact(&c, "RAW").unbalanced);
}

/// G-sol-10 (IMMATERIAL): same shape with a `{` and a comma exposed in the C reading.
#[test]
fn g_sol_10_raw_string_c_reading_exposes_brace_and_comma() {
    let c = census(&[("a.cpp", "#define RAW R\"z(\" { , \" )z\"\n")]);
    expect(&c, &[("RAW", true, true)], "raw string");
}

// ---- G-terra-11 / G-sol-09: `__VA_OPT__` ----

#[test]
fn g_terra_11_va_opt_with_a_comment_before_its_parenthesis() {
    let c = census(&[(
        "a.cpp",
        "#define VP(...) __VA_OPT__ /*c*/ (,) __VA_ARGS__\n#define VC(...) __VA_OPT__ /*c*/ (,)\n#define VS(...) __VA_OPT__ \\\n (,)\n",
    )]);
    expect(
        &c,
        &[
            ("VP", false, true),
            ("VC", false, true),
            ("VS", false, true),
        ],
        "va_opt comment/splice",
    );
}

#[test]
fn g_terra_11_va_opt_comment_inside_tight_parentheses_is_tight() {
    let c = census(&[(
        "a.cpp",
        "#define VT(...) f(__VA_OPT__ /*c*/ (,))\n#define VU(...) f(__VA_OPT__ \\\n (,) __VA_ARGS__)\n",
    )]);
    expect(&c, &[("VT", false, false), ("VU", false, false)], "tight");
}

#[test]
fn g_sol_09_va_opt_under_a_brace_or_bracket_is_loose() {
    let c = census(&[(
        "a.cpp",
        "#define A(...) __VA_OPT__({1,2})\n#define D(...) __VA_OPT__({__VA_ARGS__})\n#define SA(...) __VA_OPT__([1,2])\n#define SD(...) __VA_OPT__([__VA_ARGS__])\n",
    )]);
    expect(
        &c,
        &[
            ("A", false, true),
            ("D", false, true),
            ("SA", false, true),
            ("SD", false, true),
        ],
        "va_opt nesting",
    );
}

/// The transparent entry must not hide real depth: every one of these is balanced and tight.
#[test]
fn g_sol_09_va_opt_under_parentheses_stays_tight_and_balanced() {
    let c = census(&[(
        "a.cpp",
        "#define B(...) __VA_OPT__({(1,2)})\n#define C(...) (__VA_OPT__(__VA_ARGS__))\n#define SB(...) __VA_OPT__([(1,2)])\n#define E(...) (__VA_OPT__())\n",
    )]);
    expect(
        &c,
        &[
            ("B", false, false),
            ("C", false, false),
            ("SB", false, false),
            ("E", false, false),
        ],
        "va_opt tight",
    );
}

// ---- G-terra-12 / G-sol-12: `loose_comma` union and `#undef` ----

#[test]
fn g_sol_12_loose_comma_survives_undef_and_a_tight_redefinition() {
    for src in [
        "#define M 1,2\n#undef M\n#define M (1,2)\n",
        "#define M (1,2)\n#define M 1,2\n#undef M\n",
        "#define M (1,2)\n#undef M\n#define M 1,2\n",
    ] {
        expect(&census(&[("a.c", src)]), &[("M", false, true)], src);
    }
}

#[test]
fn g_sol_12_loose_and_tight_definitions_in_separate_files() {
    for files in [
        [("x.c", "#define M (1,2)\n"), ("y.c", "#define M 1,2\n")],
        [("x.c", "#define M 1,2\n"), ("y.c", "#define M (1,2)\n")],
    ] {
        expect(&census(&files), &[("M", false, true)], "two files");
    }
}

#[test]
fn g_terra_12_loose_comma_union_across_files_after_undef_and_alias() {
    for files in [
        [
            ("defs1.h", "#define VALUES 1, 2\n#undef VALUES\n"),
            ("defs2.h", "#define VALUES 0\n#define ALIAS VALUES\n"),
        ],
        [
            ("defs1.h", "#define VALUES 0\n#define ALIAS VALUES\n"),
            ("defs2.h", "#define VALUES 1, 2\n#undef VALUES\n"),
        ],
    ] {
        expect(
            &census(&files),
            &[("VALUES", false, true), ("ALIAS", false, true)],
            "union",
        );
    }
}

#[test]
fn g_sol_12_loose_comma_union_in_permissive_texts() {
    let t = census_with_texts(&[
        ("a.inc", "#define M 1,2\n#undef M\n#define M (1,2)\n"),
        ("b.inc", "#define N (1,2)\n"),
    ]);
    expect(&t, &[("M", false, true), ("N", false, false)], "permissive");
}

// ---- G-terra-13 / G-sol-11: permissive raw fallback ----

/// An unreadable permissive line is counted as written: a loose comma and an unmatched brace.
#[test]
fn g_terra_13_permissive_unreadable_line_sets_both_facts() {
    let t = census_with_texts(&[("defs.inc", "#define BAD(a,b) \"unterminated { ,\n")]);
    expect(&t, &[("BAD", true, true)], "unterminated");
}

#[test]
fn g_sol_11_permissive_unreadable_line_with_only_a_comma() {
    // DESIGN-v2 5.1: a permissive line no dialect reads sets EVERY hazard (unbalanced included),
    // the one expectation of this row that changed with the amended design.
    let t = census_with_texts(&[("defs.inc", "#define BAD \"a,b\n")]);
    expect(&t, &[("BAD", true, true)], "comma only");
}

// ---- G-terra-15 / G-sol-13: the digest carries `loose_comma` alone ----

#[test]
fn g_terra_15_digest_changes_when_only_the_comma_fact_changes() {
    for (a, b) in [
        ("#define X 1\n", "#define X 1, 2\n"),
        ("#define M (1,2)\n", "#define M 1,2\n"),
    ] {
        let (ca, cb) = (census(&[("a.c", a)]), census(&[("a.c", b)]));
        assert_ne!(ca.digest(), cb.digest(), "{a:?} -> {b:?}");
    }
}

#[test]
fn g_sol_13_text_aggregate_digest_changes_when_only_the_comma_fact_changes() {
    let a = census_with_texts(&[("m.inc", "#define M (1,2)\n")]);
    let b = census_with_texts(&[("m.inc", "#define M 1,2\n")]);
    assert_ne!(a.digest(), b.digest());
}

// ---- G-sol-15: normalization feeds the new facts ----

const NORM: &str =
    "%:define OP\\\nEN /* ) , */ {\n#define CL\\\nOSE }\n#define PA\\\nIR 1 /* ) */ , 2\n";

fn norm_rows() -> [(&'static str, bool, bool); 3] {
    [
        ("OPEN", true, false),
        ("CLOSE", true, false),
        ("PAIR", false, true),
    ]
}

fn norm_variants() -> Vec<(&'static str, String)> {
    vec![
        ("lf", NORM.to_owned()),
        ("crlf", NORM.replace('\n', "\r\n")),
        ("cr", NORM.replace('\n', "\r")),
        ("bom", format!("\u{feff}{NORM}")),
        (
            "continued-comment",
            "%:define OP\\\nEN /* ) , \\\n */ {\n#define CL\\\nOSE }\n#define PA\\\nIR 1 /* ) \\\n*/ , 2\n"
                .to_owned(),
        ),
    ]
}

#[test]
fn g_sol_15_split_names_and_digraph_directives_in_a_parsed_file() {
    for (what, src) in norm_variants() {
        for file in ["a.c", "a.cc"] {
            expect(
                &census(&[(file, &src)]),
                &norm_rows(),
                &format!("{file} {what}"),
            );
        }
    }
}

#[test]
fn g_sol_15_split_names_and_digraph_directives_in_a_census_text() {
    for (what, src) in norm_variants() {
        expect(
            &census_with_texts(&[("defs.inc", &src)]),
            &norm_rows(),
            &format!("text {what}"),
        );
    }
}

// ---- G-terra-10: a name assembled by a splice (census half; the owner half is in the c_prune r1 tests) ----

#[test]
fn g_terra_10_name_split_by_a_splice() {
    for src in [
        "#define OP\\\nEN {\n#define CLOSE }\n",
        "#define OP\\\r\nEN {\r\n#define CLOSE }\r\n",
    ] {
        for file in ["a.c", "a.cc"] {
            expect(
                &census(&[(file, src)]),
                &[("OPEN", true, false), ("CLOSE", true, false)],
                src,
            );
        }
    }
}

// ---- G-terra-16: digraph and plain token paste stay poisoned (K: green on base) ----

#[test]
fn g_terra_16_token_paste_poisons_whatever_the_new_facts_say() {
    for file in ["a.c", "a.cc"] {
        let c = census(&[(
            file,
            "#define JOIN(a,b) a %:%: b\n#define JOIN2(a,b) a ## b\n#define SAFE(a,b) f(a,b)\n",
        )]);
        assert!(fact(&c, "JOIN").poisoned, "{file}: digraph paste");
        assert!(fact(&c, "JOIN2").poisoned, "{file}: ## paste");
        assert!(!fact(&c, "SAFE").poisoned, "{file}: control");
    }
}
