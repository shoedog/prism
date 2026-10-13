//! Slice L: a JS/TS `const` alias is followed to what it names. Each row builds a small repo,
//! resolves every call site, and pins the target, kind and confidence. The alias is spelled
//! differently from the callable it names, and a same-named decoy (same file or another file)
//! is present whenever the old by-name search could have picked it.
use super::js_binding_scope_resolution_test::sites;
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use std::collections::BTreeMap;

fn graph_n(files: &[(&str, &str)]) -> CallGraph {
    let mut parsed = BTreeMap::new();
    for (path, src) in files {
        let lang = Language::from_path(path).unwrap();
        parsed.insert(
            (*path).to_string(),
            ParsedFile::parse(path, src, lang).unwrap(),
        );
    }
    CallGraph::build(&parsed)
}

/// The sites of `caller` only, so callee files may hold anything.
fn caller_sites(files: &[(&str, &str)]) -> Vec<String> {
    sites(&graph_n(files))
}

const CALLEE: &str = "export function target() {\n  return 1;\n}\n\
    export function Comp() {\n  return null;\n}\n";
const DECOY: &str =
    "export function f() {\n  return 2;\n}\nexport function C() {\n  return 3;\n}\n";

#[test]
fn identifier_alias_of_a_renamed_import_binds_the_import_not_the_same_named_decoy() {
    for ext in ["js", "ts"] {
        let callee = format!("callee.{ext}");
        let caller = format!("caller.{ext}");
        let decoy = format!("decoy.{ext}");
        let src = "import { target as imp } from './callee';\nexport function run() {\n  \
            const f = imp;\n  return f();\n}\n";
        let got = caller_sites(&[(&callee, CALLEE), (&caller, src), (&decoy, DECOY)]);
        assert_eq!(got, ["L4 f: Exact import_member target@1-3"], ".{ext}");
    }
}

#[test]
fn identifier_alias_beats_a_same_file_function_of_the_aliases_name() {
    let src = "import { target } from './callee';\nfunction f() {\n  return 0;\n}\n\
        export function run() {\n  const f = target;\n  return f();\n}\n";
    let got = caller_sites(&[("callee.js", CALLEE), ("caller.js", src)]);
    assert_eq!(got, ["L7 f: Exact import_member target@1-3"]);
}

#[test]
fn alias_of_a_same_file_function_binds_it_by_span_and_not_the_decoy() {
    let src = "function real() {\n  return 1;\n}\nexport function run() {\n  const f = real;\n  \
        return f();\n}\n";
    let got = caller_sites(&[("decoy.js", DECOY), ("caller.js", src)]);
    assert_eq!(got, ["L6 f: Exact local_def real@1-3"]);
}

