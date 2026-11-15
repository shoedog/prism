//! #829 round-1 enumeration (terra + sol): owner rows added after the first matrix. Ids are
//! `G-terra-NN` / `G-sol-NN` (TESTS.md has the dedupe map). Per-row pair tests read the same
//! `obligations.jsonl` as the matrices; the multi-file and loader rows judge the pair by owner
//! (chain B conversion: they used to assert a gate reason or edge totals). Written against the
//! stubs: every keep row is red until the gates exist.
use super::c_prune::{Off, BLOCK_MACROS};
use super::c_prune_brackets_r3_support::{line_of, loader_pair, Batch};
use super::c_prune_brackets_tests::{check_row, expect, verdicts};
use crate::call_graph::c::macro_census::CMacroCensus;

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

// The E01 owner-verdict tests of this file (`e01_22_*`, `e01_23_*`, `e01_24_splice_names`)
// asserted `Off::MacroBrackets|MacroComma|MacroArgSplit` or `None` per owner. They are removed
// (TESTS.md, "Converted to required-behaviour"): each owner they listed has a pair row above
// (w1-w8, a1-a12, k1-k3, y5-y11, s1-s4), and the rows judge the pair, not the reason.

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
    format!("{defs}int f(int c) {{\n    int x = 1;\n    if (c) {call};\n    x = 2;\n    return x;\n}}\n")
}

/// Converted from a gate-reason assertion to the pair: a single listed name can open or close
/// a block (kind C, rules: no liveness run per name), so `x = 1` to `return x` is kept.
fn listed_name_rows(shapes: &[&str]) {
    let mut b = Batch::new();
    for file in ["a.c", "a.cc"] {
        for name in BLOCK_NAMES {
            for shape in shapes {
                let src = wrapper_source(name, shape);
                let (from, to) = (line_of(&src, "int x = 1;"), line_of(&src, "return x;"));
                b.kept(
                    &format!("{file} {name} via {shape}"),
                    &[(file, &src)],
                    file,
                    &[],
                    Some("f"),
                    from,
                    to,
                );
            }
        }
    }
    b.finish();
}

#[test]
fn g_sol_01_every_listed_name_keeps_the_pair_through_wrappers() {
    listed_name_rows(&["function-like multi-hop", "object-like alias"]);
}

/// Direct spelling of each of the 16 names (the matrix pins two of them through b12 and b13).
#[test]
fn g_sol_01_every_listed_name_keeps_the_pair_when_spelled_directly() {
    listed_name_rows(&["direct"]);
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
#[ignore = "tentative design (#829 TEST-FORM §5)"]
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

/// The live pair is `x = src()` (line 5) to `sink(x + 1)` (line 9): `if (cond) ENTER(..)` opens
/// a block (through the wrapper) that makes `x = 2` conditional.
const WRAP_OWNER: &str = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void (*h)(void *)) {\n    int x = src();\n    if (cond) ENTER(h, 0);\n    x = 2;\n    LEAVE(0);\n    sink(x + 1);\n}\n";
const WRAP_DEFS: &str =
    "#define ENTER(h, p) pthread_cleanup_push(h, p)\n#define LEAVE(p) pthread_cleanup_pop(p)\n";
const WRAP_DEFS_BALANCED: &str = "#define ENTER(h, p) ((void)(h))\n#define LEAVE(p) ((void)(p))\n";

#[test]
fn g_terra_01_wrapper_in_a_header_c_and_cpp() {
    let mut b = Batch::new();
    for (owner, hdr) in [("a.c", "h.h"), ("a.cc", "h.hpp")] {
        b.kept(
            &format!("{owner}: the wrapper names a listed macro"),
            &[(owner, WRAP_OWNER), (hdr, WRAP_DEFS)],
            owner,
            &[],
            Some("f"),
            5,
            9,
        );
        b.pruned(
            &format!("{owner}: control, a harmless wrapper"),
            &[(owner, WRAP_OWNER), (hdr, WRAP_DEFS_BALANCED)],
            owner,
            &[],
            Some("f"),
            5,
            9,
        );
    }
    b.finish();
}

