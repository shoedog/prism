//! #829 chain B, round 3 (sonnet3, class R): no cap may fail open. Alias chains of listed
//! block macros and of an unbalanced macro (sonnet3-07) and bracket bodies nested thousands
//! deep (sonnet3-08): the live pair must be kept at every depth, with no panic, abort or stack
//! overflow, and a depth or time limit, if the implementation has one, must fail closed (keep
//! the pair), never fall back to "clean".
//!
//! Depths: the enumerators compiled and ran chains of 100, 1,000, 5,000 and 20,000 aliases
//! (clang: `41 2` every time) and a 2,000-deep brace body (`-fbracket-depth=100000`; `41 2`);
//! base finishes the 20,000 chain in well under two minutes in a RELEASE build. These tests
//! run in the debug profile of `cargo test`, so they use the largest depth that finishes in a
//! few seconds there: `CHAIN_DEPTH` 1,000 (about 3 s per test on base; 3,000 takes 25 s: the
//! free-name closure is super-linear), `BRACKET_DEPTH` 20,000 (0.2 s), `GROUP_DEPTH` 300.
//! Required behaviour only, never an `Off::` reason.
use super::c_prune_brackets_r3_support::{guard_pair, Batch};

/// Alias hops per chain.
const CHAIN_DEPTH: usize = 1000;
/// Brackets per body.
const BRACKET_DEPTH: usize = 20000;
/// Parentheses around an argument in an OWNER (tree-sitter and the producer recurse over
/// them, and a `cargo test` thread has 2 MiB of stack: 3,000 overflows it on base).
const GROUP_DEPTH: usize = 300;

/// `#define {p}0 {root}` and `#define {p}{i} {p}{i-1}` for i in 1..=n.
fn chain(p: &str, root: &str, n: usize) -> String {
    let mut s = format!("#define {p}0 {root}\n");
    for i in 1..=n {
        s.push_str(&format!("#define {p}{i} {p}{}\n", i - 1));
    }
    s
}

fn owner(open: &str, close: &str, cpp: bool) -> String {
    let ret = if cpp { "sink(x + 1)" } else { "x" };
    format!(
        "int sink(int);\nvoid (*h)(void *);\nint probe(int c) {{\n    int x = 41;\n    if (c) {open};\n    x = 2;\n    {close};\n    return {ret};\n}}\n"
    )
}

fn file(cpp: bool) -> &'static str {
    if cpp {
        "a.cc"
    } else {
        "a.c"
    }
}

/// G2-sonnet3-07: `A<n>` aliases `pthread_cleanup_push`, `B<n>` aliases `pthread_cleanup_pop`
/// through `n` object-like hops. Compiled with `<pthread.h>`: `41 2` at every depth.
#[test]
fn row_g2_sonnet3_07_alias_chain_to_a_listed_name() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let n = CHAIN_DEPTH;
        let defs = format!(
            "{}{}",
            chain("A", "pthread_cleanup_push", n),
            chain("B", "pthread_cleanup_pop", n)
        );
        let src = format!(
            "{defs}{}",
            owner(&format!("A{n}(h, 0)"), &format!("B{n}(0)"), cpp)
        );
        b.kept_guard(
            &format!("sonnet3-07 listed chain {n}"),
            file(cpp),
            &src,
            Some("probe"),
        );
    }
    b.finish();
}

/// Same, with an unbalanced macro at the end of the chain (`Z0` is `{`, `Y0` is `}`).
#[test]
fn row_g2_sonnet3_07_alias_chain_to_an_unbalanced_macro() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let n = CHAIN_DEPTH;
        let defs = format!("{}{}", chain("Z", "{", n), chain("Y", "}", n));
        let src = format!("{defs}{}", owner(&format!("Z{n}"), &format!("Y{n}"), cpp));
        b.kept_guard(
            &format!("sonnet3-07 unbalanced chain {n}"),
            file(cpp),
            &src,
            Some("probe"),
        );
    }
    b.finish();
}

