//! S1b-3b round-2 review fold (PR #334 follow-up; both reviewers found a round-1 regression).
//! Reviews: `~/prism-evidence/s1b/reviews/impl-s1b3b-r2-{opus,sol61}.md`.
//!
//! W1 (both reviewers, MATERIAL): round 1's F1 wrapper-provenance check
//! (`js_ts_classify_call` calling `js_ts_site_binding` on the wrapper callee for *every*
//! call-valued declarator, unconditionally) recurses without bound and aborts the process
//! with a stack overflow when the callee's nearest declaration is the same declarator (or a
//! cycle of them) — e.g. `const f = f(() => 1)`. Fixed by (1) checking spelling/shape
//! (`js_ts_wrapped_export`) first, lazily, and (2) replacing the classifying provenance check
//! with `js_ts_wrapper_provenance`, which only walks to the nearest declaration and inspects
//! it structurally (never calls `js_ts_classify`), so it cannot re-enter.
//!
//! Every row below was captured against the real implementation with a throwaway probe
//! harness (deleted before commit); the crash inputs were also run against round-1 head
//! (`8e14ea8a`) to capture the RED evidence (exit 134, stack overflow) before the fix.
use super::js_binding_scope_resolution_test::{check, sites, two_file_graph};

// ---------------------------------------------------------------------------------------
// The six required regression rows. Each must complete (no process abort) and keep base's
// rows exactly, in both `.jsx` and `.tsx`.
// ---------------------------------------------------------------------------------------

#[test]
fn w1_self_call_nested_does_not_crash_and_keeps_base() {
    let src = "function run(){ const f = f(() => 1); return f(); }\n";
    check(
        src,
        &["jsx", "tsx"],
        &["L1 f: Exact local_def f@1-1", "L1 f: Exact local_def f@1-1"],
    );
}

#[test]
fn w1_self_call_module_scope_d4_route_does_not_crash_and_keeps_base() {
    let src = "export const f = f(() => 1);\nexport function run(){ return f(); }\n";
    check(src, &["jsx", "tsx"], &["L2 f: Exact local_def f@1-1"]);
}

#[test]
fn w1_mutual_self_call_does_not_crash_and_keeps_base() {
    let src = "function run(){ const a = b(() => 1), b = a(() => 2); return a(); }\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L1 a: Exact local_def a@1-1",
            "L1 a: Exact local_def a@1-1",
            "L1 b: Exact local_def b@1-1",
        ],
    );
}

#[test]
fn w1_member_self_call_does_not_crash_and_keeps_base() {
    let src = "function run(){ const f = f.x(() => 1); return f(); }\n";
    check(
        src,
        &["jsx", "tsx"],
        &["L1 f: Exact local_def f@1-1", "L1 x: drop UnknownName"],
    );
}

#[test]
fn w1_no_arg_self_call_does_not_crash_and_keeps_base() {
    let src = "function run(){ const f = f(); return f(); }\n";
    check(
        src,
        &["jsx", "tsx"],
        &["L1 f: drop UnknownName", "L1 f: drop UnknownName"],
    );
}

#[test]
fn w1_local_use_callback_self_reference_does_not_crash_and_keeps_base() {
    // The React-spelling shadow itself is self-referential: `useCallback` is both the name
    // being declared and the callee of its own initializer. `js_ts_wrapper_provenance` walks
    // from the callee identifier to its nearest declaration (itself), finds it is not an
    // `import_statement`, and falls through to M1 (`MayCall`) — never classifying it.
    let src = "function run(){ const useCallback = useCallback(() => 1); return useCallback(); }\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L1 useCallback: Exact local_def useCallback@1-1",
            "L1 useCallback: Exact local_def useCallback@1-1",
        ],
    );
}

// ---------------------------------------------------------------------------------------
// SMELL (both reviewers): the round-1 catch-parameter disclosure
// (`td_captured_catch_parameter_still_masked_by_the_older_guard`) is specific to the
// in-arrow construction, not general. With the `try`/`catch` *outside* the arrow, the catch
// parameter's call site reaches the new R5 `not_callable` drop — corrected here with the
// outer-catch construction and a remote candidate making the R5 reach observable.
// ---------------------------------------------------------------------------------------

#[test]
fn td_catch_parameter_outside_the_arrow_reaches_r5_drop() {
    let a = (
        "a.jsx",
        "export function run(){\n  try {} catch (f) {\n    const cb = () => f();\n    return cb();\n  }\n}\n",
    );
    let lib = ("lib.jsx", "export function f(){ return 1; }\n");
    let cg = two_file_graph(a, lib);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L3 f: drop LocalBindingUnproven"]);
}