#[test]
fn g_terra_01_wrapper_in_a_census_text_c_and_cpp() {
    let mut b = Batch::new();
    for owner in ["a.c", "a.cc"] {
        b.kept(
            &format!("{owner}: census text wrapper"),
            &[(owner, WRAP_OWNER)],
            owner,
            &[("defs.inc", WRAP_DEFS)],
            Some("f"),
            5,
            9,
        );
        b.pruned(
            &format!("{owner}: control"),
            &[(owner, WRAP_OWNER)],
            owner,
            &[("defs.inc", WRAP_DEFS_BALANCED)],
            Some("f"),
            5,
            9,
        );
    }
    b.finish();
}

#[test]
fn g_terra_01_alias_chain_across_two_headers() {
    let owner = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void (*h)(void *)) {\n    int x = src();\n    if (cond) E2(h, 0);\n    x = 2;\n    L2(0);\n    sink(x + 1);\n}\n";
    let mut b = Batch::new();
    for (target, h1, h2) in [("a.c", "h1.h", "h2.h"), ("a.cc", "h1.hpp", "h2.hpp")] {
        let files = [
            (target, owner),
            (h1, "#define E1(h, p) pthread_cleanup_push(h, p)\n#define L1(p) pthread_cleanup_pop(p)\n"),
            (h2, "#define E2(h, p) E1(h, p)\n#define L2(p) L1(p)\n"),
        ];
        b.kept(
            &format!("{target}: two hops across two headers"),
            &files,
            target,
            &[],
            Some("f"),
            5,
            9,
        );
    }
    b.finish();
}

// ---- G-sol-03: the composed split with the census arriving from a header ----

#[test]
fn g_sol_03_rescan_split_through_a_header() {
    // The live pair is `x = 41` (line 3) to `return x` (line 9).
    let owner = "int src(void);\nint f(int c) {\n    int x = 41;\n    int b = 0;\n    {\n        H(INIT);\n        x = 2;\n        J(INIT);\n        return x;\n    }\n}\n";
    let defs = "#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n#define INIT { int a, b; }\n";
    let mut b = Batch::new();
    b.kept(
        "INIT's brace-held comma",
        &[("a.c", owner), ("h.h", defs)],
        "a.c",
        &[],
        Some("f"),
        3,
        9,
    );
    let tight = defs.replace("{ int a, b; }", "{ int a; }");
    b.pruned(
        "control: no comma in INIT",
        &[("a.c", owner), ("h.h", &tight)],
        "a.c",
        &[],
        Some("f"),
        3,
        9,
    );
    b.finish();
}

// ---- G-terra-10: a splice assembling a name, a comment before the call, header-borne ----

#[test]
fn g_terra_10_spliced_name_from_a_header_with_lf_and_crlf() {
    // The live pair is `x = src()` (line 5) to `sink(x + 1)` (line 9).
    let owner = "int src(void);\nvoid sink(int);\nint cond;\nvoid f(void) {\n    int x = src();\n    if (cond) OPEN /* comment */ ;\n    x = 2;\n    CLOSE;\n    sink(x + 1);\n}\n";
    let defs = "#define OP\\\nEN {\n#define CLOSE }\n";
    let mut b = Batch::new();
    for (target, hdr) in [("a.c", "h.h"), ("a.cc", "h.hpp")] {
        for (what, d) in [
            ("lf", defs.to_owned()),
            ("crlf", defs.replace('\n', "\r\n")),
        ] {
            b.kept(
                &format!("{target} {what}: spliced name"),
                &[(target, owner), (hdr, &d)],
                target,
                &[],
                Some("f"),
                5,
                9,
            );
        }
    }
    b.finish();
}

// ---- G-terra-14 / G-sol-14: loader discovery drives the new facts ----

pub(super) mod loaded {
    use super::*;
    use crate::repo_loader::load_repo;
    use std::fs;
    use std::path::Path;

    pub(in super::super) fn write(d: &Path, rel: &str, bytes: &str) {
        let p = d.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }

