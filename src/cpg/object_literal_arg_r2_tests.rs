//! #594 post-cap fold: a computed-member write to the formal (`p[k] = v`, `p["a"] = v`)
//! may replace any key, so it is a wildcard write and no leaf of the literal feeds any read
//! of that formal. Node spelling and helpers are those of `object_literal_arg_tests`.

use super::object_literal_arg_tests::{build, check, edge_set};
use crate::languages::Language;
use std::collections::BTreeSet;

/// One statement `write` between a bare read of `p` and reads of `p.a` and `p.z`; the
/// literal feeds `a` and `z`. `k` is a free key variable; `s` is a second formal.
fn fixture(write: &str) -> String {
    format!(
        "function f(p, s) {{\n  sink(p);\n  {write}\n  sink(p.a);\n  sink(p.z);\n}}\nfunction h(x, z, s) {{\n  f({{ a: x, z }}, s);\n}}\n"
    )
}

/// Without any suppression: both keys reach the bare read and their own member read.
const ALL_EDGES: [&str; 4] = [
    "h:x@8 -> f:p@2",
    "h:x@8 -> f:p@4",
    "h:z@8 -> f:p@2",
    "h:z@8 -> f:p@5",
];

const JS_COMPUTED_WRITES: [&str; 15] = [
    "p[\"a\"] = \"clean\";",
    "p[k] = clean();",
    "p[0] = clean();",
    "p[k] += 1;",
    "p[k] ||= 1;",
    "p[k]++;",
    "--p[k];",
    "delete p[k];",
    "[p[k]] = s;",
    "({ x: p[k] } = s);",
    "({ ...p[k] } = s);",
    "for (p[k] of s) {}",
    "for (p[k] in s) {}",
    "(() => { p[k] = 1; })();",
    "(function () { delete p[k]; })();",
];

#[test]
fn a_computed_write_to_the_formal_stops_every_leaf_reaching_any_read() {
    // The issue example is the first form: `p["a"] = "clean"` must feed neither `x` into
    // `sink(p.a)` nor the unrelated key `z` into `sink(p.z)` or the bare read.
    for form in JS_COMPUTED_WRITES {
        check(form, &fixture(form), &[]);
    }
}

#[test]
fn ts_wrapped_computed_write_targets_stop_every_leaf() {
    for form in [
        "(p as any)[k] = 1;",
        "p![k] = 1;",
        "(p satisfies object)[k] = 1;",
    ] {
        let src = fixture(form);
        for (language, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
            let cpg = build(language, &[("m", &src)], ext);
            assert_eq!(edge_set(&cpg), BTreeSet::new(), "{form} [{language:?}]");
        }
    }
    let angle = fixture("(<any>p)[k] = 1;");
    let cpg = build(Language::TypeScript, &[("m", &angle)], "ts");
    assert_eq!(
        edge_set(&cpg),
        BTreeSet::new(),
        "angle-bracket write target"
    );
}

#[test]
fn a_computed_write_in_a_member_only_formal_also_stops_the_literal() {
    // No bare read of `p`: only `p.a` reads and the subscript write.
    let src = "function f(p) {\n  p[k] = 1;\n  sink(p.a);\n}\nfunction h(x) {\n  f({ a: x });\n}\n";
    check("member-only-plus-write", src, &[]);
}

#[test]
fn a_computed_write_to_another_base_or_a_shadowed_p_keeps_binding() {
    for form in [
        "s[k] = 1;",
        "(function (p) { p[k] = 1; })(s);",
        "((p) => { p[k] = 1; })(s);",
        "(function () { const p = s; p[k] = 1; })();",
    ] {
        check(form, &fixture(form), &ALL_EDGES);
    }
}

#[test]
fn a_computed_read_of_the_formal_does_not_suppress() {
    // `p[k]` is a bare read of `p` (line 3): it sees every key, and writes nothing.
    check(
        "subscript-read",
        &fixture("sink(p[k]);"),
        &[
            "h:x@8 -> f:p@2",
            "h:x@8 -> f:p@3",
            "h:x@8 -> f:p@4",
            "h:z@8 -> f:p@2",
            "h:z@8 -> f:p@3",
            "h:z@8 -> f:p@5",
        ],
    );
    // A computed key inside a pattern is a read, not a write to the formal.
    check(
        "computed-key-in-pattern",
        &fixture("({ [p[k]]: s } = s);"),
        &ALL_EDGES,
    );
}

#[test]
fn a_write_below_a_computed_key_is_not_a_write_to_the_formal() {
    // Pinned limit, as in the member-only lane: `p.a[k] = v` and `p[k].a = v` write below a
    // key and do not suppress the literal's own leaves.
    for form in ["p.a[k] = 1;", "p[k].a = 1;"] {
        let got = {
            let src = fixture(form);
            let cpg = build(Language::JavaScript, &[("m", &src)], "js");
            edge_set(&cpg)
        };
        assert!(got.contains("h:z@8 -> f:p@5"), "{form}: {got:?}");
    }
}
