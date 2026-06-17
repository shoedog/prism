use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallKind};
use prism::cpg::CpgContext;
use prism::languages::Language;
use prism::name_resolution::engine::resolve_path;
use prism::name_resolution::rust_policy::{RustPolicy, NS_TYPE, NS_VALUE};
use prism::name_resolution::rust_populator::{enclosing_scope, file_id};
use prism::name_resolution::types::{Anchor, RawPath, ResStatus, SourceLoc};
use prism::navigation::module_graph::module_deps;
use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tempfile::TempDir;

fn write_repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, src) in files {
        let abs = dir.path().join(path);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, src).unwrap();
    }
    dir
}

fn resolve_crate_value(repo: &prism::repo_loader::LoadedRepo, segments: &[&str]) -> ResStatus {
    let graph = repo
        .scope_graph_inputs
        .as_ref()
        .and_then(|_| {
            let ctx = CpgContext::build_with_scope_graph_inputs(
                &repo.files,
                repo.type_db.as_ref(),
                repo.scope_graph_inputs.as_ref(),
            );
            ctx.cpg.call_graph.scope_graph
        })
        .expect("full build should store scope graph");
    let root_path = "src/lib.rs";
    let fid = file_id(&repo.files, root_path).expect("file id");
    let from = enclosing_scope(&graph, fid, 0).expect("crate root scope");
    let policy = RustPolicy::new(&graph, 2021);
    let path = RawPath(segments.iter().map(|s| s.to_string()).collect());
    resolve_path(
        &graph,
        &path,
        NS_VALUE,
        &Anchor::crate_root(),
        from,
        NS_TYPE,
        &SourceLoc { file: fid, byte: 0 },
        &policy,
    )
    .status
}

#[test]
fn whole_workspace_build_populates_complete_scope_graph() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nedition = \"2021\"\n[lib]\npath = \"src/lib.rs\"\n",
        ),
        (
            "src/lib.rs",
            "mod util;\nuse crate::util::target;\nfn caller(){ target(); }\n",
        ),
        ("src/util.rs", "pub fn target(){}\n"),
    ]);
    let repo = load_repo(repo_dir.path()).unwrap();
    let ctx = CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        repo.type_db.as_ref(),
        repo.scope_graph_inputs.as_ref(),
    );

    let graph = ctx
        .cpg
        .call_graph
        .scope_graph
        .as_ref()
        .expect("full build should store scope_graph");
    assert!(graph.scopes.len() > 1, "fixture should produce real scopes");
    assert_eq!(
        resolve_crate_value(&repo, &["util", "target"]),
        ResStatus::Resolved
    );
}

#[test]
fn skeleton_and_subset_do_not_populate_scope_graph() {
    let files = BTreeMap::from([(
        "src/lib.rs".to_string(),
        ParsedFile::parse(
            "src/lib.rs",
            "mod util;\nfn caller(){ f(); }\nfn f(){}\n",
            Language::Rust,
        )
        .unwrap(),
    )]);
    let only = BTreeSet::from(["src/lib.rs".to_string()]);

    assert!(CallGraph::build_skeleton(&files).scope_graph.is_none());
    assert!(CallGraph::build_direct_subset(&files, &only)
        .scope_graph
        .is_none());
}

#[test]
fn incremental_recomputes_scope_graph_from_all_files() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nedition = \"2021\"\n",
        ),
        ("src/lib.rs", "mod a;\nmod b;\n"),
        ("src/a.rs", "pub fn target(){}\n"),
        (
            "src/b.rs",
            "use crate::a::target;\npub fn caller(){ target(); }\n",
        ),
    ]);
    let repo = load_repo(repo_dir.path()).unwrap();
    let full = CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        None,
        repo.scope_graph_inputs.as_ref(),
    );
    let mut changed = BTreeSet::new();
    changed.insert("src/a.rs".to_string());
    let mut cached = full.cpg.call_graph.clone();
    cached.remove_files(&changed);
    let fresh = CallGraph::build_direct_subset(&repo.files, &changed);
    cached.merge(fresh);
    cached.rebuild_scope_graph(&repo.files, repo.scope_graph_inputs.as_ref());

    assert!(
        cached.scope_graph.is_some(),
        "incremental merge should recompute a whole-repo scope graph"
    );
}

