//! S1b-3b round-1 review fold (PR #334, both reviewers FIX; reviews
//! `~/prism-evidence/s1b/reviews/impl-s1b3b-r1-{opus,sol61}.md`). F1/F2 are the two production
//! fixes; T-a..T-g are the mandatory coverage gaps. Every expectation here was captured against
//! the real implementation with a throwaway probe harness (deleted before commit), the same
//! discipline as the sub-slice's original test files.
use super::js_binding_scope_resolution_test::{check, graph, sites, two_file_graph};
use prism::ast::ParsedFile;
use prism::call_graph::JsLocalBinding;
use prism::cpg::CodePropertyGraph;
use prism::cpg_cache::{self, CacheResult};
use prism::languages::Language;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------------------
// F1 (Opus WRONG 1 / sol WRONG 1): local wrapper admission must resolve the wrapper callee's
// (or `React.x`'s namespace/default object's) own nearest binding; it is a React wrapper only
// when that binding is the program-scope admitted import. A parameter, local `const`, block
// `const` or namespace-object parameter shadow keeps the ordinary M1 may-call class (base).
// ---------------------------------------------------------------------------------------

#[test]
fn f1_param_shadow_all_three_wrappers_keeps_base() {
    // sol's exact scenario: all three wrappers shadowed by same-named parameters.
    let src = "import { useCallback, forwardRef, memo } from 'react';\n\
        export function run(useCallback, forwardRef, memo) {\n  const cb = useCallback(() => 1, []);\n  \
        const Island = forwardRef(() => 2);\n  const M = memo(() => 3);\n  cb(); Island(); M();\n}\n\
        export function decoy() {\n  const cb = () => 4;\n  const Island = () => 5;\n  const M = () => 6;\n  return 0;\n}\n";
    let cg_jsx = graph("a.jsx", src);
    assert_eq!(
        sites(&cg_jsx),
        [
            "L3 useCallback: drop UnknownName",
            "L4 forwardRef: drop UnknownName",
            "L5 memo: drop UnknownName",
            "L6 Island: Exact local_def Island@4-4, Exact local_def Island@10-10",
            "L6 M: Exact local_def M@5-5, Exact local_def M@11-11",
            "L6 cb: Exact local_def cb@3-3, Exact local_def cb@9-9",
        ]
    );
    let stats = prism::navigation::queries::call_stats(&cg_jsx);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 3);
    let cg_tsx = graph("a.tsx", src);
    assert_eq!(sites(&cg_tsx), sites(&cg_jsx));
}

#[test]
fn f1_local_const_shadow_keeps_base() {
    let src = "import { useCallback } from 'react';\n\
        function h(){ return 1; }\n\
        export function App() {\n  const useCallback = (fn) => fn;\n  const h = useCallback(() => 2);\n  return h();\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L5 useCallback: Exact local_def useCallback@4-4",
            "L6 h: Exact local_def h@2-2, Exact local_def h@5-5",
        ],
    );
}

#[test]
fn f1_block_const_shadow_keeps_base() {
    let src = "import { forwardRef } from 'react';\n\
        function Foo(){ return 1; }\n\
        export function App() {\n  if (true) {\n    const forwardRef = (fn) => fn;\n    \
        const Foo = forwardRef(() => 2);\n    return Foo();\n  }\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L6 forwardRef: Exact local_def forwardRef@5-5",
            "L7 Foo: Exact local_def Foo@2-2, Exact local_def Foo@6-6",
        ],
    );
}

#[test]
fn f1_namespace_object_param_shadow_keeps_base() {
    let src = "import * as React from 'react';\n\
        function Foo(){ return 1; }\n\
        export function App(React) {\n  const Foo = React.memo(() => 2);\n  return Foo();\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L4 memo: drop UnknownName",
            "L5 Foo: Exact local_def Foo@2-2, Exact local_def Foo@4-4",
        ],
    );
}

#[test]
fn f1_clean_import_twin_stays_callable_non_jsx() {
    // No shadow anywhere: the wrapper is still admitted exactly as before the fold.
    let src = "import { useCallback } from 'react';\n\
        export function App() {\n  const cb = useCallback(() => 1, []);\n  return cb();\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L3 useCallback: drop UnknownName",
            "L4 cb: Exact local_def cb@3-3",
        ],
    );
}

#[test]
fn f1_clean_import_twin_stays_callable_jsx_and_wrapped_non_jsx() {
    let src = "import { forwardRef } from 'react';\n\
        export function App() {\n  const Foo = forwardRef(() => null);\n  return <Foo/>;\n}\n\
        export function App2() {\n  const Foo = forwardRef(() => null);\n  return Foo();\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L3 forwardRef: drop UnknownName",
            "L4 Foo: Exact local_def Foo@3-3",
            "L7 forwardRef: drop UnknownName",
            "L8 Foo: drop WrappedExportNonJsx",
        ],
    );
}

// ---------------------------------------------------------------------------------------
// F2 (sol WRONG 2): a written import binding keeps base under the owner's M2 ruling (every
// written binding keeps base, whatever its kind). D4/R4c are unaffected (pinned below).
// ---------------------------------------------------------------------------------------

