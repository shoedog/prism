//! Output formatters for slice results.
//!
//! - `review` — line-numbered text, paper JSON, review JSON, callers JSON
//! - `mermaid` — Mermaid flowchart rendering for SliceGraph (added in Task 5)
//!
//! Registered rows make finding references with borrowed identity accessors.
//!
//! ```
//! use prism::slice::RuleRef;
//! use prism_review::output::rule_registry::{RuleInfo, JS_EVAL};
//! let row: &'static RuleInfo = &JS_EVAL;
//! let rule: RuleRef = row.rule_ref();
//! assert_eq!((rule.id(), rule.cwe(), rule.severity()), (row.id(), row.cwe(), None));
//! ```
//!
//! ```compile_fail
//! use prism::slice::RuleRef;
//! use prism_review::output::rule_registry::JS_EVAL;
//! type Ref = RuleRef;
//! let _ = vec![Ref { id: format!("{}-computed", 42), cwe: None, severity: None }];
//! ```
//!
//! ```compile_fail
//! use prism_review::output::rule_registry::JS_EVAL;
//! let mut rule = JS_EVAL.rule_ref().clone();
//! rule.id = "computed".into();
//! ```
//!
//! ```compile_fail
//! use prism_review::output::rule_registry::JS_EVAL;
//! let mut rule = JS_EVAL.rule_ref().clone();
//! rule.cwe = None;
//! ```
//!
//! ```compile_fail
//! use prism_review::output::rule_registry::{RuleInfo, JS_EVAL};
//! static BAD: RuleInfo = RuleInfo { id: "computed", ..JS_EVAL };
//! let _ = &BAD;
//! ```

pub mod mermaid;
pub mod onboarding;
pub mod review;
pub mod review_compact;
pub mod sarif;
mod sarif_model;
mod sarif_rules;

pub use mermaid::{format_mermaid_report, render};
/// Nav evidence rendering moved to core (`navigation::render`, #701 S5); old path kept.
pub use prism::navigation::render as navigation;
pub use prism::rule_registry;
pub use sarif::{to_sarif, SarifInputs};

// Re-export the previous flat-file public API so existing imports keep working.
pub use review::{
    format_block, format_slice_result, render_review_block, to_callers_output, to_paper_format,
    to_review_output, to_review_output_for, CallerRef, CallersOutput, FunctionCallerEntry,
    MultiReviewOutput, ReviewBlock, ReviewOutput,
};
pub use review_compact::{
    count_below_floor, severity_rank, strip_finding_diagrams, to_compact_review_output,
    to_compact_review_output_for, CompactMultiReviewOutput, CompactReviewBlock,
    CompactReviewOutput,
};

#[cfg(test)]
mod rule_pin_tests;
