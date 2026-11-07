//! #660 S1c: the matrix rows that name S1c (captured-write refusal, D2-c) as label rows, plus the
//! binding-shape matrix of the slice. A write in a nested callable or an instance field initializer
//! (a region) can run between an outer def and an outer read; every outer edge of the written
//! binding whose use lies outside the region is `NameOnly(CfgIncomplete)`. Label-only: no edge moves.
//! FIX rows are `exact` on base 61cd41f5 and move to `nameonly/cfg_incomplete`; controls hold on both.
use super::census_label_tests::labels_of;
use crate::js_write_census::matrix_tests::lang_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../tests/fixtures/js_write_census/variant-matrix.tsv");

const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";
const KILLED: &str = "nameonly/killed";
const JS: Language = Language::JavaScript;

struct Row {
    fixture: String,
    edge: String,
    expected: String,
    source: String,
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .skip(1)
        .filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 8, "matrix row: {line}");
            c[5].contains("S1c").then(|| Row {
                fixture: c[0].to_string(),
                edge: c[1].to_string(),
                expected: c[4].to_string(),
                source: c[7].replace("\\n", "\n"),
            })
        })
        .collect()
}

#[test]
fn matrix_rows_move_to_the_expected_label() {
    let all = rows();
    assert_eq!(all.len(), 24, "S1c matrix rows");
    let mut failures = Vec::new();
    for r in &all {
        let got = labels_of(lang_of(&r.fixture), &r.source, &r.edge);
        if got.is_empty() || got.iter().any(|g| *g != r.expected) {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                r.fixture, r.edge, r.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The edge set never moves: each row's edge exists with some label on head.
#[test]
fn matrix_rows_keep_their_edges() {
    for r in rows() {
        assert!(
            !labels_of(lang_of(&r.fixture), &r.source, &r.edge).is_empty(),
            "{} {}: edge disappeared",
            r.fixture,
            r.edge
        );
    }
}

struct Case {
    name: &'static str,
    lang: Language,
    edge: &'static str,
    source: &'static str,
    expected: &'static str,
}

/// Cases read `a@1 -> sink(a)`. A plain assignment in a region is a recorded def (main already
/// kills the outer def by name, `nameonly/killed`), so the callable shapes write with `a++`, which the
/// producer never records: main keeps `exact` and D2-c is the only mechanism that can move it.
#[rustfmt::skip]
const CASES: &[Case] = &[
    // Callable shapes: every nested callable is a region.
    Case { name: "method_in_object_literal", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  const o = { m() { a++; } };\n  sink(a);\n}" },
    Case { name: "getter", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  const o = { get v() { a++; return 1; } };\n  sink(a);\n}" },
    Case { name: "setter", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  const o = { set v(z) { a++; } };\n  sink(a);\n}" },
    Case { name: "async_arrow", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(async () => { a++; });\n  sink(a);\n}" },
    Case { name: "generator_expression", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(function* () { a++; });\n  sink(a);\n}" },
    Case { name: "function_expression", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(function () { a++; });\n  sink(a);\n}" },
    Case { name: "class_constructor", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  class C { constructor() { a++; } }\n  sink(a);\n}" },
    Case { name: "class_static_method", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  class C { static m() { a++; } }\n  sink(a);\n}" },
    Case { name: "field_arrow", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  const C = class { x = () => { a++; }; };\n  sink(a);\n}" },
    Case { name: "write_to_an_unrelated_base_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { b++; });\n  sink(a);\n}" },
    // Two regions deep.
    Case { name: "two_regions_deep", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { h(() => { a++; }); });\n  sink(a);\n}" },
    // Write forms the producer records: the nested-sourced edge into the outer use.
    Case { name: "augmented_write_sourced", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { a += 1; });\n  sink(a);\n}" },
    Case { name: "array_pattern_write_sourced", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { [a] = [1]; });\n  sink(a);\n}" },
    Case { name: "for_of_head_write_sourced", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f(a, xs) {\n  log();\n  g(() => { for (a of xs) {} });\n  sink(a);\n}" },
    // Member paths: `affects` decides which uses a write reaches.
    Case { name: "member_update_reaches_member_use", lang: JS, edge: "o.x@2->o.x@4", expected: CFG,
        source: "function f(o) {\n  o.x = 1;\n  g(() => { o.x++; });\n  sink(o.x);\n}" },
    Case { name: "sibling_member_update_control", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  g(() => { o.y++; });\n  sink(o.x);\n}" },
    Case { name: "whole_object_write_reaches_member_use", lang: JS, edge: "o.x@2->o.x@4", expected: CFG,
        source: "function f(o) {\n  o.x = 1;\n  g(() => { o = {}; });\n  sink(o.x);\n}" },
    Case { name: "delete_reaches_member_use", lang: JS, edge: "o.x@2->o.x@4", expected: CFG,
        source: "function f(o) {\n  o.x = 1;\n  g(() => { delete o.x; });\n  sink(o.x);\n}" },
    Case { name: "closure_before_member_def", lang: JS, edge: "o.x@4->o.x@6", expected: CFG,
        source: "function f() {\n  const set = () => { o.x++; };\n  let o = {};\n  o.x = 1;\n  set();\n  sink(o.x);\n}" },
    // `this`: an arrow keeps the owner's, a function has its own (not a write of the owner's `this`).
    Case { name: "this_in_arrow", lang: JS, edge: "this.x@2->this.x@4", expected: CFG,
        source: "function f() {\n  this.x = 1;\n  g(() => { this.x++; });\n  sink(this.x);\n}" },
    Case { name: "this_in_function_control", lang: JS, edge: "this.x@2->this.x@4", expected: EXACT,
        source: "function f() {\n  this.x = 1;\n  g(function () { this.x++; });\n  sink(this.x);\n}" },
    // Which binding the write targets (B). A block `let` is a binding of its own.
    Case { name: "block_let_same_binding", lang: JS, edge: "a@3->a@5", expected: CFG,
        source: "function f() {\n  {\n    let a = 1;\n    g(() => { a++; });\n    sink(a);\n  }\n}" },
    Case { name: "write_targets_a_different_binding_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  { let a = 1; g(() => { a++; }); }\n  sink(a);\n}" },
    Case { name: "middle_region_let_inner_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { let a; h(() => { a++; }); });\n  sink(a);\n}" },
    // (`var a;` is a recorded nested def: main kills the outer def by name, and D2-c keeps a killed label.)
    Case { name: "middle_region_var_inner_pin", lang: JS, edge: "a@1->a@4", expected: KILLED,
        source: "function f(a) {\n  log();\n  g(() => { var a; h(() => { a++; }); });\n  sink(a);\n}" },
    Case { name: "middle_region_parameter_inner_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g((a) => { h(() => { a++; }); });\n  sink(a);\n}" },
    // Inner bindings that scope the write (controls: no refusal).
    Case { name: "inner_let_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { let a = 1; a++; });\n  sink(a);\n}" },
    Case { name: "body_function_declaration_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { a++; function a() {} });\n  sink(a);\n}" },
    Case { name: "for_head_let_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { for (let a = 0; a < 3; a++) {} });\n  sink(a);\n}" },
    Case { name: "for_of_let_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a, xs) {\n  log();\n  g(() => { for (let a of xs) { a++; } });\n  sink(a);\n}" },
    Case { name: "switch_let_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a, k) {\n  log();\n  g(() => { switch (k) { case 1: let a = 1; a++; } });\n  sink(a);\n}" },
    Case { name: "catch_parameter_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { try { h(); } catch (a) { a++; } });\n  sink(a);\n}" },
    Case { name: "named_function_expression_body_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(function a() { a++; });\n  sink(a);\n}" },
    Case { name: "block_function_scopes_its_block_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { { function a() {} a++; } });\n  sink(a);\n}" },
    Case { name: "block_class_scopes_its_block_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(() => { { class a {} a++; } });\n  sink(a);\n}" },
    Case { name: "field_arrow_parameter_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  const C = class { x = (a) => { a++; }; };\n  sink(a);\n}" },
    // Declarations that do not scope the write (outer: refuse).
    Case { name: "sibling_block_let_outer", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { { let a = 0; } a++; });\n  sink(a);\n}" },
    Case { name: "named_function_expression_param_default", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  (function a(z = (a++)) {})();\n  sink(a);\n}" },
    Case { name: "own_parameter_list_scopes_default_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  (function (a, z = (a++)) {})();\n  sink(a);\n}" },
    // A write in a nested callable's parameter list, inside another callable's body that declares it.
    Case { name: "param_default_under_enclosing_body_var_pin", lang: JS, edge: "a@1->a@4", expected: KILLED,
        source: "function f(a) {\n  log();\n  (function () { var a; (function (z = (a++)) {})(); })();\n  sink(a);\n}" },
    // The use shares the line with the closure: main's line-granular capture already refuses it.
    Case { name: "use_on_the_closure_line", lang: JS, edge: "a@1->a@3", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { a++; }), sink(a);\n}" },
    // A use inside the writing region is the capture rule's, and stays so.
    Case { name: "use_inside_the_region", lang: JS, edge: "a@2->a@3", expected: CFG,
        source: "function f(a) {\n  a = 1;\n  g(() => { sink(a); a++; });\n}" },
    // A killed edge keeps its label (D2-c refuses only `exact` and `sameline`).
    Case { name: "killed_edge_keeps_its_label", lang: JS, edge: "a@1->a@5", expected: KILLED,
        source: "function f(a) {\n  log();\n  g(() => { a++; });\n  a = 3;\n  sink(a);\n}" },
    // TSX / TS.
    Case { name: "tsx_class_property_handler", lang: Language::Tsx, edge: "a@1->a@4", expected: CFG,
        source: "function f(a: number) {\n  log();\n  class C { h = () => { a++; }; }\n  return sink(a);\n}" },
    Case { name: "ts_parameter_property_inner_control", lang: Language::TypeScript, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a: number) {\n  log();\n  class C { constructor(public a = 1) { a++; } }\n  sink(a);\n}" },
];

#[test]
fn binding_shape_rows() {
    let mut failures = Vec::new();
    for c in CASES {
        let got = labels_of(c.lang, c.source, c.edge);
        if got.is_empty() || got.iter().any(|g| g != c.expected) {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                c.name, c.edge, c.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Every binding-shape source and matrix row source, for the neutrality tests.
pub(super) fn sources() -> Vec<(Language, String)> {
    let mut out: Vec<(Language, String)> = CASES
        .iter()
        .map(|c| (c.lang, c.source.to_string()))
        .collect();
    out.extend(rows().into_iter().map(|r| (lang_of(&r.fixture), r.source)));
    out
}
