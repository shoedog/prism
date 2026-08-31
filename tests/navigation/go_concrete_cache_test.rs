use prism::navigation::types::SymbolRef;
use prism::navigation::{cache::nav_cache_subdir, queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use prism::resolution::GoOwnerIdentity;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex, OnceLock};

const DIRTY_LOAD_OVERRIDE: &str = "PRISM_NAV_EDGE_CACHE_LOAD_DIRTY";
static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct EnvRestore(Option<std::ffi::OsString>);

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match self.0.take() {
            Some(value) => std::env::set_var(DIRTY_LOAD_OVERRIDE, value),
            None => std::env::remove_var(DIRTY_LOAD_OVERRIDE),
        }
    }
}

fn write(root: &std::path::Path, path: &str, source: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("fixture parent")).unwrap();
    std::fs::write(path, source).unwrap();
}

fn wire_outputs(session: &NavigationSession) -> (Vec<u8>, Vec<u8>) {
    let cg = session.index.call_graph();
    (
        serde_json::to_vec(&queries::interface_dispatch_manifest(cg)).unwrap(),
        serde_json::to_vec(&queries::call_stats(cg)).unwrap(),
    )
}

fn evidence_has_file(evidence: &prism::navigation::types::Evidence, file: &str) -> bool {
    evidence.items.iter().any(|item| {
        matches!(
            item.symbol.as_ref(),
            Some(SymbolRef::Function { file: target, .. }) if target == file
        )
    })
}

fn evidence_has_function(
    evidence: &prism::navigation::types::Evidence,
    file: &str,
    name: &str,
) -> bool {
    evidence.items.iter().any(|item| {
        matches!(
            item.symbol.as_ref(),
            Some(SymbolRef::Function {
                file: target_file,
                name: target_name,
                ..
            }) if target_file == file && target_name == name
        )
    })
}

#[test]
fn receiver_owner_missing_is_terminal_for_navigation_sidecar() {
    let repo_dir = tempfile::tempdir().unwrap();
    write(
        repo_dir.path(),
        "go.mod",
        "module example.com/root\n\ngo 1.22\n",
    );
    write(
        repo_dir.path(),
        "api/types.go",
        "package api\ntype I interface{ M(); ApiOnly() }\ntype Real struct{}\nfunc (Real) M() {}\nfunc (Real) ApiOnly() {}\nfunc retain() { var _ I = Real{} }\n",
    );
    write(
        repo_dir.path(),
        "decoy/types.go",
        "package decoy\ntype I interface{ M(); DecoyOnly() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\nfunc (Wrong) DecoyOnly() {}\nfunc retain() { var _ I = Wrong{} }\n",
    );
    write(
        repo_dir.path(),
        "app/vars.go",
        "package app\nimport ext \"example.com/root/api\"\nvar Shared ext.I\n",
    );
    write(
        repo_dir.path(),
        "app/use.go",
        "package app\nimport ext \"example.com/root/decoy\"\nfunc run() { Shared.M() }\n",
    );
    let repo = Arc::new(load_repo(repo_dir.path()).unwrap());

    let proven = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build(&repo)),
    };
    let proven_edges = queries::callees(&proven, Some("run"), Some("app/use.go"), None, 1).unwrap();
    assert!(evidence_has_file(&proven_edges, "api/types.go"));
    assert!(!evidence_has_file(&proven_edges, "decoy/types.go"));

    let ownerless_index = NavigationIndex::build(&repo).with_modified_cpg_for_testing(|cpg| {
        let cg = &mut cpg.call_graph;
        let original = cg
            .calls
            .values()
            .flatten()
            .find(|site| {
                site.caller.file == "app/use.go"
                    && site.caller.name == "run"
                    && site.callee_name == "M"
            })
            .expect("run M site")
            .clone();
        assert!(original.receiver_type.is_some(), "{original:?}");
        assert!(original.receiver_recovery.is_some(), "{original:?}");
        assert!(original.receiver_owner_identity.is_some(), "{original:?}");

        let mut ownerless = original.clone();
        ownerless.receiver_owner_identity = None;
        let calls = cg
            .calls
            .get_mut(&original.caller)
            .expect("run caller bucket");
        assert!(calls.remove(&original));
        assert!(calls.insert(ownerless));
        let reverse = cg
            .callers
            .get_mut(&original.callee_name)
            .expect("M reverse bucket")
            .iter_mut()
            .find(|candidate| **candidate == original)
            .expect("mirrored run M site");
        reverse.receiver_owner_identity = None;
    });
    let ownerless = NavigationSession {
        repo,
        index: Arc::new(ownerless_index),
    };
    let ownerless_edges =
        queries::callees(&ownerless, Some("run"), Some("app/use.go"), None, 1).unwrap();
    assert!(!evidence_has_file(&ownerless_edges, "api/types.go"));
    assert!(!evidence_has_file(&ownerless_edges, "decoy/types.go"));
}

