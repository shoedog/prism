//! #674 (G1/G2): a recorded def of a JS/TS/TSX owner kills every recorded def its path `affects`
//! (a prefix write kills the paths below it, `[]` matches any component either way), as the census's
//! kill-only defs already do. Label-only: no edge moves. FIX rows are `exact` on base 3991e252 and
//! move to `nameonly/killed`; CONTROL rows hold on both. Python and Go keep main's equal-path kills.
use super::census_label_tests::labels_of;
use crate::js_write_census::matrix_tests::lang_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../tests/fixtures/js_write_census/variant-matrix.tsv");

const EXACT: &str = "exact";
const KILLED: &str = "nameonly/killed";
const CFG: &str = "nameonly/cfg_incomplete";
const SAME: &str = "nameonly/sameline";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;

struct Case {
    name: &'static str,
    lang: Language,
    edge: &'static str,
    source: &'static str,
    expected: &'static str,
}

/// The two matrix rows this slice flips (o27 = G2, o29 = G1) are FIX rows too.
#[test]
fn flipped_matrix_rows() {
    let mut seen = 0;
    for line in MATRIX.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if !c[5].contains("#674") {
            continue;
        }
        seen += 1;
        assert_eq!(c[2], "exact", "{}: main label", c[0]);
        assert_eq!(c[4], KILLED, "{}: expected label", c[0]);
        let got = labels_of(lang_of(c[0]), &c[7].replace("\\n", "\n"), c[1]);
        assert_eq!(got, [KILLED], "{} {}", c[0], c[1]);
    }
    assert_eq!(seen, 2, "o27 and o29");
}

