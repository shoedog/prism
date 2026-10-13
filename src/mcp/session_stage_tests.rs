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
