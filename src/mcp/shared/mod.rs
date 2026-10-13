//! Building blocks of shared mode, where one daemon per repository checkout is reached through
//! a stdio shim: where a daemon's files live and which configuration it serves. Nothing here
//! listens on a socket, takes a lock or starts a process; the daemon and the shim are the
//! consumers.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §4.3, §4.4, §7.

mod acl;
pub mod cache_base;
pub mod dir_chain;
pub mod identity;
pub mod paths;
pub mod preamble;
pub mod preamble_reader;
pub mod runtime_dir;

/// Version of the private shim–daemon protocol (spec §4.4). It is part of the identity key
/// (§4.3), so builds that speak different versions never look for each other's socket, and
/// the preamble repeats it so that a version difference is reported, not misparsed.
pub const SHARED_PROTOCOL_VERSION: u32 = 1;
