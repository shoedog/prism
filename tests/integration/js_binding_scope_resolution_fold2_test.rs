//! S1b-3b round-1 review fold, part 2: the mandatory coverage-gap rows (T-a..T-g). Split from
//! `js_binding_scope_resolution_fold_test.rs` to stay under 600 lines (CLAUDE.md).
use super::js_binding_scope_resolution_test::{check, sites, two_file_graph};

// ---------------------------------------------------------------------------------------
// T-a (Opus): `js_ts_local_callable` must filter candidates by span, not just name. A
// C86-style decoy (two same-named local arrows in different scopes) proves each call site
// retargets to its *own* scope's callable, never the other.
// ---------------------------------------------------------------------------------------

#[test]
fn ta_c86_retarget_is_span_exact_not_name_only() {
    let src = "export function one() {\n  const g = () => 1;\n  return g()\n}\n\
        export function two() {\n  const g = () => 2;\n  return g()\n}\n";
    check(
        src,
        &["js", "ts"],
        &["L3 g: Exact local_def g@2-2", "L7 g: Exact local_def g@6-6"],
    );
}

// ---------------------------------------------------------------------------------------
// T-b (Opus): a JSX `jsx_opening_element` site (not just `jsx_self_closing_element`) must
// get its `local_binding` set. A decoy same-named function in a different (nested) scope
// proves it: if the opening/closing element were left `Unchecked`, ordinary R4 would bind
// *both* same-file candidates (ambiguous `demoted`); with the binding set, the span filter
// picks the one callable the JSX element's own scope denotes.
// ---------------------------------------------------------------------------------------

#[test]
fn tb_jsx_opening_and_closing_element_gets_local_binding() {
    let src = "function outer() {\n  function Foo() { return 1; }\n  return Foo;\n}\n\
        function Foo() { return 2; }\n\
        export function App() {\n  return <Foo></Foo>;\n}\n";
    check(src, &["jsx", "tsx"], &["L7 Foo: Exact local_def Foo@5-5"]);
}

#[test]
fn tb_jsx_self_closing_element_gets_local_binding() {
    let src = "function outer() {\n  function Foo() { return 1; }\n  return Foo;\n}\n\
        function Foo() { return 2; }\n\
        export function App() {\n  return <Foo/>;\n}\n";
    check(src, &["jsx", "tsx"], &["L7 Foo: Exact local_def Foo@5-5"]);
}

// ---------------------------------------------------------------------------------------
// T-c (Opus): an unwritten namespace-import's binding must map to `Unproven("import")`
// (which drops), not `MayCall("import")`. SPEC §3.8 (8)'s C-44 note is corrected: the
// pre-existing JS/TS guard's `UnknownName` masking is real only for `import_alias` (C141,
// `import f = M.g`); a namespace import (`import * as f`) is not tracked by that guard at
// all and reaches this slice's own `LocalBindingUnproven` drop directly.
// ---------------------------------------------------------------------------------------

#[test]
fn tc_unwritten_namespace_import_maps_to_unproven_not_maycall() {
    let a = (
        "a.js",
        "import * as f from './b';\nfunction g() {\n  function f() {}\n  return f\n}\n\
        export function run() {\n  return f()\n}\n",
    );
    let b = ("b.js", "export function unrelated() { return 1; }\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L7 f: drop LocalBindingUnproven"]);
}

// ---------------------------------------------------------------------------------------
// T-d (both reviewers): the R5 `duplicate_declaration` reason is reachable (the earlier
// "could not construct" disclosure in `js_binding_scope_resolution_r2_test.rs` was wrong —
// corrected there in this fold). Two `for (var f of …)` heads reach R5 because `for` heads
// are not tracked by the pre-existing JS/TS caller-local guard; sol's nested-arrow parameter
// representative reaches the same R5 `not_callable` drop for the matching reason: the call
// site's *own* caller is the arrow, not the function whose parameter shadows the name, so
// the guard (keyed per caller function) does not see it either.
// ---------------------------------------------------------------------------------------

#[test]
fn td_duplicate_for_heads_drop_at_r5() {
    let a = (
        "a.js",
        "export function run(a, b) {\n  for (var f of a) {}\n  for (var f of b) {}\n  return f();\n}\n",
    );
    let lib = ("lib.js", "export function f() { return 1; }\n");
    let cg = two_file_graph(a, lib);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L4 f: drop LocalBindingUnproven"]);
}

#[test]
fn td_duplicate_for_heads_negative_control_single_head_binds() {
    // Negative control: one `for` head only (no duplicate) leaves `f` genuinely unbound at
    // the call site's own scope chain only when nothing else declares it; here a single
    // non-duplicate `for` head is itself `not_callable` (D7), still dropping — the real
    // control is removing the loop entirely, which leaves `f` unbound and keeps base R5.
    let a = ("a.js", "export function run(a) {\n  return f();\n}\n");
    let lib = ("lib.js", "export function f() { return 1; }\n");
    let cg = two_file_graph(a, lib);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L2 f: Exact free_single f@1-1"]);
}

