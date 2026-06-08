use prism::navigation::types::{QueryError, SymbolRef};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(src: &str) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.py"), src).unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn ego_includes_seed_and_call_edge() {
    let s = session("def helper():\n    return 1\n\ndef caller():\n    return helper()\n");
    let ev = queries::ego_graph(&s, Some("caller"), None, None, 1, &["Call"]).unwrap();
    let g = ev.graph.as_ref().expect("ego returns Some(graph)");
    assert!(
        ev.items.is_empty(),
        "ego carries its result in graph, not items"
    );
    assert!(g
        .nodes
        .iter()
        .any(|n| matches!(&n.symbol, Some(SymbolRef::Function { name, .. }) if name == "caller")));
    assert!(
        !g.edges.is_empty(),
        "expected at least one Call edge from the seed"
    );
}

#[test]
fn ego_location_seed_centers_exact_variable_node() {
    let s = session("def f():\n    x = 1\n    return x\n");
    let ev = queries::ego_graph(&s, None, None, Some("a.py:2"), 0, &["DataFlow"]).unwrap();
    let g = ev.graph.as_ref().expect("ego returns Some(graph)");
    assert!(g.nodes.iter().any(|n| {
        matches!(&n.symbol, Some(SymbolRef::Variable { function, line, .. }) if function == "f" && *line == 2)
    }));
    assert!(!g
        .nodes
        .iter()
        .any(|n| { matches!(&n.symbol, Some(SymbolRef::Function { name, .. }) if name == "f") }));
}

#[test]
fn ego_edges_trim_whitespace() {
    let s = session("def helper():\n    return 1\n\ndef caller():\n    return helper()\n");
    let ev = queries::ego_graph(&s, Some("caller"), None, None, 1, &["Call", " Return"]).unwrap();
    let g = ev.graph.as_ref().expect("ego returns Some(graph)");
    assert!(g.edges.iter().any(|e| e.kind == "Call"));
    assert!(g.edges.iter().any(|e| e.kind == "Return"));
}

#[test]
fn ego_edges_reject_unknown_name() {
    let s = session("def f():\n    return 1\n");
    let err = queries::ego_graph(&s, Some("f"), None, None, 1, &["Bogus"]).unwrap_err();
    assert!(matches!(err, QueryError::UnknownEdge { edge } if edge == "Bogus"));
}
