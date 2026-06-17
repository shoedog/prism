//! Name resolution scope-graph subsystem (Phase 1, PR-1).
//!
//! This module contains the language-neutral data model
//! ([`types`]), the language-neutral resolution [`engine`], and the Rust
//! [`rust_policy`].  The Rust populator (PR-2) and the consumers (PR-3) are
//! added in subsequent PRs.
//!
//! **Nothing in the rest of the codebase consumes this module yet** — it is
//! INERT (wired only via `pub mod name_resolution;` in `lib.rs`). The engine +
//! Rust policy are exercised solely by `tests/name_resolution/`.

pub mod engine;
pub mod graph;
pub mod rust_policy;
pub mod rust_populator;
pub mod types;