#[test]
fn concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _restore = EnvRestore(std::env::var_os(DIRTY_LOAD_OVERRIDE));
    std::env::set_var(DIRTY_LOAD_OVERRIDE, "1");

    let repo_dir = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    write(repo_dir.path(), "go.mod", "module example\n\ngo 1.22\n");
    write(
        repo_dir.path(),
        "q/types.go",
        "package q\ntype A struct{}\nfunc (A) M() {}\n",
    );
    write(
        repo_dir.path(),
        "p/types.go",
        "package p\ntype A interface{ M() }\ntype Wrong struct{}\nfunc (Wrong) M() {}\n",
    );
    write(
        repo_dir.path(),
        "decoy/closer.go",
        "package decoy\ntype Closer interface{ Close() error }\ntype WrongCloser struct{}\nfunc (*WrongCloser) Close() error { return nil }\nfunc retain() { _ = &WrongCloser{} }\n",
    );
    write(
        repo_dir.path(),
        "decoy/error.go",
        "package decoy\ntype error interface{ Error() string }\ntype WrongError struct{}\nfunc (WrongError) Error() string { return \"wrong\" }\nfunc retainError() { _ = WrongError{} }\n",
    );
    write(
        repo_dir.path(),
        "app/use.go",
        "package app\nimport (\"io\"; q \"example/q\"; ext \"example/p\")\nfunc run(a q.A) { a.M() }\nfunc packageRun() { Shared.M() }\nfunc external() { _ = func(cl io.Closer) error { return cl.Close() }(nil) }\nfunc poison(e error) string { return e.Error() }\n",
    );
    write(
        repo_dir.path(),
        "app/vars.go",
        "package app\nimport ext \"example/q\"\nvar Shared ext.A\n",
    );
    write(
        repo_dir.path(),
        "app/dot.go",
        "package app\nimport . \"example/q\"\nvar _ A\n",
    );
    let repo = Arc::new(load_repo(repo_dir.path()).unwrap());

    let no_cache = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build(&repo)),
    };
    let cold_create = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build_cached_under(&repo, cache_dir.path())),
    };
    let exact_cpg = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build_cached_under(&repo, cache_dir.path())),
    };
    assert_eq!(wire_outputs(&no_cache), wire_outputs(&cold_create));
    assert_eq!(wire_outputs(&no_cache), wire_outputs(&exact_cpg));
    let expected_dot_imports = BTreeSet::from(["app/dot.go".to_string()]);
    for session in [&no_cache, &cold_create, &exact_cpg] {
        assert_eq!(
            session.index.call_graph().go_dot_import_files,
            expected_dot_imports
        );
        let package_site = session
            .index
            .call_graph()
            .calls
            .values()
            .flatten()
            .find(|site| {
                site.caller.file == "app/use.go"
                    && site.caller.name == "packageRun"
                    && site.callee_name == "M"
            })
            .expect("packageRun M site");
        assert_eq!(
            package_site.receiver_owner_identity.as_ref(),
            Some(&GoOwnerIdentity {
                package_dir: "q".to_string(),
                package_clause: "q".to_string(),
                name: "A".to_string(),
            }),
            "{package_site:?}"
        );
        let local_site = session
            .index
            .call_graph()
            .calls
            .values()
            .flatten()
            .find(|site| {
                site.caller.file == "app/use.go"
                    && site.caller.name == "run"
                    && site.callee_name == "M"
            })
            .expect("run M site");
        assert_eq!(
            local_site.receiver_owner_identity.as_ref(),
            Some(&GoOwnerIdentity {
                package_dir: "q".to_string(),
                package_clause: "q".to_string(),
                name: "A".to_string(),
            }),
            "{local_site:?}"
        );
    }
    let manifest = queries::interface_dispatch_manifest(no_cache.index.call_graph());
    assert!(
        manifest["sites"]
            .as_array()
            .expect("manifest sites")
            .iter()
            .all(|site| {
                site["file"] != "app/use.go"
                    || (site["method"] != "Close" && site["method"] != "Error")
            }),
        "unproven external and predeclared receivers must be absent from the manifest"
    );
    let stats = queries::call_stats(no_cache.index.call_graph());
    assert_eq!(stats["go_external_receiver_new_recovery_drop"], 0);
    assert_eq!(stats["go_receiver_prereq_drops"]["declaration_unproven"], 1);
    assert_eq!(
        stats["go_receiver_prereq_drops"]["strict_import_unresolved"],
        1
    );

    let no_cache_edges =
        queries::callees(&no_cache, Some("run"), Some("app/use.go"), None, 1).unwrap();
    let exact_cpg_edges =
        queries::callees(&exact_cpg, Some("run"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_edges, exact_cpg_edges);
    let no_cache_package =
        queries::callees(&no_cache, Some("packageRun"), Some("app/use.go"), None, 1).unwrap();
    let exact_cpg_package =
        queries::callees(&exact_cpg, Some("packageRun"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_package, exact_cpg_package);
    assert!(evidence_has_file(&no_cache_package, "q/types.go"));
    assert!(!evidence_has_file(&no_cache_package, "p/types.go"));
    let no_cache_poison =
        queries::callees(&no_cache, Some("poison"), Some("app/use.go"), None, 1).unwrap();
    let exact_cpg_poison =
        queries::callees(&exact_cpg, Some("poison"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_poison, exact_cpg_poison);
    assert!(
        !evidence_has_file(&no_cache_poison, "decoy/error.go"),
        "predeclared receiver minted the exact decoy edge"
    );
    let no_cache_external =
        queries::callees(&no_cache, Some("external"), Some("app/use.go"), None, 1).unwrap();
    let exact_cpg_external =
        queries::callees(&exact_cpg, Some("external"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_external, exact_cpg_external);
    assert!(
        !evidence_has_file(&no_cache_external, "decoy/closer.go"),
        "unproven external receiver minted the exact decoy edge"
    );
    let sidecar = nav_cache_subdir(cache_dir.path(), &repo).join("resolved-call-edge-index.bin");
    assert!(sidecar.exists(), "exact-CPG query must create the sidecar");

    let sidecar_hit = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build_cached_under(&repo, cache_dir.path())),
    };
    assert_eq!(wire_outputs(&no_cache), wire_outputs(&sidecar_hit));
    assert_eq!(
        sidecar_hit.index.call_graph().go_dot_import_files,
        expected_dot_imports
    );
    let sidecar_local_site = sidecar_hit
        .index
        .call_graph()
        .calls
        .values()
        .flatten()
        .find(|site| {
            site.caller.file == "app/use.go" && site.caller.name == "run" && site.callee_name == "M"
        })
        .expect("sidecar-hit run M site");
    assert_eq!(
        sidecar_local_site.receiver_owner_identity.as_ref(),
        Some(&GoOwnerIdentity {
            package_dir: "q".to_string(),
            package_clause: "q".to_string(),
            name: "A".to_string(),
        }),
        "{sidecar_local_site:?}"
    );
    assert_eq!(
        no_cache_edges,
        queries::callees(&sidecar_hit, Some("run"), Some("app/use.go"), None, 1).unwrap()
    );
    assert_eq!(
        no_cache_package,
        queries::callees(
            &sidecar_hit,
            Some("packageRun"),
            Some("app/use.go"),
            None,
            1,
        )
        .unwrap()
    );
    assert_eq!(
        no_cache_poison,
        queries::callees(&sidecar_hit, Some("poison"), Some("app/use.go"), None, 1,).unwrap()
    );
    assert_eq!(
        no_cache_external,
        queries::callees(&sidecar_hit, Some("external"), Some("app/use.go"), None, 1,).unwrap()
    );
}

#[test]
fn navigation_sidecar_go_proven_interface_parity() {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _restore = EnvRestore(std::env::var_os(DIRTY_LOAD_OVERRIDE));
    std::env::set_var(DIRTY_LOAD_OVERRIDE, "1");

    let repo_dir = tempfile::tempdir().unwrap();
    let cache_dir = tempfile::tempdir().unwrap();
    write(
        repo_dir.path(),
        "go.mod",
        "module example.com/root\n\ngo 1.22\n",
    );
    write(
        repo_dir.path(),
        "app/types.go",
        "package app\ntype S struct{ Run func() }\nfunc worker() {}\nfunc New() S { return S{Run: worker} }\n",
    );
    write(
        repo_dir.path(),
        "app/use.go",
        "package app\nfunc invoke() {\n  type S struct{}\n  c := New()\n  c.Run()\n}\n",
    );
    write(
        repo_dir.path(),
        "decoy/types.go",
        "package decoy\ntype S interface{ Run() }\ntype Wrong struct{}\nfunc (Wrong) Run() {}\nfunc retain() { var _ S = Wrong{} }\n",
    );
    let repo = Arc::new(load_repo(repo_dir.path()).unwrap());

    let no_cache = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build(&repo)),
    };
    let cold = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build_cached_under(&repo, cache_dir.path())),
    };
    let no_cache_edges =
        queries::callees(&no_cache, Some("invoke"), Some("app/use.go"), None, 1).unwrap();
    let cold_edges = queries::callees(&cold, Some("invoke"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_edges, cold_edges);
    assert!(
        evidence_has_function(&no_cache_edges, "app/types.go", "worker"),
        "registered func-field target absent: {no_cache_edges:#?}"
    );
    assert!(
        !evidence_has_function(&no_cache_edges, "decoy/types.go", "Run"),
        "bare-name interface decoy survived: {no_cache_edges:#?}"
    );

    let sidecar = nav_cache_subdir(cache_dir.path(), &repo).join("resolved-call-edge-index.bin");
    assert!(sidecar.exists(), "cold query must create the sidecar");
    let hit = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build_cached_under(&repo, cache_dir.path())),
    };
    let hit_edges = queries::callees(&hit, Some("invoke"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_edges, hit_edges);
}
