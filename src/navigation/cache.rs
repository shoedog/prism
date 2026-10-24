use crate::cpg::CpgContext;
use crate::cpg_cache;
#[cfg(not(test))]
use crate::cpg_cache as cache_load;
use crate::navigation::call_edge_cache::NavigationCallEdgeCacheStore;
use crate::navigation::NavigationIndex;
use crate::repo_loader::LoadedRepo;
use cache_load::CacheResult;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "cache_lifetime_tests.rs"]
mod cache_load;

/// Prism-owned cache subdir for this repo: <base>/prism/nav/<hash(canonical root)>/.
pub fn nav_cache_subdir(base: &Path, repo: &LoadedRepo) -> PathBuf {
    let canon = std::fs::canonicalize(&repo.root).unwrap_or_else(|_| repo.root.clone());
    let mut h = Sha256::new();
    h.update(canon.to_string_lossy().as_bytes());
    let id = format!("{:x}", h.finalize());
    base.join("prism").join("nav").join(id)
}

/// Prism-owned cache dir for this repo: <user cache>/prism/nav/<hash(canonical root)>/.
pub fn nav_cache_dir(repo: &LoadedRepo) -> PathBuf {
    let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
    nav_cache_subdir(&base, repo)
}

impl NavigationIndex {
    /// Default cache location (prism-owned, per-repo).
    pub fn build_cached(repo: &LoadedRepo) -> Self {
        Self::build_cached_observed(repo).0
    }

    pub(crate) fn build_cached_observed(repo: &LoadedRepo) -> (Self, &'static str) {
        let base = dirs::cache_dir().unwrap_or_else(std::env::temp_dir);
        Self::build_cached_under_observed(repo, &base)
    }

    /// Cache under a base dir by using Prism's per-repo navigation cache subdir.
    pub fn build_cached_under(repo: &LoadedRepo, base_dir: &Path) -> Self {
        Self::build_cached_under_observed(repo, base_dir).0
    }

    pub(crate) fn build_cached_under_observed(
        repo: &LoadedRepo,
        base_dir: &Path,
    ) -> (Self, &'static str) {
        let cache_dir = nav_cache_subdir(base_dir, repo);
        Self::build_cached_at_observed(repo, &cache_dir)
    }

    /// Cache in an exact final directory (test fixtures only).
    #[cfg(test)]
    pub(crate) fn build_cached_at(repo: &LoadedRepo, cache_dir: &Path) -> Self {
        Self::build_cached_at_observed(repo, cache_dir).0
    }

    /// Does not force lazy edges or perform any cache I/O.
    #[cfg_attr(not(feature = "mcp"), allow(dead_code))]
    pub(crate) fn sidecar_outcome(&self) -> &'static str {
        self.call_edge_cache_store
            .as_ref()
            .map_or("not_attempted", |s| s.outcome())
    }

    fn build_cached_at_observed(repo: &LoadedRepo, cache_dir: &Path) -> (Self, &'static str) {
        if let Err(e) = std::fs::create_dir_all(cache_dir) {
            eprintln!(
                "nav cache: failed to create cache directory {}: {e}; continuing without aborting",
                cache_dir.display()
            );
        }
        let has_type_db = repo.type_db.is_some();
        let mut topology_key =
            cpg_cache::compute_topology_key(&repo.file_hashes, &repo.manifest_hashes);
        if let Some(type_db) = repo.type_db.as_ref() {
            topology_key.insert(
                "type_db:fingerprint".to_string(),
                type_db.cache_fingerprint(),
            );
        }
        match cache_load::load_cache_with_topology(
            &repo.file_hashes,
            &topology_key,
            has_type_db,
            cache_dir,
        ) {
            CacheResult::Hit(cpg) => (
                Self::from_ctx_with_call_edge_cache(
                    CpgContext::build_with_cached_cpg(&repo.files, cpg, repo.type_db.as_ref()),
                    Some(NavigationCallEdgeCacheStore::new(
                        cache_dir,
                        &repo.file_hashes,
                        &topology_key,
                        has_type_db,
                        true,
                    )),
                ),
                "hit",
            ),
            CacheResult::PartialHit {
                cached_call_graph,
                cached_dfg,
                ..
            } => {
                drop((cached_call_graph, cached_dfg));
                // Nav caching is exact-hit-only in v1 (spec section 9), so partial hits are
                // intentionally treated as misses instead of reusing an incomplete CPG.
                eprintln!(
                    "nav cache: partial CPG cache hit in {}, rebuilding",
                    cache_dir.display()
                );
                #[cfg(test)]
                cache_load::record_rebuild_start();
                let ctx = CpgContext::build_with_scope_graph_inputs(
                    &repo.files,
                    repo.type_db.as_ref(),
                    repo.scope_graph_inputs.as_ref(),
                );
                if let Err(e) = cpg_cache::save_cache_with_topology(
                    &ctx.cpg,
                    &repo.file_hashes,
                    &topology_key,
                    has_type_db,
                    cache_dir,
                ) {
                    eprintln!(
                        "nav cache: failed to save CPG cache in {}: {e}",
                        cache_dir.display()
                    );
                }
                (
                    Self::from_ctx_with_call_edge_cache(
                        ctx,
                        Some(NavigationCallEdgeCacheStore::new(
                            cache_dir,
                            &repo.file_hashes,
                            &topology_key,
                            has_type_db,
                            false,
                        )),
                    ),
                    "partial_hit",
                )
            }
            CacheResult::Miss => {
                eprintln!(
                    "nav cache: CPG cache miss in {}, rebuilding",
                    cache_dir.display()
                );
                let ctx = CpgContext::build_with_scope_graph_inputs(
                    &repo.files,
                    repo.type_db.as_ref(),
                    repo.scope_graph_inputs.as_ref(),
                );
                if let Err(e) = cpg_cache::save_cache_with_topology(
                    &ctx.cpg,
                    &repo.file_hashes,
                    &topology_key,
                    has_type_db,
                    cache_dir,
                ) {
                    eprintln!(
                        "nav cache: failed to save CPG cache in {}: {e}",
                        cache_dir.display()
                    );
                }
                (
                    Self::from_ctx_with_call_edge_cache(
                        ctx,
                        Some(NavigationCallEdgeCacheStore::new(
                            cache_dir,
                            &repo.file_hashes,
                            &topology_key,
                            has_type_db,
                            false,
                        )),
                    ),
                    "miss",
                )
            }
        }
    }
}
