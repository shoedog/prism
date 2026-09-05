use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallKind};
use prism::cpg::{CodePropertyGraph, CpgContext};
use prism::languages::Language;
use prism::name_resolution::engine::resolve_path;
use prism::name_resolution::rust_policy::{RustPolicy, NS_TYPE, NS_VALUE};
use prism::name_resolution::rust_populator::{enclosing_scope, file_id};
use prism::name_resolution::types::{Anchor, RawPath, ResStatus, SourceLoc};
use prism::navigation::module_graph::module_deps;
use prism::navigation::types::{Reason, SymbolRef};
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::{load_repo, scope_graph_build_inputs};
use prism::resolution_identity::{resolve_type_path_to_type_scope, ReceiverTypeKey, TypeKey};
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
fn binary_can_call_own_named_library() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "pub mod api { pub fn target() {} }\nfn control() { crate::api::target(); }",
        ),
        ("src/main.rs", "fn main() { demo::api::target(); }"),
    ]);
    let repo = load_repo(repo_dir.path()).unwrap();
    let ctx = CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        repo.type_db.as_ref(),
        repo.scope_graph_inputs.as_ref(),
    );
    let cg = &ctx.cpg.call_graph;
    for caller in ["control", "main"] {
        let function = &cg.functions[caller][0];
        let site = cg.calls[function].iter().next().unwrap();
        let targets = cg.resolve_call_site(site);
        assert_eq!(targets.len(), 1, "{caller}: {site:?}");
        assert_eq!(targets[0].target.file, "src/lib.rs");
        assert_eq!(targets[0].target.name, "target");
    }
}

