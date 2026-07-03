//! P4: JS/TS export-fact modeling tests.
//!
//! Covers raw per-file export-fact extraction (`ParsedFile::extract_js_ts_export_facts`)
//! for each export form (default / named-list incl. renames / const-arrow /
//! CommonJS / re-export chains), the whole-program barrel resolution wired
//! onto `CallGraph`, and end-to-end R4c resolution (`resolve_call_site_full`)
//! for the newly grounded forms. Companion to `import_binding_test.rs`, which
//! owns the pre-existing R4c shadow/eligibility guards (left untouched here).

use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::js_exports::JsExportTarget;
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
// 1a. Default exports
// -----------------------------------------------------------------------

#[test]
fn extract_default_export_named_function() {
    let parsed = ParsedFile::parse(
        "util.ts",
        "export default function process(): number { return 1; }\n",
        Language::TypeScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("default"),
        Some(&JsExportTarget::Local("process".to_string()))
    );
    // The default-exported function's own name is NOT itself a named export.
    assert!(!facts.named.contains_key("process"));
}

#[test]
fn extract_default_export_identifier() {
    let parsed = ParsedFile::parse(
        "util.ts",
        "function process(): number { return 1; }\nexport default process;\n",
        Language::TypeScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("default"),
        Some(&JsExportTarget::Local("process".to_string()))
    );
}

#[test]
fn default_import_resolves_default_export_function() {
    let fs = files(&[
        (
            "util.ts",
            "export default function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import runProcess from './util';\nfunction run() { runProcess(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "runProcess");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
}

#[test]
fn default_import_resolves_default_export_via_rename() {
    // export { process as default } from the same file (named-list rename to
    // "default") is a distinct syntactic path from `export default function`.
    let fs = files(&[
        (
            "util.ts",
            "export function process(): number { return 1; }\nexport { process as default };\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import runProcess from './util';\nfunction run() { runProcess(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "runProcess");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
}

#[test]
fn named_import_of_default_only_export_does_not_resolve() {
    // A named import can never bind to a default-only export.
    let fs = files(&[
        (
            "util.ts",
            "export default function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './util';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "process");
    assert!(resolved
        .iter()
        .all(|(_, k)| *k != ResolutionKind::ImportMember));
}

// -----------------------------------------------------------------------
// 1b. Named export lists, including renames (local declarations only)
// -----------------------------------------------------------------------

#[test]
fn extract_named_export_list_with_rename() {
    let parsed = ParsedFile::parse(
        "util.ts",
        "function a() { return 1; }\nfunction b() { return 2; }\nexport { a, b as c };\n",
        Language::TypeScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("a"),
        Some(&JsExportTarget::Local("a".to_string()))
    );
    assert_eq!(
        facts.named.get("c"),
        Some(&JsExportTarget::Local("b".to_string()))
    );
    assert!(!facts.named.contains_key("b"));
}

#[test]
fn named_export_list_rename_resolves_import_member() {
    let fs = files(&[
        (
            "util.ts",
            "function a() { return 1; }\nfunction b() { return 2; }\nexport { a, b as c };\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { c } from './util';\nfunction run() { c(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.ts", "run", "c");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
    // Verify it bound to the RIGHT local declaration (b, not a).
    let caller = cg
        .functions
        .get("run")
        .and_then(|v| v.iter().find(|f| f.file == "app.ts"))
        .unwrap();
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == "c"))
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    assert_eq!(out.resolved[0].target.name, "b");
}

// -----------------------------------------------------------------------
// 1c. Exported const-arrow / function-expression
// -----------------------------------------------------------------------

#[test]
fn extract_const_arrow_export() {
    let parsed = ParsedFile::parse(
        "util.ts",
        "export const process = () => 1;\n",
        Language::TypeScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("process"),
        Some(&JsExportTarget::Local("process".to_string()))
    );
}

#[test]
fn extract_const_function_expression_export() {
    let parsed = ParsedFile::parse(
        "util.js",
        "export const process = function() { return 1; };\n",
        Language::JavaScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("process"),
        Some(&JsExportTarget::Local("process".to_string()))
    );
}

// -----------------------------------------------------------------------
// 1d. CommonJS
// -----------------------------------------------------------------------

#[test]
fn extract_cjs_module_exports_identifier() {
    let parsed = ParsedFile::parse(
        "util.js",
        "function process() { return 1; }\nmodule.exports = process;\n",
        Language::JavaScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("default"),
        Some(&JsExportTarget::Local("process".to_string()))
    );
}

#[test]
fn extract_cjs_module_exports_object_literal() {
    let parsed = ParsedFile::parse(
        "util.js",
        "function a() { return 1; }\nfunction b() { return 2; }\nmodule.exports = { a, x: b };\n",
        Language::JavaScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("a"),
        Some(&JsExportTarget::Local("a".to_string()))
    );
    assert_eq!(
        facts.named.get("x"),
        Some(&JsExportTarget::Local("b".to_string()))
    );
}

#[test]
fn extract_cjs_module_exports_member_assignment() {
    let parsed = ParsedFile::parse(
        "util.js",
        "function f() { return 1; }\nmodule.exports.f = f;\n",
        Language::JavaScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("f"),
        Some(&JsExportTarget::Local("f".to_string()))
    );
}

#[test]
fn extract_cjs_exports_member_assignment() {
    let parsed = ParsedFile::parse(
        "util.js",
        "function f() { return 1; }\nexports.f = f;\n",
        Language::JavaScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(
        facts.named.get("f"),
        Some(&JsExportTarget::Local("f".to_string()))
    );
}

#[test]
fn extract_cjs_arbitrary_expression_is_skipped_and_counted() {
    let parsed = ParsedFile::parse(
        "util.js",
        "function helper() { return 1; }\nmodule.exports = helper();\n",
        Language::JavaScript,
    )
    .unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert!(facts.named.is_empty());
    assert_eq!(facts.skipped_expr_count, 1);
}

#[test]
fn cjs_named_object_export_resolves_import_member() {
    let fs = files(&[
        (
            "util.js",
            "function process() { return 1; }\nmodule.exports = { process };\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "import { process } from './util';\nfunction run() { process(); }\n",
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

#[test]
fn cjs_module_exports_identifier_resolves_default_import() {
    let fs = files(&[
        (
            "util.js",
            "function process() { return 1; }\nmodule.exports = process;\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "import runProcess from './util';\nfunction run() { runProcess(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let resolved = resolve_kind(&cg, "app.js", "run", "runProcess");
    assert_eq!(
        resolved,
        vec![(ResolutionConfidence::Exact, ResolutionKind::ImportMember)]
    );
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
