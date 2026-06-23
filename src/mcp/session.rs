use super::freshness::FreshnessProbe;
use crate::navigation::{NavigationIndex, NavigationSession};
use crate::repo_loader::load_repo;
use std::path::PathBuf;
use std::sync::Arc;

pub struct ServerConfig {
    pub repo_root: PathBuf,
    pub cache: CacheMode,
}

pub enum CacheMode {
    NoCache,
    Default,
    Dir(PathBuf),
}

pub struct SessionProvider {
    session: NavigationSession,
    freshness: FreshnessProbe,
}

impl SessionProvider {
    // `NavigationSession` holds `Arc<LoadedRepo>`/`Arc<NavigationIndex>` (the nav layer's shape);
    // `LoadedRepo` is not `Sync`, but the MCP server dispatches single-threaded (spec §8) — `Send+Sync`
    // is deferred to a future async transport (spec §17). Same `Arc<LoadedRepo>` pattern as `build_session`.
    #[allow(clippy::arc_with_non_send_sync)]
    pub fn bootstrap(cfg: &ServerConfig) -> anyhow::Result<Self> {
        let repo_root = std::fs::canonicalize(&cfg.repo_root)?;
        let loaded_repo = load_repo(&repo_root)?;
        let freshness = FreshnessProbe::from_loaded_repo(&loaded_repo);
        let repo = Arc::new(loaded_repo);
        let index = match &cfg.cache {
            CacheMode::NoCache => NavigationIndex::build(&repo),
            CacheMode::Default => NavigationIndex::build_cached(&repo),
            CacheMode::Dir(base) => NavigationIndex::build_cached_under(&repo, base),
        };
        let index = Arc::new(index);
        Ok(Self {
            session: NavigationSession { repo, index },
            freshness,
        })
    }

    pub fn session(&self) -> &NavigationSession {
        &self.session
    }

    pub fn freshness(&self) -> &FreshnessProbe {
        &self.freshness
    }
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
}