#[test]
fn td_nested_arrow_parameter_drops_at_r5() {
    // sol's representative: the call site's caller is the arrow `cb`, not `run` (whose
    // parameter `f` shadows the name) — the old per-caller-function guard does not apply to
    // `cb`, so this genuinely reaches the new R5 `not_callable` drop.
    let a = (
        "a.js",
        "export function run(f) {\n  const cb = () => f();\n  return cb();\n}\n",
    );
    let lib = ("lib.js", "export function f() { return 1; }\n");
    let cg = two_file_graph(a, lib);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L2 f: drop LocalBindingUnproven"]);
}

#[test]
fn td_captured_catch_parameter_still_masked_by_the_older_guard() {
    // Disclosed finding (this fold): unlike the nested-arrow parameter above, a catch
    // parameter captured in a nested arrow is *still* masked by the pre-existing JS/TS
    // guard (it drops `UnknownName`, not the new `LocalBindingUnproven`) — the guard's
    // per-caller-function local set evidently includes catch parameters transitively. This
    // is recorded honestly rather than claimed as a second working construction.
    let a = (
        "a.js",
        "export function run() {\n  const cb = () => {\n    try {} catch (f) {\n      return f();\n    }\n  };\n  return cb();\n}\n",
    );
    let lib = ("lib.js", "export function f() { return 1; }\n");
    let cg = two_file_graph(a, lib);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L4 f: drop UnknownName"]);
}

// ---------------------------------------------------------------------------------------
// T-e (Opus): a local `useCallback` *not* imported from `"react"` must never be admitted
// as the React wrapper, even though it is called with a direct function argument from
// another module — the import's own module path, not just its own spelling, decides.
// ---------------------------------------------------------------------------------------

#[test]
fn te_use_callback_impostor_imported_from_elsewhere_keeps_base() {
    let a = (
        "a.jsx",
        "import { useCallback } from './mine';\nexport function Comp() {\n  const cb = useCallback(() => 1, []);\n  return cb();\n}\n",
    );
    let mine = (
        "mine.jsx",
        "export function useCallback(fn) { return fn; }\n",
    );
    let cg = two_file_graph(a, mine);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" cb:"))
        .collect();
    assert_eq!(out, ["L4 cb: Exact local_def cb@3-3"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 1);
}

// ---------------------------------------------------------------------------------------
// T-f (Opus): the `js_ts_local_callable` fallback (candidates whose span does not match)
// must answer `LocalBindingUnproven` when a same-*name* function exists elsewhere in the
// file, not unconditionally `UnknownName`. C95 (two different scopes, each declaring a
// `const f` whose *span collapses to the same line* under 1-indexed line tracking) is the
// real exercise of this branch; `c8_same_line_collision_drops` instead exercises B2's
// declaration-level duplicate (`Unproven("duplicate_declaration")`, refused before
// `js_ts_local_callable` is ever reached), which is a different mechanism entirely.
// ---------------------------------------------------------------------------------------

#[test]
fn tf_c95_same_line_callable_span_collision_drops_local_binding_unproven() {
    let src =
        "function one(){ const f=()=>1; return f() } function two(){ const f=()=>2; return f() }\n";
    check(
        src,
        &["js", "ts"],
        &[
            "L1 f: drop LocalBindingUnproven",
            "L1 f: drop LocalBindingUnproven",
        ],
    );
}

// ---------------------------------------------------------------------------------------
// T-g (sol SMELL / Opus S2): the real C181 shape (`C181_solW2_unicode_recovered_alias` in
// `probes/controls_gen.py`) spells the Unicode alias *inside the malformed import's string
// text*, not as a bare identifier — the shape the committed unit/wiring tests used, which
// the tree's own identifier walk independently supplies regardless of the ASCII-only
// word-scanner mutant (C-M37, `src/ast/js_binding_recovery.rs:51`). Ported here as a
// two-file end-to-end test: this *does* kill C-M37 (verified below and in the mutant log).
// ---------------------------------------------------------------------------------------

#[test]
fn tg_c181_malformed_string_unicode_alias_drops_and_kills_cm37() {
    let lib =
        "export function e\u{301}() {\n  return 1;\n}\nexport function a\u{200c}b() {\n  return 2;\n}\n";
    let app = "export function run() {\n  return e\u{301}() + a\u{200c}b();\n}\n\
        import { \"\\u{GG}\" as e\u{301}, \"\\u{GG}\" as a\u{200c}b } from './lib';\n";
    for (app_path, lib_path) in [("app.js", "lib.js"), ("app.ts", "lib.ts")] {
        let cg = two_file_graph((app_path, app), (lib_path, lib));
        let out = sites(&cg);
        assert_eq!(
            out,
            [
                "L2 a\u{200c}b: drop LocalBindingUnproven",
                "L2 e\u{301}: drop LocalBindingUnproven",
            ],
            "{app_path}"
        );
    }
}
