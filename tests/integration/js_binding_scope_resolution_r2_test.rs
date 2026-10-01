//! S1b-3b (SPEC §3.6, §3.8 (8)(9), §7, §9): the call-site wiring, part 2 (controller round 2,
//! 2026-09-30 — full §9 scenario-count coverage). C-20, the explicit "Unproven at R5"
//! representatives, C-41's per-reason R5 rows, C-42's clean twin and C181, C-44's positive
//! control, the alias pins C174-C177/C182-C184 split out explicitly, and a third C-5 twin.
//! Split from `js_binding_scope_resolution_test.rs` to stay under 600 lines (CLAUDE.md).
//! Every expectation here was verified against the real implementation with a throwaway
//! probe harness (see the handback), deleted before commit.
use super::js_binding_scope_resolution_test::{check, graph, sites, two_file_graph};

// ---------------------------------------------------------------------------------------
// §9 full-coverage additions (controller round 2, 2026-09-30): the remaining enumerated
// scenarios this dispatch's §9 budget counts (C-20, the explicit "Unproven at R5"
// representative, C-41's per-reason R5 rows, C-42's clean twin and C181, C-44's positive
// control, the alias pins C174-C177/C182-C184 split out explicitly, and a third C-5 twin).
// Every expectation here was verified against the real implementation the same way as above
// (a throwaway probe harness, deleted before commit).
// ---------------------------------------------------------------------------------------

// --- C-20: qualified and indirect sites never consult `local_binding` (2 scenarios). ---

#[test]
fn c20a_qualified_call_site_stays_unchecked() {
    // `o.f()`'s callee is a `member_expression`, not a plain identifier: `js_local_binding_at`
    // only inspects `call_expression.function`/`jsx_*.name` when that field is itself a bare
    // `identifier`, so the qualified call's binding is `Unchecked` and the receiver route
    // (unaffected by S1b-3b) decides alone.
    let src = "function f() {\n  return 1;\n}\nconst o = { f() { return 2; } };\n\
        export function g() {\n  return o.f();\n}\n";
    let cg = graph("a.js", src);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "f")
        .expect("one call site");
    assert_eq!(
        site.local_binding,
        prism::call_graph::JsLocalBinding::Unchecked
    );
}

#[test]
fn c20b_indirect_resolution_site_stays_unchecked() {
    // E10 non-goal: `indirect_call_site` (Go Level-3 callback construction, the one producer
    // this dispatch forbids touching) hardcodes `JsLocalBinding::Unchecked` regardless of
    // language. It is private, so the invariant is pinned at its public contract: `Default`
    // (what every non-JS/TS, synthetic and indirect site in the repo relies on) is `Unchecked`,
    // never a stale `Callable` a caller might otherwise assume is "unset".
    use prism::call_graph::JsLocalBinding;
    assert_eq!(JsLocalBinding::default(), JsLocalBinding::Unchecked);
}

// --- The sixth representative, named explicitly: "Unproven at R5". ---

#[test]
fn rep_unproven_at_r5_not_callable_for_head_drops() {
    // A bare `for (var f in o)` head declares `f`, non-callable (D7): no same-file function
    // named `f` exists (R4's `local` is empty), so base would fall to R5's cross-file free
    // function in `b.js` — which the `not_callable` reason now blocks (SPEC §3.8 (8)).
    //
    // A parameter (the other common "not statically bound" ground) could not be used for this
    // representative: a parameter of the *calling* function is always also tracked by the
    // pre-existing JS/TS import-local guard (`js_ts_function_locals`), which drops it with
    // `UnknownName` before S1b-3b's R5 check is ever reached (same root cause as C-41's
    // disclosed `duplicate_declaration` gap below); a parameter of a *different*, unrelated
    // function is simply unbound at the call site (not `not_callable`) and correctly keeps
    // base. Both were probed and are recorded in the mutant/finding log.
    let a = (
        "a.js",
        "export function run(o) {\n  for (var f in o) {\n  }\n  return f();\n}\n",
    );
    let b = ("b.js", "export function f() {\n  return 2;\n}\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L4 f: drop LocalBindingUnproven"]);
}

#[test]
fn rep_unproven_at_r5_not_callable_enum_ts_drops() {
    let a = (
        "a.ts",
        "enum f { A, B }\nexport function run() {\n  return f();\n}\n",
    );
    let b = ("b.ts", "export function f() {\n  return 2;\n}\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L3 f: drop LocalBindingUnproven"]);
}

#[test]
fn rep_unproven_at_r5_not_callable_namespace_ts_drops() {
    let a = (
        "a.ts",
        "namespace f {\n  export const x = 1;\n}\nexport function run() {\n  return f();\n}\n",
    );
    let b = ("b.ts", "export function f() {\n  return 2;\n}\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L5 f: drop LocalBindingUnproven"]);
}

#[test]
fn rep_unbound_name_through_an_unrelated_parameter_keeps_base_at_r5() {
    // A parameter of a *different*, unrelated function (`helper`) is invisible from `run`'s
    // scope chain: the call site is correctly `unbound` (not `not_callable`), which is not
    // one of the three R5-narrowed reasons, so base's ordinary R5 free-function fallback
    // still binds it. This is the negative control for the row above.
    let a = (
        "a.js",
        "function helper(f) {\n  return 0;\n}\nexport function run() {\n  return f();\n}\n",
    );
    let b = ("b.js", "export function f() {\n  return 2;\n}\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L5 f: Exact free_single f@1-3"]);
}

