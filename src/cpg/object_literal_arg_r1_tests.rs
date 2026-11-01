//! #594 review round 1: member writes in a mixed formal, write targets in expression
//! position, TypeScript angle-bracket assertions, and the pinned zero-width-read gap.
//! Node spelling and helpers are those of `object_literal_arg_tests`.

use super::object_literal_arg_tests::{build, check, edge_set, LANGS};
use crate::languages::Language;
use std::collections::BTreeSet;

/// One statement `write` between a bare read of `p` and reads of `p.a` and `p.z`; the
/// literal feeds `a` (the written key) and `z` (the control).
fn write_fixture(write: &str) -> String {
    format!(
        "function f(p, s) {{\n  sink(p);\n  {write}\n  sink(p.a);\n  sink(p.z);\n}}\nfunction h(x, z, s) {{\n  f({{ a: x, z }}, s);\n}}\n"
    )
}

/// Only the control key may reach the callee, through the bare read and its own read.
const CONTROL_ONLY: [&str; 2] = ["h:z@8 -> f:p@2", "h:z@8 -> f:p@5"];

const JS_WRITE_FORMS: [&str; 19] = [
    "p.a = clean();",
    "p.a += clean();",
    "p.a ||= clean();",
    "p.a++;",
    "--p.a;",
    "delete p.a;",
    "({ k: p.a } = s);",
    "[p.a] = s;",
    "[p.a = 1] = s;",
    "({ ...p.a } = s);",
    "({ k: p.a = 1 } = s);",
    "for (p.a of s) {}",
    "for (p.a in s) {}",
    "sink(p.a = 1);",
    "sink(({ k: p.a } = s));",
    "sink([p.a = 1] = s);",
    "sink(({ ...p.a } = s));",
    "(() => { p.a = 1; })();",
    "(function () { delete p.a; })();",
];

#[test]
fn every_member_write_form_stops_the_overwritten_key_reaching_the_formal() {
    for form in JS_WRITE_FORMS {
        check(form, &write_fixture(form), &CONTROL_ONLY);
    }
}

#[test]
fn ts_wrapped_member_write_targets_stop_the_overwritten_key() {
    for form in [
        "(p as any).a = 1;",
        "p!.a = 1;",
        "(p satisfies object).a = 1;",
    ] {
        let src = write_fixture(form);
        let want: BTreeSet<String> = CONTROL_ONLY.iter().map(|s| s.to_string()).collect();
        for (language, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
            let cpg = build(language, &[("m", &src)], ext);
            assert_eq!(edge_set(&cpg), want, "{form} [{language:?}]");
        }
    }
    let angle = write_fixture("(<any>p).a = 1;");
    let cpg = build(Language::TypeScript, &[("m", &angle)], "ts");
    let want: BTreeSet<String> = CONTROL_ONLY.iter().map(|s| s.to_string()).collect();
    assert_eq!(edge_set(&cpg), want, "angle-bracket write target");
}

#[test]
fn overwritten_field_with_a_bare_read_before_the_write_is_the_issue_example() {
    // Sol W1: `observe(p); p.a = "clean"; exec(p.a);` must not feed `x` into `exec(p.a)`.
    let src = "function f(p) {\n  observe(p);\n  p.a = \"clean\";\n  exec(p.a);\n  exec(p.b);\n}\nfunction h(x, y) {\n  f({ a: x, b: y });\n}\n";
    check("w1", src, &["h:y@8 -> f:p@2", "h:y@8 -> f:p@5"]);
}

#[test]
fn expression_position_write_targets_are_not_reads() {
    // Opus W1: a write target inside a larger expression is no read of the key.
    let src = "function f(p, src) {\n  sink(p);\n  sink(p.a = 1);\n  sink(({ k: p.b } = src));\n  sink([p.c = 1] = src);\n  sink(({ ...p.d } = src));\n  sink(p.e++);\n  sink(p.z);\n}\nfunction h(a, b, c, d, e, z, s) {\n  f({ a, b, c, d, e, z }, s);\n}\n";
    check("expr-writes", src, &["h:z@11 -> f:p@2", "h:z@11 -> f:p@8"]);
}

