use prism::navigation::module_graph::{module_deps, repo_map};
use prism::navigation::types::{Source, WarningKind};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

#[test]
fn module_deps_python_cross_file_call_and_import() {
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        (
            "main.py",
            "from util import helper\n\ndef run():\n    return helper()\n",
        ),
    ]);
    let ev = module_deps(&s, "main.py");

    // Call-derived cross-file edge main.py -> util.py (PrismCpg).
    let call_item = ev
        .items
        .iter()
        .find(|it| matches!(it.source, Source::PrismCpg))
        .expect("a PrismCpg call-derived dependency");
    assert_eq!(call_item.location.file, "util.py");
    assert!(call_item.why.iter().any(|r| matches!(
        r,
        prism::navigation::types::Reason::Calls { callee, .. } if callee == "helper"
    )));

    // Extracted import labeled UnresolvedImport (HeuristicImport).
    assert!(ev
        .items
        .iter()
        .any(|it| matches!(it.source, Source::HeuristicImport)
            && it.why.iter().any(|r| matches!(
                r,
                prism::navigation::types::Reason::UnresolvedImport { module } if module == "util"
            ))));
    assert!(ev
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::UnresolvedModule)));
    assert!(
        ev.graph.is_none(),
        "module-deps is a flat item list, not a graph"
    );
}

#[test]
fn module_deps_rust_is_call_derived_only_no_import_items() {
    // NON-VACUOUS: an UNQUALIFIED use-imported call resolves cross-file by name
    // (`util::helper()` scoped would NOT — see Design-decision #4 Rust caveat).
    let s = session(&[
        ("util.rs", "pub fn helper() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod util;\nuse util::helper;\nfn run() -> i32 { helper() }\n",
        ),
    ]);
    let ev = module_deps(&s, "main.rs");
    // At least one call-derived cross-file edge to util.rs (proves the tier works).
    assert!(
        ev.items
            .iter()
            .any(|it| matches!(it.source, Source::PrismCpg) && it.location.file == "util.rs"),
        "expected a call-derived edge main.rs -> util.rs"
    );
    // Rust extracts no imports: every item is PrismCpg, and no UnresolvedModule warning.
    assert!(
        ev.items
            .iter()
            .all(|it| matches!(it.source, Source::PrismCpg)),
        "Rust (call-derived-only tier) must surface no HeuristicImport items"
    );
    assert!(
        !ev.warnings
            .iter()
            .any(|w| matches!(w.kind, WarningKind::UnresolvedModule)),
        "no extracted imports -> no UnresolvedModule warning"
    );
}

#[test]
fn module_deps_skipped_or_unknown_file_warns_not_errors() {
    let s = session(&[("util.py", "def helper():\n    return 1\n")]);
    let ev = module_deps(&s, "nope.py"); // not in the index -> empty + SkippedPath (no panic/error)
    assert!(ev.items.is_empty());
    assert!(ev
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::SkippedPath)));
}

#[test]
fn repo_map_emits_whole_repo_file_graph() {
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        (
            "main.py",
            "from util import helper\n\ndef run():\n    return helper()\n",
        ),
        ("lonely.py", "x = 1\n"), // isolated file: must still appear as a node
    ]);
    let ev = repo_map(&s);
    assert_eq!(ev.query, "repo-map");
    assert!(
        ev.items.is_empty(),
        "repo-map carries result in graph, not items"
    );
    let g = ev.graph.as_ref().expect("repo-map returns Some(graph)");
    assert!(
        g.nodes.iter().all(|n| n.symbol.is_none()),
        "file nodes have no symbol"
    );
    for f in ["main.py", "util.py", "lonely.py"] {
        assert!(g.nodes.iter().any(|n| n.location.file == f), "node for {f}");
    }
    let main_i = g
        .nodes
        .iter()
        .position(|n| n.location.file == "main.py")
        .unwrap();
    let util_i = g
        .nodes
        .iter()
        .position(|n| n.location.file == "util.py")
        .unwrap();
    assert!(g
        .edges
        .iter()
        .any(|e| e.from == main_i && e.to == util_i && e.kind == "ModuleDep"));
    assert!(ev
        .warnings
        .iter()
        .any(|w| matches!(w.kind, WarningKind::UnresolvedModule)));
}
