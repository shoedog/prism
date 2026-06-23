//! R4c import-member resolution tests.
//!
//! Tests the new import-binding rung that resolves unqualified calls
//! (e.g. `func()`) through import provenance (`from module import func`).

use prism::ast::ParsedFile;
use prism::call_graph::{
    file_matches_module, CallGraph, ImportBinding, ImportBindingKind, ModuleBindingKind,
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
fn js_named_import_resolves_exact() {
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
fn ts_named_import_resolves_exact() {
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
