use prism::navigation::types::{Reason, SymbolRef};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        std::fs::write(dir.path().join(name), src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn callees_reports_callee_and_line() {
    let s = session(&[(
        "a.py",
        "def helper():\n    return 1\n\ndef caller():\n    return helper()\n",
    )]);
    let ev = queries::callees(&s, Some("caller"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| i.why.iter().any(
        |r| matches!(r, Reason::Calls { callee, call_site_line, .. } if callee == "helper" && *call_site_line == 5)
    )));
}

#[test]
fn callees_resolves_qualified_import() {
    // qualified call `util.helper()` exercises the S3 import-qualified ladder path.
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        (
            "main.py",
            "import util\n\ndef run():\n    return util.helper()\n",
        ),
    ]);
    let ev = queries::callees(&s, Some("run"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| i.why.iter().any(
        |r| matches!(r, Reason::Calls { callee, qualifier, .. } if callee == "helper" && qualifier.as_deref() == Some("util"))
    )));
    assert!(ev.items.iter().any(|i| i
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "import_qualified"))));
}

#[test]
fn demoted_callee_scores_0_6_with_resolution_reason() {
    let s = session(&[
        (
            "owner.py",
            "class OnlyOwner:\n    def frobnicate(self):\n        return 1\n",
        ),
        ("main.py", "def run(x):\n    return x.frobnicate()\n"),
    ]);
    let ev = queries::callees(&s, Some("run"), Some("main.py"), None, 1).unwrap();
    let item = ev
        .items
        .iter()
        .find(|i| {
            matches!(
                &i.symbol,
                Some(SymbolRef::Function { file, name, .. })
                    if file == "owner.py" && name == "frobnicate"
            )
        })
        .expect("demoted R6 single-owner callee");
    assert_eq!(item.score, 0.6);
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "r6_single_owner")));
}

// F4 (P3 review-fix wave): a 2-owner unknown-receiver collision must surface
// as a nav-visible candidate edge — score 0.6 (NameOnly) with the
// `r6_multi_owner_candidate` resolution reason, not a silent drop.
#[test]
fn taint_boundary_negative_fixture_candidate_remains_name_only() {
    let s = session(&[
        (
            "app.py",
            "class A:\n    def m(self, p):\n        return p\n\ndef f(obj):\n    user = input()\n    obj.m(user)\n",
        ),
    ]);
    let ev = queries::callees(&s, Some("f"), Some("app.py"), None, 1).unwrap();
    let item = ev
        .items
        .iter()
        .find(|i| {
            matches!(
                &i.symbol,
                Some(SymbolRef::Function { file, name, .. })
                    if file == "app.py" && name == "m"
            )
        })
        .expect("single-owner NameOnly callee for fixture shape");
    assert_eq!(item.score, 0.6);
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "r6_single_owner")));
}

#[test]
fn multi_owner_candidate_callee_scores_0_6_with_resolution_reason() {
    let s = session(&[
        (
            "a.py",
            "class A:\n    def handle(self):\n        return 1\n",
        ),
        (
            "b.py",
            "class B:\n    def handle(self):\n        return 2\n",
        ),
        ("main.py", "def run(x):\n    return x.handle()\n"),
    ]);
    let ev = queries::callees(&s, Some("run"), Some("main.py"), None, 1).unwrap();
    let item = ev
        .items
        .iter()
        .find(|i| {
            matches!(
                &i.symbol,
                Some(SymbolRef::Function { file, name, .. })
                    if file == "a.py" && name == "handle"
            )
        })
        .expect("R6 multi-owner candidate callee");
    assert_eq!(item.score, 0.6);
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "r6_multi_owner_candidate")));
}

#[test]
fn exact_callee_scores_1_0_with_resolution_reason() {
    let s = session(&[(
        "engine.rs",
        "struct Engine;\nimpl Engine { fn start(&self) {} }\nfn run(e: Engine) { Engine::start(&e); }\n",
    )]);
    let ev = queries::callees(&s, Some("run"), Some("engine.rs"), None, 1).unwrap();
    let item = ev
        .items
        .iter()
        .find(|i| {
            matches!(
                &i.symbol,
                Some(SymbolRef::Function { file, name, .. })
                    if file == "engine.rs" && name == "start"
            )
        })
        .expect("exact qualified-owner callee");
    assert_eq!(item.score, 1.0);
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "qualified_owner")));
}

