use prism::navigation::{
    cache::nav_cache_subdir,
    module_graph::{module_deps, repo_map},
    queries,
    types::{SymbolRef, WarningKind},
    NavigationIndex, NavigationSession,
};
use prism::repo_loader::load_repo;
use std::sync::{Arc, Mutex, OnceLock};

const DIRTY_LOAD_OVERRIDE: &str = "PRISM_NAV_EDGE_CACHE_LOAD_DIRTY";

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn write(dir: &std::path::Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
}

struct EnvRestore {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl Drop for EnvRestore {
    fn drop(&mut self) {
        match self.old.take() {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn with_dirty_sidecar_load_override<T>(f: impl FnOnce() -> T) -> T {
    let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let _restore = EnvRestore {
        key: DIRTY_LOAD_OVERRIDE,
        old: std::env::var_os(DIRTY_LOAD_OVERRIDE),
    };
    std::env::set_var(DIRTY_LOAD_OVERRIDE, "1");
    f()
}

#[test]
fn grammar_fingerprint_is_present() {
    assert!(
        !env!("GRAMMAR_FINGERPRINT").is_empty(),
        "build.rs must emit GRAMMAR_FINGERPRINT"
    );
}

#[test]
fn fingerprint_has_real_grammar_input() {
    // R2-M4: not a tautology — the fingerprint must derive from actual tree-sitter-* versions.
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
    let n = lock
        .lines()
        .filter(|l| l.trim().starts_with("name = \"tree-sitter"))
        .count();
    assert!(n >= 1, "expected >=1 tree-sitter-* crate in Cargo.lock");
}

#[test]
fn binary_input_cache_identity_is_present() {
    assert!(
        !env!("PRISM_CACHE_BUILD_IDENTITY").is_empty(),
        "build.rs must emit PRISM_CACHE_BUILD_IDENTITY"
    );
    assert!(
        matches!(env!("PRISM_BINARY_INPUT_DIRTY"), "0" | "1"),
        "build.rs must emit PRISM_BINARY_INPUT_DIRTY as 0 or 1"
    );
}

#[test]
fn nav_cache_subdir_uses_full_sha256_repo_id() {
    let repo_d = tempfile::tempdir().unwrap();
    write(repo_d.path(), "a.py", "def f():\n    return 1\n");
    let repo = load_repo(repo_d.path()).unwrap();
    let base = tempfile::tempdir().unwrap();

    let dir = nav_cache_subdir(base.path(), &repo);
    let repo_id = dir.file_name().unwrap().to_string_lossy();

    assert_eq!(repo_id.len(), 64);
}

#[test]
fn build_cached_writes_then_hits_with_equal_query_output() {
    let repo_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(
        repo_d.path(),
        "a.py",
        "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
    );
    let repo = Arc::new(load_repo(repo_d.path()).unwrap());

    let cache_dir = nav_cache_subdir(cache.path(), &repo);

    let idx_miss = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
    assert!(
        cache_dir.join("cpg-cache.bin").exists(),
        "build 1 must write the cache"
    );
    let idx_hit = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));

    let s_miss = NavigationSession {
        repo: repo.clone(),
        index: idx_miss,
    };
    let s_hit = NavigationSession {
        repo: repo.clone(),
        index: idx_hit,
    };
    assert_eq!(
        queries::callees(&s_miss, Some("run"), None, None, 1).unwrap(),
        queries::callees(&s_hit, Some("run"), None, None, 1).unwrap()
    );
    assert_eq!(
        queries::callers(&s_miss, Some("helper"), None, None, 1).unwrap(),
        queries::callers(&s_hit, Some("helper"), None, None, 1).unwrap()
    );
    assert_eq!(
        queries::ego_graph(&s_miss, Some("run"), None, None, 1, &["Call"]).unwrap(),
        queries::ego_graph(&s_hit, Some("run"), None, None, 1, &["Call"]).unwrap()
    );
}

#[test]
fn call_edge_sidecar_is_lazy_then_hits() {
    with_dirty_sidecar_load_override(|| {
        let repo_d = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        write(repo_d.path(), "api.py", "def helper():\n    return 1\n");
        write(
            repo_d.path(),
            "app.py",
            "from api import helper\n\ndef run():\n    return helper()\n",
        );
        let repo = Arc::new(load_repo(repo_d.path()).unwrap());
        let cache_dir = nav_cache_subdir(cache.path(), &repo);
        let sidecar = cache_dir.join("resolved-call-edge-index.bin");

        let idx_miss = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
        assert!(cache_dir.join("cpg-cache.bin").exists());
        assert!(
            !sidecar.exists(),
            "CPG build/cache should not eagerly write call-edge sidecar"
        );
        let s_miss = NavigationSession {
            repo: repo.clone(),
            index: idx_miss,
        };
        let cold_edges = queries::callees(&s_miss, Some("run"), Some("app.py"), None, 1).unwrap();
        assert!(
            sidecar.exists(),
            "first call-edge query should write sidecar"
        );

        let idx_hit = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
        let s_hit = NavigationSession {
            repo: repo.clone(),
            index: idx_hit,
        };
        let warm_edges = queries::callees(&s_hit, Some("run"), Some("app.py"), None, 1).unwrap();
        assert_eq!(cold_edges, warm_edges);
    });
}

#[test]
fn cpg_hit_non_call_edge_query_does_not_touch_sidecar() {
    let repo_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(
        repo_d.path(),
        "a.py",
        "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
    );
    let repo = Arc::new(load_repo(repo_d.path()).unwrap());
    let cache_dir = nav_cache_subdir(cache.path(), &repo);
    let sidecar = cache_dir.join("resolved-call-edge-index.bin");

    let _ = NavigationIndex::build_cached_under(&repo, cache.path());
    assert!(!sidecar.exists());

    let cpg_hit = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
    let session = NavigationSession {
        repo,
        index: cpg_hit,
    };
    let ev = queries::nodes_at(&session, "a.py", 4);
    assert!(!ev.items.is_empty());
    assert!(
        !sidecar.exists(),
        "non-call-edge query on a CPG cache hit must not load/build/write sidecar"
    );
}

#[test]
fn module_deps_and_repo_map_match_after_sidecar_warmup() {
    with_dirty_sidecar_load_override(|| {
        let repo_d = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        write(repo_d.path(), "util.py", "def helper():\n    return 1\n");
        write(
            repo_d.path(),
            "main.py",
            "from util import helper\n\ndef run():\n    return helper()\n",
        );
        let repo = Arc::new(load_repo(repo_d.path()).unwrap());
        let cache_dir = nav_cache_subdir(cache.path(), &repo);
        let sidecar = cache_dir.join("resolved-call-edge-index.bin");

        let cold = NavigationSession {
            repo: repo.clone(),
            index: Arc::new(NavigationIndex::build_cached_under(&repo, cache.path())),
        };
        let cold_module = module_deps(&cold, "main.py");
        let cold_map = repo_map(&cold);
        assert!(
            sidecar.exists(),
            "module-deps/repo-map should create the lazy sidecar through outgoing call edges"
        );

        let warm = NavigationSession {
            repo,
            index: Arc::new(NavigationIndex::build_cached_under(
                &cold.repo,
                cache.path(),
            )),
        };
        assert_eq!(cold_module, module_deps(&warm, "main.py"));
        assert_eq!(cold_map, repo_map(&warm));
    });
}

#[test]
fn collision_warning_matches_after_sidecar_warmup() {
    // P3: `poll` must stay OVER the R6 fanout cap (4 owners: A/B/C/D) so this
    // still exercises the drop + warning cache round-trip — a 2-owner pool
    // now resolves to a labeled candidate edge instead.
    with_dirty_sidecar_load_override(|| {
        let repo_d = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        write(
            repo_d.path(),
            "a.py",
            "class A:\n    def poll(self):\n        return 1\n",
        );
        write(
            repo_d.path(),
            "b.py",
            "class B:\n    def poll(self):\n        return 2\n",
        );
        write(
            repo_d.path(),
            "c.py",
            "class C:\n    def poll(self):\n        return 3\n",
        );
        write(
            repo_d.path(),
            "d.py",
            "class D:\n    def poll(self):\n        return 4\n",
        );
        write(
            repo_d.path(),
            "main.py",
            "def drive(x):\n    return x.poll()\n",
        );
        let repo = Arc::new(load_repo(repo_d.path()).unwrap());
        let sidecar = nav_cache_subdir(cache.path(), &repo).join("resolved-call-edge-index.bin");

        let cold = NavigationSession {
            repo: repo.clone(),
            index: Arc::new(NavigationIndex::build_cached_under(&repo, cache.path())),
        };
        let cold_ev = queries::callers(&cold, Some("poll"), Some("a.py"), None, 1).unwrap();
        assert!(
            cold_ev
                .warnings
                .iter()
                .any(|w| matches!(w.kind, WarningKind::Collision) && w.message.contains('1')),
            "fixture should emit one collision warning before sidecar load"
        );
        assert!(sidecar.exists());

        let warm = NavigationSession {
            repo,
            index: Arc::new(NavigationIndex::build_cached_under(
                &cold.repo,
                cache.path(),
            )),
        };
        assert_eq!(
            cold_ev,
            queries::callers(&warm, Some("poll"), Some("a.py"), None, 1).unwrap()
        );
    });
}

#[test]
fn corrupt_call_edge_sidecar_rebuilds_successfully() {
    with_dirty_sidecar_load_override(|| {
        let repo_d = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        write(
            repo_d.path(),
            "a.py",
            "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
        );
        let repo = Arc::new(load_repo(repo_d.path()).unwrap());
        let cache_dir = nav_cache_subdir(cache.path(), &repo);
        let sidecar = cache_dir.join("resolved-call-edge-index.bin");

        let idx = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
        let session = NavigationSession {
            repo: repo.clone(),
            index: idx,
        };
        let expected = queries::callees(&session, Some("run"), Some("a.py"), None, 1).unwrap();
        assert!(sidecar.exists());

        std::fs::write(&sidecar, b"not a bincode sidecar").unwrap();
        let idx_rebuilt = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
        let rebuilt = NavigationSession {
            repo,
            index: idx_rebuilt,
        };
        assert_eq!(
            expected,
            queries::callees(&rebuilt, Some("run"), Some("a.py"), None, 1).unwrap()
        );
    });
}

#[test]
fn mutation_helper_drops_sidecar_store() {
    with_dirty_sidecar_load_override(|| {
        let repo_d = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        write(
            repo_d.path(),
            "a.py",
            "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
        );
        let repo = Arc::new(load_repo(repo_d.path()).unwrap());
        let sidecar = nav_cache_subdir(cache.path(), &repo).join("resolved-call-edge-index.bin");

        let priming = Arc::new(NavigationIndex::build_cached_under(&repo, cache.path()));
        let priming_session = NavigationSession {
            repo: repo.clone(),
            index: priming,
        };
        assert!(
            !queries::callees(&priming_session, Some("run"), Some("a.py"), None, 1)
                .unwrap()
                .items
                .is_empty()
        );
        assert!(sidecar.exists());

        let cached = NavigationIndex::build_cached_under(&repo, cache.path())
            .with_modified_cpg_for_testing(|cpg| {
                cpg.call_graph.calls.clear();
                cpg.call_graph.callers.clear();
            });
        let mutated = NavigationSession {
            repo,
            index: Arc::new(cached),
        };
        assert!(
            queries::callees(&mutated, Some("run"), Some("a.py"), None, 1)
                .unwrap()
                .items
                .is_empty(),
            "mutated cached index must rebuild from mutated CPG instead of reloading stale sidecar"
        );
    });
}

#[test]
fn file_add_invalidates() {
    let repo_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(repo_d.path(), "a.py", "def f():\n    return 1\n");
    let _ = NavigationIndex::build_cached_under(&load_repo(repo_d.path()).unwrap(), cache.path());
    write(repo_d.path(), "b.py", "def g():\n    return 2\n");
    let idx = NavigationIndex::build_cached_under(&load_repo(repo_d.path()).unwrap(), cache.path());
    assert!(idx.name_index.contains_key(&("b.py".into(), "g".into())));
}

#[test]
fn corrupt_cache_bin_rebuilds_successfully() {
    let repo_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(repo_d.path(), "a.py", "def stable():\n    return 1\n");
    let repo = load_repo(repo_d.path()).unwrap();

    let _ = NavigationIndex::build_cached_under(&repo, cache.path());
    let cache_bin = nav_cache_subdir(cache.path(), &repo).join("cpg-cache.bin");
    assert!(cache_bin.exists(), "first build must write the cache");

    std::fs::write(&cache_bin, b"garbage cache bytes").unwrap();

    let idx = NavigationIndex::build_cached_under(&repo, cache.path());
    assert!(
        idx.name_index
            .contains_key(&("a.py".into(), "stable".into())),
        "corrupt cache must fall back to a usable rebuild"
    );
}

#[test]
fn two_repos_same_base_are_isolated() {
    let repo_a_d = tempfile::tempdir().unwrap();
    let repo_b_d = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    write(
        repo_a_d.path(),
        "a.py",
        "def alpha():\n    return 1\n\ndef run():\n    return alpha()\n",
    );
    write(
        repo_b_d.path(),
        "b.py",
        "def beta():\n    return 2\n\ndef run():\n    return beta()\n",
    );
    let repo_a = Arc::new(load_repo(repo_a_d.path()).unwrap());
    let repo_b = Arc::new(load_repo(repo_b_d.path()).unwrap());

    let cache_a = nav_cache_subdir(cache.path(), &repo_a).join("cpg-cache.bin");
    let cache_b = nav_cache_subdir(cache.path(), &repo_b).join("cpg-cache.bin");
    let idx_a = Arc::new(NavigationIndex::build_cached_under(&repo_a, cache.path()));
    let idx_b = Arc::new(NavigationIndex::build_cached_under(&repo_b, cache.path()));

    assert_ne!(cache_a, cache_b);
    assert!(cache_a.exists(), "repo A must write its own cache");
    assert!(cache_b.exists(), "repo B must write its own cache");

    let session_a = NavigationSession {
        repo: repo_a,
        index: idx_a,
    };
    let session_b = NavigationSession {
        repo: repo_b,
        index: idx_b,
    };
    let callees_a = queries::callees(&session_a, Some("run"), None, None, 1).unwrap();
    let callees_b = queries::callees(&session_b, Some("run"), None, None, 1).unwrap();

    assert!(callees_a.items.iter().any(|item| {
        matches!(&item.symbol, Some(SymbolRef::Function { name, .. }) if name == "alpha")
    }));
    assert!(callees_b.items.iter().any(|item| {
        matches!(&item.symbol, Some(SymbolRef::Function { name, .. }) if name == "beta")
    }));
}
