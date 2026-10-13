//! Building blocks of shared mode, where one daemon per repository checkout is reached through
//! a stdio shim: where a daemon's files live, which configuration it serves, and the private
//! preamble the shim and the daemon exchange before any MCP byte. Nothing here listens on a
//! socket, takes a lock or starts a process; the daemon and the shim are the consumers.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.3, §4.4, §7.

pub mod runtime_dir;
