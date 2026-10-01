//! S1b-3b (SPEC §3.6, §3.8 (8)(9), §7 C-1..C-44, §9): the call-site wiring. Every call site's
//! static lexical binding (`CallSite.local_binding`) decides, before any import or same-name
//! lookup, what `resolve_call_site_full` produces. Each row names the exact outcome for every
//! call site in a single file (both grammars unless marked TS-only); row ids are the SPEC's.
//! Every expectation below was captured against the landed implementation with a throwaway
//! probe harness (see the handback) before being pinned here.
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

fn graph(path: &str, src: &str) -> CallGraph {
    let lang = Language::from_path(path).unwrap();
    let mut files = BTreeMap::new();
    files.insert(
        path.to_string(),
        ParsedFile::parse(path, src, lang).unwrap(),
    );
    CallGraph::build(&files)
}

fn two_file_graph(a: (&str, &str), b: (&str, &str)) -> CallGraph {
    let mut files = BTreeMap::new();
    for (path, src) in [a, b] {
        let lang = Language::from_path(path).unwrap();
        files.insert(
            path.to_string(),
            ParsedFile::parse(path, src, lang).unwrap(),
        );
    }
    CallGraph::build(&files)
}

/// Every call site in the graph, as `L<line> <callee>: <outcome>`.
fn sites(cg: &CallGraph) -> Vec<String> {
    let mut out = Vec::new();
    for site in cg.calls.values().flatten() {
        let outcome = cg.resolve_call_site_full(site);
        let shown = match outcome.drop {
            Some(reason) => format!("drop {reason:?}"),
            None => outcome
                .resolved
                .iter()
                .map(|r| {
                    let t = r.target;
                    format!(
                        "{:?} {} {}@{}-{}",
                        r.confidence,
                        r.kind.as_str(),
                        t.name,
                        t.start_line,
                        t.end_line
                    )
                })
                .collect::<Vec<_>>()
                .join(", "),
        };
        out.push(format!("L{} {}: {shown}", site.line, site.callee_name));
    }
    out.sort();
    out
}

/// Table runner: `src` with `{ext}` substituted for each of `exts`.
fn check(src: &str, exts: &[&str], expected: &[&str]) {
    for ext in exts {
        let path = format!("a.{ext}");
        assert_eq!(sites(&graph(&path, src)), expected, ".{ext}");
    }
}

// ---------------------------------------------------------------------------------------
// C-1 (C63, C106): a non-JSX call of a same-file wrapped component drops WrappedExportNonJsx;
// a JSX element of the same binding stays Exact.
// ---------------------------------------------------------------------------------------

#[test]
fn c1_non_jsx_call_of_same_file_wrapped_component_drops() {
    let src = "import { forwardRef } from 'react';\n\
        const Island = forwardRef((props, ref) => {\n  return null;\n});\n\
        export function App() {\n  return Island({});\n}\n\
        export function App2() {\n  return <Island/>;\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L6 Island: drop WrappedExportNonJsx",
            "L9 Island: Exact local_def Island@2-4",
        ],
    );
}

// ---------------------------------------------------------------------------------------
// Six outcome representatives.
// ---------------------------------------------------------------------------------------

#[test]
fn rep_callable_binds_by_registered_span() {
    let src = "function f() {\n  return 1;\n}\nfunction g() {\n  return f();\n}\n";
    check(src, &["js", "ts"], &["L5 f: Exact local_def f@1-3"]);
}

#[test]
fn rep_may_call_written_binding_keeps_base() {
    // A written binding (M2) keeps base Exact local_def; the write never erases the R4 hit.
    let src = "function f() {\n  return 1;\n}\nfunction g() {\n  f = h;\n  return f();\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L6 f: Exact local_def f@1-3"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 1);
}

#[test]
fn rep_may_call_alias_keeps_base() {
    // `t` aliases a call with no function argument (owner 2026-09-29): keeps base (unbound,
    // since no same-file or cross-file `t` function exists), counted `alias`.
    let src = "function g() {\n  const t = compute();\n  return t();\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(
        sites(&cg),
        ["L2 compute: drop UnknownName", "L3 t: drop UnknownName"]
    );
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