#[rustfmt::skip]
const FIX: &[Case] = &[
    // G1: a prefix write kills the paths below it.
    Case { name: "prefix_write_straight_line", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o = {};\n  sink(o.x);\n}" },
    Case { name: "two_deep_prefix", lang: JS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o) {\n  o.a.b = 1;\n  o.a = {};\n  sink(o.a.b);\n}" },
    Case { name: "root_write_kills_two_deep", lang: JS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o) {\n  o.a.b = 1;\n  o = {};\n  sink(o.a.b);\n}" },
    Case { name: "this_prefix", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: KILLED,
        source: "function f() {\n  this.a.b = 1;\n  this.a = {};\n  sink(this.a.b);\n}" },
    Case { name: "prefix_both_branches", lang: JS, edge: "o.x@2->o.x@8", expected: KILLED,
        source: "function f(o, c) {\n  o.x = 1;\n  if (c) {\n    o = {};\n  } else {\n    o = {};\n  }\n  sink(o.x);\n}" },
    Case { name: "prefix_write_in_the_loop_body", lang: JS, edge: "o.x@2->o.x@5", expected: KILLED,
        source: "function f(o, xs) {\n  o.x = 0;\n  for (const x of xs) {\n    o = {};\n    sink(o.x);\n  }\n}" },
    Case { name: "optional_chain_read", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o = {};\n  sink(o?.x);\n}" },
    Case { name: "ts_prefix", lang: TS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o: any) {\n  o.x = 1;\n  o = {} as any;\n  sink(o.x);\n}" },
    Case { name: "tsx_prefix", lang: Language::Tsx, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o: any) {\n  o.x = 1;\n  o = {};\n  return <b>{sink(o.x)}</b>;\n}" },
    // The prefix write sits in a later statement than the def (D3 sees both nodes clean).
    Case { name: "prefix_write_in_a_shorter_node", lang: JS, edge: "o.x@3->o.x@5", expected: KILLED,
        source: "function f(o) {\n  log();\n  g(o.x = 1,\n    o = {}); y();\n  sink(o.x);\n}" },
    // G2: `[]` is a wildcard component, both directions.
    Case { name: "subscript_write_kills_dot_def", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o[\"x\"] = 2;\n  sink(o.x);\n}" },
    Case { name: "dot_write_kills_subscript_def", lang: JS, edge: "o[]@2->o[]@4", expected: KILLED,
        source: "function f(o, k) {\n  o[k].x = 1;\n  o.a = 2;\n  sink(o[k].x);\n}" },
    Case { name: "subscript_in_the_middle", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: KILLED,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o.a[k] = 2;\n  sink(o.a.x);\n}" },
    Case { name: "root_write_kills_subscript_def", lang: JS, edge: "o[]@2->o[]@4", expected: KILLED,
        source: "function f(o, k) {\n  o[k].x = 1;\n  o = {};\n  sink(o[k].x);\n}" },
];

#[rustfmt::skip]
const CONTROLS: &[Case] = &[
    // A write at or below the def's path does not kill it more than main did.
    Case { name: "below_the_def_path", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  o.x.y = 2;\n  sink(o.x);\n}" },
    Case { name: "sibling_field", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  o.y = 2;\n  sink(o.x);\n}" },
    Case { name: "other_base", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, p) {\n  o.x = 1;\n  p = {};\n  sink(o.x);\n}" },
    Case { name: "longer_write_below_subscript_def", lang: JS, edge: "o[]@2->o[]@4", expected: EXACT,
        source: "function f(o, k) {\n  o[k].x = 1;\n  o.x.y = 2;\n  sink(o[k].x);\n}" },
    Case { name: "sibling_under_subscript", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o.b[k] = 2;\n  sink(o.a.x);\n}" },
    // Main's equal-path kills are a subset.
    Case { name: "equal_path_kill", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: KILLED,
        source: "function f(o) {\n  o.a.x = 1;\n  o.a.x = 2;\n  sink(o.a.x);\n}" },
    Case { name: "equal_subscript_kill", lang: JS, edge: "o[]@2->o[]@4", expected: KILLED,
        source: "function f(o, k, j) {\n  o[k].x = 1;\n  o[j].x = 2;\n  sink(o[k].x);\n}" },
    // Already `killed` on main (a census kill-only def): the recorded relation must not change it.
    Case { name: "subscript_writer_kills_dot_def_already_killed", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: KILLED,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o[k].x = 2;\n  sink(o.a.x);\n}" },
    // A prefix write on one branch only: the other branch still carries the def.
    Case { name: "prefix_on_one_branch", lang: JS, edge: "o.x@2->o.x@6", expected: EXACT,
        source: "function f(o, c) {\n  o.x = 1;\n  if (c) {\n    o = {};\n  }\n  sink(o.x);\n}" },
    // Another binding of the base: the block `let o` is not the def's binding (flat kills it, bound does not).
    Case { name: "shadowing_let_in_a_block", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  { let o = {}; o = {}; }\n  sink(o.x);\n}" },
    // The write is in the def's own statement node: no kill there (`affects` kills from other nodes only).
    Case { name: "prefix_in_the_def_node", lang: JS, edge: "o.x@2->o.x@3", expected: SAME,
        source: "function f(o) {\n  g(o.x = 1, o = {});\n  sink(o.x);\n}" },
    // W1 (Opus r1): a truncated writer (`AccessPath::from_expr` cuts an lvalue at its first `[`, so `o[k].x` is
    // recorded as `o.[]`) can never replace `o.n`; it keeps main's equal-path kill. The census adds the precise
    // kill-only def. Positive controls: a real `o[k] = 2` still kills `o.n`, `o[k].x = 2` still kills `o.a.x`.
    Case { name: "truncated_subscript_member_writer", lang: JS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o, k) {\n  o.n = 1;\n  o[k].x = 2;\n  sink(o.n);\n}" },
    Case { name: "truncated_nested_subscript_member_writer", lang: JS, edge: "o.a.n@2->o.a.n@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.n = 1;\n  o.a[k].x = 2;\n  sink(o.a.n);\n}" },
    Case { name: "truncated_double_subscript_writer", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, k) {\n  o.x = 1;\n  o[k][0] = 2;\n  sink(o.x);\n}" },
    Case { name: "real_subscript_write_still_kills_dot_def", lang: JS, edge: "o.n@2->o.n@4", expected: KILLED,
        source: "function f(o, k) {\n  o.n = 1;\n  o[k] = 2;\n  sink(o.n);\n}" },
    Case { name: "truncated_writer_kills_through_the_census", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: KILLED,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o[k].x = 2;\n  sink(o.a.x);\n}" },
    // W2 (Opus r1): a recorded def in a nested callable kills only by path equality (main's rule).
    Case { name: "closure_local_var_of_the_base", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  g(() => { var o = {}; use(o); });\n  sink(o.x);\n}" },
    Case { name: "callback_parameter_reassigned", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  g(function (o) { o = {}; use(o); });\n  sink(o.x);\n}" },
    Case { name: "for_var_of_in_a_callback", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, xs) {\n  o.x = 1;\n  xs.forEach((y) => { for (var o of y) use(o); });\n  sink(o.x);\n}" },
    Case { name: "var_in_a_map_callback", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, xs) {\n  o.x = 1;\n  xs.map((y) => { var o = y; return o; });\n  sink(o.x);\n}" },
    Case { name: "nested_function_this_write", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: EXACT,
        source: "function f(xs) {\n  this.a.b = 1;\n  xs.forEach(function () { this.a = {}; });\n  sink(this.a.b);\n}" },
    // Round 2, (a) storage identity through wrappers: the complete target chain is counted, not only the
    // part above the first token. `(o[k].x).y` is recorded `o.[].y` but writes `o[k].x.y`.
    Case { name: "parenthesized_truncated_receiver", lang: JS, edge: "o.a.y@2->o.a.y@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.y = 1;\n  (o[k].x).y = 2;\n  sink(o.a.y);\n}" },
    Case { name: "double_parenthesized_double_subscript", lang: JS, edge: "o.a.y@2->o.a.y@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.y = 1;\n  ((o[k]))[0].y = 2;\n  sink(o.a.y);\n}" },
    Case { name: "parenthesized_base_then_subscript", lang: JS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o, k) {\n  o.n = 1;\n  (o)[k].x = 2;\n  sink(o.n);\n}" },
    Case { name: "ts_non_null_after_subscript", lang: TS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o: any, k: any) {\n  o.n = 1;\n  o[k]!.x = 2;\n  sink(o.n);\n}" },
    Case { name: "ts_as_wrapped_truncated", lang: TS, edge: "o.a.y@2->o.a.y@4", expected: EXACT,
        source: "function f(o: any, k: any) {\n  o.a.y = 1;\n  (o[k].x as any).y = 2;\n  sink(o.a.y);\n}" },
    Case { name: "parenthesized_prefix_write_still_kills", lang: JS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o) {\n  o.a.b = 1;\n  (o).a = {};\n  sink(o.a.b);\n}" },
    // Round 2, (b) receiver identity: a `this` that is not the owner's keeps path equality.
    Case { name: "class_expression_static_field_this", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: EXACT,
        source: "function f() {\n  this.a.b = 1;\n  const C = class { static y = (this.a = {}); };\n  sink(this.a.b);\n}" },
    Case { name: "class_declaration_static_field_this", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: EXACT,
        source: "function f() {\n  this.a.b = 1;\n  class C { static y = (this.a = {}); }\n  sink(this.a.b);\n}" },
    Case { name: "static_block_this", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: EXACT,
        source: "function f() {\n  this.a.b = 1;\n  class C { static { this.a = {}; } }\n  sink(this.a.b);\n}" },
    // The computed key runs in the owner's scope, so the write stays eligible (`writer_flags_by_receiver_and_binding`);
    // D3 owns this label because the class statement holds the write (sameline on main and on head).
    Case { name: "eager_computed_key_writes_the_owners_this", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: SAME,
        source: "function f() {\n  this.a.b = 1;\n  class C { [this.a = {}]() {} }\n  sink(this.a.b);\n}" },
    Case { name: "arrow_closure_this", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: "nameonly/cfg_incomplete",
        source: "function f() {\n  this.a.b = 1;\n  g(() => { this.a = {}; });\n  sink(this.a.b);\n}" },
    // Round 2, (b) declaration identity: a catch parameter or block-level function has no recorded def, so
    // `BindingFacts` resolves a write to it as the outer binding; path equality only.
    Case { name: "catch_parameter_write", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  try { g(); } catch (o) { o = {}; use(o); }\n  sink(o.x);\n}" },
    Case { name: "catch_destructured_parameter_write", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  try { g(); } catch ({ o }) { o = {}; use(o); }\n  sink(o.x);\n}" },
    Case { name: "ts_typed_catch_parameter_write", lang: TS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o: any) {\n  o.x = 1;\n  try { g(); } catch (o: any) { o = {}; use(o); }\n  sink(o.x);\n}" },
    Case { name: "catch_parameter_member_write", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: EXACT,
        source: "function f(o) {\n  o.a.x = 1;\n  try { g(); } catch (o) { o.a = {}; }\n  sink(o.a.x);\n}" },
    Case { name: "block_function_declaration_write", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  { function o() {} o = 1; }\n  sink(o.x);\n}" },
    Case { name: "outer_write_after_try_still_kills", lang: JS, edge: "o.x@2->o.x@5", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  try { g(); } catch (e) { h(); }\n  o = {};\n  sink(o.x);\n}" },
    // (A wrapped identifier receiver is normalized by the producer: `(o satisfies any).a` is recorded `o.a`, the census
    // agrees, and the write really overwrites `o.a`, so those two rows kill.)
    // Round 3: eligibility is identity equality with the census's precise identity for the same site.
    Case { name: "call_in_chain_member", lang: JS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o, k) {\n  o.n = 1;\n  o[k]().x = 2;\n  sink(o.n);\n}" },
    Case { name: "call_in_chain_subscript", lang: JS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o, k) {\n  o.n = 1;\n  o[k]()[0] = 2;\n  sink(o.n);\n}" },
    Case { name: "tagged_template_in_chain", lang: JS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o, k, t) {\n  o.n = 1;\n  o[k]`t`.x = 2;\n  sink(o.n);\n}" },
    Case { name: "call_in_nested_chain", lang: JS, edge: "o.a.n@2->o.a.n@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.n = 1;\n  o.a[k]().x = 2;\n  sink(o.a.n);\n}" },
    Case { name: "ts_non_null_call_in_chain", lang: TS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o: any, k: any) {\n  o.n = 1;\n  o[k]!().x = 2;\n  sink(o.n);\n}" },
    Case { name: "call_in_destructuring_target", lang: JS, edge: "o.n@2->o.n@4", expected: EXACT,
        source: "function f(o, k, src) {\n  o.n = 1;\n  ({ v: o[k]().x } = src);\n  sink(o.n);\n}" },
    Case { name: "asserted_this_in_static_field", lang: TS, edge: "this as any.a.b@2->this as any.a.b@4", expected: EXACT,
        source: "function f() {\n  (this as any).a.b = 1;\n  const C = class { static y = ((this as any).a = {}); };\n  sink((this as any).a.b);\n}" },
    Case { name: "asserted_this_direct_write_keeps_main", lang: TS, edge: "this as any.a.b@2->this as any.a.b@4", expected: EXACT,
        source: "function f() {\n  (this as any).a.b = 1;\n  (this as any).a = {};\n  sink((this as any).a.b);\n}" },
    Case { name: "text_split_key_in_truncated_chain", lang: JS, edge: "o.a.y@2->o.a.y@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.y = 1;\n  (o[k].x).y[\"a.b.c\"].z = 2;\n  sink(o.a.y);\n}" },
    Case { name: "ts_satisfies_wrapped_prefix_kills", lang: TS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o: any) {\n  o.a.b = 1;\n  (o satisfies any).a = {};\n  sink(o.a.b);\n}" },
    Case { name: "ts_type_assertion_wrapped_prefix_kills", lang: TS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o: any) {\n  o.a.b = 1;\n  (<any>o).a = {};\n  sink(o.a.b);\n}" },
    Case { name: "switch_case_function_declaration_write", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(t, o) {\n  o.x = 1;\n  switch (t) { case 0: function o() {} o = {}; }\n  sink(o.x);\n}" },
    // A plain declarator (`let`/`const`/`var o = ..`) is eligible (round 3 amendment); formals, destructured and
    // loop-head names are not: a block-scoped pattern name is not told from the outer binding.
    Case { name: "loop_const_redeclaration_resize_shape_killed", lang: JS, edge: "update.b.c@5->update.b.c@8", expected: KILLED,
        source: "function f(es, c) {\n  for (const e of es) {\n    const update = {};\n    if (c) {\n      update.b.c = 1;\n    } else {\n      update.b.c = 2;\n      use(update.b.c);\n    }\n    push(update);\n  }\n}" },
    Case { name: "loop_const_redeclaration_resize_shape_unflagged_route", lang: JS, edge: "update.b@5->update.b@7", expected: CFG,
        source: "function f(es, c) {\n  for (const e of es) {\n    const update = {};\n    if (c) {\n      update.b = 1;\n    } else {\n      update.b = g(update.b);\n      use(update.b);\n    }\n    push(update);\n  }\n}" },
    Case { name: "let_redeclared_in_loop_body_kills_previous_iteration", lang: JS, edge: "o.b.c@5->o.b.c@8", expected: KILLED,
        source: "function f(es, c) {\n  for (const e of es) {\n    let o = {};\n    if (c) {\n      o.b.c = 1;\n    } else {\n      o.b.c = 2;\n      use(o.b.c);\n    }\n    push(o);\n  }\n}" },
    Case { name: "var_destructuring_redeclaration_keeps_main", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, src) {\n  o.x = 1;\n  var { o } = src();\n  sink(o.x, o);\n}" },
    Case { name: "block_destructured_shadow_does_not_kill_outer", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, src) {\n  o.x = 1;\n  { const { o } = src(); use(o); }\n  sink(o.x);\n}" },
    Case { name: "inner_block_let_shadow_does_not_kill_outer_decl", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  { let o = {}; use(o); }\n  sink(o.x);\n}" },
    Case { name: "var_redeclaration_kills", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  var o = {};\n  sink(o.x);\n}" },
    // Other languages keep main's equal-path kills.
    Case { name: "python_prefix", lang: Language::Python, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "def f(o):\n    o.x = 1\n    o = {}\n    sink(o.x)\n" },
    Case { name: "go_prefix", lang: Language::Go, edge: "o.x@4->o.x@6", expected: EXACT,
        source: "package p\n\nfunc f(o T) {\n\to.x = 1\n\to = T{}\n\tsink(o.x)\n}\n" },
];

fn run(cases: &[Case]) -> Vec<String> {
    let mut failures = Vec::new();
    for c in cases {
        let got = labels_of(c.lang, c.source, c.edge);
        if got.is_empty() || got.iter().any(|g| g != c.expected) {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                c.name, c.edge, c.expected
            ));
        }
    }
    failures
}