#[test]
fn callees_depth_zero_has_no_expansion() {
    let s = session(&[(
        "a.py",
        "def helper():\n    return 1\n\ndef caller():\n    return helper()\n",
    )]);
    let ev = queries::callees(&s, Some("caller"), None, None, 0).unwrap();
    assert!(ev.items.is_empty());
}

#[test]
fn callees_emits_each_resolved_definition_for_one_call_site() {
    let s = session(&[
        ("a.py", "def helper():\n    return 1\n"),
        ("b.py", "def helper():\n    return 2\n"),
        ("main.py", "def run():\n    return helper()\n"),
    ]);
    let ev = queries::callees(&s, Some("run"), None, None, 1).unwrap();
    let mut helper_files: Vec<_> = ev
        .items
        .iter()
        .filter_map(|i| match &i.symbol {
            Some(SymbolRef::Function { file, name, .. }) if name == "helper" => Some(file.as_str()),
            _ => None,
        })
        .collect();
    helper_files.sort_unstable();
    assert_eq!(helper_files, vec!["a.py", "b.py"]);
    assert_eq!(
        ev.items
            .iter()
            .filter(|i| i.why.iter().any(
                |r| matches!(r, Reason::Calls { callee, call_site_line, .. } if callee == "helper" && *call_site_line == 2)
            ))
            .count(),
        2
    );
}

#[test]
fn disabled_level3_emits_no_callback_edges_in_either_direction() {
    let s = session(&[
        ("a.js", "export function invoke(cb) { cb(); }\n"),
        (
            "b.js",
            "import { invoke } from './a';\nfunction safe() {}\nfunction startB() { invoke(safe); }\n",
        ),
        (
            "c.js",
            "import { invoke } from './a';\nfunction safe() {}\nfunction startC() { invoke(safe); }\n",
        ),
    ]);

    let callees = queries::callees(&s, Some("invoke"), Some("a.js"), None, 1).unwrap();
    let mut safe_files: Vec<_> = callees
        .items
        .iter()
        .filter_map(|item| match &item.symbol {
            Some(SymbolRef::Function { file, name, .. }) if name == "safe" => Some(file.as_str()),
            _ => None,
        })
        .collect();
    safe_files.sort_unstable();
    assert!(safe_files.is_empty());

    for file in ["b.js", "c.js"] {
        let callers = queries::callers(&s, Some("safe"), Some(file), None, 1).unwrap();
        assert!(
            !callers.items.iter().any(|item| {
                matches!(
                    &item.symbol,
                    Some(SymbolRef::Function { file, name, .. })
                        if file == "a.js" && name == "invoke"
                )
            }),
            "disabled Level-3 must not report a.js::invoke as caller of {file}::safe"
        );
    }
}

#[test]
fn go_level3_callback_surfaces_exactly_in_both_navigation_directions() {
    let s = session(&[(
        "callbacks.go",
        "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
    )]);

    let callees = queries::callees(&s, Some("invoke"), Some("callbacks.go"), None, 1).unwrap();
    let callback_callee = callees
        .items
        .iter()
        .find(|item| {
            matches!(
                &item.symbol,
                Some(SymbolRef::Function { file, name, .. })
                    if file == "callbacks.go" && name == "safe"
            )
        })
        .expect("invoke must navigate to the proven callback target");
    assert!(callback_callee.why.iter().any(
        |reason| matches!(reason, Reason::Resolution { kind } if kind == "parameter_callback")
    ));

    let callers = queries::callers(&s, Some("safe"), Some("callbacks.go"), None, 1).unwrap();
    let callback_caller = callers
        .items
        .iter()
        .find(|item| {
            matches!(
                &item.symbol,
                Some(SymbolRef::Function { file, name, .. })
                    if file == "callbacks.go" && name == "invoke"
            )
        })
        .expect("safe must navigate back to its callback invocation");
    assert!(callback_caller.why.iter().any(
        |reason| matches!(reason, Reason::Resolution { kind } if kind == "parameter_callback")
    ));
}