#[test]
fn callsite_kind_is_call_and_macros_are_not_sites_in_pr2() {
    let files = BTreeMap::from([(
        "src/lib.rs".to_string(),
        ParsedFile::parse(
            "src/lib.rs",
            "macro_rules! m { () => {} }\nfn f(){}\nfn caller(){ m!(); f(); }\n",
            Language::Rust,
        )
        .unwrap(),
    )]);
    let cg = CallGraph::build(&files);
    let kinds: BTreeMap<String, CallKind> = cg
        .calls
        .values()
        .flat_map(|sites| sites.iter())
        .map(|s| (s.callee_name.clone(), s.kind))
        .collect();

    // MacroInvocation tagging arrives in PR-3; PR-2 keeps macro invocations
    // dropped so call graph/nav behavior stays inert relative to main.
    assert!(!kinds.contains_key("m"), "m!() must not produce a CallSite");
    assert_eq!(kinds.get("f").copied(), Some(CallKind::Call));
}

#[test]
fn macro_invocations_do_not_create_call_graph_or_nav_edges_in_pr2() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "macro_rules! foo { () => {} }\nfn foo() {}\nfn foo_real() {}\nfn g(){ foo!(); foo_real(); }\n",
        ),
    ]);
    let repo = Arc::new(load_repo(repo_dir.path()).unwrap());
    let index = NavigationIndex::build(&repo);
    let cg = &index.cpg.call_graph;
    let g = cg.functions.get("g").unwrap().first().unwrap();
    let sites = cg.calls.get(g).expect("g should have call sites");

    assert!(
        sites.iter().any(|s| s.callee_name == "foo_real"
            && s.kind == CallKind::Call
            && cg
                .resolve_call_site(s)
                .iter()
                .any(|r| r.target.name == "foo_real")),
        "normal foo_real() call should remain in the call graph"
    );
    assert!(
        !sites.iter().any(|s| s.callee_name == "foo"),
        "foo!() must not produce a call site"
    );
    assert!(
        !cg.callers.contains_key("foo"),
        "foo!() must not create a callers entry that can resolve to fn foo"
    );

    let session = NavigationSession {
        repo,
        index: Arc::new(index),
    };
    let ev = queries::callees(&session, Some("g"), Some("src/lib.rs"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|item| matches!(
            &item.symbol,
            Some(SymbolRef::Function { file, name, .. })
                if file == "src/lib.rs" && name == "foo_real"
        )),
        "nav callees should still report the normal foo_real() call"
    );
    assert!(
        !ev.items.iter().any(|item| matches!(
            &item.symbol,
            Some(SymbolRef::Function { file, name, .. })
                if file == "src/lib.rs" && name == "foo"
        )),
        "nav callees must not include a foo!() -> fn foo edge"
    );
}

#[test]
fn inert_nav_and_resolution_outputs_ignore_scope_graph() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "mod util;\nuse crate::util::target;\nfn caller(){ target(); }\n",
        ),
        ("src/util.rs", "pub fn target(){}\n"),
    ]);
    let repo = load_repo(repo_dir.path()).unwrap();
    let repo = Arc::new(repo);
    let nav_with = NavigationIndex::build(&repo);
    let mut nav_without = NavigationIndex::build(&repo);
    nav_without.cpg.call_graph.scope_graph = None;
    let s_with = NavigationSession {
        repo: Arc::clone(&repo),
        index: Arc::new(nav_with),
    };
    let s_without = NavigationSession {
        repo: Arc::clone(&repo),
        index: Arc::new(nav_without),
    };
    assert_eq!(
        serde_json::to_string(&module_deps(&s_with, "src/lib.rs")).unwrap(),
        serde_json::to_string(&module_deps(&s_without, "src/lib.rs")).unwrap(),
        "module-deps must not consume scope_graph in PR-2"
    );

    let site_with = s_with
        .index
        .cpg
        .call_graph
        .calls
        .values()
        .flat_map(|s| s.iter())
        .find(|s| s.callee_name.contains("target"))
        .unwrap();
    let site_without = s_without
        .index
        .cpg
        .call_graph
        .calls
        .values()
        .flat_map(|s| s.iter())
        .find(|s| s.callee_name.contains("target"))
        .unwrap();
    assert_eq!(
        format!(
            "{:?}",
            s_with
                .index
                .cpg
                .call_graph
                .resolve_call_site_full(site_with)
        ),
        format!(
            "{:?}",
            s_without
                .index
                .cpg
                .call_graph
                .resolve_call_site_full(site_without)
        ),
        "resolution output must be unchanged while scope_graph is inert"
    );
}
