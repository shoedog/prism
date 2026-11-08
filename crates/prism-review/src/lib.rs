//! # prism-review
//!
//! Diff-driven review over the [`prism`] core: the slicing algorithms, the output formats, the
//! instrumentation-targets projection and the clap grammar of the `prism` CLI (the binary of this
//! package). [`api`] is the published review facade; it re-exports all of `prism::api`.

pub mod algorithms;
pub mod api;
pub mod cli;
pub mod output;
pub mod targets;
