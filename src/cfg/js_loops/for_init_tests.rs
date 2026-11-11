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
    ("r1_declarator_value_write", "function g(more) {\n  for (let i = (x = 0); more(); i++) {\n    var x = 1;\n  }\n  sink(x);\n}\n", "x@3->x@5", "exact"), // `let i = (x = 0)` writes `x`, it does not bind it
    ("r2_subscript_suffixes", "function f(o, k, more) {\n  for (o[k].a = 0; more(); ) {\n    o[k].b = 1;\n  }\n  sink(o[k].b);\n}\n", "o[]@3->o[]@5", "exact"), // RD records `o[k].a` and `o[k].b` both as `o.[]`
    ("r3_initializer_delete", "function f(o, more) {\n  for (delete o.x; more(); ) {\n    o.x = 1;\n  }\n  sink(o.x);\n}\n", "o.x@3->o.x@5", "exact"), // `delete o.x` is a write
    ("c13_subscript_update", "function f(o, k, more) {\n  let z = 1;\n  for (o[k].a = 0; more(); o[k].a++) {\n    o[k].b = 1;\n  }\n  sink(o[k].b, z);\n}\n", "o[]@4->o[]@6", "exact"), // a `[]` update is no exact-path repeated write
    ("s2_call_chain_init_and_body", "function f(a, k, more) {\n  for (this.get(a[k]).x = 0; more(); ) {\n    this.get(a[k]).y = 1;\n  }\n  sink(this.get(a[k]).y);\n}\n", "this.get(a[]@3->this.get(a[]@5", "exact"), // calls in the chain: no static path
    ("s3_delete_comment", "function f(o, more) {\n  for (delete (/* c */ o.x); more(); ) {\n    o.x = 1;\n  }\n  sink(o.x);\n}\n", "o.x@3->o.x@5", "exact"), // `delete (/* c */ o.x)`: comments are skipped
    ("t1_pattern_leaf", "function f(a, b, more) {\n  for (({p: this.x, a} = {p: 0, a: 0}); more(); ) {\n    ({p: this.x, b} = {p: 1, b: 1});\n  }\n  sink(this.x);\n}\n", "this.x@3->this.x@5", "exact"), // `({p: this.x, a} = ..)`: the leaf `this.x` is its own target
    ("t2_comment_before_bracket", "function f(o, more) {\n  for (o.a /*[*/ .x = 0; more(); ) {\n    o.a /*[*/ .y = 1;\n  }\n  sink(o.a /*[*/ .y);\n}\n", "o.a /*[]@3->o.a /*[]@5", "exact"), // RD records both writes as one garbled path: they affect each other
    ("c9_assignment_as_receiver", "function f(r, n) {\n  let z = 1;\n  let m = 0;\n  for (let i = 0; i < n; i++) {\n    (r ??= [])[i] = g(i);\n    m = i;\n  }\n  sink(m, z);\n}\n", "m@6->m@8", "exact"), // RD records no path for `(r ??= [])[i]`: unknown, fail closed
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
    ("c11_inner_let_block", "function f(n) {\n  let z = 1;\n  let s = 0;\n  for (let i = 0; i < n; i++) {\n    { let i = 5; s = i; }\n  }\n  sink(s, z);\n}\n", "s@5->s@7", true, "exact"), // an inner block `let i` is another binding
    ("c12_repeated_delete", "function f(o, more) {\n  let z = 1;\n  for (delete o.x; more(); delete o.x) {\n    o.x = 1;\n  }\n  sink(o.x, z);\n}\n", "o.x@4->o.x@6", true, "nameonly/killed"), // `delete o.x` in the update kills the body write
    ("c14_field_in_body", "function f(o, n) {\n  let z = 1;\n  for (let i = 0; i < n; i++) {\n    o.f = i;\n  }\n  sink(o.f, z);\n}\n", "o.f@4->o.f@6", true, "exact"), // a representable body field write is still admitted
    ("c10_call_chain_body_write", "function f(w, arr, n) {\n  let z = 1;\n  for (let i = 0; i < n; i++) {\n    w.get(arr[i]).count--;\n  }\n  sink(w.count, z);\n}\n", "z@2->z@6", true, "exact"), // RD records a path disjoint from the initializer's: admitted again
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

/// TypeScript wrappers, peeled as the write census peels them: an `as` receiver in the initializer is a hazard
/// source (it was `killed` before the fold, main's label is `exact`), and a deleted operand wrapped in `as`,
/// `satisfies` or `<T>` is still a write whose replay is a hazard only when no repeated write kills it.
#[rustfmt::skip]
const TS_ROWS: &[(&str, &str, &str, &str)] = &[
    ("s1_as_receiver_init", "function f(more) {\n  for ((this as any).x = 0; more(); ) {\n    this.x = 1;\n  }\n  sink(this.x);\n}\n", "this.x@3->this.x@5", "exact"),
    ("s4_delete_as", "function f(o: any, more: () => boolean) {\n  for (delete (o as any).x; more(); ) {\n    o.x = 1;\n  }\n  sink(o.x);\n}\n", "o.x@3->o.x@5", "exact"),
    ("s5_delete_satisfies", "function f(o: any, more: () => boolean) {\n  for (delete (o satisfies any).x; more(); ) {\n    o.x = 1;\n  }\n  sink(o.x);\n}\n", "o.x@3->o.x@5", "exact"),
    ("s6_delete_type_assertion", "function f(o: any, more: () => boolean) {\n  for (delete (<any>o).x; more(); ) {\n    o.x = 1;\n  }\n  sink(o.x);\n}\n", "o.x@3->o.x@5", "exact"),
];

#[test]
fn a_wrapped_initializer_write_is_a_hazard_and_keeps_mains_label() {
    const TS: Language = Language::TypeScript;
    for (name, src, edge, want) in TS_ROWS {
        let parsed = ParsedFile::parse("a.ts", src, TS).unwrap();
        let verdicts: Vec<_> = parsed
            .js_loop_plan()
            .verdicts
            .iter()
            .map(|v| v.why.clone())
            .collect();
        assert_eq!(
            verdicts,
            [Some(Why::Legacy(super::Legacy::ForInit))],
            "{name}"
        );
        assert_eq!(labels_of(TS, src, edge), [*want], "{name}");
        let main = crate::cfg::js_loops::with_main_loop_cfg(|| labels_of(TS, src, edge));
        assert_eq!(labels_of(TS, src, edge), main, "{name}");
    }
}
