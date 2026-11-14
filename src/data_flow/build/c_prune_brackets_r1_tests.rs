//! #829 round-1 enumeration (terra + sol): owner rows added after the first matrix. Ids are
//! `G-terra-NN` / `G-sol-NN` (TESTS.md has the dedupe map). Per-row pair tests read the same
//! `obligations.jsonl` as the matrices; the gate verdicts go through `c_prune::scan`. Written
//! against the stubs: every keep row is red until the gates exist.
use super::c_prune::{Off, BLOCK_MACROS};
use super::c_prune_brackets_tests::{
    c_case, check_row, counts, cpp_case, expect, owner_nodes, verdicts,
};
use super::Producer;
use crate::call_graph::c::macro_census::CMacroCensus;
use crate::call_graph::rust::macro_args::RustMacroCensus;
use std::collections::BTreeMap;

macro_rules! rows {
    ($file:expr => $($name:ident $id:expr),+ $(,)?) => {
        $(#[test] fn $name() { check_row($file, $id); })+
    };
}

// Wrappers around listed external macros. w6 is a control (K): pruning must survive.
rows!("22_macro_wrappers.c" =>
    row_g_terra_01_w1 "w1", row_g_sol_01_w2 "w2", row_g_sol_01_w3 "w3", row_g_sol_01_w4 "w4",
    row_g_sol_01_w5 "w5", row_g_sol_01_w8 "w8", row_g_sol_08_w6 "w6", row_g_terra_07_w7 "w7");
rows!("22_macro_wrappers.cc" =>
    row_cpp_g_terra_01_w1 "w1", row_cpp_g_sol_01_w2 "w2", row_cpp_g_sol_01_w3 "w3",
    row_cpp_g_sol_01_w4 "w4", row_cpp_g_sol_08_w6 "w6");
// Live argument splits (S), rescan (S), square-held commas (C) and held-comma controls (K).
rows!("23_macro_args_live.c" =>
    row_a1_g_sol_02 "a1", row_a1t_g_terra_02 "a1t", row_a9_g_sol_03 "a9",
    row_a10_g_sol_04_alias "a10", row_a11_g_sol_04_comment "a11", row_a12_g_sol_04_chain "a12",
    row_k1_g_sol_06 "k1", row_k2_g_sol_06 "k2", row_k3_g_sol_07 "k3");
rows!("23_macro_args_live.cc" =>
    row_y5_g_sol_05 "y5", row_y6_g_sol_05 "y6", row_y7_g_sol_05 "y7", row_y8_g_sol_05 "y8",
    row_y9_g_terra_05 "y9", row_y10_g_sol_05_subscript "y10", row_y11_g_terra_06 "y11");
// A macro name assembled by a splice, LF and CRLF, C and C++.
rows!("24_macro_splice.c" => row_s2_g_terra_10 "s2");
rows!("24_macro_splice_crlf.c" => row_s1_g_terra_10_crlf "s1");
rows!("24_macro_splice.cc" => row_s3_g_terra_10_cpp "s3");
rows!("24_macro_splice_crlf.cc" => row_s4_g_terra_10_cpp_crlf "s4");

// ---- owner verdicts of the new inline cases ----

#[test]
fn e01_22_wrappers_c() {
    let v = verdicts("22_macro_wrappers.c", &c_case("22_macro_wrappers.c"));
    expect(
        &v,
        Some(Off::MacroBrackets),
        &[
            "alias_object_like",
            "wrapper_function_like",
            "wrapper_multi_hop",
            "wrapper_critical_section",
            "wrapper_allow_threads",
            "wrapper_returns_a_callable_name",
            "parenthesized_unbalanced_name",
        ],
    );
}

#[test]
fn e01_22_wrappers_c_control() {
    let v = verdicts("22_macro_wrappers.c", &c_case("22_macro_wrappers.c"));
    expect(&v, None, &["parameter_shadow_control"]);
}

#[test]
fn e01_22_wrappers_cc() {
    let v = verdicts("22_macro_wrappers.cc", &cpp_case("22_macro_wrappers.cc"));
    expect(
        &v,
        Some(Off::MacroBrackets),
        &[
            "cpp_alias_object_like",
            "cpp_wrapper_function_like",
            "cpp_wrapper_multi_hop",
            "cpp_wrapper_critical_section",
        ],
    );
}

#[test]
fn e01_22_wrappers_cc_control() {
    let v = verdicts("22_macro_wrappers.cc", &cpp_case("22_macro_wrappers.cc"));
    expect(&v, None, &["cpp_parameter_shadow_control"]);
}

#[test]
fn e01_23_live_c() {
    let v = verdicts("23_macro_args_live.c", &c_case("23_macro_args_live.c"));
    expect(
        &v,
        Some(Off::MacroArgSplit),
        &[
            "a1_live",
            "a1_terra",
            "split_through_an_object_alias",
            "split_after_a_comment",
            "split_through_a_chained_group",
        ],
    );
    // INIT carries the brace-held comma; no group of the owner holds it before the rescan.
    expect(&v, Some(Off::MacroComma), &["rescan_split"]);
}

#[test]
fn e01_23_live_c_controls() {
    let v = verdicts("23_macro_args_live.c", &c_case("23_macro_args_live.c"));
    expect(
        &v,
        None,
        &[
            "held_comma_in_a_char_literal",
            "held_comma_inside_parentheses",
            "inhibited_invocation",
        ],
    );
}

#[test]
fn e01_23_live_cc() {
    let v = verdicts("23_macro_args_live.cc", &cpp_case("23_macro_args_live.cc"));
    expect(
        &v,
        Some(Off::MacroArgSplit),
        &[
            "lambda_capture_plain",
            "lambda_capture_reference",
            "lambda_capture_initialized",
            "lambda_capture_mixed",
            "alias_comment_square_split",
            "subscript_comma_split",
        ],
    );
}

#[test]
fn e01_23_live_cc_control() {
    let v = verdicts("23_macro_args_live.cc", &cpp_case("23_macro_args_live.cc"));
    expect(&v, None, &["capture_comma_inside_parentheses"]);
}

#[test]
fn e01_24_splice_names() {
    for (file, owners) in [
        ("24_macro_splice.c", &["spliced_name"][..]),
        ("24_macro_splice_crlf.c", &["spliced_name_crlf"][..]),
    ] {
        let v = verdicts(file, &c_case(file));
        expect(&v, Some(Off::MacroBrackets), owners);
    }
    for (file, owners) in [
        ("24_macro_splice.cc", &["cpp_spliced_name"][..]),
        ("24_macro_splice_crlf.cc", &["cpp_spliced_name_crlf"][..]),
    ] {
        let v = verdicts(file, &cpp_case(file));
        expect(&v, Some(Off::MacroBrackets), owners);
    }
}

// ---- G-sol-01: the closure rule for every one of the 16 listed names ----

fn wrapper_source(name: &str, shape: &str) -> String {
    let (defs, call) = match shape {
        "function-like multi-hop" => (
            format!("#define W1(a) {name}(a)\n#define W3(a) W1(a)\n"),
            "W3(0)",
        ),
        "object-like alias" => (format!("#define AL {name}\n#define AL2 AL\n"), "AL2(0)"),
        "direct" => (String::new(), "NAME(0)"),
        other => panic!("{other}"),
    };
    let call = call.replace("NAME", name);
    format!("{defs}int f(void) {{\n    int x = 1;\n    {call};\n    x = 2;\n    return x;\n}}\n")
}

#[test]
fn g_sol_01_every_listed_name_is_refused_through_wrappers() {
    for file in ["a.c", "a.cc"] {
        for name in BLOCK_NAMES {
            for shape in ["function-like multi-hop", "object-like alias"] {
                let v = verdicts(file, &wrapper_source(name, shape));
                assert_eq!(
                    v.get("f"),
                    Some(&Some(Off::MacroBrackets)),
                    "{file} {name} via {shape}"
                );
            }
        }
    }
}

/// Direct spelling of each of the 16 names (the matrix pins two of them through b12 and b13).
#[test]
fn g_sol_01_every_listed_name_is_refused_when_spelled_directly() {
    for file in ["a.c", "a.cc"] {
        for name in BLOCK_NAMES {
            let v = verdicts(file, &wrapper_source(name, "direct"));
            assert_eq!(v.get("f"), Some(&Some(Off::MacroBrackets)), "{file} {name}");
        }
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

/// The names the closure tests iterate over are the production list (a rename in either
/// place turns this red, not the loops above silently green).
#[test]
fn g_sol_01_test_names_are_the_production_list() {
    let mut have: Vec<&str> = BLOCK_MACROS.to_vec();
    let mut want: Vec<&str> = BLOCK_NAMES.to_vec();
    have.sort_unstable();
    want.sort_unstable();
    assert_eq!(have, want);
}

// ---- G-terra-16: paste stays G5 whatever the new facts say (K: green on base) ----

#[test]
fn g_terra_16_digraph_and_plain_paste_owners_are_g5() {
    for file in ["a.c", "a.cc"] {
        let src = "#define JOIN(a,b) a %:%: b\n#define JOIN2(a,b) a ## b\nint foobar;\nint f(void) {\n    int z = 1;\n    JOIN(foo,bar) = 0;\n    z = 2;\n    return z;\n}\nint g(void) {\n    int z = 1;\n    JOIN2(foo,bar) = 0;\n    z = 2;\n    return z;\n}\n";
        let v = verdicts(file, src);
        expect(&v, Some(Off::G5), &["f", "g"]);
    }
}

// ---- multi-file rows: wrappers arrive through headers and census texts ----

const WRAP_OWNER: &str = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void (*h)(void *)) {\n    int x = src();\n    if (cond) ENTER(h, 0);\n    x = 2;\n    LEAVE(0);\n    sink(x + 1);\n}\n";
const WRAP_DEFS: &str =
    "#define ENTER(h, p) pthread_cleanup_push(h, p)\n#define LEAVE(p) pthread_cleanup_pop(p)\n";
const WRAP_DEFS_BALANCED: &str = "#define ENTER(h, p) ((void)(h))\n#define LEAVE(p) ((void)(p))\n";

#[test]
fn g_terra_01_wrapper_in_a_header_c_and_cpp() {
    for (owner, hdr) in [("a.c", "h.h"), ("a.cc", "h.hpp")] {
        let (b, a) = counts(&[(owner, WRAP_OWNER), (hdr, WRAP_DEFS)], owner, &[]);
        assert_eq!(
            a, b,
            "{owner}: the wrapper names a listed macro ({b} -> {a})"
        );
        let (b, a) = counts(
            &[(owner, WRAP_OWNER), (hdr, WRAP_DEFS_BALANCED)],
            owner,
            &[],
        );
        assert!(
            a < b,
            "{owner}: control, a harmless wrapper prunes ({b} -> {a})"
        );
    }
}

#[test]
fn g_terra_01_wrapper_in_a_census_text_c_and_cpp() {
    for owner in ["a.c", "a.cc"] {
        let (b, a) = counts(&[(owner, WRAP_OWNER)], owner, &[("defs.inc", WRAP_DEFS)]);
        assert_eq!(a, b, "{owner}: census text wrapper ({b} -> {a})");
        let (b, a) = counts(
            &[(owner, WRAP_OWNER)],
            owner,
            &[("defs.inc", WRAP_DEFS_BALANCED)],
        );
        assert!(a < b, "{owner}: control prunes ({b} -> {a})");
    }
}

#[test]
fn g_terra_01_alias_chain_across_two_headers() {
    let owner = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void (*h)(void *)) {\n    int x = src();\n    if (cond) E2(h, 0);\n    x = 2;\n    L2(0);\n    sink(x + 1);\n}\n";
    for (target, h1, h2) in [("a.c", "h1.h", "h2.h"), ("a.cc", "h1.hpp", "h2.hpp")] {
        let files = [
            (target, owner),
            (h1, "#define E1(h, p) pthread_cleanup_push(h, p)\n#define L1(p) pthread_cleanup_pop(p)\n"),
            (h2, "#define E2(h, p) E1(h, p)\n#define L2(p) L1(p)\n"),
        ];
        let (b, a) = counts(&files, target, &[]);
        assert_eq!(a, b, "{target}: two hops across two headers ({b} -> {a})");
    }
}

// ---- G-sol-03: the composed split with the census arriving from a header ----

#[test]
fn g_sol_03_rescan_split_through_a_header() {
    let owner = "int src(void);\nint f(int c) {\n    int x = 41;\n    int b = 0;\n    {\n        H(INIT);\n        x = 2;\n        J(INIT);\n        return x;\n    }\n}\n";
    let defs = "#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n#define INIT { int a, b; }\n";
    let (b, a) = counts(&[("a.c", owner), ("h.h", defs)], "a.c", &[]);
    assert_eq!(a, b, "INIT's brace-held comma refuses ({b} -> {a})");
    let tight = defs.replace("{ int a, b; }", "{ int a; }");
    let (b, a) = counts(&[("a.c", owner), ("h.h", &tight)], "a.c", &[]);
    assert!(a < b, "control: no comma in INIT prunes ({b} -> {a})");
}

// ---- G-terra-10: a splice assembling a name, a comment before the call, header-borne ----

#[test]
fn g_terra_10_spliced_name_from_a_header_with_lf_and_crlf() {
    let owner = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void) {\n    int x = src();\n    if (cond) OPEN /* comment */ ;\n    x = 2;\n    CLOSE;\n    sink(x + 1);\n}\n";
    let defs = "#define OP\\\nEN {\n#define CLOSE }\n";
    for (target, hdr) in [("a.c", "h.h"), ("a.cc", "h.hpp")] {
        for (what, d) in [
            ("lf", defs.to_owned()),
            ("crlf", defs.replace('\n', "\r\n")),
        ] {
            let (b, a) = counts(&[(target, owner), (hdr, &d)], target, &[]);
            assert_eq!(a, b, "{target} {what}: spliced name ({b} -> {a})");
        }
    }
}

