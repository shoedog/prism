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
    // qualified call `util.helper()` — exercises resolve_callees_qualified (B2/M8).
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
