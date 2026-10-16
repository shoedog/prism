//! The shim: `prism-mcp --repo X --shared` (spec §3, §4.1). It holds no index. It finds the
//! shared daemon for its configuration, starting one if none is alive, and is then a byte pipe
//! between its client's stdio and the daemon's socket. If no daemon answers `READY` within one
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6, §10.

mod arbitrate;
mod connect;
mod diagnostic;
mod key_files;
mod pump;
mod run;
#[cfg(test)]
mod shim_support;
mod socket;
mod spawn;

pub use run::{run_shim, ShimOptions, DEFAULT_DAEMON_START_TIMEOUT_SECS};
