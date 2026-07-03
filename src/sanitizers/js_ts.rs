//! JavaScript / TypeScript sanitizers for Phase 3 CWE coverage.
//!
//! Sink-time helpers handle APIs with argument-specific or guard-direction-sensitive
//! safe forms (SQL parametrization, YAML Safe schema, literal-binary execFile,
//! URL allowlists, and path-prefix checks). These recognizers only cover simple
//! value transforms that fit the existing `FlowPath.cleansed_for` model.

use super::SanitizerRecognizer;
use crate::frameworks::SanitizerCategory;
use crate::languages::Language;

const JS_TS_LANGUAGES: &[Language] = &[Language::JavaScript, Language::TypeScript, Language::Tsx];

pub const JS_TS_RECOGNIZERS: &[SanitizerRecognizer] = &[
    SanitizerRecognizer {
        call_path: "DOMPurify.sanitize",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: JS_TS_LANGUAGES,
        // JS/TS calls have no keyword-argument syntax, so `data_param` is never consulted for
        // these entries (`src/reasoning/sanitizer_walk.rs` only checks it for Python
        // `keyword_argument` nodes); `None` here documents "not applicable", not "unverified".
        data_param: None,
    },
    SanitizerRecognizer {
        call_path: "escapeHtml",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: JS_TS_LANGUAGES,
        data_param: None,
    },
    SanitizerRecognizer {
        call_path: "escape",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: JS_TS_LANGUAGES,
        data_param: None,
    },
];