#[test]
fn rep_unproven_at_r4_param_shadow_drops() {
    // The Tier-A fixture's scenario: `f` is a parameter at the call site, and a same-file
    // declarator named `f` exists elsewhere (R4's `local` candidate set is non-empty).
    let src = "export function run(f) {\n  return f();\n}\n\
        export function holder() {\n  const f = () => 1;\n  return f;\n}\n";
    check(src, &["js", "ts"], &["L2 f: drop LocalBindingUnproven"]);
}

#[test]
fn rep_unproven_at_r4_unbound_name_out_of_scope_candidate_drops() {
    // No same-file candidate is in scope at the call site (`f` is unbound there), but a
    // same-file function named `f` exists elsewhere (nested, out of scope): R4's `local` is
    // non-empty, so the `Unproven("unbound")` binding drops rather than binding it (C88, C93).
    let src = "function outer() {\n  function f() {\n    return 1;\n  }\n  return f();\n}\n\
        export function run() {\n  return f();\n}\n";
    check(
        src,
        &["js", "ts"],
        &[
            "L5 f: Exact local_def f@2-4",
            "L8 f: drop LocalBindingUnproven",
        ],
    );
}

#[test]
fn rep_position_unknown_strictness_keeps_base() {
    // A plain `.js` script (no module marker, no directive, not `.cjs`): strictness Unknown.
    // The Annex-B hoisted block function is unproven at its position (Option K), which keeps
    // base (the ordinary R4 same-file candidate still binds).
    let src = "function outer() {\n  if (true) {\n    function f() {\n      return 1;\n    }\n  }\n  return f();\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L7 f: Exact local_def f@3-5"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(
        stats["local_binding_unchecked_position"]["annex_b_strictness"],
        1
    );
}

// ---------------------------------------------------------------------------------------
// C-8 (C95): a same-line collision between two declarations refuses `duplicate_declaration`,
// which drops at R4 (a same-file candidate is in scope).
// ---------------------------------------------------------------------------------------

#[test]
fn c8_same_line_collision_drops() {
    let src = "function f() { return 1; } function f() { return 2; }\n\
        export function g() {\n  return f();\n}\n";
    check(src, &["js", "ts"], &["L3 f: drop LocalBindingUnproven"]);
}

// ---------------------------------------------------------------------------------------
// C-18 (C134): a generator function expression value is "any other value" under the closed
// NoFn class (SPEC §3.8 (11)), so it classifies Alias, not `not_callable`; either way it
// keeps base (non-goal, unchanged: no edge before or after S1b-3b).
// ---------------------------------------------------------------------------------------

