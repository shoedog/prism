pub mod concise_shape;
pub mod error;
pub mod evidence_view;
pub mod freshness;
pub mod input;
pub mod lazy;
pub mod output;
pub mod registry;
pub mod session;
pub mod tools;
pub mod tools_reasoning;
pub mod tools_refresh;
pub mod transport;

pub(crate) use session::{AutoRefreshSummary, RefreshVerification};
pub use session::{
    CacheMode, RefreshPolicy, RefreshSummary, ServerConfig, SessionProvider, StartupMode,
    FIRST_CALL_WAIT_MAX,
};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let r = registry::ToolRegistry::all_v1();
    match cfg.startup {
        StartupMode::Eager => {
            let mut p = SessionProvider::bootstrap(&cfg)?;
            transport::serve_stdio(&mut p, &r)
        }
        StartupMode::Lazy => {
            let mut p = lazy::LazySessionProvider::new(&cfg)?;
            transport::serve_stdio_runtime(&mut p, &r)
        }
    }
}
