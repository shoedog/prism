//! The shared daemon (spec §3.1, §4.2): `prism-mcp daemon ...`. The readiness coordinator
//! decides when the one index is built and how long each client waits for it; the provider slot
//! holds the built index; the accept loop owns the key's files, admits connections through a
//! bounded handshake stage, and drains when the daemon retires or loses its root.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §3.1, §4.2–§4.4, §5.

mod admission;
mod budget;
mod clock;
mod connection;
mod control;
mod coordinator;
mod fatal;
mod files;
mod log;
pub(super) mod peer;
mod run;
mod server;
#[cfg(test)]
mod server_support;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod wire_support;

pub use run::{run_daemon, DaemonOptions, DEFAULT_IDLE_TTL_SECS, DEFAULT_MAX_CLIENTS};

// One provider is queried from many connection threads at once, one registry serves all of
// them, and the coordinator is shared by every one (spec §4.2). A field that stops being
// `Send + Sync` fails here, by name.
const _: fn() = || {
    fn shared_between_threads<T: Send + Sync>() {}
    shared_between_threads::<crate::mcp::SessionProvider>();
    shared_between_threads::<crate::mcp::registry::ToolRegistry>();
    shared_between_threads::<coordinator::DaemonState>();
};
