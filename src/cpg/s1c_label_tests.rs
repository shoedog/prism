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
    // Pin of MAIN's name-kill, not a D2-c decision: `var a;` is a recorded nested def, main kills the
    // outer def by name, and D2-c keeps a killed label. (The var rule itself is `captured_tests`'.)
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
    // Pin of MAIN's name-kill (the recorded `var a`), not a D2-c decision.
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
    // Round 1 (Opus W1): a closure names a binding declared later in its scope (TDZ is no scoping): the
    // written binding is that later declaration, not the outer one a visibility-based lookup finds.
    Case { name: "tdz_let_after_closure_written_binding", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function f2(a) {\n  {\n    const k = () => { a++; };\n    let a = 1;\n    k();\n    sink(a);\n  }\n  sink2(a);\n}" },
    Case { name: "tdz_let_after_closure_outer_edge_untouched", lang: JS, edge: "a@1->a@8", expected: EXACT,
        source: "function f2(a) {\n  {\n    const k = () => { a++; };\n    let a = 1;\n    k();\n    sink(a);\n  }\n  sink2(a);\n}" },
    Case { name: "hoisted_block_function_written_binding", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function f2(a) {\n  {\n    function k() { a++; }\n    let a = 1;\n    k();\n    sink(a);\n  }\n  sink2(a);\n}" },
    Case { name: "hoisted_block_function_outer_edge_untouched", lang: JS, edge: "a@1->a@8", expected: EXACT,
        source: "function f2(a) {\n  {\n    function k() { a++; }\n    let a = 1;\n    k();\n    sink(a);\n  }\n  sink2(a);\n}" },
    Case { name: "class_method_before_let_written_binding", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function f2(a) {\n  {\n    class K { m() { a++; } }\n    let a = 1;\n    new K().m();\n    sink(a);\n  }\n  sink2(a);\n}" },
    Case { name: "class_method_before_let_outer_edge_untouched", lang: JS, edge: "a@1->a@8", expected: EXACT,
        source: "function f2(a) {\n  {\n    class K { m() { a++; } }\n    let a = 1;\n    new K().m();\n    sink(a);\n  }\n  sink2(a);\n}" },
    // Round 1 (sol W1): a parameter's scope excludes its own decorators and the callables nested in them.
    Case { name: "ts_parameter_decorator_callback", lang: Language::TypeScript, edge: "a@1->a@6", expected: CFG,
        source: "function f(a) {\n  log();\n  const C = class {\n    constructor(@dec(() => { a++; }) a) {}\n  };\n  sink(a);\n}" },
    Case { name: "tsx_parameter_decorator_callback", lang: Language::Tsx, edge: "a@1->a@6", expected: CFG,
        source: "function f(a) {\n  log();\n  const C = class {\n    constructor(@dec(() => { a++; }) a) {}\n  };\n  sink(a);\n}" },
    Case { name: "ts_parameter_decorator_direct_update_in_deferred", lang: Language::TypeScript, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { class C { constructor(@dec(a++) a) {} } });\n  sink(a);\n}" },
    Case { name: "ts_parameter_default_control", lang: Language::TypeScript, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  class C { constructor(a, public x = (a++)) {} }\n  sink(a);\n}" },
    // Round 1 (sol W2): the innermost declaration is the written binding; an outer one is not a union member.
    Case { name: "nested_blocks_inner_edge_refused", lang: JS, edge: "a@6->a@8", expected: CFG,
        source: "function f() {\n  log();\n  g(() => {\n    let a = 1;\n    {\n      let a = 2;\n      h(() => { a++; });\n      sink(a);\n    }\n  });\n}" },
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

/// sol W2: the outer `let a` edge is not the written binding's. It keeps the labels the owners give it
/// (`g` itself sees `exact`, `f` sees `sameline` across the one callable statement), never `cfg_incomplete`.
#[test]
fn nested_blocks_outer_edge_is_not_refused() {
    let src = "function f() {\n  log();\n  g(() => {\n    let a = 1;\n    {\n      let a = 2;\n      h(() => { a++; });\n    }\n    sink(a);\n  });\n}";
    let got = labels_of(JS, src, "a@4->a@9");
    assert!(!got.is_empty() && !got.iter().any(|g| g == CFG), "{got:?}");
}

/// Cases whose frozen main rows D2-c (#660 S1c) moves: a write in a nested callable (a closure, a
/// parameter default arrow) refuses every `exact` and `sameline` edge into a use outside it. Every
/// such row of these cases moves to `NameOnly(CfgIncomplete)`; nothing else changes.
const S1C_MOVED: &[&str] = &[
    "C1-closure",
    "E-deferred-default-write-named-sloppy",
    "E-invoked-default-write-named-sloppy",
];

fn s1c_refused(row: &serde_json::Value) -> serde_json::Value {
    let mut row = row.as_array().unwrap().clone();
    let moves = row[6] == "exact" || row[7] == "sameline";
    if moves {
        row[6] = "nameonly".into();
        row[7] = "cfg_incomplete".into();
    }
    row.into()
}

/// The frozen main rows of case `id`, with D2-c's moves applied when the case is one it moves.
pub(super) fn refuse_moved(id: &str, rows: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    if S1C_MOVED.contains(&id) {
        rows.iter().map(s1c_refused).collect()
    } else {
        rows
    }
}

/// Round 2: a row is judged by whether any owner refuses it (`g` and `f` both label an edge inside a
/// callable body, and the DataFlow row keeps both).
fn refused_by_some_owner(lang: Language, src: &str, edge: &str) -> bool {
    let got = labels_of(lang, src, edge);
    assert!(!got.is_empty(), "{edge} is missing in {src}");
    got.iter().any(|g| g == CFG)
}

#[rustfmt::skip]
const ROUND2: &[(&str, &str, &str, bool)] = &[
    // sol W1: a switch-local `let` has the switch's scope, not the function's.
    ("switch_local_let_after_closure", "a@1->a@4", "function f(a) {\n  log();\n  g(() => { a++; });\n  sink(a);\n  switch (tag) {\n    case 0:\n      let a = 2;\n  }\n}", true),
    ("switch_local_let_nested", "a@4->a@6", "function f() {\n  log();\n  g(() => {\n    let a = 1;\n    h(() => { a++; });\n    sink(a);\n    switch (tag) {\n      case 0:\n        let a = 2;\n    }\n  });\n}", true),
    ("switch_local_write_refuses_its_own_edge", "a@5->a@8", "function f(a) {\n  log();\n  switch (tag) {\n    case 0:\n      let a = 2;\n      g(() => { a++; });\n      log();\n      sink(a);\n  }\n  sink2(a);\n}", true),
    ("switch_local_write_leaves_the_outer_edge", "a@1->a@10", "function f(a) {\n  log();\n  switch (tag) {\n    case 0:\n      let a = 2;\n      g(() => { a++; });\n      log();\n      sink(a);\n  }\n  sink2(a);\n}", false),
    // sol W2 / Opus W1: a def is the written declaration's by identity, not by lying in its scope.
    ("outer_write_inner_block_edge_untouched", "a@6->a@7", "function f() {\n  log();\n  g(() => {\n    let a = 1;\n    {\n      let a = 2;\n      sink(a);\n    }\n    h(() => { a++; });\n  });\n}", false),
    ("outer_write_inner_block_after_closure_untouched", "a@7->a@9", "function f() {\n  log();\n  g(() => {\n    let a = 1;\n    h(() => { a++; });\n    {\n      let a = 2;\n      log();\n      sink(a);\n    }\n  });\n}", false),
    ("outer_write_inner_block_outer_read_refused", "a@4->a@10", "function f() {\n  log();\n  g(() => {\n    let a = 1;\n    {\n      let a = 2;\n      log();\n    }\n    h(() => { a++; });\n    sink(a);\n  });\n}", true),
    // Opus r3: a `var` in a nested callable has the owner's whole scope in `BindingFacts`, so it ties an
    // owner-level `let`/`const`; the later owner-level declaration is the written binding.
    ("tie1_nested_var_then_let", "a@3->a@6", "function f() {\n  g(() => { var a = 0; });\n  let a = 1;\n  h(() => { a++; });\n  log();\n  sink(a);\n}", true),
    ("tie2_nested_var_then_const_member", "a.n@4->a.n@6", "function f() {\n  g(function () { var a = 0; });\n  const a = { n: 1 };\n  h(() => { a.n = 2; });\n  log();\n  sink(a.n);\n}", true),
    ("tie1ctl_var_after_the_let_merges", "a@2->a@6", "function f() {\n  let a = 1;\n  g(() => { var a = 0; });\n  h(() => { a++; });\n  log();\n  sink(a);\n}", false),
    ("outer_param_inner_block_untouched", "a@6->a@8", "function f(a) {\n  log();\n  g((a) => {\n    h(() => { a++; });\n    {\n      let a = 2;\n      log();\n      sink(a);\n    }\n  });\n}", false),
];

#[test]
fn round_two_rows() {
    let mut failures = Vec::new();
    for (name, edge, src, refused) in ROUND2 {
        if refused_by_some_owner(JS, src, edge) != *refused {
            failures.push(format!("{name} {edge}: want refused={refused}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
