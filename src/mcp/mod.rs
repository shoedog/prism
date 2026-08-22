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
    let mut p = SessionProvider::bootstrap(&cfg)?;
    let r = registry::ToolRegistry::all_v1();
    transport::serve_stdio(&mut p, &r)
}