    /// The two guard owners of a repo (`a.<ext>` and `b.<ext>`): the pair `x = 41` to `return x`
    /// of each, `kept` or (control) `pruned`.
    fn judge(b: &mut Batch, what: &str, d: &Path, ext: &str, kept: bool) {
        for (file, owner) in [(format!("a.{ext}"), "f"), (format!("b.{ext}"), "comma")] {
            let text = fs::read_to_string(d.join(&file)).unwrap();
            let (from, to) = (line_of(&text, "int x = "), line_of(&text, "return x;"));
            let r = loader_pair(d, &file, Some(owner), from, to);
            if kept {
                b.kept_result(&format!("{what} {file}"), r, Some(owner), from, to);
            } else {
                b.pruned_result(&format!("{what} {file}"), r, Some(owner), from, to);
            }
        }
    }

    /// Premise: the loader's census holds the payload's macros (a base failure must be the
    /// pair assertion, never a payload the loader did not read).
    fn assert_read(d: &Path) {
        let repo = load_repo(d).unwrap();
        let census = CMacroCensus::from_files(&repo.files);
        for n in ["BEGIN", "END", "INIT"] {
            assert!(census.get(n).is_some(), "{n} not read by the loader");
        }
    }

    /// `f`: the b1 guard owner (live pair `x = 41` to `return x`). `comma`: the rescan split
    /// owner, whose brace-held comma comes from `INIT` (the pair is live when `c == 0`).
    /// (Converted: the old `comma` owner, `int y = (PAIR);`, has no live pair: `(1,2)` sits in
    /// parentheses and `x = 2` dominates, so no required behaviour was left to assert.)
    const GUARD: &str = "int f(int c) {\n    int x = 41;\n    if (c) BEGIN;\n    x = 2;\n    END;\n    return x;\n}\n";
    const SPLIT: &str = "int comma(int c) {\n    int x = 41;\n    int b = 0;\n    {\n        H(INIT);\n        x = 2;\n        J(INIT);\n        return x;\n    }\n}\n";
    const DEFS: &str = "#define BEGIN {\n#define END }\n#define INIT { int a, b; }\n#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n";
    const DEFS_BALANCED: &str = "#define BEGIN (void)0\n#define END (void)0\n#define INIT { int a; }\n#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n";

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
            &format!("{include_line}\n{SPLIT}"),
        );
        for (rel, body) in tree {
            write(d.path(), rel, body);
        }
        d
    }

    /// sol-14: `first.payload` includes `defs.payload`, neither is C: the loader reads both
    /// as census texts, and the facts reach the owners of both dialects.
    #[test]
    fn g_sol_14_transitive_non_c_include_keeps_the_pairs() {
        let mut b = Batch::new();
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
            judge(&mut b, &format!("{ext}"), d.path(), ext, true);
        }
        b.finish();
    }

    /// terra-14: quoted and angle forms, one hop through a `.inc` file.
    #[test]
    fn g_terra_14_quoted_and_angle_transitive_includes() {
        let mut b = Batch::new();
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
            judge(&mut b, &format!("{ext} {line}"), d.path(), ext, true);
        }
        b.finish();
    }

    /// The C-like-extension net: nothing includes `defs.inc`, yet it is read.
    #[test]
    fn g_sol_14_net_file_without_an_include_keeps_the_pairs() {
        let mut b = Batch::new();
        for ext in ["c", "cc"] {
            let d = guard_repo("", ext, &[("defs.inc", DEFS)]);
            assert_read(d.path());
            judge(&mut b, &format!("{ext}"), d.path(), ext, true);
        }
        b.finish();
    }

    /// Control: the same tree with balanced, tight definitions prunes (K, green on base).
    #[test]
    fn g_sol_14_control_balanced_payload_prunes() {
        let mut b = Batch::new();
        for ext in ["c", "cc"] {
            let d = guard_repo(
                "#include \"first.payload\"",
                ext,
                &[
                    ("first.payload", "#include \"defs.payload\"\n"),
                    ("defs.payload", DEFS_BALANCED),
                ],
            );
            judge(&mut b, &format!("{ext} control"), d.path(), ext, false);
        }
        b.finish();
    }
}
