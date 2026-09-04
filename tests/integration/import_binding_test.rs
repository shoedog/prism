//! R4c import-member resolution tests.
//!
//! Tests the new import-binding rung that resolves unqualified calls
//! (e.g. `func()`) through import provenance (`from module import func`).

use prism::ast::ParsedFile;
use prism::call_graph::{
    file_matches_js_ts_relative_module_exact, file_matches_module, CallGraph, ImportBindingKind,
    ModuleBindingKind,
};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

// -----------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------

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
) -> (ResolutionConfidence, ResolutionKind) {
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
    assert!(
        !out.resolved.is_empty(),
        "expected resolution for {callee} in {caller_name}"
    );
    (out.resolved[0].confidence, out.resolved[0].kind)
}

fn resolve_count(cg: &CallGraph, caller_file: &str, caller_name: &str, callee: &str) -> usize {
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
    out.resolved.len()
}

// -----------------------------------------------------------------------
// Extraction tests
// -----------------------------------------------------------------------

#[test]
fn python_extract_member_import() {
    let src = "from utils import process\nprocess()\n";
    let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
    let bindings = parsed.extract_import_bindings();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].local, "process");
    assert_eq!(bindings[0].module_path, "utils");
    assert_eq!(bindings[0].member.as_deref(), Some("process"));
    assert!(matches!(bindings[0].kind, ImportBindingKind::MemberImport));
}

#[test]
fn python_extract_aliased_member_import() {
    let src = "from utils import process as p\np()\n";
    let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
    let bindings = parsed.extract_import_bindings();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].local, "p");
    assert_eq!(bindings[0].module_path, "utils");
    assert_eq!(bindings[0].member.as_deref(), Some("process"));
}

#[test]
fn python_extract_module_import() {
    let src = "import os\nimport json\n";
    let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
    let bindings = parsed.extract_import_bindings();
    assert_eq!(bindings.len(), 2);
    assert!(bindings
        .iter()
        .all(|b| matches!(b.kind, ImportBindingKind::ModuleImport)));
}

#[test]
fn python_extract_unaliased_dotted_module_import_binds_root() {
    let parsed = ParsedFile::parse(
        "app.py",
        "import pkg.models\npkg.models.Client()\n",
        Language::Python,
    )
    .unwrap();
    let bindings = parsed.extract_import_bindings();

    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].local, "pkg");
    assert_eq!(bindings[0].module_path, "pkg.models");
    assert!(matches!(bindings[0].kind, ImportBindingKind::ModuleImport));
}