#[test]
fn f2_written_namespace_import_keeps_base() {
    let a = (
        "a.jsx",
        "import * as f from './m';\nfunction holder(){\n  function f(){return 1}\n  return f;\n}\n\
        export function run(){\n  f=g;\n  return f();\n}\n",
    );
    let m = ("m.jsx", "export function g(){ return 2; }\n");
    let cg = two_file_graph(a, m);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L8 f: Exact local_def f@3-3"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 1);
}

#[test]
fn f2_unwritten_namespace_import_drops() {
    let a = (
        "a.jsx",
        "import * as f from './m';\nfunction holder(){\n  function f(){return 1}\n  return f;\n}\n\
        export function run(){\n  return f();\n}\n",
    );
    let m = ("m.jsx", "export function g(){ return 2; }\n");
    let cg = two_file_graph(a, m);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L7 f: drop LocalBindingUnproven"]);
}

#[test]
fn f2_named_import_route_unaffected_by_write() {
    // R4c (import-member) decides a named import regardless of M2: written and unwritten
    // give the identical row, pinning that F2 did not touch D4/R4c.
    let a_written = (
        "a.jsx",
        "import { f } from './m';\nfunction holder(){\n  function f(){return 1}\n  return f;\n}\n\
        export function run(){\n  f=g;\n  return f();\n}\n",
    );
    let a_unwritten = (
        "a.jsx",
        "import { f } from './m';\nfunction holder(){\n  function f(){return 1}\n  return f;\n}\n\
        export function run(){\n  return f();\n}\n",
    );
    let m = ("m.jsx", "export function f(){ return 2; }\n");
    let written = sites(&two_file_graph(a_written, m));
    let unwritten = sites(&two_file_graph(a_unwritten, m));
    let want = vec!["L8 f: Exact import_member f@1-1".to_string()];
    assert_eq!(
        written
            .iter()
            .filter(|s| s.contains(" f:"))
            .cloned()
            .collect::<Vec<_>>(),
        want
    );
    assert_eq!(
        unwritten
            .iter()
            .filter(|s| s.contains(" f:"))
            .map(|s| s.replacen("L7", "L8", 1))
            .collect::<Vec<_>>(),
        want
    );
}

#[test]
fn f2_d4_import_alias_export_written_unaffected() {
    // D4 (TS `import f = require()`, exported): never reaches the site route's write check
    // (`local_route` is false there), so a written `import f = require()` export keeps the
    // landed `UnprovenLocal` / `not_callable` refusal unchanged.
    let src = "import f = require('./m');\nf = g;\nexport { f };\n";
    let cg = graph("a.ts", src);
    assert_eq!(
        cg.js_ts_exports
            .get("a.ts")
            .map(|f| f.local_export_refusals.get("not_callable").copied()),
        Some(Some(1))
    );
}

// ---------------------------------------------------------------------------------------
// SMELL S1 (Opus): a serde round trip and a cold-vs-cache-hit equality check for
// `CallSite.local_binding`, mirroring 2b's `b11_facts_serde_and_cpg_cache_full_hit` pattern.
// ---------------------------------------------------------------------------------------

#[test]
fn s1_local_binding_serde_round_trip() {
    let src = "function f() {\n  return 1;\n}\nexport function g() {\n  return f();\n}\n";
    let cg = graph("a.js", src);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "f")
        .expect("one call site");
    assert!(matches!(site.local_binding, JsLocalBinding::Callable(_)));
    let json = serde_json::to_string(site).unwrap();
    let back: prism::call_graph::CallSite = serde_json::from_str(&json).unwrap();
    assert_eq!(back.local_binding, site.local_binding);
    // A cache row persisted before S1b-3b (no `local_binding` field at all) still
    // deserializes, with the field defaulting to `Unchecked` (`#[serde(default)]`).
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("local_binding")
        .expect("local_binding field present in the serialized site");
    let old: prism::call_graph::CallSite = serde_json::from_value(value).unwrap();
    assert_eq!(old.local_binding, JsLocalBinding::Unchecked);
}

#[test]
fn s1_local_binding_cold_vs_cpg_cache_hit() {
    let lib = "function f() {\n  return 1;\n}\nexport { f };\n";
    let app = "import { f } from './lib';\nexport function run() {\n  return f();\n}\n";
    let sources: BTreeMap<String, String> = [("lib.jsx", lib), ("app.jsx", app)]
        .map(|(p, s)| (p.into(), s.into()))
        .into();
    let parse = |p: &String, s: &String| ParsedFile::parse(p, s, Language::from_path(p).unwrap());
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(p, s)| (p.clone(), parse(p, s).unwrap()))
        .collect();
    let cold = CodePropertyGraph::build(&files);
    let site = |cg: &prism::call_graph::CallGraph| {
        cg.calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "f")
            .unwrap()
            .local_binding
            .clone()
    };
    let cold_binding = site(&cold.call_graph);
    assert!(
        matches!(cold_binding, JsLocalBinding::Unproven(_))
            || cold_binding == JsLocalBinding::Unchecked
    );
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full CPG cache hit");
    };
    assert_eq!(site(&hit.call_graph), cold_binding);
}
