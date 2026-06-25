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
    AutoIncremental,
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
    pub strategy: &'static str,
    pub fallback_reason: Option<&'static str>,
    pub generation: u64,
    pub indexed_files: usize,
    pub tracked_paths: usize,
    pub stale_before_refresh: bool,
    pub stale_index_total_before_refresh: usize,
    pub stale_index_paths_before_refresh: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct AutoRefreshSummary {
    pub strategy: &'static str,
    pub fallback_reason: Option<&'static str>,
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
    snapshot: SnapshotFingerprint,
    known_stale_after_refresh: Option<FreshnessReport>,
    generation: u64,
    #[cfg(test)]
    forced_refresh: Option<ForcedRefreshForTests>,
}

struct SessionState {
    session: NavigationSession,
    freshness: FreshnessProbe,
    snapshot: SnapshotFingerprint,
    strategy: RefreshStrategy,
    fallback_reason: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SnapshotFingerprint {
    file_hashes: BTreeMap<String, String>,
    manifest_hashes: BTreeMap<String, String>,
    topology_key: BTreeMap<String, String>,
    has_type_db: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RefreshStrategyPolicy {
    FullOnly,
    PreferIncremental,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RefreshStrategy {
    Full,
    Incremental,
}

impl RefreshStrategy {
    fn as_str(self) -> &'static str {
        match self {
            RefreshStrategy::Full => "full",
            RefreshStrategy::Incremental => "incremental",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RefreshPlan {
    strategy: RefreshStrategy,
    changed_files: BTreeSet<String>,
    fallback_reason: Option<&'static str>,
    bypass_cache: bool,
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
            snapshot: state.snapshot,
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
        let committed = self.refresh_verified(RefreshStrategyPolicy::FullOnly)?;
        let status = match &committed.verification {
            RefreshVerification::Clean => "refreshed",
            RefreshVerification::Diverged(_) => "raced_stale",
        };
        Ok(RefreshSummary {
            status,
            strategy: committed.strategy.as_str(),
            fallback_reason: committed.fallback_reason,
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
        let policy = match self.cfg.refresh_policy {
            RefreshPolicy::AutoIncremental => RefreshStrategyPolicy::PreferIncremental,
            RefreshPolicy::WarnOnly | RefreshPolicy::AutoFull => RefreshStrategyPolicy::FullOnly,
        };
        let committed = self.refresh_verified(policy)?;
        Ok(AutoRefreshSummary {
            strategy: committed.strategy.as_str(),
            fallback_reason: committed.fallback_reason,
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

    fn refresh_verified(
        &mut self,
        policy: RefreshStrategyPolicy,
    ) -> anyhow::Result<CommittedRefresh> {
        #[cfg(test)]
        let verified = if let Some(forced) = self.forced_refresh.take() {
            match forced {
                ForcedRefreshForTests::Verification(verification) => VerifiedCandidate {
                    state: build_candidate_state(
                        &self.cfg,
                        &self.snapshot,
                        self.session.index.as_ref(),
                        policy,
                    )?,
                    verification,
                },
                ForcedRefreshForTests::Error(message) => anyhow::bail!(message),
            }
        } else {
            build_verified_candidate(
                &self.cfg,
                &self.snapshot,
                self.session.index.as_ref(),
                policy,
            )?
        };

        #[cfg(not(test))]
        let VerifiedCandidate {
            state,
            verification,
        } = build_verified_candidate(
            &self.cfg,
            &self.snapshot,
            self.session.index.as_ref(),
            policy,
        )?;

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
        let strategy = state.strategy;
        let fallback_reason = state.fallback_reason;
        self.session = state.session;
        self.freshness = state.freshness;
        self.snapshot = state.snapshot;
        Ok(CommittedRefresh {
            generation,
            indexed_files,
            tracked_paths,
            verification,
            strategy,
            fallback_reason,
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
    strategy: RefreshStrategy,
    fallback_reason: Option<&'static str>,
}

fn build_verified_candidate(
    cfg: &ServerConfig,
    active_snapshot: &SnapshotFingerprint,
    active_index: &NavigationIndex,
    policy: RefreshStrategyPolicy,
) -> anyhow::Result<VerifiedCandidate> {
    let first = build_candidate_state(cfg, active_snapshot, active_index, policy)?;
    match verify_snapshot(cfg, &first.snapshot)? {
        RefreshVerification::Clean => Ok(VerifiedCandidate {
            state: first,
            verification: RefreshVerification::Clean,
        }),
        RefreshVerification::Diverged(_) => {
            let second = build_candidate_state(cfg, active_snapshot, active_index, policy)?;
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
        strategy: RefreshStrategy::Full,
        fallback_reason: None,
    })
}

#[allow(clippy::arc_with_non_send_sync)]
fn build_candidate_state(
    cfg: &ServerConfig,
    active_snapshot: &SnapshotFingerprint,
    active_index: &NavigationIndex,
    policy: RefreshStrategyPolicy,
) -> anyhow::Result<SessionState> {
    let loaded_repo = load_repo(&cfg.repo_root)?;
    let snapshot = SnapshotFingerprint::from_repo(&loaded_repo);
    let freshness = FreshnessProbe::from_loaded_repo(&loaded_repo);
    let plan = match policy {
        RefreshStrategyPolicy::FullOnly => RefreshPlan::full(None),
        RefreshStrategyPolicy::PreferIncremental => plan_refresh(active_snapshot, &snapshot),
    };
    let repo = Arc::new(loaded_repo);
    let index = match plan.strategy {
        RefreshStrategy::Incremental => NavigationIndex::build_incremental_from_previous(
            active_index,
            &repo,
            &plan.changed_files,
        ),
        RefreshStrategy::Full if plan.bypass_cache => NavigationIndex::build(&repo),
        RefreshStrategy::Full => match &cfg.cache {
            CacheMode::NoCache => NavigationIndex::build(&repo),
            CacheMode::Default => NavigationIndex::build_cached(&repo),
            CacheMode::Dir(base) => NavigationIndex::build_cached_under(&repo, base),
        },
    };
    let index = Arc::new(index);
    Ok(SessionState {
        session: NavigationSession { repo, index },
        freshness,
        snapshot,
        strategy: plan.strategy,
        fallback_reason: plan.fallback_reason,
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
            has_type_db: repo.type_db.is_some(),
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
        if self.has_type_db != current.has_type_db {
            changed.insert("type_db".to_string());
        }
        FreshnessReport::from_changed_paths(changed)
    }
}

impl RefreshPlan {
    fn full(fallback_reason: Option<&'static str>) -> Self {
        Self {
            strategy: RefreshStrategy::Full,
            changed_files: BTreeSet::new(),
            fallback_reason,
            bypass_cache: fallback_reason == Some("type_db_present"),
        }
    }

    fn incremental(changed_files: BTreeSet<String>) -> Self {
        Self {
            strategy: RefreshStrategy::Incremental,
            changed_files,
            fallback_reason: None,
            bypass_cache: false,
        }
    }
}

fn plan_refresh(old: &SnapshotFingerprint, new: &SnapshotFingerprint) -> RefreshPlan {
    if old.has_type_db || new.has_type_db {
        return RefreshPlan::full(Some("type_db_present"));
    }

    let old_files = key_set(&old.file_hashes);
    let new_files = key_set(&new.file_hashes);
    if old_files != new_files {
        return RefreshPlan::full(Some("file_set_changed"));
    }

    if old.manifest_hashes != new.manifest_hashes {
        return RefreshPlan::full(Some("manifest_changed"));
    }

    if topology_residual(&old.topology_key) != topology_residual(&new.topology_key) {
        return RefreshPlan::full(Some("topology_changed"));
    }

    let changed_files = old
        .file_hashes
        .iter()
        .filter_map(|(file, old_hash)| {
            (new.file_hashes.get(file) != Some(old_hash)).then(|| file.clone())
        })
        .collect::<BTreeSet<_>>();
    if changed_files.is_empty() {
        return RefreshPlan::full(Some("no_semantic_change"));
    }

    RefreshPlan::incremental(changed_files)
}

fn key_set(map: &BTreeMap<String, String>) -> BTreeSet<&str> {
    map.keys().map(String::as_str).collect()
}

fn topology_residual(map: &BTreeMap<String, String>) -> BTreeMap<&str, &str> {
    map.iter()
        .filter(|(key, _)| !key.starts_with("source:") && !key.starts_with("manifest:"))
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
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

    fn snapshot(
        files: &[(&str, &str)],
        manifests: &[(&str, &str)],
        topology: &[(&str, &str)],
    ) -> SnapshotFingerprint {
        SnapshotFingerprint {
            file_hashes: files
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            manifest_hashes: manifests
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            topology_key: topology
                .iter()
                .map(|(k, v)| ((*k).into(), (*v).into()))
                .collect(),
            has_type_db: false,
        }
    }

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
            has_type_db: false,
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
            has_type_db: true,
        };
        let report = before.diff_report(&after);
        assert!(report.stale);
        assert_eq!(report.total_changed, 4);
        assert_eq!(
            report.changed_paths,
            ["Cargo.toml", "a.py", "b.py", "type_db"]
        );
    }

    #[test]
    fn auto_incremental_plan_uses_unbounded_changed_files() {
        let old_files = (0..8)
            .map(|i| (format!("f{i}.py"), "old".to_string()))
            .collect::<Vec<_>>();
        let new_files = (0..8)
            .map(|i| (format!("f{i}.py"), format!("new{i}")))
            .collect::<Vec<_>>();
        let old_file_refs = old_files
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        let new_file_refs = new_files
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        let topology = old_files
            .iter()
            .map(|(k, _)| (format!("source:{k}"), "present".to_string()))
            .collect::<Vec<_>>();
        let topology_refs = topology
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();

        let old = snapshot(&old_file_refs, &[], &topology_refs);
        let new = snapshot(&new_file_refs, &[], &topology_refs);
        let plan = plan_refresh(&old, &new);

        assert_eq!(plan.strategy, RefreshStrategy::Incremental);
        assert_eq!(plan.changed_files.len(), 8);
        assert!(plan.fallback_reason.is_none());
    }

    #[test]
    fn auto_incremental_plan_reports_file_set_before_topology() {
        let old = snapshot(&[("a.py", "old")], &[], &[("source:a.py", "present")]);
        let new = snapshot(
            &[("a.py", "old"), ("b.py", "new")],
            &[],
            &[("source:a.py", "present"), ("source:b.py", "present")],
        );
        let plan = plan_refresh(&old, &new);

        assert_eq!(plan.strategy, RefreshStrategy::Full);
        assert_eq!(plan.fallback_reason, Some("file_set_changed"));
    }

    #[test]
    fn auto_incremental_plan_disallows_type_db_and_bypasses_cache() {
        let old = snapshot(&[("a.py", "old")], &[], &[("source:a.py", "present")]);
        let mut new = snapshot(&[("a.py", "new")], &[], &[("source:a.py", "present")]);
        new.has_type_db = true;
        let plan = plan_refresh(&old, &new);

        assert_eq!(plan.strategy, RefreshStrategy::Full);
        assert_eq!(plan.fallback_reason, Some("type_db_present"));
        assert!(plan.bypass_cache);
    }

    #[test]
    fn auto_incremental_plan_reports_manifest_change() {
        let old = snapshot(
            &[("src/lib.rs", "old")],
            &[("Cargo.toml", "old")],
            &[
                ("source:src/lib.rs", "present"),
                ("manifest:Cargo.toml", "old"),
            ],
        );
        let new = snapshot(
            &[("src/lib.rs", "old")],
            &[("Cargo.toml", "new")],
            &[
                ("source:src/lib.rs", "present"),
                ("manifest:Cargo.toml", "new"),
            ],
        );
        let plan = plan_refresh(&old, &new);

        assert_eq!(plan.strategy, RefreshStrategy::Full);
        assert_eq!(plan.fallback_reason, Some("manifest_changed"));
    }

    #[test]
    fn auto_incremental_plan_reports_no_semantic_change() {
        let old = snapshot(&[("a.py", "same")], &[], &[("source:a.py", "present")]);
        let new = snapshot(&[("a.py", "same")], &[], &[("source:a.py", "present")]);
        let plan = plan_refresh(&old, &new);

        assert_eq!(plan.strategy, RefreshStrategy::Full);
        assert_eq!(plan.fallback_reason, Some("no_semantic_change"));
    }

    #[test]
    fn auto_incremental_plan_reports_residual_topology_after_file_and_manifest_checks() {
        let old = snapshot(
            &[("a.py", "old")],
            &[("Cargo.toml", "same")],
            &[("source:a.py", "present"), ("future:layout", "old")],
        );
        let new = snapshot(
            &[("a.py", "old")],
            &[("Cargo.toml", "same")],
            &[("source:a.py", "present"), ("future:layout", "new")],
        );
        let plan = plan_refresh(&old, &new);

        assert_eq!(plan.strategy, RefreshStrategy::Full);
        assert_eq!(plan.fallback_reason, Some("topology_changed"));
    }

    #[test]
    fn auto_incremental_retry_replans_against_active_snapshot() {
        let active = snapshot(
            &[("a.py", "old"), ("b.py", "old")],
            &[],
            &[("source:a.py", "present"), ("source:b.py", "present")],
        );
        let first_attempt = snapshot(
            &[("a.py", "new"), ("b.py", "old")],
            &[],
            &[("source:a.py", "present"), ("source:b.py", "present")],
        );
        let second_attempt = snapshot(
            &[("a.py", "new"), ("b.py", "old"), ("c.py", "new")],
            &[],
            &[
                ("source:a.py", "present"),
                ("source:b.py", "present"),
                ("source:c.py", "present"),
            ],
        );

        let first_plan = plan_refresh(&active, &first_attempt);
        assert_eq!(first_plan.strategy, RefreshStrategy::Incremental);
        assert_eq!(first_plan.changed_files, BTreeSet::from(["a.py".into()]));

        let retry_plan = plan_refresh(&active, &second_attempt);
        assert_eq!(retry_plan.strategy, RefreshStrategy::Full);
        assert_eq!(retry_plan.fallback_reason, Some("file_set_changed"));
    }
}
