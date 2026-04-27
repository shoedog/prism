//! Path-validation sanitizers (CWE-22 / PathTraversal category).
//!
//! See spec §3.8. These recognizers still record the transform half of the
//! cleanser pair (`filepath.Clean` / `filepath.Rel`) and the paired-check token.
//! Go `PathTraversal` suppression is refined at sink evaluation time by
//! `taint.rs`: the sink-specific helper couples the sanitized result variable
//! to the sink argument and validates `strings.HasPrefix` guard direction with
//! AST + CFG reachability.

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
