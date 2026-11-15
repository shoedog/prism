//! #829 chain B, round 3: census-fact rows. TENTATIVE-DESIGN FACT TESTS: they pin the
//! `unbalanced` / `loose_comma` values of the tentative rule (DESIGN.md) and will be revisited
//! at the redesign; the required BEHAVIOUR of the same inputs is pinned by the pair rows in
//! `data_flow/build/c_prune_brackets_r3_*`. Same stubs as the other bracket census tests:
//! every `true` row is red on base, every `false` row is a green guard against
//! over-approximation. Each row runs in a parsed `.c`, a parsed `.cpp` and a census text.
use super::macro_census_brackets_r2_tests::{expect_everywhere, text_census};

// ---- G2-sonnet3-01 .. 03: a delimiter that exists only after phase 2 (splice) ----

/// `%\<nl>>` and `<\<nl>%` are the digraphs `%>` and `<%`; `<\<nl>:` and `:\<nl>>` are `<:`
/// and `:>`. The fact must be computed on the spliced text.
#[test]
fn g2_sonnet3_01_digraph_brackets_written_across_a_splice() {
    expect_everywhere(
        "#define CL %\\\n>\n#define OP <\\\n%\n#define SCL :\\\n>\n#define SOP <\\\n:\n",
        &[
            ("CL", true, false),
            ("OP", true, false),
            ("SCL", true, false),
            ("SOP", true, false),
        ],
    );
}

/// `/\<nl>*` opens a comment that swallows the `}`; `*\<nl>/` closes one; `/\<nl>/` starts a
/// line comment. Each body is `{` after comment removal.
#[test]
fn g2_sonnet3_02_03_comment_delimiters_formed_by_a_splice() {
    expect_everywhere(
        "#define O1 { /\\\n* } *\\\n/\n#define O2 { /* } *\\\n/\n#define O3 { /\\\n/ }\n",
        &[
            ("O1", true, false),
            ("O2", true, false),
            ("O3", true, false),
        ],
    );
}

/// Controls: balanced bodies around the same splices stay `false`.
#[test]
fn g2_sonnet3_01_to_03_balanced_splice_controls() {
    expect_everywhere(
        "#define B1 { /\\\n* } *\\\n/ }\n#define B2 <\\\n% %\\\n>\n#define B3 { } /\\\n/ {\n",
        &[
            ("B1", false, false),
            ("B2", false, false),
            ("B3", false, false),
        ],
    );
}

/// The `...` of a variadic parameter list written across a splice (`.\<nl>..`) reads as a
/// plain variadic: `VF` has a loose comma, `VG` forwards inside parentheses and has none.
#[test]
fn g2_sonnet3_splice_member_ellipsis_across_a_splice() {
    expect_everywhere(
        "#define VF(a, .\\\n..) a, __VA_ARGS__\n#define VG(a, .\\\n..) f(a, __VA_ARGS__)\n",
        &[("VF", false, true), ("VG", false, false)],
    );
}

// ---- G2-sonnet3-04: comment introducers inside literals must not hide a real bracket ----

/// The lexer must not read a comment out of a literal: each body keeps its brace. In a census
/// text (no string tracking by design) the comment-kept reading still sees it.
#[test]
fn g2_sonnet3_04_comment_introducers_inside_literals_do_not_hide_a_brace() {
    expect_everywhere(
        "#define U1 \"a//b\" {\n#define U2 \"/*\" {\n#define V2 \"*/\"\n#define U3 '/' {\n#define U4 \"\\\"//\" {\n",
        &[
            ("U1", true, false),
            ("U2", true, false),
            ("V2", false, false),
            ("U3", true, false),
            ("U4", true, false),
        ],
    );
}

// ---- G2-terra3-01, 02: a string hides a comment start, a real comment holds the other bracket ----

#[test]
fn g2_terra3_01_string_comment_introducer_then_a_real_brace() {
    expect_everywhere(
        "#define OPEN (void) \"/*\"; if (c) { /* } */\n#define CLOSE }\n#define OPEN2 (void) \"//\"; if (c) { /* } */\n",
        &[
            ("OPEN", true, false),
            ("CLOSE", true, false),
            ("OPEN2", true, false),
        ],
    );
}

/// The braces of `INIT` balance, its comma is brace-held (loose); the parentheses around the
/// comma sit in real comments and must not tighten it.
#[test]
fn g2_terra3_02_comma_between_comment_parentheses_after_a_string() {
    expect_everywhere(
        "#define INIT { /* ( */ (void)\"/*\"; int a, b; /* ) */ }\n#define TIGHT { /* ( */ (void)\"/*\"; int a; /* ) */ }\n",
        &[("INIT", false, true), ("TIGHT", false, false)],
    );
}

// ---- G2-sonnet3-08: nesting depth of the bracket scan (no panic, no stack overflow) ----

const DEEP: usize = 100_000;

/// A body of 10^5 openers is unbalanced; its balanced twin must not crash or be flagged.
/// (The pair rows of `c_prune_brackets_r3_depth_tests` pin the behaviour at the owner level.)
#[test]
fn g2_sonnet3_08_deep_bodies_in_a_census_text() {
    for (name, o, c) in [("P", '(', ')'), ("B", '{', '}'), ("S", '[', ']')] {
        let open = o.to_string().repeat(DEEP);
        let close = c.to_string().repeat(DEEP);
        let src = format!("#define {name}OPEN {open}\n#define {name}BAL {open}{close}\n");
        let census = text_census(&src);
        assert!(
            census.get(&format!("{name}OPEN")).unwrap().unbalanced,
            "{name}: {DEEP} openers must be flagged"
        );
        assert!(
            !census.get(&format!("{name}BAL")).unwrap().unbalanced,
            "{name}: the balanced twin must not be flagged"
        );
    }
}
