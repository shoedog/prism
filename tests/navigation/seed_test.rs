use prism::navigation::types::{QueryError, SymbolRef};
use prism::navigation::{seed, NavigationIndex, NavigationSession};
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
fn resolves_symbol_unique() {
    let s = session(&[("a.py", "def foo():\n    return 1\n")]);
    let r = seed::resolve_fn(&s, Some("foo"), None, None).unwrap();
    assert!(matches!(r.symbol, SymbolRef::Function { ref name, .. } if name == "foo"));
}

#[test]
fn resolves_location_to_enclosing() {
    let s = session(&[("a.py", "def foo():\n    x = 1\n    return x\n")]);
    let r = seed::resolve_fn(&s, None, None, Some("a.py:2")).unwrap();
    assert!(matches!(r.symbol, SymbolRef::Function { ref name, .. } if name == "foo"));
}

#[test]
fn location_resolves_to_innermost_nested_function() {
    // R2 major: the innermost-enclosing property of line_range_index (spec §16/R2-M3).
    // Line 3 is inside `inner`, which is nested in `outer`.
    let s = session(&[(
        "a.py",
        "def outer():\n    def inner():\n        return 1\n    return inner()\n",
    )]);
    let r = seed::resolve_fn(&s, None, None, Some("a.py:3")).unwrap();
    assert!(
        matches!(r.symbol, SymbolRef::Function { ref name, .. } if name == "inner"),
        "expected innermost `inner`, got {:?}",
        r.symbol
    );
}

#[test]
fn ambiguous_symbol_errors_with_candidates() {
    // Naming-robust: two TOP-LEVEL `dup` functions in two files (m11).
    let s = session(&[
        ("a.py", "def dup():\n    return 1\n"),
        ("b.py", "def dup():\n    return 2\n"),
    ]);
    match seed::resolve_fn(&s, Some("dup"), None, None) {
        Err(QueryError::AmbiguousSymbol { candidates }) => assert_eq!(candidates.len(), 2),
        other => panic!("expected AmbiguousSymbol, got {other:?}"),
    }
}