#[test]
fn binary_library_lexical_and_visibility_barriers() {
    let mut failures = Vec::new();
    for (library, body) in [
        ("pub(crate) fn target(){}", "demo::target();"),
        ("fn target(){}", "demo::target();"),
        ("pub fn target(){}", "mod demo {} demo::target();"),
        ("pub fn target(){}", "struct demo; demo::target();"),
        ("pub fn target(){}", "type demo = Other; demo::target();"),
        ("pub fn target(){}", "trait demo {} demo::target();"),
        (
            "pub fn target(){}",
            "extern crate unknown as demo; demo::target();",
        ),
        (
            "pub fn target(){}",
            "mod local { pub mod demo {} } use local::*; demo::target();",
        ),
        ("pub fn target(){}", "fn nested<demo>() { demo::target(); }"),
        (
            "pub fn target(){}",
            "struct S; impl S { fn nested<demo>() { demo::target(); } }",
        ),
        (
            "pub fn target(){}",
            "struct S<T>(T); impl<demo> S<demo> { fn nested() { demo::target(); } }",
        ),
        ("pub fn target(){}", "crate::demo::target();"),
        ("pub fn target(){}", "::demo::target();"),
    ] {
        let dir = write_repo(&[
            (
                "Cargo.toml",
                "[package]\nname='demo'\nversion='0.1.0'\nedition='2021'",
            ),
            ("src/lib.rs", library),
            ("src/main.rs", &format!("fn main(){{{body}}}")),
        ]);
        let repo = load_repo(dir.path()).unwrap();
        let ctx = CpgContext::build_with_scope_graph_inputs(
            &repo.files,
            None,
            repo.scope_graph_inputs.as_ref(),
        );
        let cg = &ctx.cpg.call_graph;
        let site = cg
            .calls
            .values()
            .flat_map(|s| s.iter())
            .find(|s| s.callee_name.ends_with("::target"))
            .unwrap();
        let edges = cg.resolve_call_site(site);
        if edges
            .iter()
            .any(|e| e.confidence == prism::resolution::ResolutionConfidence::Exact)
        {
            failures.push(format!("{library} / {body}: {edges:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn binary_library_manifest_names_paths_and_shadowing() {
    for (manifest, binary, library, source, expected) in [
        ("[package]\nname='demo-kit'\nversion='0.1.0'\nedition='2021'", "src/main.rs", "src/lib.rs", "fn main(){demo_kit::api::target();}", Some("src/lib.rs")),
        ("[package]\nname='package'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='api_crate'\npath='lib/entry.rs'\n[[bin]]\nname='tool'\npath='tools/run.rs'", "tools/run.rs", "lib/entry.rs", "fn main(){api_crate::api::target();}", Some("lib/entry.rs")),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'", "src/bin/tool/main.rs", "src/lib.rs", "use demo::api::target; fn main(){target();}", Some("src/lib.rs")),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'", "src/bin/tool.rs", "src/lib.rs", "mod demo { pub mod api { pub fn target(){} } } fn main(){demo::api::target();}", Some("src/bin/tool.rs")),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2015'", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\n[[bin]]\nname='demo'\nedition='2015'", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\nautobins=false", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\nautolib=false", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='different'", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\n[lib]\npath='missing.rs'", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\n[lib]\nproc-macro=true", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\n[lib]\ncrate-type=['cdylib']", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
        ("[package]\nname='demo'\nversion='0.1.0'\nedition='2021'\n[dependencies]\ndemo='1'", "src/main.rs", "src/lib.rs", "fn main(){demo::api::target();}", None),
    ] {
        let dir = write_repo(&[("Cargo.toml", manifest), (binary, source), (library, "pub mod api { pub fn target(){} }")]);
        let repo = Arc::new(load_repo(dir.path()).unwrap());
        let index = Arc::new(NavigationIndex::build(&repo));
        let cg = index.call_graph();
        let site = cg.calls[&cg.functions["main"][0]].iter().next().unwrap();
        let targets = cg.resolve_call_site(site);
        let exact: Vec<_> = targets.iter().filter(|t| t.confidence == prism::resolution::ResolutionConfidence::Exact).collect();
        assert_eq!(exact.len(), usize::from(expected.is_some()), "{manifest}\n{source}: {targets:?}");
        if let Some(expected) = expected {
            assert_eq!(exact[0].target.file, expected);
            let session = NavigationSession { repo, index };
            let callees = queries::callees(&session, Some("main"), Some(binary), None, 1).unwrap();
            assert!(callees.items.iter().any(|item| matches!(&item.symbol, Some(SymbolRef::Function { file, name, .. }) if file == expected && name == "target")));
            let callers = queries::callers(&session, Some("target"), Some(expected), None, 1).unwrap();
            assert!(callers.items.iter().any(|item| matches!(&item.symbol, Some(SymbolRef::Function { file, name, .. }) if file == binary && name == "main")));
        }
    }
}

#[test]
fn binary_library_roots_do_not_cross_workspace_members() {
    let dir = write_repo(&[
        ("Cargo.toml", "[workspace]\nmembers=['a','b']"),
        ("a/Cargo.toml", "[package]\nname='a'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='shared'\npath='lib/entry.rs'"),
        ("b/Cargo.toml", "[package]\nname='b'\nversion='0.1.0'\nedition='2021'\n[lib]\nname='shared'\npath='other/entry.rs'"),
        ("a/lib/entry.rs", "pub fn target(){}"),
        ("b/other/entry.rs", "pub fn target(){}"),
        ("a/src/main.rs", "fn main(){shared::target();}"),
        ("b/src/main.rs", "fn main(){shared::target();}"),
    ]);
    let repo = load_repo(dir.path()).unwrap();
    let ctx = CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        repo.type_db.as_ref(),
        repo.scope_graph_inputs.as_ref(),
    );
    let cg = &ctx.cpg.call_graph;
    for (binary, library) in [
        ("a/src/main.rs", "a/lib/entry.rs"),
        ("b/src/main.rs", "b/other/entry.rs"),
    ] {
        let caller = cg.functions["main"]
            .iter()
            .find(|f| f.file == binary)
            .unwrap();
        let site = cg.calls[caller].iter().next().unwrap();
        let targets = cg.resolve_call_site(site);
        assert_eq!(targets.len(), 1, "{binary}: {targets:?}");
        assert_eq!(targets[0].target.file, library);
    }
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
    assert!(
        repo.scope_graph_inputs.as_ref().unwrap().complete,
        "whole workspace load should mark scope graph inputs complete"
    );
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
    assert!(graph.complete, "stored full-build graph must be complete");
    assert!(graph.scopes.len() > 1, "fixture should produce real scopes");
    assert_eq!(
        resolve_crate_value(&repo, &["util", "target"]),
        ResStatus::Resolved
    );
}

#[test]
fn review_diff_only_build_does_not_store_authoritative_scope_graph() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nedition = \"2021\"\n",
        ),
        ("src/lib.rs", "mod util;\nfn caller(){ util::target(); }\n"),
        ("src/util.rs", "pub fn target(){}\n"),
    ]);
    let changed_only = BTreeMap::from([(
        "src/lib.rs".to_string(),
        ParsedFile::parse(
            "src/lib.rs",
            "mod util;\nfn caller(){ util::target(); }\n",
            Language::Rust,
        )
        .unwrap(),
    )]);
    let inputs = scope_graph_build_inputs(repo_dir.path(), &changed_only);
    assert!(
        !inputs.complete,
        "review/diff-only builds parse a subset and must not be authoritative"
    );

    let ctx = CpgContext::build_with_scope_graph_inputs(&changed_only, None, Some(&inputs));
    assert!(
        ctx.cpg.call_graph.scope_graph.is_none(),
        "incomplete review builds must leave scope_graph unset"
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
fn incremental_rebuild_rematerializes_receiver_outcome_no_stale_scopeid() {
    let parse_files = |model_src: &str| {
        BTreeMap::from([
            (
                "src/lib.rs".to_string(),
                ParsedFile::parse(
                    "src/lib.rs",
                    "mod model;\n\
                     use crate::model::Outer;\n\
                     pub fn run(o: Outer) { let x = o.inner; x.poke(); }\n",
                    Language::Rust,
                )
                .unwrap(),
            ),
            (
                "src/model.rs".to_string(),
                ParsedFile::parse("src/model.rs", model_src, Language::Rust).unwrap(),
            ),
        ])
    };
    let initial_model = "pub struct Inner;\n\
                         impl Inner { pub fn poke(&self) {} }\n\
                         pub struct Inner2;\n\
                         impl Inner2 { pub fn poke(&self) {} }\n\
                         pub struct Outer { pub inner: Inner }\n";
    let updated_model = "pub struct Inner;\n\
                         impl Inner { pub fn poke(&self) {} }\n\
                         pub struct Inner2;\n\
                         impl Inner2 { pub fn poke(&self) {} }\n\
                         pub struct Outer { pub inner: Inner2 }\n";

    let initial_files = parse_files(initial_model);
    let initial_inputs =
        prism::call_graph::ScopeGraphBuildInputs::from_files_convention(&initial_files);
    let initial =
        CpgContext::build_with_scope_graph_inputs(&initial_files, None, Some(&initial_inputs));

    let updated_files = parse_files(updated_model);
    let updated_inputs =
        prism::call_graph::ScopeGraphBuildInputs::from_files_convention(&updated_files);
    let changed = BTreeSet::from(["src/model.rs".to_string()]);
    let incremental = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
        initial.cpg.call_graph.clone(),
        initial.cpg.dfg.clone(),
        &changed,
        &updated_files,
        None,
        Some(&updated_inputs),
    );
    let cg = &incremental.call_graph;

    let graph = cg.scope_graph.as_ref().expect("fresh scope graph");
    let lib_file = file_id(&updated_files, "src/lib.rs").expect("lib file id");
    let lib_scope = enclosing_scope(graph, lib_file, 0).expect("lib module scope");
    let inner2_scope =
        match resolve_type_path_to_type_scope(graph, lib_scope, "crate::model::Inner2") {
            Some(TypeKey::InRepo(scope)) => scope,
            other => panic!("expected current Inner2 scope, got {other:?}"),
        };
    let inner_scope = match resolve_type_path_to_type_scope(graph, lib_scope, "crate::model::Inner")
    {
        Some(TypeKey::InRepo(scope)) => scope,
        other => panic!("expected current Inner scope, got {other:?}"),
    };
    assert_ne!(inner_scope, inner2_scope, "decoy scopes must be distinct");

    let site = cg
        .calls
        .iter()
        .find(|(fid, _)| fid.name == "run")
        .and_then(|(_, sites)| sites.iter().find(|site| site.callee_name == "poke"))
        .cloned()
        .expect("run -> poke call site");
    let outcome = site
        .receiver_outcome
        .expect("receiver outcome rematerialized");
    assert_eq!(outcome.key, ReceiverTypeKey::InRepo(inner2_scope));
    assert_ne!(outcome.key, ReceiverTypeKey::InRepo(inner_scope));

    let callers_outcome = cg
        .callers
        .get("poke")
        .and_then(|sites| sites.iter().find(|site| site.caller.name == "run"))
        .and_then(|site| site.receiver_outcome.clone())
        .expect("callers index receiver outcome rematerialized");
    assert_eq!(callers_outcome, outcome);
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
            "macro_rules! foo { () => {} }\nfn foo() {}\nfn foo_real() {}\nfn g(){ foo_real(); foo!(); }\n",
        ),
    ]);
    let repo = Arc::new(load_repo(repo_dir.path()).unwrap());
    let index = NavigationIndex::build(&repo);
    let cg = index.call_graph();
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
fn malformed_member_manifest_does_not_discard_valid_sibling_manifest() {
    let repo_dir = write_repo(&[
        ("Cargo.toml", "[workspace]\nmembers = [\"good\", \"bad\"]\n"),
        (
            "good/Cargo.toml",
            "[package]\nname = \"good\"\nedition = \"2021\"\n[lib]\npath = \"custom_lib.rs\"\n",
        ),
        ("good/custom_lib.rs", "pub fn good(){}\n"),
        ("bad/Cargo.toml", "[package\nname = \"bad\"\n"),
        ("bad/src/lib.rs", "pub fn bad(){}\n"),
    ]);
    let repo = load_repo(repo_dir.path()).unwrap();
    let cfg = &repo.scope_graph_inputs.as_ref().unwrap().cfg;

    assert_eq!(cfg.edition, 2021, "valid sibling edition must survive");
    assert_eq!(
        cfg.lib_path.as_deref(),
        Some("good/custom_lib.rs"),
        "valid sibling [lib] path must survive"
    );
    assert!(
        cfg.crate_roots.iter().any(|p| p == "good/custom_lib.rs"),
        "valid sibling manifest root must survive"
    );
    assert!(
        cfg.crate_roots.iter().any(|p| p == "bad/src/lib.rs"),
        "malformed sibling should fall back to convention for that member"
    );
    assert_eq!(
        cfg.workspace_members,
        vec!["bad".to_string(), "good".to_string()],
        "workspace members from the valid root manifest must survive"
    );
}