#[test]
fn a_write_below_the_key_stops_reads_at_or_above_the_written_path_only() {
    // `p.a.b = 1` may change what `p.a` holds, so `p.a` is not fed; `p.a.c` and the bare
    // read still see the object `x`.
    let src = "function f(p) {\n  sink(p);\n  p.a.b = 1;\n  sink(p.a);\n  sink(p.a.c);\n  sink(p.z);\n}\nfunction h(x, z) {\n  f({ a: x, z });\n}\n";
    check(
        "deeper-write",
        src,
        &[
            "h:x@9 -> f:p@2",
            "h:x@9 -> f:p@5",
            "h:z@9 -> f:p@2",
            "h:z@9 -> f:p@6",
        ],
    );
}

#[test]
fn a_write_to_a_prefix_stops_every_leaf_below_it() {
    let src = "function f(p) {\n  sink(p.a.b);\n  sink(p.c);\n  p.a = clean();\n}\nfunction h(x, z) {\n  f({ a: { b: x }, c: z });\n}\n";
    check("prefix-write", src, &["h:z@7 -> f:p.c@3"]);
}

#[test]
fn writes_to_other_paths_other_bases_or_other_bindings_do_not_suppress() {
    // Each formal is read bare too (a mixed formal), so the write analysis is what decides.
    let other_key = "function f(p) {\n  sink(p);\n  p.q = 1;\n  sink(p.a);\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check(
        "other-key",
        other_key,
        &["h:x@7 -> f:p@2", "h:x@7 -> f:p@4"],
    );
    let other_base = "function f(p, q) {\n  sink(p);\n  q.a = 1;\n  sink(p.a);\n}\nfunction h(x) {\n  f({ a: x }, {});\n}\n";
    check(
        "other-base",
        other_base,
        &["h:x@7 -> f:p@2", "h:x@7 -> f:p@4"],
    );
    let shadowed = "function f(p) {\n  sink(p);\n  sink(p.a);\n  function inner(p) {\n    p.a = 1;\n  }\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("shadowed", shadowed, &["h:x@9 -> f:p@2", "h:x@9 -> f:p@3"]);
}

#[test]
fn ts_angle_bracket_assertion_around_the_literal_is_transparent() {
    let src = "function f(p: any) {\n  sinkA(p.a);\n  sinkB(p.b);\n}\nfunction h(a: any) {\n  f(<any>{ a });\n  f((<any>{ a }));\n  f(<any>({ a }));\n  f({ a } as any);\n}\n";
    let cpg = build(Language::TypeScript, &[("m", src)], "ts");
    assert_eq!(
        edge_set(&cpg),
        BTreeSet::from([6, 7, 8, 9].map(|line| format!("h:a@{line} -> f:p.a@2"))),
    );
}

#[test]
fn the_literal_prefilter_admits_angle_brackets_only_for_typescript() {
    // In TSX a leading `<` is JSX, never an assertion: no literal edge, no crash.
    let src = "function f(p) {\n  sinkA(p.a);\n}\nfunction h(a) {\n  f(<div a={a} />);\n}\n";
    let cpg = build(Language::Tsx, &[("m", src)], "tsx");
    assert_eq!(edge_set(&cpg), BTreeSet::new());
}

#[test]
fn zero_width_base_reads_of_a_mixed_formal_are_unfed_today() {
    // Known gap (js-ts-known-gaps.md, #594): a mixed formal's reads in a declarator
    // initialiser, a condition, a `for-in` head and an `in` test carry a zero-width base
    // Use, so the member chain cannot be read back and the edge fails closed. The bare
    // read (line 2) and the argument-position read (line 8) are fed. If this test starts
    // failing because the unfed lines gained edges, update the gap entry.
    let src = "function f(o) {\n  sink(o);\n  const t = o.a;\n  const { a } = o;\n  if (o.a) {}\n  for (const k in o) {}\n  \"x\" in o;\n  sink(o.a);\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    for (language, ext) in LANGS {
        let cpg = build(language, &[("m", src)], ext);
        assert_eq!(
            edge_set(&cpg),
            BTreeSet::from(["h:x@11 -> f:o@2", "h:x@11 -> f:o@8"].map(String::from)),
            "{language:?}"
        );
    }
}
