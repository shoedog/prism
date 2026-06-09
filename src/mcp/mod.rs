pub mod error;
pub mod input;
pub mod output;
pub mod registry;
pub mod session;
pub mod tools;
pub mod transport;

pub use session::{CacheMode, ServerConfig, SessionProvider};

pub fn run(cfg: ServerConfig) -> anyhow::Result<()> {
    let p = SessionProvider::bootstrap(&cfg)?;
    let r = registry::ToolRegistry::nav_v1();
    transport::serve_stdio(&p, &r)
}