#[test]
fn clean_module_import_is_eligible_without_triggering_r4c() {
    let fs = files(&[
        (
            "models.py",
            "class Client:\n    pass\ndef process():\n    pass\n",
            Language::Python,
        ),
        (
            "app.py",
            "import models\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let binding = &cg.import_bindings["app.py"][0];

    assert!(binding.eligible);
    let (_, kind) = resolve_kind(&cg, "app.py", "run", "process");
    assert_ne!(kind, ResolutionKind::ImportMember);
}

#[test]
fn python_extract_wildcard_import() {
    let src = "from utils import *\n";
    let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
    let bindings = parsed.extract_import_bindings();
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].local, "*");
    assert!(matches!(
        bindings[0].kind,
        ImportBindingKind::WildcardImport
    ));
}

#[test]
fn js_extract_named_import() {
    let src = "import { func } from './utils';\nfunc();\n";
    let parsed = ParsedFile::parse("app.js", src, Language::JavaScript).unwrap();
    let bindings = parsed.extract_import_bindings();
    let member = bindings
        .iter()
        .find(|b| b.local == "func")
        .expect("should have func binding");
    assert_eq!(member.module_path, "./utils");
    assert_eq!(member.member.as_deref(), Some("func"));
    assert!(matches!(member.kind, ImportBindingKind::MemberImport));
}

#[test]
fn js_extract_aliased_named_import() {
    let src = "import { process as p } from './utils';\np();\n";
    let parsed = ParsedFile::parse("app.js", src, Language::JavaScript).unwrap();
    let bindings = parsed.extract_import_bindings();
    let member = bindings
        .iter()
        .find(|b| b.local == "p")
        .expect("should have p binding");
    assert_eq!(member.module_path, "./utils");
    assert_eq!(member.member.as_deref(), Some("process"));
    assert!(matches!(member.kind, ImportBindingKind::MemberImport));
}

#[test]
fn ts_extract_named_import() {
    let src = "import { func } from './utils';\nfunc();\n";
    let parsed = ParsedFile::parse("app.ts", src, Language::TypeScript).unwrap();
    let bindings = parsed.extract_import_bindings();
    let member = bindings
        .iter()
        .find(|b| b.local == "func")
        .expect("should have func binding");
    assert!(matches!(member.kind, ImportBindingKind::MemberImport));
}

// -----------------------------------------------------------------------
// Module-binding extraction tests
// -----------------------------------------------------------------------

#[test]
fn python_module_bindings() {
    let src = "from utils import func\nclass MyClass:\n    pass\ndef helper():\n    pass\nx = 42\n";
    let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
    let mb = parsed.extract_module_bindings();
    assert!(matches!(mb.get("func"), Some(ModuleBindingKind::Import)));
    assert!(matches!(
        mb.get("MyClass"),
        Some(ModuleBindingKind::ClassDef)
    ));
    assert!(matches!(
        mb.get("helper"),
        Some(ModuleBindingKind::FunctionDef)
    ));
    assert!(matches!(mb.get("x"), Some(ModuleBindingKind::Assignment)));
}

// -----------------------------------------------------------------------
// Eligibility tests
// -----------------------------------------------------------------------

#[test]
fn wildcard_poisons_all_bindings() {
    let src = "from utils import func\nfrom other import *\n";
    let fs = files(&[("app.py", src, Language::Python)]);
    let cg = CallGraph::build(&fs);
    let bindings = cg.import_bindings.get("app.py").expect("bindings");
    // All bindings should be ineligible due to wildcard
    for b in bindings {
        assert!(!b.eligible, "binding {} should be ineligible", b.local);
    }
}

#[test]
fn rebound_name_ineligible() {
    let src = "from utils import func\nfunc = lambda: None\ndef caller():\n    func()\n";
    let fs = files(&[
        ("app.py", src, Language::Python),
        ("utils.py", "def func():\n    return 42\n", Language::Python),
    ]);
    let cg = CallGraph::build(&fs);
    let bindings = cg.import_bindings.get("app.py").expect("bindings");
    let func_binding = bindings.iter().find(|b| b.local == "func").unwrap();
    assert!(!func_binding.eligible, "re-bound func should be ineligible");
}

#[test]
fn indexed_files_populated() {
    let fs = files(&[
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.py",
            "from utils import process\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert!(cg.indexed_files.contains("utils.py"));
    assert!(cg.indexed_files.contains("app.py"));
}

// -----------------------------------------------------------------------
// Resolution tests (R4c behavior change)
// -----------------------------------------------------------------------

#[test]
fn basic_member_import_resolves_exact() {
    let fs = files(&[
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.py",
            "from utils import process\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.py", "run", "process");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn aliased_import_resolves_to_original() {
    let fs = files(&[
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.py",
            "from utils import process as p\ndef run():\n    p()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.py", "run", "p");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
    // Verify it resolves to the correct target
    let caller = cg
        .functions
        .get("run")
        .and_then(|v| v.iter().find(|f| f.file == "app.py"))
        .unwrap();
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == "p"))
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    assert_eq!(out.resolved[0].target.file, "utils.py");
    assert_eq!(out.resolved[0].target.name, "process");
}

#[test]
fn wildcard_poisoned_falls_through_to_r5() {
    let fs = files(&[
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.py",
            "from utils import process\nfrom other import *\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.py", "run", "process");
    // Should NOT be ImportMember (wildcard poisons); falls to R5 FreeSingle
    assert_eq!(kind, ResolutionKind::FreeSingle);
}

#[test]
fn multiple_matches_demoted() {
    // Two files with the same stem-matching module name
    let fs = files(&[
        (
            "pkg/utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "lib/utils.py",
            "def process():\n    return 2\n",
            Language::Python,
        ),
        (
            "app.py",
            "from utils import process\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.py", "run", "process");
    assert_eq!(kind, ResolutionKind::ImportMember);
    assert_eq!(
        conf,
        ResolutionConfidence::NameOnly,
        "multiple matches -> demoted"
    );
}

#[test]
fn module_import_does_not_trigger_r4c() {
    // `import utils` should NOT trigger R4c for unqualified calls
    let fs = files(&[
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.py",
            "import utils\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (_, kind) = resolve_kind(&cg, "app.py", "run", "process");
    // Should fall through to R5 (module import, not member import)
    assert_ne!(kind, ResolutionKind::ImportMember);
}

#[test]
fn non_python_file_inert() {
    // Go file should not be affected by R4c
    let fs = files(&[(
        "main.go",
        "package main\nfunc process() {}\nfunc run() { process() }\n",
        Language::Go,
    )]);
    let cg = CallGraph::build(&fs);
    assert!(cg.import_bindings.is_empty());
}

#[test]
fn method_not_resolved_via_r4c() {
    // `from m import f` should resolve to the free function `f`, not the method `f`
    let fs = files(&[
        (
            "m.py",
            "def f(p):\n    return p\n\nclass K:\n    def f(q):\n        return q\n",
            Language::Python,
        ),
        (
            "c.py",
            "from m import f\n\ndef call():\n    f(42)\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let caller = cg
        .functions
        .get("call")
        .and_then(|v| v.iter().find(|f| f.file == "c.py"))
        .unwrap();
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == "f"))
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    assert_eq!(
        out.resolved.len(),
        1,
        "should resolve to exactly one target"
    );
    assert_eq!(out.resolved[0].target.name, "f");
    // Should resolve to the free function (line 1), not the method (line 5)
    assert_eq!(out.resolved[0].target.start_line, 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::ImportMember);
}

#[test]
fn js_commonjs_named_object_export_resolves_import_member() {
    // P4: `module.exports = { process }` is modeled (js_exports::JsExportFacts),
    // so a named ESM-style import of a CJS-exported object member resolves.
    let fs = files(&[
        (
            "utils.js",
            "function process() { return 1; }\nmodule.exports = { process };\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "import { process } from './utils';\nfunction run() { process(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.js", "run", "process");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn js_export_function_named_import_resolves_import_member() {
    let fs = files(&[
        (
            "utils.js",
            "export function process() { return 1; }\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "import { process as runProcess } from './utils';\nfunction run() { runProcess(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.js", "run", "runProcess");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn ts_export_function_named_import_resolves_import_member() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.ts", "run", "process");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn ts_type_only_import_does_not_resolve_import_member() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import type { process } from './utils';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_type_named_specifier_does_not_resolve_import_member() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { type process } from './utils';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn tsx_export_function_named_import_resolves_import_member() {
    let fs = files(&[
        (
            "view.tsx",
            "export function process(): number { return 1; }\n",
            Language::Tsx,
        ),
        (
            "app.tsx",
            "import { process } from './view';\nfunction run() { process(); }\n",
            Language::Tsx,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.tsx", "run", "process");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn ts_named_import_collision_uses_imported_module_only() {
    let fs = files(&[
        (
            "alpha/util.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "beta/util.ts",
            "export function process(): number { return 2; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './alpha/util';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
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
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].target.file, "alpha/util.ts");
    assert_eq!(out.resolved[0].kind, ResolutionKind::ImportMember);
}

#[test]
fn ts_default_export_function_is_not_named_export() {
    let fs = files(&[
        (
            "utils.ts",
            "export default function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_import_member_rejects_non_exported_function() {
    let fs = files(&[
        (
            "utils.ts",
            "function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run() { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_import_member_rejects_param_shadow() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run(process: () => number) { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_import_member_ignores_type_annotation_identifier() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run(arg: process) { process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.ts", "run", "process");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn ts_import_member_rejects_catch_shadow() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run() { try { throw 1; } catch (process) { process(); } }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_import_member_rejects_local_shadow() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction run() { const process = () => 2; process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_import_member_rejects_arrow_param_shadow() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nconst run = (process: () => number) => { process(); };\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

#[test]
fn ts_import_member_resolves_arrow_const_export() {
    // P4: exported const-arrow declarations are modeled, so a renamed named
    // import of one resolves like any other named function export.
    let fs = files(&[
        (
            "utils.ts",
            "export const process = () => 1;\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import { process as runProcess } from './utils';\nfunction run() { runProcess(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.ts", "run", "runProcess");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}

#[test]
fn ts_import_member_rejects_default_and_namespace_imports() {
    let fs = files(&[
        (
            "utils.ts",
            "export function process(): number { return 1; }\n",
            Language::TypeScript,
        ),
        (
            "app.ts",
            "import runProcess from './utils';\nimport * as utils from './utils';\nfunction run() { runProcess(); utils.process(); }\n",
            Language::TypeScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.ts", "run", "runProcess");
    assert_not_import_member(&cg, "app.ts", "run", "process");
}

// -----------------------------------------------------------------------
// file_matches_module tests
// -----------------------------------------------------------------------

#[test]
fn file_matches_simple_module() {
    let indexed: BTreeSet<String> = ["utils.py"].iter().map(|s| s.to_string()).collect();
    assert!(file_matches_module("utils.py", "utils", "app.py", &indexed));
    assert!(!file_matches_module(
        "other.py", "utils", "app.py", &indexed
    ));
}

#[test]
fn file_matches_dotted_module() {
    let indexed: BTreeSet<String> = ["myapp/utils.py"].iter().map(|s| s.to_string()).collect();
    assert!(file_matches_module(
        "myapp/utils.py",
        "myapp.utils",
        "app.py",
        &indexed
    ));
}

#[test]
fn file_matches_init_package() {
    let indexed: BTreeSet<String> = ["utils/__init__.py"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(file_matches_module(
        "utils/__init__.py",
        "utils",
        "app.py",
        &indexed
    ));
}

#[test]
fn file_matches_relative_import() {
    let indexed: BTreeSet<String> = ["pkg/utils.py"].iter().map(|s| s.to_string()).collect();
    assert!(file_matches_module(
        "pkg/utils.py",
        ".utils",
        "pkg/app.py",
        &indexed
    ));
}

#[test]
fn file_matches_js_relative() {
    let indexed: BTreeSet<String> = ["src/utils.js"].iter().map(|s| s.to_string()).collect();
    // Stem match should work for JS too
    assert!(file_matches_module(
        "src/utils.js",
        "./utils",
        "src/app.js",
        &indexed
    ));
}

#[test]
fn js_ts_exact_relative_matcher_accepts_file_and_index() {
    let indexed: BTreeSet<String> = ["src/utils.ts", "src/pkg/index.ts"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(file_matches_js_ts_relative_module_exact(
        "src/utils.ts",
        "./utils",
        "src/app.ts",
        &indexed
    ));
    assert!(file_matches_js_ts_relative_module_exact(
        "src/pkg/index.ts",
        "./pkg",
        "src/app.ts",
        &indexed
    ));
}

#[test]
fn js_ts_exact_relative_matcher_accepts_parent_path() {
    let indexed: BTreeSet<String> = ["src/utils.ts"].iter().map(|s| s.to_string()).collect();
    assert!(file_matches_js_ts_relative_module_exact(
        "src/utils.ts",
        "../utils",
        "src/sub/app.ts",
        &indexed
    ));
}

#[test]
fn js_ts_exact_relative_matcher_rejects_wrong_directory_stem() {
    let indexed: BTreeSet<String> = ["elsewhere/utils.ts"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(!file_matches_js_ts_relative_module_exact(
        "elsewhere/utils.ts",
        "./utils",
        "src/app.ts",
        &indexed
    ));
}

#[test]
fn js_ts_exact_relative_matcher_rejects_bare_package_specifier() {
    let indexed: BTreeSet<String> = ["node_modules/pkg/index.ts"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(!file_matches_js_ts_relative_module_exact(
        "node_modules/pkg/index.ts",
        "pkg",
        "src/app.ts",
        &indexed
    ));
}

#[test]
fn js_ts_exact_relative_matcher_rejects_parent_above_root() {
    let indexed: BTreeSet<String> = ["util.ts"].iter().map(|s| s.to_string()).collect();
    assert!(
        !file_matches_js_ts_relative_module_exact("util.ts", "../util", "app.ts", &indexed),
        "relative traversal above the caller root should not clamp to repo root"
    );
}

// -----------------------------------------------------------------------
// Incremental cache support (remove_files + merge)
// -----------------------------------------------------------------------

#[test]
fn remove_files_clears_import_bindings() {
    let fs = files(&[
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.py",
            "from utils import process\ndef run():\n    process()\n",
            Language::Python,
        ),
    ]);
    let mut cg = CallGraph::build(&fs);
    assert!(cg.import_bindings.contains_key("app.py"));
    assert!(cg.indexed_files.contains("app.py"));

    let exclude: BTreeSet<String> = ["app.py".to_string()].into_iter().collect();
    cg.remove_files(&exclude);

    assert!(!cg.import_bindings.contains_key("app.py"));
    assert!(!cg.indexed_files.contains("app.py"));
    // utils.py should still be there (not excluded)
    assert!(cg.indexed_files.contains("utils.py"));
}

#[test]
fn merge_preserves_import_bindings() {
    let fs1 = files(&[(
        "utils.py",
        "def process():\n    return 1\n",
        Language::Python,
    )]);
    let fs2 = files(&[(
        "app.py",
        "from utils import process\ndef run():\n    process()\n",
        Language::Python,
    )]);
    let cg1 = CallGraph::build(&fs1);
    let mut cg2 = CallGraph::build(&fs2);
    cg2.merge(cg1);
    assert!(cg2.import_bindings.contains_key("app.py"));
    assert!(cg2.indexed_files.contains("utils.py"));
    assert!(cg2.indexed_files.contains("app.py"));
}

// -----------------------------------------------------------------------
// Duplicate local name ineligibility
// -----------------------------------------------------------------------

#[test]
fn duplicate_local_name_makes_both_ineligible() {
    let src = "from a import func\nfrom b import func\ndef run():\n    func()\n";
    let fs = files(&[
        ("app.py", src, Language::Python),
        ("a.py", "def func():\n    return 1\n", Language::Python),
        ("b.py", "def func():\n    return 2\n", Language::Python),
    ]);
    let cg = CallGraph::build(&fs);
    let bindings = cg.import_bindings.get("app.py").expect("bindings");
    // Both `func` bindings should be ineligible
    for b in bindings.iter().filter(|b| b.local == "func") {
        assert!(!b.eligible, "duplicate import should be ineligible");
    }
    // Should fall through to R5, not R4c
    let (_, kind) = resolve_kind(&cg, "app.py", "run", "func");
    assert_ne!(kind, ResolutionKind::ImportMember);
}

// -----------------------------------------------------------------------
// Codex diff-review fixes: module-scope-only imports + dotted-path matching
// -----------------------------------------------------------------------

#[test]
fn function_local_import_not_collected() {
    // `from utils import func` inside a function body must NOT create
    // a file-wide R4c binding.
    let src = "def run():\n    from utils import func\n    func()\n";
    let parsed = ParsedFile::parse("app.py", src, Language::Python).unwrap();
    let bindings = parsed.extract_import_bindings();
    assert!(
        bindings.is_empty(),
        "function-local import should not produce a file-wide binding"
    );
}

#[test]
fn function_local_shadow_does_not_kill_module_import() {
    // Module-level `from utils import func` IS eligible. A function-local
    // `func = local` must not affect the module-level binding since
    // extract_module_bindings only walks root children.
    let src =
        "from utils import func\ndef run():\n    func()\ndef other():\n    func = 42\n    func\n";
    let fs = files(&[
        ("app.py", src, Language::Python),
        ("utils.py", "def func():\n    return 1\n", Language::Python),
    ]);
    let cg = CallGraph::build(&fs);
    let bindings = cg.import_bindings.get("app.py").expect("bindings");
    let func_binding = bindings.iter().find(|b| b.local == "func").unwrap();
    assert!(
        func_binding.eligible,
        "module-level import should remain eligible despite function-local shadow"
    );
    let (conf, kind) = resolve_kind(&cg, "app.py", "run", "func");
    assert_eq!(kind, ResolutionKind::ImportMember);
    assert_eq!(conf, ResolutionConfidence::Exact);
}

#[test]
fn dotted_import_no_stem_fallback_to_wrong_file() {
    // `from myapp.utils import f` must NOT stem-match a different `other/utils.py`.
    let indexed: BTreeSet<String> = ["other/utils.py"].iter().map(|s| s.to_string()).collect();
    assert!(
        !file_matches_module("other/utils.py", "myapp.utils", "app.py", &indexed),
        "dotted import should NOT stem-match a different package's utils.py"
    );
}

#[test]
fn dotted_import_matches_correct_path() {
    // `from myapp.utils import f` SHOULD match `myapp/utils.py`.
    let indexed: BTreeSet<String> = ["myapp/utils.py"].iter().map(|s| s.to_string()).collect();
    assert!(
        file_matches_module("myapp/utils.py", "myapp.utils", "app.py", &indexed),
        "dotted import should match the correct path"
    );
}

#[test]
fn relative_multi_component_import_matches_full_path() {
    // `.pkg.utils` from `src/app.py` should match `src/pkg/utils.py`, not just
    // any `utils.py` in the same directory.
    let indexed: BTreeSet<String> = ["src/pkg/utils.py", "src/utils.py"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(
        file_matches_module("src/pkg/utils.py", ".pkg.utils", "src/app.py", &indexed),
        "relative multi-component should match full path"
    );
    assert!(
        !file_matches_module("src/utils.py", ".pkg.utils", "src/app.py", &indexed),
        "relative multi-component should NOT match partial path"
    );
}

// -----------------------------------------------------------------------
// Soundness fixes (codex round-2 review)
// -----------------------------------------------------------------------

/// Helper: assert that a call site does NOT resolve via R4c (ImportMember).
fn assert_not_import_member(cg: &CallGraph, caller_file: &str, caller_name: &str, callee: &str) {
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
    for r in &out.resolved {
        assert_ne!(
            r.kind,
            ResolutionKind::ImportMember,
            "{callee} in {caller_name} should NOT resolve via ImportMember"
        );
    }
}

#[test]
fn test_import_binding_nested_function_excluded() {
    // `def outer(): def f(): pass` — the nested `f` is NOT module-level
    // and must not be resolved by R4c.
    let fs = files(&[
        (
            "m.py",
            "def outer():\n    def f():\n        pass\n",
            Language::Python,
        ),
        (
            "c.py",
            "from m import f\n\ndef call():\n    f()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "c.py", "call", "f");
}

#[test]
fn test_import_binding_compound_rebinding_ineligible() {
    // `from utils import f` then `if DEBUG: f = other_f` at module scope.
    // Module bindings should show `f` as both Import and Assignment, making
    // the import ineligible (Assignment wins via insert overwrite).
    let fs = files(&[
        ("utils.py", "def f():\n    return 1\n", Language::Python),
        (
            "app.py",
            "from utils import f\nif True:\n    f = lambda: 2\ndef run():\n    f()\n",
            Language::Python,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    assert_not_import_member(&cg, "app.py", "run", "f");
}

#[test]
fn test_import_binding_relative_single_component_no_stem() {
    // `.utils` from `pkg/app.py` must only match `pkg/utils.py`, NOT
    // `other/utils.py` (single-component relative must not stem-fallback).
    let indexed: BTreeSet<String> = ["pkg/utils.py", "other/utils.py"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert!(
        file_matches_module("pkg/utils.py", ".utils", "pkg/app.py", &indexed),
        "relative single-component should match same-directory file"
    );
    assert!(
        !file_matches_module("other/utils.py", ".utils", "pkg/app.py", &indexed),
        "relative single-component should NOT stem-match another directory"
    );
}

#[test]
fn test_import_binding_js_cjs_object_export_resolves() {
    // P4: CommonJS `module.exports = { f }` is modeled — a named import of a
    // CJS-exported object member now resolves via ImportMember.
    let fs = files(&[
        (
            "m.js",
            "function f() { return 1; }\nmodule.exports = { f };\n",
            Language::JavaScript,
        ),
        (
            "app.js",
            "import { f } from './m';\nfunction run() { f(); }\n",
            Language::JavaScript,
        ),
    ]);
    let cg = CallGraph::build(&fs);
    let (conf, kind) = resolve_kind(&cg, "app.js", "run", "f");
    assert_eq!(conf, ResolutionConfidence::Exact);
    assert_eq!(kind, ResolutionKind::ImportMember);
}
