//! P4: JS/TS export-fact modeling tests — destructured `require` bindings
//! (2b) and re-export chains/barrels (1e).
//!
//! Split from `js_export_test.rs` (600-line cap), which covers default
//! exports (1a), named export lists (1b), const-arrow exports (1c), and
//! CommonJS assignments (1d). Companion to `import_binding_test.rs`, which
//! owns the pre-existing R4c shadow/eligibility guards.

use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn files(pairs: &[(&str, &str, Language)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(path, src, lang)| {
            (
                path.to_string(),
                ParsedFile::parse(path, src, *lang).unwrap(),
            )
        })
        .collect()
}

fn resolve_kind(
    cg: &CallGraph,
    caller_file: &str,
    caller_name: &str,
    callee: &str,
) -> Vec<(ResolutionConfidence, ResolutionKind)> {
    let caller = cg
        .functions
        .get(caller_name)
        .and_then(|v| v.iter().find(|f| f.file == caller_file))
        .expect("caller fn not found");
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == callee))
        .expect("call site not found");
    let out = cg.resolve_call_site_full(site);
    out.resolved
        .iter()
        .map(|r| (r.confidence, r.kind))
        .collect()
}

// -----------------------------------------------------------------------
// 2b. Destructured `require` bindings
// -----------------------------------------------------------------------

#[test]
fn extract_destructured_require_binding() {
    let parsed = ParsedFile::parse(
        "app.js",
        "const { process } = require('./util');\n",
        Language::JavaScript,
    )
    .unwrap();
    let bindings = parsed.extract_import_bindings();
    let b = bindings
        .iter()
        .find(|b| b.local == "process")
        .expect("should have process binding");
    assert_eq!(b.module_path, "./util");
    assert_eq!(b.member.as_deref(), Some("process"));
    assert!(matches!(
        b.kind,
        prism::call_graph::ImportBindingKind::MemberImport
    ));
}

#[test]
fn extract_destructured_require_binding_renamed() {
    let parsed = ParsedFile::parse(
        "app.js",
        "const { process: p } = require('./util');\n",
        Language::JavaScript,
    )
    .unwrap();
    let bindings = parsed.extract_import_bindings();
    let b = bindings
        .iter()
        .find(|b| b.local == "p")
        .expect("should have p binding");
    assert_eq!(b.module_path, "./util");
    assert_eq!(b.member.as_deref(), Some("process"));
}

