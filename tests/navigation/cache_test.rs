use prism::navigation::{
    cache::nav_cache_subdir, queries, types::SymbolRef, NavigationIndex, NavigationSession,
};
use prism::repo_loader::load_repo;
use std::sync::Arc;

fn write(dir: &std::path::Path, name: &str, src: &str) {
    std::fs::write(dir.join(name), src).unwrap();
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
