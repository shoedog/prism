use prism::navigation::{cache::nav_cache_subdir, queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
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
        "app/use.go",
        "package app\nimport q \"example/q\"\nfunc run(a q.A) { a.M() }\n",
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

    let no_cache_edges =
        queries::callees(&no_cache, Some("run"), Some("app/use.go"), None, 1).unwrap();
    let exact_cpg_edges =
        queries::callees(&exact_cpg, Some("run"), Some("app/use.go"), None, 1).unwrap();
    assert_eq!(no_cache_edges, exact_cpg_edges);
    let sidecar = nav_cache_subdir(cache_dir.path(), &repo).join("resolved-call-edge-index.bin");
    assert!(sidecar.exists(), "exact-CPG query must create the sidecar");

    let sidecar_hit = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(NavigationIndex::build_cached_under(&repo, cache_dir.path())),
    };
    assert_eq!(wire_outputs(&no_cache), wire_outputs(&sidecar_hit));
    assert_eq!(
        no_cache_edges,
        queries::callees(&sidecar_hit, Some("run"), Some("app/use.go"), None, 1).unwrap()
    );
}
