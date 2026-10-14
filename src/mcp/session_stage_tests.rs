use super::*;

fn config(dir: &tempfile::TempDir) -> ServerConfig {
    std::fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    ServerConfig {
        repo_root: dir.path().to_path_buf(),
        cache: CacheMode::NoCache,
        refresh_policy: RefreshPolicy::WarnOnly,
        startup: StartupMode::Eager,
        first_call_wait: std::time::Duration::from_secs(20),
    }
}

#[test]
fn bootstrap_observed_reports_each_stage_once_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let mut seen = Vec::new();
    let provider =
        SessionProvider::bootstrap_observed(&config(&dir), &mut |stage| seen.push(stage)).unwrap();
    assert_eq!(seen, [BuildStage::RepoLoaded, BuildStage::IndexBuilt]);
    assert_eq!(provider.session().repo.files.len(), 1);
}

#[test]
fn a_failed_load_reports_no_stage() {
    let mut cfg = ServerConfig::new("/prism/no/such/repository".into());
    cfg.cache = CacheMode::NoCache;
    let mut seen = Vec::new();
    assert!(SessionProvider::bootstrap_observed(&cfg, &mut |stage| seen.push(stage)).is_err());
    assert!(seen.is_empty());
}

#[test]
fn bootstrap_builds_the_same_session_as_bootstrap_observed() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = config(&dir);
    let plain = SessionProvider::bootstrap(&cfg).unwrap();
    let observed = SessionProvider::bootstrap_observed(&cfg, &mut |_| {}).unwrap();
    assert_eq!(
        plain.session().index.cpg().node_count(),
        observed.session().index.cpg().node_count()
    );
    assert_eq!(
        plain.session().repo.file_hashes,
        observed.session().repo.file_hashes
    );
}

/// Whether a file named `name` exists anywhere under `dir`.
fn exists_under(dir: &std::path::Path, name: &str) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries.flatten().any(|entry| {
            let path = entry.path();
            path.is_dir() && exists_under(&path, name) || entry.file_name() == name
        })
    })
}

#[test]
fn the_stages_fire_around_the_index_build_not_just_in_order() {
    let repo = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let mut cfg = config(&repo);
    cfg.cache = CacheMode::Dir(cache.path().to_path_buf());

    let mut seen = Vec::new();
    let _provider = SessionProvider::bootstrap_observed(&cfg, &mut |stage| {
        seen.push((
            stage,
            exists_under(cache.path(), "cpg-cache.bin"),
            exists_under(cache.path(), "resolved-call-edge-index.bin"),
        ));
    })
    .unwrap();

    // RepoLoaded fires before the index is built, IndexBuilt after its cache is written. The
    // call-edge sidecar is written by the first call-edge query, not by the build.
    assert_eq!(
        seen,
        [
            (BuildStage::RepoLoaded, false, false),
            (BuildStage::IndexBuilt, true, false),
        ]
    );
}

/// The shared daemon reports this number to every client; it must count committed refreshes
/// and nothing else. Rules out an accessor that returns a constant or counts attempts.
#[test]
fn generation_counts_committed_refreshes_and_nothing_else() {
    let dir = tempfile::tempdir().unwrap();
    let mut provider = SessionProvider::bootstrap(&config(&dir)).unwrap();
    assert_eq!(provider.generation(), 0, "a freshly built index");

    provider.refresh().unwrap();
    assert_eq!(provider.generation(), 1);

    provider.force_next_refresh_error_for_tests("refused");
    assert!(provider.refresh().is_err());
    assert_eq!(provider.generation(), 1, "a failed refresh commits nothing");

    provider.auto_refresh().unwrap();
    assert_eq!(
        provider.generation(),
        2,
        "an automatic refresh is a refresh"
    );
}