// --- C-41: one R5 row per dropping reason (4 total with the alias row already pinned). ---
// `import_parse_recovery` is `c42_recovered_static_import_poisons_its_names_at_r5` above;
// `not_callable` is `rep_unproven_at_r5_not_callable_for_head_drops`/`..._enum_ts_drops`/
// `..._namespace_ts_drops` above. `duplicate_declaration` at R5 **is** reachable (round-1
// review fold, sol WRONG 3): `js_binding_scope_resolution_fold2_test.rs`'s
// `td_duplicate_for_heads_drop_at_r5` (two `for (var f of …)` heads, no same-file
// candidate) and `td_nested_arrow_parameter_drops_at_r5` (a parameter shadow whose call
// site's own caller is a nested arrow, not the shadowing function) both genuinely exercise
// this reason at R5, past every pre-existing guard. A captured catch parameter is the one
// representative that remains masked by the pre-existing JS/TS import-local guard
// (`td_captured_catch_parameter_still_masked_by_the_older_guard`, disclosed there, not
// claimed as a working construction). The original "could not be constructed" claim here
// was wrong; module-level duplicates and duplicates inside the caller's own function are
// still masked (routing through R4 instead, as `c8_same_line_collision_drops` and
// `c41_duplicate_declaration_drops_at_r4` below pin), but that is not the general case.

#[test]
fn c41_duplicate_declaration_drops_at_r4() {
    let a = (
        "a.js",
        "function f() { return 1; } function f() { return 2; }\n\
        export function g() {\n  return f();\n}\n",
    );
    check(a.1, &["js", "ts"], &["L3 f: drop LocalBindingUnproven"]);
}

// --- C-42: the clean twin (contrast to the recovered-import R4/R5 rows above), plus C181. ---

#[test]
fn c42_clean_import_twin_keeps_r4c() {
    // Unlike the recovered (malformed) import above, a clean import's name is `JsBinding::Import`
    // (not poisoned): base's R4c import-member route still binds it, unaffected by S1b-3b.
    let a = (
        "a.js",
        "import { h } from './m';\nexport function run() {\n  return h();\n}\n",
    );
    let m = ("m.js", "export function h() {\n  return 1;\n}\n");
    let cg = two_file_graph(a, m);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" h:"))
        .collect();
    assert_eq!(out, ["L3 h: Exact import_member h@1-3"]);
}

#[test]
fn c181_unicode_alias_e2e_drops() {
    // The same shape as 3a's own `c181_unicode_and_zwnj_names_stay_whole` unit fixture
    // (a combining-mark spelling before `as h`), pinned end to end: `h` still poisons.
    let src = "import { \u{e9}\u{301} as h };\nexport function h() {\n  return 1;\n}\n\
        export function run() {\n  return h();\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L6 h: drop LocalBindingUnproven"]);
}

// --- C-44: a positive control (a route S1b-3b does not touch stays unaffected). ---

#[test]
fn c44_positive_control_import_member_route_unaffected() {
    let a = (
        "a.js",
        "import { f } from './lib';\nexport function run() {\n  return f();\n}\n",
    );
    let lib = ("lib.js", "export function f() {\n  return 1;\n}\n");
    let cg = two_file_graph(a, lib);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L3 f: Exact import_member f@1-3"]);
}

// --- Alias pins C174-C177, C182-C184, named individually (SPEC §3.8 (11), addendum 9/9b). ---

#[test]
fn c174_hook_returned_destructured_call_value_is_alias() {
    let src = "export function run() {\n  const { t } = useI18n();\n  return t();\n}\n";
    let cg = graph("a.js", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

#[test]
fn c175_member_expression_value_is_alias() {
    let src = "export function run() {\n  const make = ctx.make;\n  return make();\n}\n";
    let cg = graph("a.js", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

#[test]
fn c176_destructure_from_identifier_is_alias() {
    let src = "export function run() {\n  const { g } = ctx;\n  return g();\n}\n";
    let cg = graph("a.js", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

#[test]
fn c177_alias_at_r4_keeps_local_def_named() {
    let src = "function t() {\n  return 1;\n}\n\
        export function outer() {\n  const t = ctx.make;\n  return t();\n}\n";
    check(src, &["js", "ts"], &["L6 t: Exact local_def t@1-3"]);
}

#[test]
fn c182_number_literal_is_not_callable() {
    let src = "export function f() {\n  return 1;\n}\n\
        export function run() {\n  const f = 0;\n  return f();\n}\n";
    check(src, &["js", "ts"], &["L6 f: drop LocalBindingUnproven"]);
}

#[test]
fn c183_object_literal_whose_every_member_is_nofn_is_not_callable() {
    let src = "export function run() {\n  const { k } = { k: 1 };\n  return k();\n}\n\
        export function k() {\n  return 1;\n}\n";
    check(src, &["js", "ts"], &["L3 k: drop LocalBindingUnproven"]);
}

#[test]
fn c184_array_with_a_non_nofn_element_is_alias_not_not_callable() {
    // `[1, g]` is not wholly NoFn (`g` is not a literal): the whole array literal is "any
    // other value", so `m` is Alias (keeps base), not `not_callable` (would drop).
    let src = "export function g() {\n  return 1;\n}\n\
        export function run() {\n  const [m] = [1, g];\n  return m();\n}\n";
    let cg = graph("a.js", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

// --- C-5's third twin: `memo` impostor (forwardRef and useCallback impostors are above). ---

#[test]
fn c5_memo_impostor_without_react_provenance_keeps_base() {
    let src = "function memo(x) {\n  return x;\n}\n\
        const Island = memo((props) => {\n  return null;\n});\n\
        export function App() {\n  return Island({});\n}\n";
    let cg = graph("a.jsx", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 1);
}
