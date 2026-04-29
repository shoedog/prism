//! JavaScript / TypeScript sanitizers for Phase 3 CWE coverage.
//!
//! Sink-time helpers handle APIs with argument-specific or guard-direction-sensitive
//! safe forms (SQL parametrization, YAML Safe schema, literal-binary execFile,
//! URL allowlists, and path-prefix checks). These recognizers only cover simple
//! value transforms that fit the existing `FlowPath.cleansed_for` model.

use super::SanitizerRecognizer;
use crate::frameworks::SanitizerCategory;

pub const JS_TS_RECOGNIZERS: &[SanitizerRecognizer] = &[
    SanitizerRecognizer {
        call_path: "DOMPurify.sanitize",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
    },
    SanitizerRecognizer {
        call_path: "escapeHtml",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
    },
    SanitizerRecognizer {
        call_path: "escape",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
    },
];
