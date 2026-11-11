//! #693/#697 slice 4 (F3): the path-level initializer replay criterion (`for_init.rs`). Labels are hand-derived
//! from the loop semantics; the RED column of each refusal case is the head before the slice (afa0d8df), where a
//! root-name exemption admitted the tree and the body's `o.x` was wrongly `killed` before the sink.
use super::Why;
use crate::ast::ParsedFile;
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const JS: Language = Language::JavaScript;

fn why(src: &str) -> Vec<Option<Why>> {
    let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    parsed
        .js_loop_plan()
        .verdicts
        .iter()
        .map(|v| v.why.clone())
        .collect()
}

fn main_labels(src: &str, edge: &str) -> Vec<String> {
    crate::cfg::js_loops::with_main_loop_cfg(|| labels_of(JS, src, edge))
}

/// Refused (`ForInit`): the owner keeps main's label, and the truth (body write reaches the sink) is `want`.
#[rustfmt::skip]
const HAZARDS: &[(&str, &str, &str, &str)] = &[
    ("p1_sibling", "function f(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); o.y++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", "exact"), // sibling `o.y++` does not excuse replaying `o.x = 0`
    ("p2_deferred", "function f(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); () => o.x++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", "nameonly/cfg_incomplete"), // an arrow update is never run: no repeated write
    ("p3_conditional", "function f(o, more, flag) {\n  let z = 1;\n  for (o.x = 0; more(); flag && o.x++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", "exact"), // `flag && o.x++` is conditional
    ("p4_ternary", "function f(o, more, flag) {\n  let z = 1;\n  for (o.x = 0; more(); flag ? o.x++ : noop()) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", "exact"), // `flag ? o.x++ : noop()` is conditional
    ("p5_prefix", "function f(o, more) {\n  let z = 1;\n  for (o = {}; more(); o.y++) {\n    o = g();\n  }\n  sink(o, z);\n}\n", "o@4->o@6", "exact"), // `o.y++` does not affect the body's `o = g()`
];

#[test]
fn a_replayed_initializer_write_no_exempt_repeated_write_excuses_refuses_the_tree() {
    for (name, src, edge, want) in HAZARDS {
        assert_eq!(
            why(src),
            [Some(Why::Legacy(super::Legacy::ForInit))],
            "{name}"
        );
        assert_eq!(labels_of(JS, src, edge), [*want], "{name}");
        assert_eq!(labels_of(JS, src, edge), main_labels(src, edge), "{name}");
    }
}

/// Admitted: an exact-path eager test or update write (or a comma sequence of them) kills the body definition
/// before the sink, so `killed` is the truth; the read-only loop has no body write; the shadowed root is not
/// provably one binding, so the tree is refused and keeps main's label.
#[rustfmt::skip]
const CONTROLS: &[(&str, &str, &str, bool, &str)] = &[
    ("c1_scalar", "function f(x, n) {\n  let z = 1;\n  for (x = 0; x < n; x++) {\n    x = g(x);\n  }\n  sink(x, z);\n}\n", "x@4->x@6", true, "nameonly/killed"),
    ("c2_samepath", "function f(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); o.x++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", true, "nameonly/killed"),
    ("c5_comma", "function f(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); o.y++, o.x++) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", true, "nameonly/killed"),
    ("c7_testwrite", "function f(o) {\n  let z = 1;\n  for (o.x = 0; (o.x = next()); ) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", true, "nameonly/killed"),
    ("c3_readonly", "function f(n) {\n  let z = 1;\n  for (let i = 0; i < n; i++) {\n    use(i);\n  }\n  sink(z);\n}\n", "i@3->i@4", true, "nameonly/sameline"),
    ("c8_inner_decl", "function f(items, n) {\n  let z = 1;\n  let s = 0;\n  for (let i = 0; i < n; i++) {\n    for (const i of items) {\n      s = i;\n    }\n  }\n  sink(s, z);\n}\n", "s@6->s@9", true, "exact"), // an inner `for (const i ..)` is another binding
    ("c9_index_read", "function f(r, n) {\n  let z = 1;\n  let m = 0;\n  for (let i = 0; i < n; i++) {\n    (r ??= [])[i] = g(i);\n    m = i;\n  }\n  sink(m, z);\n}\n", "m@6->m@8", true, "exact"), // `(r ??= [])[i] = ..` reads `i`, writes an element
    ("c4_shadow", "function f(o, more) {\n  let z = 1;\n  for (o.x = 0; more(); o.y++) {\n    { let o = {}; o.x = 1; }\n  }\n  sink(o.x, z);\n}\n", "o.x@3->o.x@6", false, "exact"),
];

#[test]
fn an_exempt_repeated_write_keeps_the_tree_and_a_shadowed_root_is_not_exempted() {
    for (name, src, edge, admitted, want) in CONTROLS {
        let verdicts = why(src);
        assert_eq!(verdicts.len(), 1, "{name}");
        assert_eq!(verdicts[0].is_none(), *admitted, "{name}: {verdicts:?}");
        assert_eq!(labels_of(JS, src, edge), [*want], "{name}");
        if !admitted {
            assert_eq!(labels_of(JS, src, edge), main_labels(src, edge), "{name}");
        }
    }
}
