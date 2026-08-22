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
use crate::languages::Language;

/// Aggregate all active recognizers across categories. Iteration order is by
/// the const arrays in `shell.rs`, `path.rs`, `js_ts.rs`, and `python.rs`.
pub fn active_recognizers() -> impl Iterator<Item = &'static SanitizerRecognizer> {
    shell::SHELL_RECOGNIZERS
        .iter()
        .chain(path::PATH_RECOGNIZERS.iter())
        .chain(js_ts::JS_TS_RECOGNIZERS.iter())
        .chain(python::PYTHON_RECOGNIZERS.iter())
}

/// Languages with at least one active sanitizer recognizer. Item B (#7): derived from the
/// recognizer tables themselves (`active_recognizers().any(|r| r.languages.contains(&language))`)
/// rather than hand-maintained — adding a recognizer table for a new language, or a `languages`
/// entry to an existing one, is automatically reflected here; there is no longer a second source
/// of truth to keep in sync. (Currently equivalent to Go/Python/JavaScript/TypeScript/Tsx: Go via
/// `PATH_RECOGNIZERS`' paired-check family, Python via `PYTHON_RECOGNIZERS`, JS/TS/Tsx via
/// `JS_TS_RECOGNIZERS`; `SHELL_RECOGNIZERS` is empty per spec §3.9.)
pub fn sanitizer_supported(language: Language) -> bool {
    active_recognizers().any(|r| r.languages.contains(&language))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitizer_supported_matches_previous_hardcoded_set_for_every_language() {
        // Item B (#7) delta 1: `sanitizer_supported` is now derived from the recognizer tables
        // (`active_recognizers().any(|r| r.languages.contains(&language))`). This test pins that
        // the derived set is provably equal to the set it replaces (Go via PATH_RECOGNIZERS,
        // Python, JavaScript, TypeScript, Tsx) for EVERY `Language` variant — a drift guard for
        // future table edits, not a behavior change.
        for language in Language::all() {
            let previously_hardcoded = matches!(
                language,
                Language::Go
                    | Language::Python
                    | Language::JavaScript
                    | Language::TypeScript
                    | Language::Tsx
            );
            assert_eq!(
                sanitizer_supported(language),
                previously_hardcoded,
                "sanitizer_supported({language:?}) diverged from the previous hardcoded set"
            );
        }
    }
}
