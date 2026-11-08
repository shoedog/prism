//! #660 S1d-1: the matrix rows that name S1d as label rows, plus the per-class enumeration of the
//! refusal classes (DESIGN.md 6.1, 6.1b-6.1e, G6a, G6b). Some writes are not spelled as an assignment
//! target (sloppy mapped `arguments`, direct `eval`, `with`, a hoisted function declaration that replaces
//! a formal at entry, Annex B block functions); a wrong `exact` is worse than an honest `NameOnly`.
//! Label-only: no edge moves. FIX rows are `exact` on base 8197b4f1 and move to the expected label;
//! controls hold on both. The one-node rows (`S1d (guard deleted)`) are S1d-2's.
use super::census_label_tests::labels_of;
use crate::js_write_census::matrix_tests::lang_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../tests/fixtures/js_write_census/variant-matrix.tsv");

const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";
const KILLED: &str = "nameonly/killed";
/// Any label but `cfg_incomplete` (a nested edge has several owners, and main labels one of them `sameline`).
const NOT_CFG: &str = "!cfg";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

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
            (c[5] == "S1d").then(|| Row {
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
    assert_eq!(all.len(), 24, "S1d matrix rows");
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

/// Per-class enumeration. Each case reads one edge `x@def -> x@use`; the expected label comes first.
#[rustfmt::skip]
const CASES: &[Case] = &[
    // 6.1 sloppy mapped `arguments`: name refusal for the owner's formals.
    Case { name: "args_nested_arrow", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { arguments[0] = src(); });\n  sink(a);\n}" },
    Case { name: "args_nested_function_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(function () { arguments[0] = src(); });\n  sink(a);\n}" },
    Case { name: "args_read_only_is_refused_too", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  use(arguments.length);\n  sink(a);\n}" },
    Case { name: "args_in_a_default_unmapped_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a, b = arguments[0]) {\n  log();\n  sink(a);\n}" },
    Case { name: "args_unrelated_local_control", lang: JS, edge: "n@2->n@5", expected: EXACT,
        source: "function f(a) {\n  var n = src();\n  log();\n  arguments[0] = 1;\n  sink(n);\n}" },
    Case { name: "args_member_of_a_formal", lang: JS, edge: "o.x@2->o.x@5", expected: CFG,
        source: "function f(o) {\n  o.x = 1;\n  log();\n  arguments[0] = {};\n  sink(o.x);\n}" },
    Case { name: "args_second_formal", lang: JS, edge: "b@1->b@4", expected: CFG,
        source: "function f(a, b) {\n  log();\n  arguments[0] = src();\n  sink(b);\n}" },
    Case { name: "args_strict_function_prologue_control", lang: JS, edge: "a@1->a@5", expected: EXACT,
        source: "function f(a) {\n  'use strict';\n  log();\n  arguments[0] = src();\n  sink(a);\n}" },
    Case { name: "args_arrow_owner_has_no_own_arguments_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "const f = (a) => {\n  log();\n  sink(a, arguments);\n};" },
    Case { name: "args_ts", lang: TS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a: number) {\n  log();\n  arguments[0] = src();\n  sink(a);\n}" },
    Case { name: "args_tsx", lang: TSX, edge: "a@1->a@4", expected: CFG,
        source: "function f(a: number) {\n  log();\n  arguments[0] = src();\n  sink(a);\n}" },
    // 6.1b direct eval: owner refusal, nested callables included.
    Case { name: "eval_in_nested_callable", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a) {\n  log();\n  g(() => { eval('a = 1'); });\n  sink(a);\n}" },
    Case { name: "eval_in_a_default", lang: JS, edge: "a@1->a@3", expected: CFG,
        source: "function f(a, b = eval('a = 1')) {\n  log();\n  sink(a);\n}" },
    Case { name: "eval_refuses_every_name_of_the_owner", lang: JS, edge: "n@2->n@5", expected: CFG,
        source: "function f() {\n  var n = src();\n  log();\n  eval('1');\n  sink(n);\n}" },
    Case { name: "eval_member_callee_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  o.eval('a = 1');\n  sink(a);\n}" },
    Case { name: "eval_optional_call_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  eval?.('a = 1');\n  sink(a);\n}" },
    Case { name: "eval_ts_assertion_callee", lang: TS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a: number) {\n  log();\n  (eval as any)('a = 1');\n  sink(a);\n}" },
    Case { name: "eval_tsx", lang: TSX, edge: "a@1->a@4", expected: CFG,
        source: "function f(a: number) {\n  log();\n  eval('a = 1');\n  sink(a);\n}" },
    // 6.1c `with`: uses in the body are refused; a write there is a may-write.
    Case { name: "with_nested_callable_use", lang: JS, edge: "a@1->a@3", expected: CFG,
        source: "function f(a, o) {\n  log();\n  with (o) { g(() => sink(a)); }\n}" },
    Case { name: "with_head_use_on_the_body_line_is_refused_pin", lang: JS, edge: "a@1->a@3", expected: CFG,
        source: "function f(a, o) {\n  log();\n  with (a) { log(); }\n}" },
    Case { name: "with_use_after_the_body_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a, o) {\n  log();\n  with (o) { log(); }\n  sink(a);\n}" },
    Case { name: "with_body_write_kills_the_formal_pin", lang: JS, edge: "a@1->a@6", expected: KILLED,
        source: "function f(a, o) {\n  log();\n  with (o) {\n    a = src();\n  }\n  sink(a);\n}" },
    Case { name: "with_body_write_is_a_may_write_for_its_own_edge", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function f(a, o) {\n  log();\n  with (o) {\n    a = src();\n  }\n  sink(a);\n}" },
    Case { name: "with_multi_line_body_use", lang: JS, edge: "a@1->a@4", expected: CFG,
        source: "function f(a, o) {\n  log();\n  with (o) {\n    sink(a);\n  }\n}" },
    Case { name: "with_member_use", lang: JS, edge: "o.x@2->o.x@4", expected: CFG,
        source: "function f(o, p) {\n  o.x = 1;\n  with (p) {\n    sink(o.x);\n  }\n}" },
    Case { name: "with_ts", lang: TS, edge: "a@1->a@3", expected: CFG,
        source: "function f(a: number, o: any) {\n  log();\n  with (o) { sink(a); }\n}" },
    // 6.1d dead formals: a hoisted declaration (labelled, any depth) replaces the formal at entry.
    Case { name: "dead_two_labels_deep", lang: JS, edge: "a@1->a@3", expected: KILLED,
        source: "function f(a) {\n  log();\n  sink(a);\n  l: m: function a() {}\n}" },
    Case { name: "dead_hoisted_before_the_use", lang: JS, edge: "a@1->a@4", expected: KILLED,
        source: "function f(a) {\n  function a() {}\n  log();\n  sink(a);\n}" },
    Case { name: "dead_other_formal_control", lang: JS, edge: "b@1->b@3", expected: EXACT,
        source: "function f(a, b) {\n  log();\n  sink(b);\n  function a() {}\n}" },
    Case { name: "dead_nested_callable_declaration_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  sink(a);\n  g(function () { function a() {} });\n}" },
    Case { name: "dead_strict_control_is_still_dead", lang: JS, edge: "a@2->a@4", expected: KILLED,
        source: "'use strict';\nfunction f(a) {\n  log();\n  sink(a);\n  function a() {}\n}" },
    Case { name: "dead_strict_default_write", lang: JS, edge: "a@2->a@4", expected: KILLED,
        source: "'use strict';\nfunction f(a, b = a++) {\n  log();\n  sink(a);\n}" },
    Case { name: "dead_default_assignment_write", lang: JS, edge: "a@1->a@6", expected: KILLED,
        source: "function f(\n  a,\n  b = (a = 5)\n) {\n  log();\n  sink(a);\n}" },
    Case { name: "dead_default_member_write_control", lang: JS, edge: "a@1->a@6", expected: EXACT,
        source: "function f(\n  a,\n  b = (a.x = 5)\n) {\n  log();\n  sink(a);\n}" },
    Case { name: "dead_ts", lang: TS, edge: "a@1->a@3", expected: KILLED,
        source: "function f(a: number) {\n  log();\n  sink(a);\n  function a() {}\n}" },
    Case { name: "dead_tsx_generator", lang: TSX, edge: "a@1->a@3", expected: KILLED,
        source: "function f(a: number) {\n  log();\n  sink(a);\n  function* a() {}\n}" },
    // 6.1e Annex B (sloppy only): block-level function declarations named like an owner def.
    Case { name: "annexb_switch_case", lang: JS, edge: "a@2->a@7", expected: CFG,
        source: "function f(k) {\n  var a = src();\n  switch (k) {\n    case 1:\n      function a() {}\n  }\n  sink(a);\n}" },
    Case { name: "annexb_unbraced_else", lang: JS, edge: "a@2->a@4", expected: CFG,
        source: "function f(c) {\n  var a = src();\n  if (c) log(); else function a() {}\n  sink(a);\n}" },
    Case { name: "annexb_labelled_in_a_block", lang: JS, edge: "a@2->a@7", expected: CFG,
        source: "function f(c) {\n  var a = src();\n  if (c) {\n    log();\n    l: function a() {}\n  }\n  sink(a);\n}" },
    Case { name: "annexb_generator_in_a_block", lang: JS, edge: "a@2->a@4", expected: CFG,
        source: "function f() {\n  var a = src();\n  { function* a() {} }\n  sink(a);\n}" },
    Case { name: "annexb_inside_a_loop_body", lang: JS, edge: "a@2->a@7", expected: CFG,
        source: "function f(xs) {\n  var a = src();\n  for (const x of xs) {\n    function a() {}\n  }\n  log();\n  sink(a);\n}" },
    Case { name: "annexb_other_name_control", lang: JS, edge: "a@2->a@4", expected: EXACT,
        source: "function f() {\n  var a = src();\n  { function b() {} }\n  sink(a);\n}" },
    Case { name: "annexb_nested_callable_control", lang: JS, edge: "a@2->a@4", expected: EXACT,
        source: "function f() {\n  var a = src();\n  g(function () { { function a() {} } });\n  sink(a);\n}" },
    Case { name: "annexb_strict_block_scoped_control", lang: JS, edge: "a@3->a@5", expected: EXACT,
        source: "'use strict';\nfunction f() {\n  var a = src();\n  { function a() {} }\n  sink(a);\n}" },
    Case { name: "annexb_formal_named_like_it_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  sink(a);\n  { function a() {} }\n}" },
    Case { name: "annexb_ts", lang: TS, edge: "a@2->a@4", expected: CFG,
        source: "function f(): void {\n  var a = src();\n  { function a() {} }\n  sink(a);\n}" },
    Case { name: "annexb_tsx", lang: TSX, edge: "a@2->a@4", expected: CFG,
        source: "function f(): void {\n  var a = src();\n  { function a() {} }\n  sink(a);\n}" },
    // G6a (Annex B.3.5): `var a` inside `catch (a)` writes the catch parameter, not the function's `a`.
    // On head the only edges from that declarator reach reads inside the clause (the binding facts give
    // the def the catch parameter's binding, so no edge reaches a read after the clause); they are real
    // flow, refused anyway by the design's rule (an over-refusal, disclosed).
    Case { name: "catch_var_sloppy", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f() {\n  try { g(); } catch (a) {\n    var a = 2;\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_strict", lang: JS, edge: "a@4->a@5", expected: CFG,
        source: "'use strict';\nfunction f() {\n  try { g(); } catch (a) {\n    var a = 2;\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_other_name_control", lang: JS, edge: "a@3->a@4", expected: EXACT,
        source: "function f() {\n  try { g(); } catch (e) {\n    var a = 2;\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_second_declarator", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f() {\n  try { g(); } catch (a) {\n    var b = 1, a = 2;\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_second_declarator_other_name_control", lang: JS, edge: "b@3->b@4", expected: EXACT,
        source: "function f() {\n  try { g(); } catch (a) {\n    var b = 1, a = 2;\n    sink(b);\n  }\n}" },
    Case { name: "catch_var_destructured_param_is_an_early_error_control", lang: JS, edge: "a@3->a@4", expected: EXACT,
        source: "function f() {\n  try { g(); } catch ({ a }) {\n    var a = 2;\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_nested_in_a_block_of_the_clause", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function f(c) {\n  try { g(); } catch (a) {\n    if (c) {\n      var a = 2;\n    }\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_in_a_nested_callable_control", lang: JS, edge: "a@4->a@5", expected: NOT_CFG,
        source: "function f() {\n  try { g(); } catch (a) {\n    h(function () {\n      var a = 2;\n      sink(a);\n    });\n  }\n}" },
    Case { name: "catch_var_outside_any_clause_control", lang: JS, edge: "a@2->a@4", expected: EXACT,
        source: "function f() {\n  var a = 1;\n  try { g(); } catch (e) {}\n  sink(a);\n}" },
    Case { name: "catch_var_ts_typed_param", lang: TS, edge: "a@3->a@4", expected: CFG,
        source: "function f() {\n  try { g(); } catch (a: unknown) {\n    var a = 2;\n    sink(a);\n  }\n}" },
    Case { name: "catch_var_tsx", lang: TSX, edge: "a@3->a@4", expected: CFG,
        source: "function f() {\n  try { g(); } catch (a) {\n    var a = 2;\n    sink(a);\n  }\n}" },
    // G6b: a write to a function or class expression's own name writes nothing (sloppy) or throws.
    Case { name: "inner_name_function_expression_sloppy", lang: JS, edge: "g@3->g@4", expected: CFG,
        source: "(function g(a) {\n  log();\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_function_expression_strict", lang: JS, edge: "g@4->g@5", expected: CFG,
        source: "(function g(a) {\n  'use strict';\n  log();\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_shadowed_by_var_control", lang: JS, edge: "g@3->g@4", expected: EXACT,
        source: "(function g(a) {\n  var g;\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_shadowed_by_parameter_control", lang: JS, edge: "g@2->g@3", expected: EXACT,
        source: "(function g(g) {\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_shadowed_by_block_let_control", lang: JS, edge: "g@4->g@5", expected: EXACT,
        source: "(function g(a) {\n  {\n    let g;\n    g = 2;\n    sink(g);\n  }\n})();" },
    Case { name: "inner_name_shadow_in_a_sibling_block_does_not_scope", lang: JS, edge: "g@4->g@5", expected: CFG,
        source: "(function g(a) {\n  { let g; }\n  log();\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_destructured_write", lang: JS, edge: "g@2->g@3", expected: CFG,
        source: "(function g(a) {\n  [g] = [2];\n  sink(g);\n})();" },
    Case { name: "inner_name_member_write_control", lang: JS, edge: "g.x@2->g.x@3", expected: EXACT,
        source: "(function g(a) {\n  g.x = 1;\n  sink(g.x);\n})();" },
    Case { name: "inner_name_other_name_control", lang: JS, edge: "g@2->g@3", expected: EXACT,
        source: "(function h(a) {\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_in_a_nested_arrow", lang: JS, edge: "g@3->g@4", expected: CFG,
        source: "(function g() {\n  const h = () => {\n    g = 2;\n    sink(g);\n  };\n})();" },
    Case { name: "inner_name_arrow_shadowed_by_the_arrow_control", lang: JS, edge: "g@3->g@4", expected: NOT_CFG,
        source: "(function g() {\n  const h = (g) => {\n    g = 2;\n    sink(g);\n  };\n})();" },
    Case { name: "inner_name_class_expression_method", lang: JS, edge: "K@3->K@4", expected: CFG,
        source: "const C = class K {\n  m() {\n    K = 2;\n    sink(K);\n  }\n};" },
    Case { name: "inner_name_class_expression_method_shadowed_control", lang: JS, edge: "K@4->K@5", expected: EXACT,
        source: "const C = class K {\n  m() {\n    var K;\n    K = 2;\n    sink(K);\n  }\n};" },
    Case { name: "inner_name_class_declaration", lang: JS, edge: "K@3->K@4", expected: CFG,
        source: "class K {\n  m() {\n    K = 2;\n    sink(K);\n  }\n}" },
    Case { name: "inner_name_function_declaration_control", lang: JS, edge: "g@2->g@3", expected: EXACT,
        source: "function g() {\n  g = 2;\n  sink(g);\n}" },
    Case { name: "inner_name_ts", lang: TS, edge: "g@3->g@4", expected: CFG,
        source: "(function g(a: number) {\n  log();\n  g = 2;\n  sink(g);\n})();" },
    Case { name: "inner_name_tsx_class_expression", lang: TSX, edge: "K@3->K@4", expected: CFG,
        source: "const C = class K {\n  m() {\n    K = 2;\n    sink(K);\n  }\n};" },
    // Round 1 fold. SOL-W1: a body declaration does not scope a write made in a parameter default.
    Case { name: "inner_name_param_default_closure_body_var_does_not_scope", lang: JS, edge: "g@2->g@3", expected: CFG,
        source: "(function g(h = () => {\n  g = 2;\n  sink(g);\n}) {\n  var g;\n  log();\n  h();\n})();" },
    Case { name: "inner_name_enclosing_param_default_closure_body_var_does_not_scope", lang: JS, edge: "g@3->g@4", expected: CFG,
        source: "(function g() {\n  const h = (z = () => {\n    g = 2;\n    sink(g);\n  }) => {\n    var g;\n    z();\n  };\n  h();\n})();" },
    // Opus W1: an owner nested in a sloppy function with mapped `arguments` (directly or by alias).
    Case { name: "args_enclosing_arrow_owner", lang: JS, edge: "a@3->a@5", expected: CFG,
        source: "function f(a) {\n  const g = () => {\n    a = 1;\n    arguments[0] = 2;\n    sink(a);\n  };\n  g();\n}" },
    Case { name: "args_enclosing_alias_nested_function_owner", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function f(a) {\n  var args = arguments;\n  function g() {\n    a = 1;\n    args[0] = 2;\n    sink(a);\n  }\n  g();\n}" },
    Case { name: "args_enclosing_strict_control", lang: JS, edge: "a@4->a@6", expected: NOT_CFG,
        source: "'use strict';\nfunction f(a) {\n  const g = () => {\n    a = 1;\n    arguments[0] = 2;\n    sink(a);\n  };\n  g();\n}" },
    Case { name: "args_enclosing_non_simple_parameters_control", lang: JS, edge: "a@3->a@5", expected: NOT_CFG,
        source: "function f(a, b = 1) {\n  const g = () => {\n    a = 1;\n    arguments[0] = 2;\n    sink(a);\n  };\n  g();\n}" },
    // Opus W2: an owner defined inside a `with` body.
    Case { name: "with_owner_defined_inside_a_body", lang: JS, edge: "a@4->a@6", expected: CFG,
        source: "function outer(obj) {\n  with (obj) {\n    var g = function () {\n      a = 1;\n      obj.a = 5;\n      sink(a);\n    };\n  }\n  return g;\n}" },
    Case { name: "with_in_a_sibling_control", lang: JS, edge: "a@6->a@8", expected: NOT_CFG,
        source: "function outer(obj) {\n  with (obj) {\n    log();\n  }\n  var g = function () {\n    a = 1;\n    obj.a = 5;\n    sink(a);\n  };\n  return g;\n}" },
    // Opus W3: a class declaration's own name inside its body.
    Case { name: "inner_name_class_declaration_try", lang: JS, edge: "K@3->K@4", expected: CFG,
        source: "class K {\n  m() {\n    try { K = 1; } catch (e) {}\n    sink(K);\n  }\n}" },
    Case { name: "inner_name_class_declaration_outside_control", lang: JS, edge: "K@3->K@4", expected: EXACT,
        source: "function f() {\n  class K {}\n  K = 2;\n  sink(K);\n}" },
    Case { name: "inner_name_class_declaration_shadowed_control", lang: JS, edge: "K@4->K@5", expected: NOT_CFG,
        source: "class K {\n  m() {\n    var K;\n    K = 2;\n    sink(K);\n  }\n}" },
    Case { name: "inner_name_class_declaration_ts", lang: TS, edge: "K@3->K@4", expected: CFG,
        source: "class K {\n  m(): void {\n    K = 2;\n    sink(K);\n  }\n}" },
    // Untested arms: catch `var` loop heads and destructuring, and the shadow arms of an immutable name.
    Case { name: "catch_var_for_in_head", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f(o) {\n  try { g(); } catch (a) {\n    for (var a in o) {\n      sink(a);\n    }\n  }\n}" },
    Case { name: "catch_var_destructured_declarator", lang: JS, edge: "a@3->a@4", expected: CFG,
        source: "function f() {\n  try { g(); } catch (a) {\n    var { a } = src();\n    sink(a);\n  }\n}" },
    Case { name: "inner_name_for_let_shadow_control", lang: JS, edge: "g@3->g@4", expected: NOT_CFG,
        source: "(function g(a) {\n  for (let g = 0; g < 2; g++) {\n    g = 5;\n    sink(g);\n  }\n})();" },
    Case { name: "inner_name_for_of_let_shadow_control", lang: JS, edge: "g@3->g@4", expected: NOT_CFG,
        source: "(function g(xs) {\n  for (let g of xs) {\n    g = 5;\n    sink(g);\n  }\n})();" },
    Case { name: "inner_name_switch_let_shadow_control", lang: JS, edge: "g@5->g@6", expected: NOT_CFG,
        source: "(function g(k) {\n  switch (k) {\n    case 1:\n      let g = 1;\n      g = 2;\n      sink(g);\n  }\n})();" },
    Case { name: "inner_name_catch_shadow_control", lang: JS, edge: "g@3->g@4", expected: NOT_CFG,
        source: "(function g() {\n  try { h(); } catch (g) {\n    g = 2;\n    sink(g);\n  }\n})();" },
    Case { name: "inner_name_block_function_shadow_control", lang: JS, edge: "g@5->g@6", expected: NOT_CFG,
        source: "(function g(a) {\n  'use strict';\n  {\n    function g() {}\n    g = 2;\n    sink(g);\n  }\n})();" },
    Case { name: "inner_name_labelled_block_class_shadow_control", lang: JS, edge: "g@4->g@5", expected: NOT_CFG,
        source: "(function g(a) {\n  {\n    class g {}\n    g = 2;\n    sink(g);\n  }\n})();" },
];

#[test]
fn enumeration_rows() {
    let mut failures = Vec::new();
    for c in CASES {
        let got = labels_of(c.lang, c.source, c.edge);
        let bad = if c.expected == NOT_CFG {
            got.iter().any(|g| g == CFG)
        } else {
            got.iter().any(|g| g != c.expected)
        };
        if got.is_empty() || bad {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                c.name, c.edge, c.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The edge set never moves: every enumeration row's edge exists on head.
#[test]
fn enumeration_rows_keep_their_edges() {
    for c in CASES {
        assert!(
            !labels_of(c.lang, c.source, c.edge).is_empty(),
            "{} {}: edge disappeared",
            c.name,
            c.edge
        );
    }
}

/// Every enumeration and matrix source, for the neutrality tests.
pub(super) fn sources() -> Vec<(Language, String)> {
    let mut out: Vec<(Language, String)> = CASES
        .iter()
        .map(|c| (c.lang, c.source.to_string()))
        .collect();
    out.extend(rows().into_iter().map(|r| (lang_of(&r.fixture), r.source)));
    out
}

/// Frozen main rows of the case `id` (layout `[from_line, from_start, from_end, to_line, to_start,
/// to_end, label, doubt, kill_line]`) with S1d-1's moves applied. Each listed case holds an `exact`
/// row the refusal classes now label: `C1-catchvar` (the catch-parameter `var`, G6a), the mapped
/// `arguments` and the `eval` in a default (6.1, 6.1b: every edge of the owner), and `R5b-body-2`
/// (a hoisted `function f` replaces the formal `f`: the formal's edges are killed at its line).
pub(super) fn refuse_moved(id: &str, rows: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    let rows = super::s1c_label_tests::refuse_moved(id, rows);
    rows.into_iter()
        .map(|row| {
            let mut row = row.as_array().unwrap().clone();
            let (from, exact) = (row[0].as_u64().unwrap(), row[6] == "exact");
            let moved = exact
                && match id {
                    "C1-catchvar" => from == 6,
                    "E-mapped-arguments-named-sloppy" | "E-eval-default-named-sloppy" => true,
                    "R5b-body-2" => from == 1,
                    _ => false,
                };
            if moved && id == "R5b-body-2" {
                row[6] = "nameonly".into();
                row[7] = "killed".into();
                row[8] = 2.into();
            } else if moved {
                row[6] = "nameonly".into();
                row[7] = "cfg_incomplete".into();
            }
            row.into()
        })
        .collect()
}

/// Frozen rows of a labelled hoisted `function y` with a seam: the formal's `exact` rows (from line 1)
/// are killed at the declaration's line 2 (6.1d).
pub(super) fn kill_formal(mut rows: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
    for row in &mut rows {
        if row[0] == 1 && row[6] == "exact" {
            row[6] = "nameonly".into();
            row[7] = "killed".into();
            row[8] = 2.into();
        }
    }
    rows
}
