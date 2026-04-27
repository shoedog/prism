//! Python sanitizers for Phase 2 CWE coverage.
//!
//! Jinja2 default autoescape, SQL parametrization, URL allowlists, and YAML
//! SafeLoader are sink-time decisions in `taint.rs`. These recognizers only
//! cover value-transform sanitizers whose result can be treated as cleansed for
//! the corresponding category in the existing `FlowPath.cleansed_for` model.

use super::SanitizerRecognizer;
use crate::frameworks::SanitizerCategory;

pub const PYTHON_RECOGNIZERS: &[SanitizerRecognizer] = &[
    SanitizerRecognizer {
        call_path: "html.escape",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
    },
    SanitizerRecognizer {
        call_path: "markupsafe.escape",
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
    SanitizerRecognizer {
        call_path: "bleach.clean",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
    },
    SanitizerRecognizer {
        call_path: "bleach.linkify",
        category: SanitizerCategory::Xss,
        semantic_check: None,
        paired_check: None,
    },
];
