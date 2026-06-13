use prism::navigation::types::{Reason, SymbolRef, WarningKind};
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

#[test]
fn callers_query_emits_collision_warning_for_dropped_sites() {
    let s = session(&[
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        ("main.py", "def drive(x):\n    return x.poll()\n"),
    ]);
    let ev = queries::callers(&s, Some("poll"), Some("a.py"), None, 1).unwrap();
    assert!(!ev.items.iter().any(|i| {
        matches!(
            &i.symbol,
            Some(SymbolRef::Function { file, name, .. }) if file == "main.py" && name == "drive"
        )
    }));
    assert!(ev.warnings.iter().any(|w| {
        matches!(w.kind, WarningKind::Collision)
            && w.message.contains('1')
            && w.message
                .contains("unknown receiver type across multiple owner types")
    }));
}

#[test]
fn ego_graph_emits_collision_warning_for_seed() {
    let s = session(&[
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        ("main.py", "def drive(x):\n    return x.poll()\n"),
    ]);
    let ev = queries::ego_graph(&s, Some("poll"), Some("a.py"), None, 1, &["Call"]).unwrap();
    assert!(ev.warnings.iter().any(|w| {
        matches!(w.kind, WarningKind::Collision)
            && w.message.contains('1')
            && w.message
                .contains("unknown receiver type across multiple owner types")
    }));
}

#[test]
fn ego_graph_does_not_emit_collision_warning_when_call_edges_not_collected() {
    let s = session(&[
        ("a.py", "class A:\n    def poll(self):\n        return 1\n"),
        ("b.py", "class B:\n    def poll(self):\n        return 2\n"),
        ("main.py", "def drive(x):\n    return x.poll()\n"),
    ]);
    let dataflow_only =
        queries::ego_graph(&s, Some("poll"), Some("a.py"), None, 1, &["DataFlow"]).unwrap();
    assert!(!dataflow_only
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::Collision)));

    let zero_hops = queries::ego_graph(&s, Some("poll"), Some("a.py"), None, 0, &["Call"]).unwrap();
    assert!(!zero_hops
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::Collision)));
}