#[test]
fn glob_member_workspace_resolves_cross_crate_glob_facade_type() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[workspace]\nmembers = [\"crates/*\"]\n[workspace.package]\nedition = \"2021\"\n",
        ),
        (
            "crates/foo/Cargo.toml",
            "[package]\nname = \"foo\"\nedition = \"2021\"\n",
        ),
        (
            "crates/foo/src/lib.rs",
            "pub use inner::*;\nmod inner { pub struct SomeType; }\n",
        ),
        (
            "crates/bar/Cargo.toml",
            "[package]\nname = \"bar\"\nedition = \"2021\"\n[dependencies]\nfoo = { path = \"../foo\" }\n",
        ),
        (
            "crates/bar/src/lib.rs",
            "use foo::SomeType;\npub fn bar(_: Option<SomeType>) {}\n",
        ),
    ]);
    let repo = load_repo(repo_dir.path()).unwrap();
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

    let bar_file = file_id(&repo.files, "crates/bar/src/lib.rs").expect("bar lib file id");
    let from = enclosing_scope(&graph, bar_file, 0).expect("bar root scope");
    let policy = RustPolicy::new(&graph, 2021);
    let res = resolve_path(
        &graph,
        &RawPath(vec!["foo".to_string(), "SomeType".to_string()]),
        NS_TYPE,
        &Anchor::use_path_2015(),
        from,
        NS_TYPE,
        &SourceLoc {
            file: bar_file,
            byte: 0,
        },
        &policy,
    );

    assert_eq!(
        res.status,
        ResStatus::Resolved,
        "bar's `use foo::SomeType` must cross the glob workspace dep and foo's pub-use glob facade"
    );
    assert_eq!(res.candidates.len(), 1);
}

