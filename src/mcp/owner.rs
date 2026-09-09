//! A separate runtime prevents ordinary stale-on-refresh-error fallback from
//! serving proof-derived edges. No session accessor is publicly exposed here.
use super::{
    freshness::{FreshnessProbe, FreshnessReport},
    lazy::Readiness,
    transport::SessionRuntime,
    AutoRefreshSummary, CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, StartupMode,
};
use crate::executable_owner::admission::Failure;
use crate::{api::OwnerOptions, navigation::NavigationSession};

pub(super) struct OwnerRuntime {
    cfg: ServerConfig,
    owner: OwnerOptions,
    active: Option<NavigationSession>,
    snapshot: Option<FreshnessProbe>,
    before: FreshnessReport,
    generation: u64,
    last_admission: Option<serde_json::Value>,
}

impl OwnerRuntime {
    pub(super) fn new(cfg: ServerConfig, owner: OwnerOptions) -> anyhow::Result<Self> {
        anyhow::ensure!(
            cfg.startup == StartupMode::Eager,
            "owner activation requires --eager; lazy activation is unsupported"
        );
        anyhow::ensure!(
            matches!(cfg.cache, CacheMode::NoCache),
            "owner activation requires --no-cache"
        );
        anyhow::ensure!(
            cfg.refresh_policy == RefreshPolicy::WarnOnly,
            "owner activation requires warn-only policy; acquisition is fresh per tool call"
        );
        owner.validate()?;
        let cfg = super::session::canonical_config(&cfg)?;
        let mut runtime = Self {
            cfg,
            owner,
            active: None,
            snapshot: None,
            before: FreshnessReport::from_changed_paths(Vec::new()),
            generation: 0,
            last_admission: None,
        };
        runtime.acquire()?;
        Ok(runtime)
    }

    fn acquire(&mut self) -> anyhow::Result<()> {
        // Clear before even metadata IO. Prior stamps report refresh status only;
        // they never authorize proof reuse (config/libs require full acquisition).
        self.active = None;
        self.last_admission = None;
        self.before = self
            .snapshot
            .take()
            .map(|p| p.check())
            .unwrap_or_else(|| FreshnessReport::from_changed_paths(Vec::new()));
        self.owner
            .replace(&mut self.active, &self.cfg.repo_root)
            .inspect_err(|error| {
                self.last_admission = error.downcast_ref::<Failure>().map(Failure::diagnostic);
            })?;
        self.snapshot = self
            .active
            .as_ref()
            .map(|s| FreshnessProbe::from_loaded_repo(&s.repo));
        self.generation = self.generation.saturating_add(1);
        Ok(())
    }
}

impl SessionRuntime for OwnerRuntime {
    fn ensure_ready(&mut self) -> Readiness {
        match self.acquire() {
            Ok(()) => Readiness::Ready,
            Err(error) => Readiness::Failed {
                error: error
                    .downcast_ref::<Failure>()
                    .map(Failure::message)
                    .unwrap_or_else(|| error.to_string()),
            },
        }
    }
    fn admission_diagnostic(&self) -> Option<serde_json::Value> {
        self.last_admission.clone()
    }
    fn startup_mode(&self) -> StartupMode {
        StartupMode::Eager
    }
    fn session(&self) -> &NavigationSession {
        self.active
            .as_ref()
            .expect("transport only queries after successful readiness")
    }
    fn freshness(&self) -> Option<&FreshnessProbe> {
        None
    }
    fn known_stale_after_refresh(&self) -> Option<&FreshnessReport> {
        None
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::WarnOnly
    }
    fn refresh_index(&mut self) -> anyhow::Result<RefreshSummary> {
        // Transport has just acquired via ensure_ready, including for refresh_index.
        let session = self
            .active
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("owner session unavailable"))?;
        Ok(RefreshSummary {
            status: "refreshed",
            strategy: "full",
            fallback_reason: Some("owner_fresh_per_call"),
            generation: self.generation,
            indexed_files: session.repo.files.len(),
            tracked_paths: self
                .snapshot
                .as_ref()
                .map_or(0, FreshnessProbe::tracked_len),
            stale_before_refresh: self.before.stale,
            stale_index_total_before_refresh: self.before.total_changed,
            stale_index_paths_before_refresh: self.before.changed_paths.clone(),
        })
    }
    fn auto_refresh_index(&mut self) -> anyhow::Result<AutoRefreshSummary> {
        anyhow::bail!("owner runtime does not use automatic stale-snapshot refresh")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_runtime_refuses_unsupported_policies_before_loading() {
        let options = OwnerOptions::new("/absent/compiler.js", "tsconfig.json");
        let mut cfg = ServerConfig::new("/absent/root".into());
        assert!(OwnerRuntime::new(cfg.clone(), options.clone())
            .err()
            .unwrap()
            .to_string()
            .contains("--eager"));
        cfg.startup = StartupMode::Eager;
        assert!(OwnerRuntime::new(cfg.clone(), options.clone())
            .err()
            .unwrap()
            .to_string()
            .contains("--no-cache"));
        cfg.cache = CacheMode::NoCache;
        for policy in [RefreshPolicy::AutoFull, RefreshPolicy::AutoIncremental] {
            cfg.refresh_policy = policy;
            assert!(OwnerRuntime::new(cfg.clone(), options.clone())
                .err()
                .unwrap()
                .to_string()
                .contains("warn-only"));
        }
    }
}
