use prism::name_resolution::graph::ScopeGraph;
use prism::navigation::module_graph::{module_deps, repo_map};
use prism::navigation::types::{Reason, Source, WarningKind};
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

fn rust_session(files: &[(&str, &str)]) -> NavigationSession {
    let mut owned = vec![(
        "Cargo.toml",
        "[package]\nname = \"fixture\"\nedition = \"2021\"\n[lib]\npath = \"src/lib.rs\"\n",
    )];
    owned.extend_from_slice(files);
    session(&owned)
}

fn resolved_import_targets(ev: &prism::navigation::types::Evidence) -> Vec<String> {
    ev.items
        .iter()
        .filter_map(|it| {
            it.why.iter().find_map(|r| match r {
                Reason::ResolvedImport { target_file, .. } => Some(target_file.clone()),
                _ => None,
            })
        })
        .collect()
}

fn has_resolved_import(
    ev: &prism::navigation::types::Evidence,
    module: &str,
    target: &str,
) -> bool {
    ev.items.iter().any(|it| {
        it.location.file == target
            && it.why.iter().any(|r| {
                matches!(
                    r,
                    Reason::ResolvedImport {
                        module: actual_module,
                        target_file
                    } if actual_module == module && target_file == target
                )
            })
    })
}

fn repo_map_has_edge(
    ev: &prism::navigation::types::Evidence,
    from_file: &str,
    to_file: &str,
) -> bool {
    let graph = ev.graph.as_ref().expect("repo-map graph");
    let from = graph
        .nodes
        .iter()
        .position(|n| n.location.file == from_file)
        .expect("from node");
    let to = graph
        .nodes
        .iter()
        .position(|n| n.location.file == to_file)
        .expect("to node");
    graph
        .edges
        .iter()
        .any(|e| e.from == from && e.to == to && e.kind == "ModuleDep")
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
    // `from util import helper; helper()` is a BARE call (qualifier-less). With R4c
    // import-member resolution, this resolves via the import binding to the target
    // in util.py (import_member). Previously it fell through to R5 free_single.
    assert!(call_item.why.iter().any(|r| matches!(
        r,
        Reason::Resolution { kind } if kind == "import_member"
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
fn module_deps_non_rust_is_byte_identical_when_scope_graph_is_present() {
    let s = session(&[
        ("util.py", "def helper():\n    return 1\n"),
        (
            "main.py",
            "from util import helper\n\ndef run():\n    return helper()\n",
        ),
    ]);
    let baseline = serde_json::to_string(&module_deps(&s, "main.py")).unwrap();

    let mut index = NavigationIndex::build(&s.repo);
    index.cpg.call_graph.scope_graph = Some(ScopeGraph::new());
    let with_graph = NavigationSession {
        repo: Arc::clone(&s.repo),
        index: Arc::new(index),
    };
    assert_eq!(
        serde_json::to_string(&module_deps(&with_graph, "main.py")).unwrap(),
        baseline,
        "non-Rust module-deps output must remain byte-identical"
    );
}

#[test]
fn module_deps_rust_resolves_named_type_external_and_glob_imports() {
    let s = rust_session(&[
        (
            "src/lib.rs",
            "mod prelude;\nmod types;\nmod util;\nuse crate::prelude::*;\nuse crate::types::Config;\nuse crate::util::helper;\nuse std::collections::BTreeMap;\nfn run() -> i32 { helper() }\n",
        ),
        ("src/prelude.rs", "pub fn ready() {}\n"),
        ("src/types.rs", "pub struct Config;\n"),
        ("src/util.rs", "pub fn helper() -> i32 { 1 }\n"),
    ]);
    let ev = module_deps(&s, "src/lib.rs");

    assert!(
        has_resolved_import(&ev, "helper", "src/util.rs"),
        "named function import should resolve to util.rs"
    );
    assert!(
        has_resolved_import(&ev, "Config", "src/types.rs"),
        "type-only import should resolve to types.rs without callable gating"
    );
    assert!(
        has_resolved_import(&ev, "*", "src/prelude.rs"),
        "glob import should resolve to the imported module file without member expansion"
    );
    assert!(
        ev.items.iter().any(|it| {
            matches!(it.source, Source::HeuristicImport)
                && it.why.iter().any(
                    |r| matches!(r, Reason::UnresolvedImport { module } if module == "BTreeMap"),
                )
        }),
        "external std import should surface as an unresolved/external module label"
    );
    assert!(
        !resolved_import_targets(&ev)
            .iter()
            .any(|target| target.contains("std")),
        "external std import must not mint an in-repo edge"
    );

    let repo = repo_map(&s);
    assert!(repo_map_has_edge(&repo, "src/lib.rs", "src/util.rs"));
    assert!(repo_map_has_edge(&repo, "src/lib.rs", "src/types.rs"));
    assert!(repo_map_has_edge(&repo, "src/lib.rs", "src/prelude.rs"));
}

#[test]
fn module_deps_rust_reexport_chain_targets_defining_file() {
    let s = rust_session(&[
        ("src/lib.rs", "pub mod a;\npub mod b;\nmod c;\n"),
        ("src/a.rs", "pub use crate::b::thing;\n"),
        ("src/b.rs", "pub fn thing() {}\n"),
        ("src/c.rs", "use crate::a::thing;\nfn run() { thing(); }\n"),
    ]);
    let ev = module_deps(&s, "src/c.rs");

    assert!(
        has_resolved_import(&ev, "thing", "src/b.rs"),
        "re-export import should resolve through a.rs to the defining b.rs file"
    );
    assert!(
        !has_resolved_import(&ev, "thing", "src/a.rs"),
        "re-export import must not stop at the facade file"
    );
}

#[test]
fn module_deps_rust_without_authoritative_scope_graph_keeps_existing_fallback() {
    let s = session(&[
        ("util.rs", "pub fn helper() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod util;\nuse util::helper;\nfn run() -> i32 { helper() }\n",
        ),
    ]);
    let mut index = NavigationIndex::build(&s.repo);
    index.cpg.call_graph.scope_graph = None;
    let no_graph = NavigationSession {
        repo: Arc::clone(&s.repo),
        index: Arc::new(index),
    };
    let ev = module_deps(&no_graph, "main.rs");

    assert!(
        ev.items
            .iter()
            .any(|it| matches!(it.source, Source::PrismCpg) && it.location.file == "util.rs"),
        "fallback should retain the existing call-derived edge"
    );
    assert!(
        ev.items.iter().all(|it| !it
            .why
            .iter()
            .any(|r| matches!(r, Reason::ResolvedImport { .. }))),
        "fallback must not mint scope-graph import edges"
    );
}

#[test]
fn module_deps_uses_resolution_score_and_reason() {
    let s = session(&[
        (
            "owner.py",
            "class OnlyOwner:\n    def frobnicate(self):\n        return 1\n",
        ),
        ("main.py", "def run(x):\n    return x.frobnicate()\n"),
    ]);
    let ev = module_deps(&s, "main.py");
    let item = ev
        .items
        .iter()
        .find(|it| matches!(it.source, Source::PrismCpg) && it.location.file == "owner.py")
        .expect("call-derived dependency to owner.py");
    assert_eq!(item.score, 0.6);
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "r6_single_owner")));
}

#[test]
fn module_deps_aggregates_file_pair_with_max_resolution_score() {
    let s = session(&[
        (
            "owner.py",
            "class OnlyOwner:\n    def frobnicate(self):\n        return 1\n\ndef exact():\n    return 2\n",
        ),
        (
            "main.py",
            "from owner import exact\n\ndef run(x):\n    x.frobnicate()\n    return exact()\n",
        ),
    ]);
    let ev = module_deps(&s, "main.py");
    let item = ev
        .items
        .iter()
        .find(|it| matches!(it.source, Source::PrismCpg) && it.location.file == "owner.py")
        .expect("call-derived dependency to owner.py");
    assert_eq!(item.score, 1.0);
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "r6_single_owner")));
    // `from owner import exact; exact()` now resolves via R4c import_member.
    assert!(item
        .why
        .iter()
        .any(|r| matches!(r, Reason::Resolution { kind } if kind == "import_member")));
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
