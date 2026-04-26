//! Category-aware sanitizer registry.
//!
//! Sanitizers cleanse tainted values for specific categories (XSS, SQLi, SSRF,
//! Deserialization, OsCommand, PathTraversal). A `cleansed_for` set on each
//! `FlowPath` tracks which categories a value has been cleansed for; sinks check
//! this set when evaluating suppression.
//!
//! See `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §3.4–§3.9.

pub mod path;
pub mod shell;

pub use crate::frameworks::{CallSite, SanitizerCategory, SanitizerRecognizer};

/// Aggregate all active recognizers across categories. Iteration order is by
/// the const arrays in `shell.rs` and `path.rs` (shell first, then path).
pub fn active_recognizers() -> impl Iterator<Item = &'static SanitizerRecognizer> {
    shell::SHELL_RECOGNIZERS
        .iter()
        .chain(path::PATH_RECOGNIZERS.iter())
}

/// Check whether a `paired_check` token appears anywhere in the given source slice.
/// Used by paired-check recognizers (e.g., `filepath.Clean` → `strings.HasPrefix`).
/// Textual co-occurrence per spec §3.4 / §3.8.
///
/// Known limitation: does not distinguish positive vs negative guard direction —
/// both `if strings.HasPrefix(rel, "..") { return error }` (correct) and
/// `if strings.HasPrefix(rel, "..") { use rel }` (real bug) suppress equally.
/// CFG-aware refinement is deferred to Phase 1.5+.
pub fn paired_check_satisfied(function_body_source: &str, check_name: &str) -> bool {
    function_body_source.contains(check_name)
}