// ---- G-terra-14 / G-sol-14: loader discovery drives the new facts ----

pub(super) mod loaded {
    use super::*;
    use crate::repo_loader::{current_c_census_identity, load_repo};
    use std::fs;
    use std::path::Path;

    pub(in super::super) fn write(d: &Path, rel: &str, bytes: &str) {
        let p = d.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    /// The `scan` verdict of every owner of `target`, with the census the loader builds.
    fn verdicts_loaded(d: &Path, target: &str) -> BTreeMap<String, Option<Off>> {
        let repo = load_repo(d).unwrap();
        let census = CMacroCensus::from_files(&repo.files);
        let parsed = &repo.files[target];
        owner_nodes(parsed)
            .into_iter()
            .map(|(name, node)| {
                let v = super::super::c_prune::scan(parsed, node, &census)
                    .map(|_| ())
                    .err();
                (name, v)
            })
            .collect()
    }

    /// `(edges before the rules, edges after)` over `target` with the loader's census.
    fn counts_loaded(d: &Path, target: &str) -> (usize, usize) {
        let repo = load_repo(d).unwrap();
        let census = CMacroCensus::from_files(&repo.files);
        let p = &repo.files[target];
        let file = target.to_owned();
        let before = Producer::file_refs_frozen(&file, p);
        let after = Producer::file_refs_with_census(&file, p, &RustMacroCensus::default(), &census);
        (before.edges.len(), after.edges.len())
    }

    /// Premise: the loader's census holds the payload's macros (a base failure must be the
    /// gate assertion, never a payload the loader did not read).
    fn assert_read(d: &Path) {
        let repo = load_repo(d).unwrap();
        let census = CMacroCensus::from_files(&repo.files);
        for n in ["BEGIN", "END", "PAIR"] {
            assert!(census.get(n).is_some(), "{n} not read by the loader");
        }
    }

    const GUARD: &str = "int src(void);\nvoid sink(int);\nint cond;\nint f(void) {\n    int x = src();\n    if (cond) BEGIN;\n    x = 2;\n    END;\n    sink(x + 1);\n    return 0;\n}\n";
    const COMMA: &str = "int src(void);\nvoid sink(int);\nint comma(void) {\n    int x = src();\n    int y = (PAIR);\n    x = 2;\n    sink(x + y);\n    return 0;\n}\n";
    const DEFS: &str = "#define BEGIN {\n#define END }\n#define PAIR 1,2\n";

    fn guard_repo(include_line: &str, ext: &str, tree: &[(&str, &str)]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        write(
            d.path(),
            &format!("a.{ext}"),
            &format!("{include_line}\n{GUARD}"),
        );
        write(
            d.path(),
            &format!("b.{ext}"),
            &format!("{include_line}\n{COMMA}"),
        );
        for (rel, body) in tree {
            write(d.path(), rel, body);
        }
        d
    }

    /// sol-14: `first.payload` includes `defs.payload`, neither is C: the loader reads both
    /// as census texts, and the new facts reach the owners of both dialects.
    #[test]
    fn g_sol_14_transitive_non_c_include_reaches_the_owner_gates() {
        for ext in ["c", "cc"] {
            let d = guard_repo(
                "#include \"first.payload\"",
                ext,
                &[
                    ("first.payload", "#include \"defs.payload\"\n"),
                    ("defs.payload", DEFS),
                ],
            );
            assert_read(d.path());
            let v = verdicts_loaded(d.path(), &format!("a.{ext}"));
            assert_eq!(v.get("f"), Some(&Some(Off::MacroBrackets)), "{ext}: {v:?}");
            let v = verdicts_loaded(d.path(), &format!("b.{ext}"));
            assert_eq!(v.get("comma"), Some(&Some(Off::MacroComma)), "{ext}: {v:?}");
            let (b, a) = counts_loaded(d.path(), &format!("a.{ext}"));
            assert_eq!(a, b, "{ext}: the guarded write is live ({b} -> {a})");
            let (b, a) = counts_loaded(d.path(), &format!("b.{ext}"));
            assert_eq!(a, b, "{ext}: the comma owner keeps its pair ({b} -> {a})");
        }
    }

    /// terra-14: quoted and angle forms, one hop through a `.inc` file.
    #[test]
    fn g_terra_14_quoted_and_angle_transitive_includes() {
        for (ext, line) in [
            ("c", "#include \"outer.inc\""),
            ("c", "#include <outer.inc>"),
            ("cc", "#include \"outer.inc\""),
            ("cc", "#include <outer.inc>"),
        ] {
            let d = guard_repo(
                line,
                ext,
                &[("outer.inc", "#include \"defs.txt\"\n"), ("defs.txt", DEFS)],
            );
            assert_read(d.path());
            let v = verdicts_loaded(d.path(), &format!("a.{ext}"));
            assert_eq!(
                v.get("f"),
                Some(&Some(Off::MacroBrackets)),
                "{ext} {line}: {v:?}"
            );
        }
    }

    /// The C-like-extension net: nothing includes `defs.inc`, yet it is read.
    #[test]
    fn g_sol_14_net_file_without_an_include_reaches_the_owner_gates() {
        for ext in ["c", "cc"] {
            let d = guard_repo("", ext, &[("defs.inc", DEFS)]);
            assert_read(d.path());
            let v = verdicts_loaded(d.path(), &format!("a.{ext}"));
            assert_eq!(v.get("f"), Some(&Some(Off::MacroBrackets)), "{ext}: {v:?}");
            let v = verdicts_loaded(d.path(), &format!("b.{ext}"));
            assert_eq!(v.get("comma"), Some(&Some(Off::MacroComma)), "{ext}: {v:?}");
        }
    }

    /// Control: the same tree with balanced, tight definitions prunes (K, green on base).
    #[test]
    fn g_sol_14_control_balanced_payload_prunes() {
        let d = guard_repo(
            "#include \"first.payload\"",
            "c",
            &[
                ("first.payload", "#include \"defs.payload\"\n"),
                (
                    "defs.payload",
                    "#define BEGIN (void)0\n#define END (void)0\n#define PAIR (1,2)\n",
                ),
            ],
        );
        let v = verdicts_loaded(d.path(), "a.c");
        assert_eq!(v.get("f"), Some(&None), "{v:?}");
        let (b, a) = counts_loaded(d.path(), "a.c");
        assert!(a < b, "control prunes ({b} -> {a})");
    }

    /// The aggregate identity moves when only the comma fact of a discovered text changes.
    #[test]
    fn g_sol_13_aggregate_identity_moves_on_a_loose_only_edit() {
        let d = guard_repo(
            "#include \"defs.payload\"",
            "c",
            &[("defs.payload", "#define PAIR (1,2)\n")],
        );
        let key = |d: &Path| {
            let repo = load_repo(d).unwrap();
            current_c_census_identity(&repo.root, &repo.files, &repo.skipped)
        };
        let before = key(d.path());
        write(d.path(), "defs.payload", "#define PAIR 1,2\n");
        let after = key(d.path());
        assert_ne!(before, after, "loose_comma alone must move the aggregate");
        write(d.path(), "defs.payload", "#define PAIR (1,2)\n");
        assert_eq!(key(d.path()), before, "and moving back restores it");
    }
}