#[test]
fn fix_rows_move_to_killed() {
    let failures = run(FIX);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn control_rows_hold() {
    let failures = run(CONTROLS);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Full `FlowConfidence` (doubt and `kill_line`), not just the label class.
fn doubts_of(language: Language, source: &str, edge: &str) -> Vec<String> {
    use crate::ast::ParsedFile;
    use crate::data_flow::test_support::label_snapshot;
    let ext = match language {
        Language::TypeScript => "ts",
        Language::Tsx => "tsx",
        _ => "js",
    };
    let name = format!("a.{ext}");
    let file = ParsedFile::parse(&name, source, language).unwrap();
    let cpg =
        crate::cpg::CodePropertyGraph::build(&std::collections::BTreeMap::from([(name, file)]));
    let (from, to) = edge.split_once("->").unwrap();
    let split = |s: &str| {
        let (p, l) = s.rsplit_once('@').unwrap();
        (p.to_string(), l.parse::<usize>().unwrap())
    };
    let (from, to) = (split(from), split(to));
    label_snapshot(&cpg.dfg)
        .into_iter()
        .filter(|((f, t), _)| {
            (f.path.to_string(), f.line) == from && (t.path.to_string(), t.line) == to
        })
        .map(|(_, label)| format!("{label:?}"))
        .collect()
}

fn killed_at(line: u32) -> String {
    format!("NameOnly(Killed {{ kill_line: {line} }})")
}

/// The kill line moves with the kill (T1/S1 of review r1): the prefix and subscript kills name their own line,
/// and when an equal-path kill competes the lowest reachable kill line wins, as for main's own kills.
#[test]
fn kill_lines_follow_the_new_kills() {
    #[rustfmt::skip]
    let rows: &[(&str, &str, &str, u32)] = &[
        ("g1_prefix", "o.x@2->o.x@4", "function f(o) {\n  o.x = 1;\n  o = {};\n  sink(o.x);\n}", 3),
        ("g2_subscript", "o.x@2->o.x@4", "function f(o) {\n  o.x = 1;\n  o[\"x\"] = 2;\n  sink(o.x);\n}", 3),
        ("two_deep_prefix", "o.a.b@2->o.a.b@4", "function f(o) {\n  o.a.b = 1;\n  o.a = {};\n  sink(o.a.b);\n}", 3),
        // Main's kill is line 4 (`o.x = 2`, equal path); the prefix write at line 3 is the lowest kill now.
        ("competing_lines", "o.x@2->o.x@5", "function f(o) {\n  o.x = 1;\n  o = {};\n  o.x = 2;\n  sink(o.x);\n}", 3),
        // Representative target forms.
        ("destructuring_target", "o.x@2->o.x@4", "function f(o) {\n  o.x = 1;\n  ({ o } = src());\n  sink(o.x, o);\n}", 3),
        ("logical_assignment_target", "o.a.b@2->o.a.b@4", "function f(o) {\n  o.a.b = 1;\n  o.a ||= {};\n  sink(o.a.b, o.a);\n}", 3),
    ];
    let mut failures = Vec::new();
    for (name, edge, src, line) in rows {
        let got = doubts_of(JS, src, edge);
        if got != [killed_at(*line)] {
            failures.push(format!("{name} {edge}: want kill_line {line}, got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