#[test]
fn namespace_member_alias_binds_the_member_through_the_namespace_route() {
    let src = "import * as N from './callee';\nexport function run() {\n  const f = N.target;\n  \
        return f();\n}\n";
    let got = caller_sites(&[
        ("callee.ts", CALLEE),
        ("caller.ts", src),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(got, ["L4 f: Exact import_qualified target@1-3"]);
}

#[test]
fn flat_pattern_over_a_namespace_binds_shorthand_and_renamed_keys() {
    let src =
        "import * as N from './callee';\nexport function run() {\n  const { target } = N;\n  \
        const { target: f } = N;\n  return [target(), f()];\n}\n";
    let got = caller_sites(&[
        ("callee.ts", CALLEE),
        ("caller.ts", src),
        ("decoy.ts", DECOY),
    ]);
    assert_eq!(
        got,
        [
            "L5 f: Exact import_qualified target@1-3",
            "L5 target: Exact import_qualified target@1-3",
        ]
    );
}

#[test]
fn jsx_element_sites_follow_aliases_too() {
    let src = "import * as N from './callee';\nimport { Comp } from './callee';\n\
        export function App() {\n  const C = N.Comp;\n  const D = Comp;\n  \
        return <div><C /><D /></div>;\n}\n";
    let got = caller_sites(&[
        ("callee.jsx", CALLEE),
        ("caller.jsx", src),
        ("decoy.jsx", DECOY),
    ]);
    // (the graph also holds an unrelated derived `Comp` site; the two element sites are pinned)
    for want in [
        "L6 C: Exact import_qualified Comp@4-6",
        "L6 D: Exact import_member Comp@4-6",
    ] {
        assert!(got.iter().any(|s| s == want), "{want} in {got:?}");
    }
}

#[test]
fn a_wrapped_terminal_binds_from_jsx_only() {
    let src = "import { memo } from 'react';\nconst W = memo(() => null);\nconst A = W;\n\
        export function App() {\n  A();\n  return <A />;\n}\n";
    let got = caller_sites(&[("a.jsx", src)]);
    assert!(
        got.contains(&"L5 A: drop WrappedExportNonJsx".to_string())
            && got.contains(&"L6 A: Exact local_def W@2-2".to_string()),
        "{got:?}"
    );
}

#[test]
fn an_import_local_with_no_candidate_drops_and_never_reaches_the_name_rungs() {
    // `function f` is a same-file decoy of the inner alias `f` (R4 would bind it by name).
    let src = "import { missing } from './callee';\nimport ext from 'some-package';\n\
        function f() {\n  return 0;\n}\n\
        export function run() {\n  const f = missing;\n  const g = ext;\n  return [f(), g()];\n}\n";
    let decoys = "export function f() {\n  return 2;\n}\nexport function g() {\n  return 3;\n}\n";
    let got = caller_sites(&[
        ("callee.js", CALLEE),
        ("caller.js", src),
        ("decoy.js", decoys),
    ]);
    assert_eq!(
        got,
        ["L9 f: drop UnknownName", "L9 g: drop UnknownName"],
        "decoys f/g exist repo-wide but the aliases name imports"
    );
}

#[test]
fn a_namespace_member_with_no_candidate_drops_and_never_reaches_the_name_rungs() {
    let src = "import * as N from './callee';\nimport * as P from 'some-package';\n\
        function f() {\n  return 0;\n}\n\
        export function run() {\n  const f = N.ghost;\n  const g = P.g;\n  return [f(), g()];\n}\n";
    let decoys = "export function ghost() {\n  return 2;\n}\n\
        export function g() {\n  return 3;\n}\n";
    let got = caller_sites(&[
        ("callee.js", CALLEE),
        ("caller.js", src),
        ("decoy.js", decoys),
    ]);
    assert_eq!(
        got,
        ["L9 f: drop ImportExternal", "L9 g: drop ImportExternal"],
        "same-file and repo-wide decoys must not bind"
    );
}

#[test]
fn an_unfollowable_alias_keeps_the_base_outcome() {
    // `let`, an object member and a call result are not alias links: base behavior is
    // unchanged by this slice (the lexical guard drops the by-name hit on the decoys).
    let src = "import { target } from './callee';\nexport function run(o) {\n  let a = target;\n  \
        const b = o.target;\n  const c = make();\n  return [a(), b(), c()];\n}\n";
    let decoys = "export function a() {\n  return 1;\n}\nexport function b() {\n  return 2;\n}\n\
        export function c() {\n  return 3;\n}\nexport function make() {\n  return 4;\n}\n";
    let got = caller_sites(&[
        ("callee.js", CALLEE),
        ("caller.js", src),
        ("decoy.js", decoys),
    ]);
    for name in ["a", "b", "c"] {
        let row = format!("L6 {name}: drop UnknownName");
        assert!(got.contains(&row), "{row} in {got:?}");
    }
}

#[test]
fn an_exported_alias_stays_unproven_across_files() {
    // D4 is unchanged: `export const x = f;` is still an unproven local export, so a consumer
    // of `x` gets what it got before this slice.
    let producer = "function f() {\n  return 1;\n}\nexport const x = f;\n";
    let consumer = "import { x } from './producer';\nexport function run() {\n  return x();\n}\n";
    let got = caller_sites(&[("producer.js", producer), ("consumer.js", consumer)]);
    assert_eq!(got, ["L3 x: drop UnknownName"]);
}

#[test]
fn an_alias_of_a_namespace_import_local_is_not_a_member_import() {
    // `X` is a program-scope import, but a namespace, not a member: no candidate, and the
    // same-file decoy `a` must not bind.
    let src = "import * as X from './callee';\nfunction a() {\n  return 0;\n}\n\
        export function run() {\n  const a = X;\n  return a();\n}\n";
    let got = caller_sites(&[("callee.js", CALLEE), ("caller.js", src)]);
    assert_eq!(got, ["L7 a: drop UnknownName"]);
}

#[test]
fn an_alias_of_a_tsconfig_paths_import_binds_through_the_paths_route() {
    use super::js_paths_common::{config, fixture, graph, outcome, write};
    for ext in ["jsx", "tsx"] {
        let d = fixture(
            ext,
            "@lib",
            &config(serde_json::json!({"@lib": ["lib/real"]})),
        );
        write(
            d.path(),
            &format!("app.{ext}"),
            "import { real as picked } from '@lib';\nexport function run() {\n  \
             const viaAlias = picked;\n  viaAlias();\n}\n",
        );
        let row = outcome(&graph(d.path()), &format!("app.{ext}"), "viaAlias");
        let targets = row["resolved_targets"].as_array().unwrap();
        assert_eq!(targets.len(), 1, "{row}");
        assert_eq!(targets[0]["function_id"]["file"], format!("lib/real.{ext}"));
        assert_eq!(targets[0]["confidence"], "exact", "{row}");
        assert_eq!(targets[0]["kind"], "import_member", "{row}");
    }
}

#[test]
fn a_namespace_member_alias_without_an_export_proof_grades_like_the_qualified_call() {
    // `export const renamed = target;` makes the callee's namespace proof incomplete, so the
    // route falls back to the stem match exactly as `N.target()` does: Exact import_qualified.
    let callee = format!("{CALLEE}export const renamed = target;\n");
    let src = "import * as N from './callee';\nexport function run() {\n  const f = N.target;\n  \
        return [f(), N.target()];\n}\n";
    let got = caller_sites(&[
        ("callee.js", &callee),
        ("caller.js", src),
        ("decoy.js", DECOY),
    ]);
    assert_eq!(
        got,
        [
            "L4 f: Exact import_qualified target@1-3",
            "L4 target: Exact import_qualified target@1-3",
        ]
    );
}
