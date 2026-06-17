//! Name resolution scope-graph subsystem (Phase 1, PR-1).
//!
//! This module contains the language-neutral data model and the
//! `ResolutionPolicy` trait.  The engine (Task 2) and the Rust populator
//! (PR-2) will be added in subsequent PRs.
//!
//! **Nothing in the rest of the codebase consumes this module yet.**
//! It is wired only via `pub mod name_resolution;` in `lib.rs`.

pub mod types;
