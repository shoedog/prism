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
    let resolution = idle::IdlePolicy::resolve(None, ambient.as_deref(), &cfg, false)?;
    if let Some(note) = resolution.note {
        eprintln!("{note}");
    }
    run_with_idle_policy(cfg, resolution.policy)
}

pub fn run_with_idle_policy(cfg: ServerConfig, policy: idle::IdlePolicy) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
    if policy.timeout.is_some() {
        return idle_stdio::serve(&cfg, policy, &r);
    }
    match cfg.startup {
        StartupMode::Eager => {
            let mut p = SessionProvider::bootstrap(&cfg)?;
            transport::serve_stdio(&mut p, &r)
        }
        StartupMode::Lazy | StartupMode::Background => {
            let mut p = lazy::LazySessionProvider::new(&cfg)?;
            transport::serve_stdio_runtime(&mut p, &r)
        }
    }
}

/// Experimental eager, cache-free navigation. Reacquires before every tool call;
/// ordinary lazy/refresh-policy behavior is intentionally not activated here.
pub fn run_with_owner(cfg: ServerConfig, owner: crate::api::OwnerOptions) -> anyhow::Result<()> {
    let mut runtime = owner::OwnerRuntime::new(cfg, owner)?;
    transport::serve_stdio_runtime(&mut runtime, &registry::ToolRegistry::all_v1())
}
