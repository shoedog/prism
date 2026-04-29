//! Category-aware sanitizer registry.
//!
//! Sanitizers cleanse tainted values for specific categories (XSS, SQLi, SSRF,
//! Deserialization, OsCommand, PathTraversal). A `cleansed_for` set on each
//! `FlowPath` tracks which categories a value has been cleansed for; sinks check
//! this set when evaluating suppression.
//!
//! See `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §3.4–§3.9.

pub mod js_ts;
pub mod path;
pub mod python;
pub mod shell;

pub use crate::frameworks::{CallSite, SanitizerCategory, SanitizerRecognizer};

/// Aggregate all active recognizers across categories. Iteration order is by
/// the const arrays in `shell.rs`, `path.rs`, and `python.rs`.
pub fn active_recognizers() -> impl Iterator<Item = &'static SanitizerRecognizer> {
    shell::SHELL_RECOGNIZERS
        .iter()
        .chain(path::PATH_RECOGNIZERS.iter())
        .chain(js_ts::JS_TS_RECOGNIZERS.iter())
        .chain(python::PYTHON_RECOGNIZERS.iter())
}

/// Check whether a `paired_check` token appears anywhere in the given source slice.
/// Used by paired-check recognizers (e.g., `filepath.Clean` → `strings.HasPrefix`)
/// as the legacy category-wide `FlowPath.cleansed_for` marker.
///
/// Go `PathTraversal` sink suppression no longer trusts this textual marker by
/// itself: `taint.rs` performs sink-time AST + CFG validation so inverted guards,
/// unrelated `HasPrefix` calls, and guard-after-sink shapes do not suppress.
pub fn paired_check_satisfied(function_body_source: &str, check_name: &str) -> bool {
    function_body_source.contains(check_name)
}
