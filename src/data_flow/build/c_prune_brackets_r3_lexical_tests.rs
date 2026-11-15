//! #829 chain B, round 3: tokens and comment delimiters that exist only after line splicing
//! (sonnet3-01 to 03), a bracket hidden behind a string/comment interaction in a permissive
//! census text (terra3-01, 02), and an external macro that supplies the split (sonnet3-10,
//! BOUNDARY). Every row asserts the REQUIRED behaviour (the live pair `x = 41 -> return` is
//! kept, or the control pair is pruned), never a specific `Off::` reason: the rule of
//! DESIGN.md is tentative. `boundary_*` rows depend on an input the model cannot see.
//! Kinds: S live pair dropped on base (compiled), C conservative (rules), K control.
//! The depth rows (sonnet3-07, 08) live in `c_prune_brackets_r3_depth_tests.rs`.
use super::c_prune_brackets_r3_support::{guard_pair, guard_source, Batch};
use super::c_prune_oracle_rows::declare;

fn ext(cpp: bool) -> &'static str {
    if cpp {
        "a.cc"
    } else {
        "a.c"
    }
}

// ---- G2-sonnet3-01 .. 03: a delimiter that exists only after phase 2 (splice) ----

/// The bracket digraph `%>` / `<%` is written across a splice, so it is no token in the raw
/// body. `clang -E -P`: `if (c) <%;` / `x = 2;` / `%>;`. Compiled, C11 and C++17: `41 2`
/// (`42 3` in C++).
#[test]
fn row_g2_sonnet3_01_digraph_brace_formed_across_a_splice() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = guard_source("#define CLOSE %\\\n>\n#define OPEN <\\\n%\n", cpp);
        b.kept_guard("sonnet3-01 brace digraph", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// Member (rules): the square digraphs `<:` and `:>` written across a splice. No liveness run.
#[test]
fn row_g2_sonnet3_01_member_square_digraph_formed_across_a_splice() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = guard_source("#define CLOSE :\\\n>\n#define OPEN <\\\n:\n", cpp);
        b.kept_guard("sonnet3-01 square digraph", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// `/\<nl>*` opens a comment that swallows the `}` (the body of OPEN is `{`); `*\<nl>/` closes
/// it. Compiled, C and C++: `41 2` / `42 3`.
#[test]
fn row_g2_sonnet3_02_comment_opener_formed_by_a_splice() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = guard_source("#define OPEN { /\\\n* } *\\\n/\n#define CLOSE }\n", cpp);
        b.kept_guard("sonnet3-02 comment opener", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// Member (obs, C: `41 2`): only the comment CLOSER is split by a splice (`*\<nl>/`).
#[test]
fn row_g2_sonnet3_02_member_comment_closer_formed_by_a_splice() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = guard_source("#define OPEN { /* } *\\\n/\n#define CLOSE }\n", cpp);
        b.kept_guard("sonnet3-02 comment closer", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// `/\<nl>/ }` is a line comment introducer built by a splice. Compiled: `41 2` / `42 3`.
#[test]
fn row_g2_sonnet3_03_line_comment_introducer_formed_by_a_splice() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let src = guard_source("#define OPEN { /\\\n/ }\n#define CLOSE }\n", cpp);
        b.kept_guard("sonnet3-03 line comment", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// Control (K): the same splice shapes around BALANCED bodies prune (`<% %>` and `{ }` close
/// themselves, so `x = 2` is unconditional).
#[test]
fn row_g2_sonnet3_01_to_03_balanced_splice_controls_prune() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        for (what, defs) in [
            (
                "digraph pair",
                "#define OPEN <\\\n% %\\\n>\n#define CLOSE (void)0\n",
            ),
            (
                "comment opener",
                "#define OPEN { /\\\n* } *\\\n/ }\n#define CLOSE (void)0\n",
            ),
        ] {
            let src = guard_source(defs, cpp);
            b.pruned_guard(what, ext(cpp), &src, Some("probe"));
        }
    }
    b.finish();
}

// ---- G2-terra3-01, 02: the permissive census text reads strings and comments ----

/// Census text and the same definitions in a parsed header: a comment introducer inside a
/// string precedes the real unmatched brace, a real comment holds the apparent matching brace.
fn text_and_header_rows(what: &str, defs: &str, owner_with: fn(bool) -> String) {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let owner = owner_with(cpp);
        let (from, to) = guard_pair(&owner);
        let (target, hdr) = if cpp {
            ("a.cc", "h.hpp")
        } else {
            ("a.c", "h.h")
        };
        b.kept(
            &format!("{what} (census text, {target})"),
            &[(target, &owner)],
            target,
            &[("defs.payload", defs)],
            Some("probe"),
            from,
            to,
        );
        b.kept(
            &format!("{what} (parsed header, {target})"),
            &[(target, &owner), (hdr, defs)],
            target,
            &[],
            Some("probe"),
            from,
            to,
        );
    }
    b.finish();
}

fn payload_owner(cpp: bool) -> String {
    let ret = if cpp { "sink(x + 1)" } else { "x" };
    format!(
        "#include \"defs.payload\"\nint sink(int);\nint probe(int c) {{\n    int x = 41;\n    OPEN;\n    x = 2;\n    CLOSE;\n    return {ret};\n}}\n"
    )
}

#[test]
fn row_g2_terra3_01_string_comment_introducer_then_a_real_brace() {
    text_and_header_rows(
        "terra3-01",
        "#define OPEN (void) \"/*\"; if (c) { /* } */\n#define CLOSE }\n",
        payload_owner,
    );
}

/// Member: the string holds `//` instead, so erasure runs to the end of the line.
#[test]
fn row_g2_terra3_01_member_line_comment_introducer_in_a_string() {
    text_and_header_rows(
        "terra3-01 member",
        "#define OPEN (void) \"//\"; if (c) { /* } */\n#define CLOSE }\n",
        payload_owner,
    );
}

fn payload_comma_owner(cpp: bool) -> String {
    let ret = if cpp { "sink(x + 1)" } else { "x" };
    format!(
        "#include \"defs.payload\"\nint sink(int);\nint probe(int c) {{\n    int x = 41;\n    int b = 0;\n    {{\n        H(INIT);\n        x = 2;\n        J(INIT);\n        return {ret};\n    }}\n}}\n"
    )
}

/// The brace-held comma of `INIT` is exposed after `INIT` expands; comment-kept text holds a
/// `(` before and `)` after it that are really in comments; the string hides a comment start.
#[test]
fn row_g2_terra3_02_comma_between_comment_parentheses_after_a_string() {
    text_and_header_rows(
        "terra3-02",
        "#define INIT { /* ( */ (void)\"/*\"; int a, b; /* ) */ }\n#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n",
        payload_comma_owner,
    );
}

/// Control (K): the same payload without the brace-held comma prunes.
#[test]
fn row_g2_terra3_02_control_without_the_comma_prunes() {
    let defs = "#define INIT { /* ( */ (void)\"/*\"; int a; /* ) */ }\n#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\n";
    let mut b = Batch::new();
    for cpp in [false, true] {
        let owner = payload_comma_owner(cpp);
        let (from, to) = guard_pair(&owner);
        let target = ext(cpp);
        b.pruned(
            &format!("terra3-02 control {target}"),
            &[(target, &owner)],
            target,
            &[("defs.payload", defs)],
            Some("probe"),
            from,
            to,
        );
    }
    b.finish();
}

// ---- G2-sonnet3-10 (BOUNDARY): an external macro supplies the brace-held comma ----

/// `EXT_INIT` (`{ int a, b; }`) is defined in a header outside the repository
/// (`#include <ext.h>`, `-I` dir). The owner shows only `H(EXT_INIT)`; in-repo `H`/`J` forward
/// the expansion to two-parameter macros. Compiled with the header: `41 2` (C), `42 3` (C++,
/// compiled here). Real analogue: `PyModuleDef_HEAD_INIT` passed to a forwarding macro.
#[test]
fn boundary_g2_sonnet3_10_external_macro_supplies_the_split() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let ret = if cpp { "x + 1" } else { "x" };
        let src = format!(
            "#include <ext.h>\n#define F(p, q) q if (c) p\n#define G(p, q) q p\n#define H(p) F(p)\n#define J(p) G(p)\nint probe(int c) {{\n    int x = 41;\n    int b = 0;\n    {{\n        H(EXT_INIT);\n        x = 2;\n        J(EXT_INIT);\n        return {ret};\n    }}\n}}\n"
        );
        declare(
            "A-ext",
            &["-I{PRELUDE}"],
            &[("ext.h", "#define EXT_INIT { int a, b; }\n")],
        );
        b.kept_guard("sonnet3-10", ext(cpp), &src, Some("probe"));
    }
    b.finish();
}
