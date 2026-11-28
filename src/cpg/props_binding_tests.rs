//! Guards of `props_binding::read_chain` pinned through the object-literal edges.
//!
//! `read_chain` classifies each bare Use of a mixed formal (`sinkP(p)` plus member reads)
//! by the static member chain read through it. A literal key feeds a read only when the
//! key path and the read chain are prefix-comparable, so the chain the walk reports is
//! observable as WHICH keys reach a read: a chain `[a]` feeds key `a` only, an empty chain
//! (bare, computed or unreadable use) feeds every key, and `None` feeds nothing.
//!
//! Callee `f(p)`: line 2 `sinkP(p);` (the bare read), line 3 the read under test; caller
//! `h` passes `{ a, b }` at line 6, so every edge is `h:<key>@6 -> f:p@<line>`.

use crate::cpg::object_literal_arg_tests::{build, edge_set};
use crate::languages::Language;
use std::collections::BTreeSet;

const TS: [(Language, &str); 2] = [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")];
const TS_ONLY: [(Language, &str); 1] = [(Language::TypeScript, "ts")];
const JS_FAMILY: [(Language, &str); 3] = [
    (Language::JavaScript, "js"),
    (Language::TypeScript, "ts"),
    (Language::Tsx, "tsx"),
];

/// A mixed formal whose line-3 read is `read`, called with `{ a, b }`.
fn fixture(read: &str) -> String {
    format!(
        "function f(p) {{\n  sinkP(p);\n  {read}\n}}\nfunction h(a, b) {{\n  f({{ a, b }});\n}}\n"
    )
}

fn assert_edges(langs: &[(Language, &str)], read: &str, expected: &[&str]) {
    let want: BTreeSet<String> = expected.iter().map(|e| e.to_string()).collect();
    for &(language, ext) in langs {
        let got = edge_set(&build(language, &[("m", &fixture(read))], ext));
        assert_eq!(got, want, "{read} [{language:?}]");
    }
}

/// Only key `a` reaches the line-3 read (and both keys the bare read of line 2).
const KEY_A: [&str; 3] = ["h:a@6 -> f:p@2", "h:a@6 -> f:p@3", "h:b@6 -> f:p@2"];
/// Only key `b` reaches the line-3 read.
const KEY_B: [&str; 3] = ["h:a@6 -> f:p@2", "h:b@6 -> f:p@2", "h:b@6 -> f:p@3"];
/// Every key reaches the line-3 read (empty chain).
const EVERY_KEY: [&str; 4] = [
    "h:a@6 -> f:p@2",
    "h:a@6 -> f:p@3",
    "h:b@6 -> f:p@2",
    "h:b@6 -> f:p@3",
];
/// The line-3 read is skipped: only the bare read of line 2 is fed.
const SKIPPED: [&str; 2] = ["h:a@6 -> f:p@2", "h:b@6 -> f:p@2"];

#[test]
fn a_transparent_wrapper_around_the_formal_continues_the_chain_walk() {
    // `(p as any).a`: the `as` wrapper's first expression is `p`, so the walk goes on to
    // `.a` and the read is chain `[a]`: it feeds key `a` and not key `b`.
    assert_edges(&TS, "sinkA((p as any).a);", &KEY_A);
    // Control: `g(p).a` has `p` as a call argument, not as the member object, so the
    // chain is `[]` and the read feeds every key.
    assert_edges(&JS_FAMILY, "sinkA(g(p).a);", &EVERY_KEY);
}

#[test]
fn a_type_assertion_around_a_member_read_keeps_the_member_chain() {
    // `<any>p.a` (TypeScript only; TSX has no angle assertions): the assertion carries
    // the expression last, after the type. The member chain `[a]` is read first, then the
    // walk reaches the assertion (the `type_assertion` last-child branch of
    // `first_expression`) and the chain stays `[a]`. Which child is "first" is not
    // observable here: a parenthesised form that would extend the chain past the
    // assertion has no identifier-exact use span, so it is never fed.
    assert_edges(&TS_ONLY, "sinkA(<any>p.a);", &KEY_A);
    // Control: the same read without the assertion has the same chain.
    assert_edges(&JS_FAMILY, "sinkA(p.a);", &KEY_A);
}

#[test]
fn a_non_identifier_member_property_ends_the_walk_with_an_empty_chain() {
    // `p.#x` names a private property, not a `property_identifier`: the walk stops at
    // `p` with chain `[]`, so the read may be anything under `p` and feeds every key.
    // (Pushing `#x` as a segment would make it comparable to no key and feed none.)
    assert_edges(&JS_FAMILY, "sinkA(p.#x);", &EVERY_KEY);
    // Control: a plain property keeps its own key.
    assert_edges(&JS_FAMILY, "sinkA(p.b);", &KEY_B);
}

#[test]
fn a_bare_use_that_is_not_exactly_the_identifier_feeds_nothing() {
    // The use of `p` in a declarator initialiser has no identifier span of its own
    // (`read_chain` is `None`): the read is skipped, not treated as an empty chain that
    // would feed every key.
    assert_edges(&JS_FAMILY, "const x = p.a;", &SKIPPED);
    // Same for a non-null assertion and for an assertion-wrapped object.
    assert_edges(&TS, "sinkA(p!.a);", &SKIPPED);
    assert_edges(&TS_ONLY, "sinkA((<any>p).a);", &SKIPPED);
    // Control: the identifier-spanned read on the same line still feeds its key.
    assert_edges(&JS_FAMILY, "sinkA(p.a);", &KEY_A);
}