#[test]
fn module_deps_consumes_scope_graph_but_resolution_output_stays_inert() {
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
    let nav_without = NavigationIndex::build(&repo).with_modified_cpg_for_testing(|cpg| {
        cpg.call_graph.scope_graph = None;
    });
    let s_with = NavigationSession {
        repo: Arc::clone(&repo),
        index: Arc::new(nav_with),
    };
    let s_without = NavigationSession {
        repo: Arc::clone(&repo),
        index: Arc::new(nav_without),
    };
    let deps_with = module_deps(&s_with, "src/lib.rs");
    let deps_without = module_deps(&s_without, "src/lib.rs");
    assert!(
        deps_with.items.iter().any(|it| {
            it.why.iter().any(|r| {
                matches!(
                    r,
                    Reason::ResolvedImport {
                        module,
                        target_file
                    } if module == "target" && target_file == "src/util.rs"
                )
            })
        }),
        "authoritative scope graph should add the resolved Rust import edge"
    );
    assert!(
        deps_without.items.iter().all(|it| {
            !it.why
                .iter()
                .any(|r| matches!(r, Reason::ResolvedImport { .. }))
        }),
        "without a scope graph module-deps must keep the existing fallback"
    );

    let site_with = s_with
        .index
        .call_graph()
        .calls
        .values()
        .flat_map(|s| s.iter())
        .find(|s| s.callee_name.contains("target"))
        .unwrap();
    let site_without = s_without
        .index
        .call_graph()
        .calls
        .values()
        .flat_map(|s| s.iter())
        .find(|s| s.callee_name.contains("target"))
        .unwrap();
    assert_eq!(
        format!(
            "{:?}",
            s_with.index.call_graph().resolve_call_site_full(site_with)
        ),
        format!(
            "{:?}",
            s_without
                .index
                .call_graph()
                .resolve_call_site_full(site_without)
        ),
        "resolution output must be unchanged while scope_graph is inert"
    );
}
