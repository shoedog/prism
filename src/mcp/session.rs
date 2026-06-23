use super::freshness::FreshnessProbe;
use crate::navigation::{NavigationIndex, NavigationSession};
use crate::repo_loader::load_repo;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub repo_root: PathBuf,
    pub cache: CacheMode,
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

pub struct SessionProvider {
    cfg: ServerConfig,
    session: NavigationSession,
    freshness: FreshnessProbe,
    generation: u64,
}

struct SessionState {
    session: NavigationSession,
    freshness: FreshnessProbe,
}

impl SessionProvider {
    pub fn bootstrap(cfg: &ServerConfig) -> anyhow::Result<Self> {
        let cfg = canonical_config(cfg)?;
        let state = build_state(&cfg)?;
        Ok(Self {
            cfg,
            session: state.session,
            freshness: state.freshness,
            generation: 0,
        })
    }

    pub fn session(&self) -> &NavigationSession {
        &self.session
    }

    pub fn freshness(&self) -> &FreshnessProbe {
        &self.freshness
    }

    pub fn refresh(&mut self) -> anyhow::Result<RefreshSummary> {
        let before = self.freshness.check();
        let state = build_state(&self.cfg)?;
        let indexed_files = state.session.repo.files.len();
        let tracked_paths = state.freshness.tracked_len();
        self.generation = self.generation.saturating_add(1);
        let generation = self.generation;
        self.session = state.session;
        self.freshness = state.freshness;
        Ok(RefreshSummary {
            status: "refreshed",
            generation,
            indexed_files,
            tracked_paths,
            stale_before_refresh: before.stale,
            stale_index_total_before_refresh: before.total_changed,
            stale_index_paths_before_refresh: before.changed_paths,
        })
    }
}

fn canonical_config(cfg: &ServerConfig) -> anyhow::Result<ServerConfig> {
    Ok(ServerConfig {
        repo_root: std::fs::canonicalize(&cfg.repo_root)?,
        cache: cfg.cache.clone(),
    })
}

// `NavigationSession` holds `Arc<LoadedRepo>`/`Arc<NavigationIndex>` (the nav layer's shape);
// `LoadedRepo` is not `Sync`, but the MCP server dispatches single-threaded (spec §8) — `Send+Sync`
// is deferred to a future async transport (spec §17). Same `Arc<LoadedRepo>` pattern as `build_session`.
#[allow(clippy::arc_with_non_send_sync)]
fn build_state(cfg: &ServerConfig) -> anyhow::Result<SessionState> {
    let loaded_repo = load_repo(&cfg.repo_root)?;
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
    })
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
}
