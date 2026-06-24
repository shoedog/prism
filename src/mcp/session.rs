use super::freshness::{FreshnessProbe, FreshnessReport};
use crate::cpg_cache;
use crate::navigation::{NavigationIndex, NavigationSession};
use crate::repo_loader::{load_repo, LoadedRepo};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub repo_root: PathBuf,
    pub cache: CacheMode,
    pub refresh_policy: RefreshPolicy,
}

impl ServerConfig {
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            cache: CacheMode::Default,
            refresh_policy: RefreshPolicy::WarnOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RefreshPolicy {
    #[default]
    WarnOnly,
    AutoFull,
}

#[derive(Clone, Debug)]
pub enum CacheMode {
    NoCache,
    Default,
    Dir(PathBuf),
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RefreshSummary {
    pub status: &'static str,
    pub generation: u64,
    pub indexed_files: usize,
    pub tracked_paths: usize,
    pub stale_before_refresh: bool,
    pub stale_index_total_before_refresh: usize,
    pub stale_index_paths_before_refresh: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct AutoRefreshSummary {
    pub generation: u64,
    pub indexed_files: usize,
    pub tracked_paths: usize,
    pub stale_before_refresh: FreshnessReport,
    pub verification: RefreshVerification,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RefreshVerification {
    Clean,
    Diverged(FreshnessReport),
}

pub struct SessionProvider {
    cfg: ServerConfig,
    session: NavigationSession,
    freshness: FreshnessProbe,
    known_stale_after_refresh: Option<FreshnessReport>,
    generation: u64,
    #[cfg(test)]
    forced_refresh: Option<ForcedRefreshForTests>,
}

struct SessionState {
    session: NavigationSession,
    freshness: FreshnessProbe,
    snapshot: SnapshotFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SnapshotFingerprint {
    file_hashes: BTreeMap<String, String>,
    manifest_hashes: BTreeMap<String, String>,
    topology_key: BTreeMap<String, String>,
}

#[cfg(test)]
enum ForcedRefreshForTests {
    Verification(RefreshVerification),
    Error(&'static str),
}

impl SessionProvider {
    pub fn bootstrap(cfg: &ServerConfig) -> anyhow::Result<Self> {
        let cfg = canonical_config(cfg)?;
        let state = build_state(&cfg)?;
        Ok(Self {
            cfg,
            session: state.session,
            freshness: state.freshness,
            known_stale_after_refresh: None,
            generation: 0,
            #[cfg(test)]
            forced_refresh: None,
        })
    }

    pub fn session(&self) -> &NavigationSession {
        &self.session
    }

    pub fn freshness(&self) -> &FreshnessProbe {
        &self.freshness
    }

    pub fn refresh_policy(&self) -> RefreshPolicy {
        self.cfg.refresh_policy
    }

    pub(crate) fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        self.known_stale_after_refresh.as_ref()
    }

    #[cfg(test)]
    pub(crate) fn force_next_verification_for_tests(&mut self, verification: RefreshVerification) {
        self.forced_refresh = Some(ForcedRefreshForTests::Verification(verification));
    }

    #[cfg(test)]
    pub(crate) fn force_next_refresh_error_for_tests(&mut self, message: &'static str) {
        self.forced_refresh = Some(ForcedRefreshForTests::Error(message));
    }

    pub fn refresh(&mut self) -> anyhow::Result<RefreshSummary> {
        let before = self.effective_stale_report();
        let committed = self.refresh_verified()?;
        let status = match &committed.verification {
            RefreshVerification::Clean => "refreshed",
            RefreshVerification::Diverged(_) => "raced_stale",
        };
        Ok(RefreshSummary {
            status,
            generation: committed.generation,
            indexed_files: committed.indexed_files,
            tracked_paths: committed.tracked_paths,
            stale_before_refresh: before.stale,
            stale_index_total_before_refresh: before.total_changed,
            stale_index_paths_before_refresh: before.changed_paths,
        })
    }

    pub(crate) fn auto_refresh(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        let stale_before_refresh = self.effective_stale_report();
        let committed = self.refresh_verified()?;
        Ok(AutoRefreshSummary {
            generation: committed.generation,
            indexed_files: committed.indexed_files,
            tracked_paths: committed.tracked_paths,
            stale_before_refresh,
            verification: committed.verification,
        })
    }

    fn effective_stale_report(&self) -> FreshnessReport {
        if let Some(report) = &self.known_stale_after_refresh {
            return report.clone();
        }
        self.freshness.check()
    }

    fn refresh_verified(&mut self) -> anyhow::Result<CommittedRefresh> {
        #[cfg(test)]
        let verified = if let Some(forced) = self.forced_refresh.take() {
            match forced {
                ForcedRefreshForTests::Verification(verification) => VerifiedCandidate {
                    state: build_state(&self.cfg)?,
                    verification,
                },
                ForcedRefreshForTests::Error(message) => anyhow::bail!(message),
            }
        } else {
            build_verified_candidate(&self.cfg)?
        };

        #[cfg(not(test))]
        let VerifiedCandidate {
            state,
            verification,
        } = build_verified_candidate(&self.cfg)?;

        #[cfg(test)]
        let VerifiedCandidate {
            state,
            verification,
        } = verified;

        let indexed_files = state.session.repo.files.len();
        let tracked_paths = state.freshness.tracked_len();
        self.generation = self.generation.saturating_add(1);
        let generation = self.generation;
        self.known_stale_after_refresh = match &verification {
            RefreshVerification::Clean => None,
            RefreshVerification::Diverged(report) => Some(report.clone()),
        };
        self.session = state.session;
        self.freshness = state.freshness;
        Ok(CommittedRefresh {
            generation,
            indexed_files,
            tracked_paths,
            verification,
        })
    }
}

fn canonical_config(cfg: &ServerConfig) -> anyhow::Result<ServerConfig> {
    Ok(ServerConfig {
        repo_root: std::fs::canonicalize(&cfg.repo_root)?,
        cache: cfg.cache.clone(),
        refresh_policy: cfg.refresh_policy,
    })
}

struct VerifiedCandidate {
    state: SessionState,
    verification: RefreshVerification,
}

struct CommittedRefresh {
    generation: u64,
    indexed_files: usize,
    tracked_paths: usize,
    verification: RefreshVerification,
}

fn build_verified_candidate(cfg: &ServerConfig) -> anyhow::Result<VerifiedCandidate> {
    let first = build_state(cfg)?;
    match verify_snapshot(cfg, &first.snapshot)? {
        RefreshVerification::Clean => Ok(VerifiedCandidate {
            state: first,
            verification: RefreshVerification::Clean,
        }),
        RefreshVerification::Diverged(_) => {
            let second = build_state(cfg)?;
            let verification = verify_snapshot(cfg, &second.snapshot)?;
            Ok(VerifiedCandidate {
                state: second,
                verification,
            })
        }
    }
}

fn verify_snapshot(
    cfg: &ServerConfig,
    snapshot: &SnapshotFingerprint,
) -> anyhow::Result<RefreshVerification> {
    let current = load_repo(&cfg.repo_root)?;
    let current = SnapshotFingerprint::from_repo(&current);
    if snapshot == &current {
        return Ok(RefreshVerification::Clean);
    }
    Ok(RefreshVerification::Diverged(
        snapshot.diff_report(&current),
    ))
}

// `NavigationSession` holds `Arc<LoadedRepo>`/`Arc<NavigationIndex>` (the nav layer's shape);
// `LoadedRepo` is not `Sync`, but the MCP server dispatches single-threaded (spec §8) — `Send+Sync`
// is deferred to a future async transport (spec §17). Same `Arc<LoadedRepo>` pattern as `build_session`.
#[allow(clippy::arc_with_non_send_sync)]
fn build_state(cfg: &ServerConfig) -> anyhow::Result<SessionState> {
    let loaded_repo = load_repo(&cfg.repo_root)?;
    let snapshot = SnapshotFingerprint::from_repo(&loaded_repo);
    let freshness = FreshnessProbe::from_loaded_repo(&loaded_repo);
    let repo = Arc::new(loaded_repo);
    let index = match &cfg.cache {
        CacheMode::NoCache => NavigationIndex::build(&repo),
        CacheMode::Default => NavigationIndex::build_cached(&repo),
        CacheMode::Dir(base) => NavigationIndex::build_cached_under(&repo, base),
    };
    let index = Arc::new(index);
    Ok(SessionState {
        session: NavigationSession { repo, index },
        freshness,
        snapshot,
    })
}

impl SnapshotFingerprint {
    fn from_repo(repo: &LoadedRepo) -> Self {
        let topology_key =
            cpg_cache::compute_topology_key(&repo.file_hashes, &repo.manifest_hashes);
        Self {
            file_hashes: repo.file_hashes.clone(),
            manifest_hashes: repo.manifest_hashes.clone(),
            topology_key,
        }
    }

    fn diff_report(&self, current: &Self) -> FreshnessReport {
        let mut changed = BTreeSet::new();
        collect_changed_hash_paths(&self.file_hashes, &current.file_hashes, "", &mut changed);
        collect_changed_hash_paths(
            &self.manifest_hashes,
            &current.manifest_hashes,
            "",
            &mut changed,
        );
        collect_changed_hash_paths(
            &self.topology_key,
            &current.topology_key,
            "topology:",
            &mut changed,
        );
        FreshnessReport::from_changed_paths(changed)
    }
}

fn collect_changed_hash_paths(
    before: &BTreeMap<String, String>,
    after: &BTreeMap<String, String>,
    fallback_prefix: &str,
    changed: &mut BTreeSet<String>,
) {
    for key in before.keys().chain(after.keys()) {
        if before.get(key) == after.get(key) {
            continue;
        }
        changed.insert(display_changed_key(key, fallback_prefix));
    }
}

fn display_changed_key(key: &str, fallback_prefix: &str) -> String {
    key.strip_prefix("source:")
        .or_else(|| key.strip_prefix("manifest:"))
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{fallback_prefix}{key}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_builds_a_queryable_session() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def helper():\n    return 1\n").unwrap();
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
            refresh_policy: RefreshPolicy::WarnOnly,
        };
        let p = SessionProvider::bootstrap(&cfg).expect("bootstrap");
        assert_eq!(
            crate::navigation::queries::nodes_at(p.session(), "a.py", 1).query,
            "nodes-at:a.py:1"
        );
    }

    #[test]
    fn refresh_rebuilds_session_and_resets_generation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def old():\n    return 1\n").unwrap();
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
            refresh_policy: RefreshPolicy::WarnOnly,
        };
        let mut p = SessionProvider::bootstrap(&cfg).expect("bootstrap");
        std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 1\n").unwrap();

        let summary = p.refresh().expect("refresh");
        assert_eq!(summary.status, "refreshed");
        assert_eq!(summary.generation, 1);
        assert!(summary.stale_before_refresh);
        assert_eq!(summary.stale_index_paths_before_refresh, ["a.py"]);
        assert!(p
            .session()
            .repo
            .files
            .get("a.py")
            .unwrap()
            .source
            .contains("fresh"));
        assert!(!p.freshness().check().stale);
    }

