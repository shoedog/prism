use prism::navigation::types::SymbolRef;
use prism::navigation::{cache::nav_cache_subdir, queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
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
        "package app\nimport (\"io\"; q \"example/q\")\nfunc run(a q.A) { a.M() }\nfunc external() { _ = func(cl io.Closer) error { return cl.Close() }(nil) }\nfunc poison(e error) string { return e.Error() }\n",
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
    assert_eq!(
        no_cache_edges,
        queries::callees(&sidecar_hit, Some("run"), Some("app/use.go"), None, 1).unwrap()
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