#[test]
fn destructured_require_resolves_import_member() {
    let fs = files(&[
        (
            "util.js",
            "function process() { return 1; }\nmodule.exports = { process };\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "const { process } = require('./util');\nfunction run() { process(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.js", "run", "process");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
}

// F1 (review-fix wave, codex BLOCKER 1): `let`/`var` destructured `require`
// bindings are extraction-ineligible entirely, not just when reassigned --
// this branch deliberately does NOT attempt assignment-rebind tracking; it
// fails closed at extraction instead (ADJUDICATION: const-only).

#[test]
fn let_destructured_require_binding_is_not_extracted() {
    let parsed = ParsedFile::parse(
        "app.js",
        "let { process } = require('./util');\n",
        Language::JavaScript,
    )
    .unwrap();
    let bindings = parsed.extract_import_bindings();
    assert!(bindings.iter().all(|b| b.local != "process"));
}

#[test]
fn var_destructured_require_binding_is_not_extracted() {
    let parsed = ParsedFile::parse(
        "app.js",
        "var { process } = require('./util');\n",
        Language::JavaScript,
    )
    .unwrap();
    let bindings = parsed.extract_import_bindings();
    assert!(bindings.iter().all(|b| b.local != "process"));
}

#[test]
fn let_destructured_require_without_reassignment_does_not_resolve_import_member() {
    // Plain `let { f } = require(...)`, never reassigned, still refuses --
    // that's the point of restricting extraction to `const`, not building
    // assignment-rebind tracking.
    let fs = files(&[
        (
            "util.js",
            "function process() { return 1; }\nmodule.exports = { process };\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "let { process } = require('./util');\nfunction run() { process(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.js", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
}

#[test]
fn let_destructured_require_reassigned_does_not_resolve_import_member() {
    // The exact F1 blocker scenario: `let { f } = require(...); f = localFn;`
    // must not mint a false Exact import_member edge to the require target.
    let fs = files(&[
        (
            "util.js",
            "function process() { return 1; }\nmodule.exports = { process };\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "function localFn() { return 2; }\nlet { process } = require('./util');\nprocess = localFn;\nfunction run() { process(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.js", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
}

// -----------------------------------------------------------------------
// 1e. Re-export chains / barrels (whole-program, via CallGraph)
// -----------------------------------------------------------------------

#[test]
fn reexport_named_from_resolves_import_member() {
    let fs = files(&[
        (
            "impl.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export { process } from './impl';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './index';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
    let caller = cg
        .functions
        .get("run")
        .and_then(|v| v.iter().find(|f| f.file == "app.ts"))
        .unwrap();
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == "process"))
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    // Bound to the REAL declaration in impl.ts, not the re-export in index.ts.
    assert_eq!(out.resolved[0].target.file, "impl.ts");
}

#[test]
fn barrel_depth_2_chain_resolves() {
    // index.ts -> mid.ts -> impl.ts (2 hops): within MAX_REEXPORT_DEPTH.
    let fs = files(&[
        (
            "impl.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "mid.ts",
            "export { process } from './impl';\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export { process } from './mid';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './index';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
}

#[test]
fn barrel_depth_3_chain_fails_closed() {
    // index.ts -> mid.ts -> mid2.ts -> impl.ts (3 hops): exceeds MAX_REEXPORT_DEPTH.
    let fs = files(&[
        (
            "impl.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "mid2.ts",
            "export { process } from './impl';\n",
            Language::TypeScript,
        ),
        (
            "mid.ts",
            "export { process } from './mid2';\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export { process } from './mid';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './index';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
    assert!(cg.js_export_chain_unresolved > 0);
}

#[test]
fn barrel_depth_3_star_only_chain_fails_closed_and_counts() {
    // F5 (review-fix wave, codex MINOR = opus Minor 1): mirrors
    // `barrel_depth_3_chain_fails_closed` above but with `export * from`
    // barrels the whole way instead of named re-export lists -- previously
    // this star-only form escaped `js_export_chain_unresolved` telemetry
    // entirely, since a too-deep star chain never even produced a candidate
    // name for `resolve_one` to attempt (and count).
    let fs = files(&[
        (
            "impl.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        ("mid2.ts", "export * from './impl';\n", Language::TypeScript),
        ("mid.ts", "export * from './mid2';\n", Language::TypeScript),
        ("index.ts", "export * from './mid';\n", Language::TypeScript),
        (
            "app.ts",
            "import { process } from './index';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
    assert!(cg.js_export_chain_unresolved > 0);
}

#[test]
fn star_only_reexport_cycle_fails_closed_and_counts() {
    // Codex MINOR (fix wave 2): a pure `export * from` cycle (a.ts <-> b.ts,
    // no named export anywhere in the cycle) previously vanished from
    // `js_export_chain_unresolved` telemetry entirely -- the cycle guard in
    // candidate-name collection returned silently instead of counting,
    // unlike the depth-exceeded branch right next to it (see
    // `barrel_depth_3_star_only_chain_fails_closed_and_counts` above).
    let fs = files(&[
        ("a.ts", "export * from './b';\n", Language::TypeScript),
        ("b.ts", "export * from './a';\n", Language::TypeScript),
        (
            "app.ts",
            "import { process } from './a';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
    assert!(cg.js_export_chain_unresolved > 0);
}

#[test]
fn star_reexport_barrel_resolves() {
    let fs = files(&[
        (
            "impl.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export * from './impl';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './index';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
}

#[test]
fn conflicting_star_reexports_fail_closed_end_to_end() {
    let fs = files(&[
        (
            "a.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "b.ts",
            "export function process(): number { return 2; }\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export * from './a';\nexport * from './b';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './index';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
    assert!(cg.js_export_barrel_conflicts > 0);
}

// F3 (review-fix wave, codex MAJOR 1): duplicate exported names poison the
// name -- fail-closed before target emission, counted via the (reused)
// `js_export_barrel_conflicts` mechanism.

#[test]
fn duplicate_reexport_from_two_modules_fails_closed_end_to_end() {
    let fs = files(&[
        (
            "a.ts",
            "export function f(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "b.ts",
            "export function f(): number { return 2; }\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export { f } from './a';\nexport { f } from './b';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { f } from './index';\nfunction run() { f(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "f");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
    assert!(cg.js_export_barrel_conflicts > 0);
}

#[test]
fn duplicate_local_plus_reexport_same_name_fails_closed_end_to_end() {
    let fs = files(&[
        (
            "other.ts",
            "export function f(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "index.ts",
            "export function f(): number { return 2; }\nexport { f } from './other';\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { f } from './index';\nfunction run() { f(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "f");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
}

// -----------------------------------------------------------------------
// Incremental plumbing
// -----------------------------------------------------------------------

#[test]
fn remove_files_clears_export_facts_and_resolution() {
    let fs = files(&[
        (
            "util.ts",
            "export const process = () => 1;\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './util';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let mut cg = CallGraph::build(&fs);
    assert!(cg.js_ts_exports.contains_key("util.ts"));
    assert!(cg.js_ts_resolved_exports.contains_key("util.ts"));
    let mut exclude = std::collections::BTreeSet::new();
    exclude.insert("util.ts".to_string());
    cg.remove_files(&exclude);
    assert!(!cg.js_ts_exports.contains_key("util.ts"));
    assert!(!cg.js_ts_resolved_exports.contains_key("util.ts"));
}
