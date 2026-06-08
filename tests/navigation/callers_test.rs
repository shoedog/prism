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
fn callers_reports_caller_and_call_site_line() {
    let s = session(&[(
        "a.py",
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )]);
    let ev = queries::callers(&s, Some("target"), None, None, 1).unwrap();
    assert!(ev.items.iter().any(|i| matches!(&i.symbol,
        Some(SymbolRef::Function { name, .. }) if name == "caller")));
    assert!(ev.items.iter().any(|i| i.why.iter().any(|r|
        matches!(r, Reason::CalledBy { caller, call_site_line } if caller == "caller" && *call_site_line == 5))));
}

#[test]
fn callers_direct_hit_scores_1_0_and_hop2_decays() {
    // A() called by B() called by C() — transitive (R3-M5).
    let s = session(&[(
        "a.py",
        "def a():\n    return 1\n\ndef b():\n    return a()\n\ndef c():\n    return b()\n",
    )]);
    let ev = queries::callers(&s, Some("a"), None, None, 2).unwrap();
    let b = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function{name,..}) if name=="b"))
        .unwrap();
    assert_eq!(b.score, 1.0); // direct caller
    let c = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function{name,..}) if name=="c"))
        .unwrap();
    assert_eq!(c.score, 0.5); // hop-2 caller
}

#[test]
fn callers_depth_zero_has_no_expansion() {
    let s = session(&[(
        "a.py",
        "def target():\n    return 1\n\ndef caller():\n    return target()\n",
    )]);
    let ev = queries::callers(&s, Some("target"), None, None, 0).unwrap();
    assert!(ev.items.is_empty());
}
