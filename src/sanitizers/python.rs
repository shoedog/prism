//! Python sanitizers for Phase 2 CWE coverage.
//!
//! Jinja2 default autoescape, SQL parametrization, URL allowlists, and YAML
//! SafeLoader are sink-time decisions in `taint.rs`. These recognizers only
//! cover value-transform sanitizers whose result can be treated as cleansed for
//! the corresponding category in the existing `FlowPath.cleansed_for` model.

use super::SanitizerRecognizer;
use crate::frameworks::SanitizerCategory;
use crate::languages::Language;

const PYTHON_LANGUAGES: &[Language] = &[Language::Python];

pub const PYTHON_RECOGNIZERS: &[SanitizerRecognizer] = &[
    // `html.escape(s, quote=True)` — stdlib `html` module. Data param is `s`.
    // https://docs.python.org/3/library/html.html#html.escape
    SanitizerRecognizer {
        call_path: "html.escape",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: PYTHON_LANGUAGES,
        data_param: Some("s"),
    },
    // `markupsafe.escape(s)` — MarkupSafe (Flask/Jinja2's dependency). Data param is `s`.
    SanitizerRecognizer {
        call_path: "markupsafe.escape",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: PYTHON_LANGUAGES,
        data_param: Some("s"),
    },
    // Bare `escape(...)` covers an unqualified import of either `html.escape` or
    // `markupsafe.escape` (both take `s` as their first/data parameter) — the two realistic
    // sources for a bare `escape` symbol in this recognizer's scope.
    SanitizerRecognizer {
        call_path: "escape",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: PYTHON_LANGUAGES,
        data_param: Some("s"),
    },
    // `bleach.clean(text, tags=..., attributes=..., ...)` — data param is `text`.
    // https://bleach.readthedocs.io/en/latest/clean.html
    SanitizerRecognizer {
        call_path: "bleach.clean",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: PYTHON_LANGUAGES,
        data_param: Some("text"),
    },
    // `bleach.linkify(text, callbacks=..., skip_tags=..., parse_email=False)` — data param is
    // `text`. https://bleach.readthedocs.io/en/latest/linkify.html
    SanitizerRecognizer {
        call_path: "bleach.linkify",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
        languages: PYTHON_LANGUAGES,
        data_param: Some("text"),
    },
];
