//! The shim: `prism-mcp --repo X --shared` (spec §3, §4.1). It holds no index. It finds the
//! shared daemon for its configuration, starting one if none is alive, and is then a byte pipe
//! between its client's stdio and the daemon's socket. If no daemon answers `READY` within one
//! deadline it fails closed with a one-line diagnostic: it never builds a private index.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.1, §4.4, §4.6, §10.
// The pieces are wired into a production caller by `run_shim` (Task 6), which removes this.
#![allow(dead_code)]

mod connect;
#[cfg(test)]
mod shim_support;
mod socket;
