//! Path-validation sanitizers (CWE-22 / PathTraversal category).
//!
//! See spec §3.8. Heuristic textual co-occurrence — the recognizer fires when
//! `filepath.Clean(X)` (or `filepath.Rel(base, X)`) is followed by a textual
//! `strings.HasPrefix` call anywhere in the function body. Known limitation:
//! does not distinguish positive vs negative guard direction (Phase 1.5+ for
//! CFG-aware refinement).

use super::SanitizerRecognizer;
use crate::frameworks::SanitizerCategory;

pub const PATH_RECOGNIZERS: &[SanitizerRecognizer] = &[
    SanitizerRecognizer {
        call_path: "filepath.Clean",
        category: SanitizerCategory::PathTraversal,
        semantic_check: None,
        paired_check: Some("strings.HasPrefix"),
    },
    SanitizerRecognizer {
        call_path: "filepath.Rel",
        category: SanitizerCategory::PathTraversal,
        semantic_check: None,
        paired_check: Some("strings.HasPrefix"),
    },
];