/// Same chain split across two headers (the second continues the first): the closure must
/// cross files at any depth.
#[test]
fn row_g2_sonnet3_07_alias_chain_split_across_headers() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let n = CHAIN_DEPTH;
        let half = n / 2;
        let (h1, h2) = if cpp {
            ("h1.hpp", "h2.hpp")
        } else {
            ("h1.h", "h2.h")
        };
        let first = format!(
            "{}{}",
            chain("A", "pthread_cleanup_push", half),
            chain("B", "pthread_cleanup_pop", half)
        );
        let mut second = String::new();
        for (p, from) in [("A", half), ("B", half)] {
            for i in (from + 1)..=n {
                second.push_str(&format!("#define {p}{i} {p}{}\n", i - 1));
            }
        }
        let src = owner(&format!("A{n}(h, 0)"), &format!("B{n}(0)"), cpp);
        let (from, to) = guard_pair(&src);
        b.kept(
            &format!("sonnet3-07 split chain {n}"),
            &[(file(cpp), &src), (h1, &first), (h2, &second)],
            file(cpp),
            &[],
            Some("probe"),
            from,
            to,
        );
    }
    b.finish();
}

/// G2-sonnet3-08: `OPEN` is `BRACKET_DEPTH` `{`, `CLOSE` the same number of `}`. Compiled
/// with `-fbracket-depth=100000` at 2,000: `41 2`. No panic, abort or stack overflow, and the
/// pair kept (a depth cap must fail closed).
#[test]
fn row_g2_sonnet3_08_deep_brace_bodies() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let defs = format!(
            "#define OPEN {}\n#define CLOSE {}\n",
            "{".repeat(BRACKET_DEPTH),
            "}".repeat(BRACKET_DEPTH)
        );
        let src = format!("{defs}{}", owner("OPEN", "CLOSE", cpp));
        b.kept_guard("sonnet3-08 braces", file(cpp), &src, Some("probe"));
    }
    b.finish();
}

/// Member (rules): deep parenthesis and bracket bodies in the opener only, the closer
/// balanced by a separate macro (the opener alone is unbalanced).
#[test]
fn row_g2_sonnet3_08_deep_paren_and_square_bodies() {
    let mut b = Batch::new();
    for (what, o, c) in [("parens", '(', ')'), ("squares", '[', ']')] {
        let defs = format!(
            "#define OPEN {}\n#define CLOSE {}\n",
            o.to_string().repeat(BRACKET_DEPTH),
            c.to_string().repeat(BRACKET_DEPTH)
        );
        let src = format!("{defs}{}", owner("OPEN", "CLOSE", false));
        b.kept_guard(&format!("sonnet3-08 {what}"), "a.c", &src, Some("probe"));
    }
    b.finish();
}

/// Control (K): a deeply nested BALANCED body is still a harmless macro; the dead pair prunes
/// (a limit must not turn every deep body into a refusal that costs the control).
#[test]
fn row_g2_sonnet3_08_balanced_deep_body_control_prunes() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let defs = format!(
            "#define OPEN {}{}\n#define CLOSE (void)0\n",
            "{".repeat(BRACKET_DEPTH),
            "}".repeat(BRACKET_DEPTH)
        );
        let src = format!("{defs}{}", owner("OPEN", "CLOSE", cpp));
        b.pruned_guard(
            "sonnet3-08 balanced control",
            file(cpp),
            &src,
            Some("probe"),
        );
    }
    b.finish();
}

/// Derived (rules, base handles `ID((((...c...))))` at 10^5 in 0.6 s in release): an owner
/// group nested deep must not break the argument-group walk; the pair stays.
#[test]
fn row_g2_sonnet3_08_owner_group_nested_deep() {
    let mut b = Batch::new();
    for cpp in [false, true] {
        let n = GROUP_DEPTH;
        let ret = if cpp { "sink(x + 1)" } else { "x" };
        let src = format!(
            "#define OPEN {{\n#define CLOSE }}\n#define ID(v) v\nint sink(int);\nint probe(int c) {{\n    int x = 41;\n    ID({}c{});\n    if (c) OPEN;\n    x = 2;\n    CLOSE;\n    return {ret};\n}}\n",
            "(".repeat(n),
            ")".repeat(n)
        );
        b.kept_guard(
            "sonnet3-08 deep owner group",
            file(cpp),
            &src,
            Some("probe"),
        );
    }
    b.finish();
}