#[test]
fn c18_generator_expression_value_is_not_a_goal() {
    let src = "const f = function* () {\n  yield 1;\n};\nexport function g() {\n  return f();\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L5 f: drop UnknownName"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

// ---------------------------------------------------------------------------------------
// C-41 (C170): a block-nested `const { f } = o` keeps base (alias, an identifier/member value
// never drops); C-44's duplicate-declaration representative covers the R5 reason directly.
// ---------------------------------------------------------------------------------------

#[test]
fn c41_block_nested_destructured_alias_keeps_base() {
    let src = "export function outer() {\n  const { f } = globalThis;\n  return f();\n}\n\
        export function f() {\n  return 1;\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L3 f: Exact local_def f@5-7"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

// ---------------------------------------------------------------------------------------
// C-42 (C173, C179, C181): a recovered static import poisons the names it spells; a recovered
// `import.meta`/`import(...)` does not.
// ---------------------------------------------------------------------------------------

#[test]
fn c42_recovered_static_import_poisons_its_names() {
    // The fixture 3a's own unit tests pin (`js_binding_recovery_tests.rs` C173): no `from`
    // clause, so the malformed specifier recovers as a top-level ERROR starting `import`.
    let src = "import { \"\\u{47}\\u{47}\" as h };\nexport function h() {\n  return 1;\n}\n\
        export function run() {\n  return h();\n}\n";
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L6 h: drop LocalBindingUnproven"]);
}

#[test]
fn c42_recovered_static_import_poisons_its_names_at_r5() {
    // Unlike the R4 row above, no same-file function competes (`local` is empty): the
    // recovered-import marker is not tracked by the pre-existing JS/TS import-local guard
    // (it is not a well-formed import, so `extract_import_bindings` never sees it), so this
    // genuinely exercises the new R5 drop for `import_parse_recovery` (SPEC §3.8 (8)).
    let a = (
        "a.js",
        "import { \"\\u{47}\\u{47}\" as h };\nexport function run() {\n  return h();\n}\n",
    );
    let b = ("b.js", "export function h() {\n  return 2;\n}\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" h:"))
        .collect();
    assert_eq!(out, ["L3 h: drop LocalBindingUnproven"]);
}

#[test]
fn c41_with_reason_keeps_base_at_r5() {
    // `with` is a closed non-goal refusal (Unproven("with")), which is NOT one of the three
    // reasons SPEC §3.8 (8) narrows the R5 drop to: it must keep base (Exact), proving the
    // drop is reason-scoped, not blanket (C-M34's target).
    let a = (
        "a.js",
        "export function run(o) {\n  with (o) {\n    return f();\n  }\n}\n",
    );
    let b = ("b.js", "export function f() {\n  return 2;\n}\n");
    let cg = two_file_graph(a, b);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains(" f:"))
        .collect();
    assert_eq!(out, ["L3 f: Exact free_single f@1-3"]);
}

#[test]
fn c42_import_meta_and_dynamic_import_are_not_recovered_imports() {
    let src = "function load() {\n  return (import.meta).url;\n}\n\
        function load2() {\n  return import('./x');\n}\n\
        export function run() {\n  load();\n  load2();\n}\n";
    let cg = graph("a.js", src);
    let out: Vec<String> = sites(&cg)
        .into_iter()
        .filter(|s| s.contains("load"))
        .collect();
    assert_eq!(
        out,
        [
            "L8 load: Exact local_def load@1-3",
            "L9 load2: Exact local_def load2@4-6",
        ]
    );
}

// ---------------------------------------------------------------------------------------
// C-43 (C171, TS-only): a name bound inside a namespace body is out of the module-level
// binding's reach (the site walk never enters the namespace body); a module-level function
// of the same name stays Exact.
// ---------------------------------------------------------------------------------------

#[test]
fn c43_namespace_body_binding_does_not_shadow_module_level() {
    let src = "namespace N {\n  export const { f } = ({} as any);\n}\n\
        function f() {\n  return 1;\n}\n\
        export function run() {\n  return f();\n}\n";
    check(src, &["ts"], &["L8 f: Exact local_def f@4-6"]);
}

// ---------------------------------------------------------------------------------------
// C-44: an `import f = M.g`/`require()` alias and an unbound name with only out-of-scope
// same-file functions drop (no edge); the same names keep base at R4c/R5.
// ---------------------------------------------------------------------------------------

#[test]
fn c44_import_alias_drops() {
    // `f` is poisoned by the pre-existing JS/TS import-local guard before S1b-3b's R4/R5
    // checks even run (`local` is empty: no same-file function competes), so the drop reason
    // is base's own `UnknownName`; either way the row is "no edge", matching the SPEC's
    // qualitative claim for C-44 and C141.
    let src = "import f = require('./m');\nexport function run() {\n  return f();\n}\n";
    check(src, &["ts"], &["L3 f: drop UnknownName"]);
}

#[test]
fn c44_unbound_name_with_only_out_of_scope_same_file_functions_drops() {
    let src = "function outer() {\n  function f() {\n    return 1;\n  }\n  return f();\n}\n\
        export function run() {\n  return f();\n}\n";
    check(
        src,
        &["js", "ts"],
        &[
            "L5 f: Exact local_def f@2-4",
            "L8 f: drop LocalBindingUnproven",
        ],
    );
}

// ---------------------------------------------------------------------------------------
// Alias pins: C174-C177, C182-C184 (identifier, member, call-without-function-argument,
// `await`, conditional, logical, `new`): every alias value keeps base.
// ---------------------------------------------------------------------------------------

#[test]
fn alias_identifier_value_keeps_base() {
    let src =
        "function f() {\n  return 1;\n}\nconst t = f;\nexport function g() {\n  return t();\n}\n";
    // `t` is unbound as a function anywhere, so base drops UnknownName either way; the
    // counter is what proves the alias classification fired (not a `not_callable` refusal).
    let cg = graph("a.js", src);
    assert_eq!(sites(&cg), ["L6 t: drop UnknownName"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

#[test]
fn alias_member_value_keeps_base() {
    let src = "function g() {\n  const ctx = { make: () => 1 };\n  const make = ctx.make;\n  \
        return make();\n}\n";
    let cg = graph("a.js", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

#[test]
fn alias_await_conditional_logical_new_values_keep_base() {
    let rows = [
        "const x = await f();\n",
        "const x = a ? f : g;\n",
        "const x = f || g;\n",
        "const x = new X();\n",
    ];
    for row in rows {
        let src = format!("export function run() {{\n  {row}  return x();\n}}\n");
        let cg = graph("a.js", &src);
        let stats = prism::navigation::queries::call_stats(&cg);
        assert_eq!(stats["local_binding_may_call"]["alias"], 1, "row {row:?}");
    }
}

#[test]
fn alias_at_r4_keeps_local_def() {
    // C177: an alias binding at a call site (the nearest declaration of `t` is an alias) that
    // shadows a different same-file function also named `t` keeps the ordinary R4 hit — the
    // alias classification only ever keeps base, never drops (the accepted value-flow cost).
    let src = "function t() {\n  return 1;\n}\n\
        export function outer() {\n  const t = ctx.make;\n  return t();\n}\n";
    check(src, &["js", "ts"], &["L6 t: Exact local_def t@1-3"]);
}

// ---------------------------------------------------------------------------------------
// C-5 impostor twins: a call to a name spelled like an admitted wrapper, but not imported
// from `"react"`, is never admitted by `js_ts_wrapped_export` (R6 P1's provenance check);
// it falls to the ordinary M1 call-with-function-argument class (`MayCall`), which keeps
// base exactly as before S1b-3 — the admitted-wrapper path is never reached.
// ---------------------------------------------------------------------------------------

#[test]
fn c5_impostor_wrapper_name_without_react_provenance_keeps_base() {
    let src = "function forwardRef(x) {\n  return x;\n}\n\
        const Island = forwardRef((props) => {\n  return null;\n});\n\
        export function App() {\n  return Island({});\n}\n";
    let cg = graph("a.jsx", src);
    assert_eq!(sites(&cg), ["L8 Island: Exact local_def Island@4-6"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 1);
}

#[test]
fn c5_use_callback_impostor_without_react_provenance_keeps_base() {
    let src = "function useCallback(fn) {\n  return fn;\n}\n\
        const cb = useCallback(() => 1);\n\
        export function run() {\n  return cb();\n}\n";
    let cg = graph("a.jsx", src);
    assert_eq!(sites(&cg), ["L6 cb: Exact local_def cb@4-4"]);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["may_call"], 1);
}

// ---------------------------------------------------------------------------------------
// `useCallback` (SPEC §3.7, §3.8 (9)): admitted on the call-site route, unwrapped.
// ---------------------------------------------------------------------------------------

#[test]
fn use_callback_binds_as_a_plain_unwrapped_callable() {
    let src = "import { useCallback } from 'react';\n\
        export function Comp() {\n  const cb = useCallback(() => 1, []);\n  return cb();\n}\n";
    check(
        src,
        &["jsx", "tsx"],
        &[
            "L3 useCallback: drop UnknownName",
            "L4 cb: Exact local_def cb@3-3",
        ],
    );
}

// ---------------------------------------------------------------------------------------
// C114: TS `using` keeps base (a call value is an alias, owner 2026-09-29).
// ---------------------------------------------------------------------------------------

#[test]
fn c114_using_declaration_call_value_keeps_base() {
    let src = "export function run() {\n  using h = res();\n  return h();\n}\n";
    let cg = graph("a.ts", src);
    let stats = prism::navigation::queries::call_stats(&cg);
    assert_eq!(stats["local_binding_may_call"]["alias"], 1);
}

// ---------------------------------------------------------------------------------------
// Round-2 scenarios (C185-C188 defaults, C189 trivia) pinned end to end.
// ---------------------------------------------------------------------------------------

#[test]
fn round2_destructuring_default_keeps_base() {
    let src = "export function f() {\n  return 1;\n}\n\
        export function run() {\n  const { missing: x = f } = ({} as any);\n  return x();\n}\n";
    check(src, &["ts"], &["L6 x: drop UnknownName"]);
}

#[test]
fn round2_comment_trivia_does_not_defeat_the_import_token_check() {
    let src = "import /* c */ . meta;\nexport function run() {\n  return h();\n}\n\
        export function h() {\n  return 1;\n}\n";
    check(src, &["js", "ts"], &["L3 h: Exact local_def h@5-7"]);
}
