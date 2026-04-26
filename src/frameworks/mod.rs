//! Framework-aware source/sink/sanitizer detection.
//!
//! Each framework module defines a `pub const SPEC: FrameworkSpec` describing
//! detection signals, source patterns, sinks, and (optional) sanitizers.
//! Detection is per-file and lazy via `ParsedFile::framework()`.
//!
//! See `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §2 for the
//! full design.

use crate::ast::ParsedFile;

pub mod gin;
pub mod gorilla_mux;
pub mod nethttp;

/// Origin classification matching `provenance_slice.rs::Origin` enum.
/// Re-exported here for convenience in framework specs.
pub use crate::algorithms::provenance_slice::Origin;

/// A category that a sink consumes or a sanitizer cleanses.
/// Defined here in Commit 1 (used by FrameworkSpec.sinks); SanitizerCategory
/// is the same enum, fully populated in Commit 3 with all variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SanitizerCategory {
    Xss,
    Sqli,
    Ssrf,
    Deserialization,
    OsCommand,
    PathTraversal,
}

/// One framework's detection + pattern data.
pub struct FrameworkSpec {
    pub name: &'static str,
    pub detect: fn(&ParsedFile) -> bool,
    pub sources: &'static [SourcePattern],
    pub sinks: &'static [SinkPattern],
    pub sanitizers: &'static [SanitizerRecognizer],
}

/// A source pattern: a call expression that produces tainted values.
pub struct SourcePattern {
    /// Identifier path matched against call expressions, e.g. "c.Param", "r.URL.Query".
    pub call_path: &'static str,
    /// Origin classification.
    pub origin: Origin,
    /// Which call argument *receives* taint as a side-effect. `None` = the call result itself
    /// is the tainted value. `Some(i)` = the i-th argument becomes tainted (e.g.,
    /// `c.BindJSON(&v)` taints `&v` at index 0).
    pub taints_arg: Option<usize>,
}

/// A sink pattern: a call expression that consumes tainted values.
pub struct SinkPattern {
    pub call_path: &'static str,
    pub category: SanitizerCategory,
    pub tainted_arg_indices: &'static [usize],
    pub semantic_check: Option<fn(&CallSite) -> bool>,
}

/// A sanitizer recognizer (defined here in Commit 1 as an empty type for forward
/// compatibility; populated in Commit 3 via `src/sanitizers/mod.rs`).
pub struct SanitizerRecognizer {
    pub call_path: &'static str,
    pub category: SanitizerCategory,
    pub semantic_check: Option<fn(&CallSite) -> bool>,
    /// For pattern-pair cleansers (Clean→HasPrefix, Rel→check), the recognizer is the *first
    /// half*; the second-half check name is resolved at suppression time by textual
    /// co-occurrence in the same function body.
    pub paired_check: Option<&'static str>,
}

/// Call-site reflection helper. Wraps a tree-sitter call expression node + the
/// originating source so `semantic_check` callbacks can inspect literal arguments.
pub struct CallSite<'a> {
    pub call_node: tree_sitter::Node<'a>,
    pub source: &'a str,
}

impl<'a> CallSite<'a> {
    /// Returns the literal string value of argument `i` (0-indexed) if it is a
    /// string-literal expression; `None` if the argument is non-literal (variable,
    /// expression, etc.) or out of range.
    ///
    /// String-literal kinds recognized:
    /// - Go `interpreted_string_literal` (double-quoted) — quotes stripped.
    /// - Go `raw_string_literal` (backtick-quoted) — backticks stripped.
    pub fn literal_arg(&self, i: usize) -> Option<&'a str> {
        let args = self.call_node.child_by_field_name("arguments")?;
        let mut cursor = args.walk();
        for (idx, child) in args.named_children(&mut cursor).enumerate() {
            if idx != i {
                continue;
            }
            if child.kind() == "interpreted_string_literal" || child.kind() == "raw_string_literal"
            {
                let text = child.utf8_text(self.source.as_bytes()).ok()?;
                let trimmed = text
                    .trim_start_matches('"')
                    .trim_end_matches('"')
                    .trim_start_matches('`')
                    .trim_end_matches('`');
                return Some(trimmed);
            }
            // Argument exists at this index but is not a string literal.
            return None;
        }
        None
    }
}

/// Ordered registry of all known frameworks. Ordering matters: more specific frameworks
/// (gin, gorilla/mux) take precedence over net/http per spec §2.3.
pub const ALL_FRAMEWORKS: &[&FrameworkSpec] = &[&gin::SPEC, &gorilla_mux::SPEC, &nethttp::SPEC];

/// Detect the active framework for a file. First match wins.
/// Returns `None` if no framework matches (quiet-mode default per ACK §3 Q5).
pub fn detect_for(parsed: &ParsedFile) -> Option<&'static FrameworkSpec> {
    ALL_FRAMEWORKS
        .iter()
        .copied()
        .find(|spec| (spec.detect)(parsed))
}