    #[test]
    fn diverged_refresh_remains_known_stale_until_clean_refresh() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def old():\n    return 1\n").unwrap();
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
            refresh_policy: RefreshPolicy::WarnOnly,
        };
        let mut p = SessionProvider::bootstrap(&cfg).expect("bootstrap");
        std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 1\n").unwrap();
        p.force_next_verification_for_tests(RefreshVerification::Diverged(
            FreshnessReport::from_changed_paths(["a.py".to_string()]),
        ));

        let summary = p.refresh().expect("refresh");
        assert_eq!(summary.status, "raced_stale");
        assert!(p.known_stale_after_refresh().is_some());

        let summary = p.refresh().expect("clean refresh");
        assert_eq!(summary.status, "refreshed");
        assert!(summary.stale_before_refresh);
        assert_eq!(summary.stale_index_paths_before_refresh, ["a.py"]);
        assert!(p.known_stale_after_refresh().is_none());
    }

    #[test]
    fn refresh_error_keeps_existing_session_generation_and_known_stale() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.py"), "def old():\n    return 1\n").unwrap();
        let cfg = ServerConfig {
            repo_root: dir.path().to_path_buf(),
            cache: CacheMode::NoCache,
            refresh_policy: RefreshPolicy::WarnOnly,
        };
        let mut p = SessionProvider::bootstrap(&cfg).expect("bootstrap");
        std::fs::write(dir.path().join("a.py"), "def fresh():\n    return 1\n").unwrap();
        p.force_next_verification_for_tests(RefreshVerification::Diverged(
            FreshnessReport::from_changed_paths(["a.py".to_string()]),
        ));
        let diverged = p.refresh().expect("diverged refresh");
        assert_eq!(diverged.status, "raced_stale");
        assert_eq!(p.generation, 1);
        assert!(p
            .session()
            .repo
            .files
            .get("a.py")
            .unwrap()
            .source
            .contains("fresh"));
        assert!(p.known_stale_after_refresh().is_some());

        std::fs::write(dir.path().join("a.py"), "def newer():\n    return 2\n").unwrap();
        p.force_next_refresh_error_for_tests("forced verifier error");
        let error = p.refresh().expect_err("refresh error");

        assert!(error.to_string().contains("forced verifier error"));
        assert_eq!(p.generation, 1);
        assert!(p
            .session()
            .repo
            .files
            .get("a.py")
            .unwrap()
            .source
            .contains("fresh"));
        assert!(p.known_stale_after_refresh().is_some());
    }

    #[test]
    fn snapshot_diff_reports_source_manifest_and_topology_changes() {
        let before = SnapshotFingerprint {
            file_hashes: [("a.py".into(), "old".into())].into_iter().collect(),
            manifest_hashes: [("Cargo.toml".into(), "old".into())].into_iter().collect(),
            topology_key: [("source:a.py".into(), "present".into())]
                .into_iter()
                .collect(),
        };
        let after = SnapshotFingerprint {
            file_hashes: [("a.py".into(), "new".into()), ("b.py".into(), "new".into())]
                .into_iter()
                .collect(),
            manifest_hashes: [("Cargo.toml".into(), "new".into())].into_iter().collect(),
            topology_key: [
                ("source:a.py".into(), "present".into()),
                ("source:b.py".into(), "present".into()),
            ]
            .into_iter()
            .collect(),
        };
        let report = before.diff_report(&after);
        assert!(report.stale);
        assert_eq!(report.total_changed, 3);
        assert_eq!(report.changed_paths, ["Cargo.toml", "a.py", "b.py"]);
    }
}
