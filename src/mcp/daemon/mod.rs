//! The core of the shared daemon (spec §4.2): the readiness coordinator, which decides when
//! the one index is built and how long each client waits for it, and the provider slot the
//! built index is published into. Nothing here accepts a connection, takes a file lock or
//! starts a process: the accept loop, admission and retirement are slice S4.
//!
//! Design: `docs/superpowers/specs/2026-10-09-shared-prism-daemon-design.md` §3.1, §4.2, §5.
// No production code calls into this module until slice S4 adds the accept loop. Remove this
// allowance there.
#![cfg_attr(not(test), allow(dead_code))]

mod budget;
#[cfg(test)]
mod test_support;
