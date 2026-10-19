pub mod concise_shape;
#[cfg(unix)]
mod daemon;
pub mod error;
pub mod evidence_view;
pub mod framing;
pub mod freshness;
pub mod idle;
mod idle_runtime;
mod idle_stdio;
pub mod input;
pub mod lazy;
pub(crate) mod mem_attribution;
pub mod mem_report;
pub(crate) mod mem_sample;
pub mod mem_tree_alloc;
pub mod output;
mod owner;
pub mod registry;
pub mod session;
pub mod settings;
#[cfg(unix)]
pub mod shared;
#[cfg(unix)]
mod shim;
pub mod tools;
pub mod tools_reasoning;
pub mod tools_refresh;
pub mod transport;

#[cfg(unix)]
pub use daemon::{run_daemon, DaemonOptions, DEFAULT_IDLE_TTL_SECS, DEFAULT_MAX_CLIENTS};
pub(crate) use session::{AutoRefreshSummary, BuildStage, RefreshVerification};
pub use session::{
    CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider, StartupMode,
    FIRST_CALL_WAIT_MAX,
};
#[cfg(unix)]
pub use shim::{
    run_daemon_shutdown, run_daemon_status, run_shim, ShimOptions, ShutdownOptions, StatusOptions,
    DEFAULT_CONTROL_TIMEOUT_SECS, DEFAULT_DAEMON_START_TIMEOUT_SECS,
};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let ambient = std::env::var("PRISM_MCP_INDEX_IDLE_SECS").ok();
    let policy = library_idle_policy(&cfg, ambient.as_deref())?;
    run_with_idle_policy(cfg, policy)
}

fn library_idle_policy(
    cfg: &ServerConfig,
    ambient: Option<&str>,
) -> anyhow::Result<idle::IdlePolicy> {
    let resolution = idle::IdlePolicy::resolve(None, ambient, cfg, false)?;
    if let Some(note) = resolution.note {
        eprintln!("{note}");
    }
    Ok(resolution.policy)
}

pub fn run_with_idle_policy(cfg: ServerConfig, policy: idle::IdlePolicy) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
    route_stdio(
        &cfg,
        policy,
        |cfg, policy| idle_stdio::serve(cfg, policy, &r),
        |cfg| serve_legacy(cfg, &r),
    )
}

fn route_stdio<T>(
    cfg: &ServerConfig,
    policy: idle::IdlePolicy,
    idle: impl FnOnce(&ServerConfig, idle::IdlePolicy) -> T,
    legacy: impl FnOnce(&ServerConfig) -> T,
) -> T {
    if policy.timeout.is_some() {
        idle(cfg, policy)
    } else {
        legacy(cfg)
    }
}

fn serve_legacy(cfg: &ServerConfig, r: &registry::ToolRegistry) -> anyhow::Result<()> {
    match cfg.startup {
        StartupMode::Eager => {
            let mut p = SessionProvider::bootstrap(cfg)?;
            transport::serve_stdio(&mut p, r)
        }
        StartupMode::Lazy | StartupMode::Background => {
            let mut p = lazy::LazySessionProvider::new(cfg)?;
            transport::serve_stdio_runtime(&mut p, r)
        }
    }
}

/// Experimental eager, cache-free navigation. Reacquires before every tool call;
/// ordinary lazy/refresh-policy behavior is intentionally not activated here.
pub fn run_with_owner(cfg: ServerConfig, owner: crate::api::OwnerOptions) -> anyhow::Result<()> {
    let mut runtime = owner::OwnerRuntime::new(cfg, owner)?;
    transport::serve_stdio_runtime(&mut runtime, &registry::ToolRegistry::all_v1())
}

#[cfg(test)]
mod idle_route_tests {
    use super::*;

    #[test]
    fn idle_default_and_library_run_route_legacy() {
        for startup in [
            StartupMode::Lazy,
            StartupMode::Background,
            StartupMode::Eager,
        ] {
            let mut cfg = ServerConfig::new(".".into());
            cfg.startup = startup;
            let default = idle::IdlePolicy::resolve(None, None, &cfg, false)
                .unwrap()
                .policy;
            let library = library_idle_policy(&cfg, None).unwrap();
            for policy in [
                default,
                library,
                idle::IdlePolicy::enabled(std::time::Duration::ZERO),
            ] {
                assert_eq!(
                    route_stdio(&cfg, policy, |_, _| "idle", |_| "legacy"),
                    "legacy"
                );
            }
        }
        let cfg = ServerConfig::new(".".into());
        let opt_in = idle::IdlePolicy::resolve(
            Some(&idle::PROVISIONAL_IDLE_SECS.to_string()),
            None,
            &cfg,
            false,
        )
        .unwrap()
        .policy;
        assert_eq!(
            route_stdio(&cfg, opt_in, |_, _| "idle", |_| "legacy"),
            "idle"
        );
    }
}
