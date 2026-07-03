//! Output formatters for slice results.
//!
//! - `review` — line-numbered text, paper JSON, review JSON, callers JSON
//! - `mermaid` — Mermaid flowchart rendering for SliceGraph (added in Task 5)

pub mod mermaid;
pub mod navigation;
pub mod review;
pub mod review_compact;

pub use mermaid::{format_mermaid_report, render};

// Re-export the previous flat-file public API so existing imports keep working.
pub use review::{
    format_block, format_slice_result, render_review_block, to_callers_output, to_paper_format,
    to_review_output, CallerRef, CallersOutput, FunctionCallerEntry, MultiReviewOutput,
    ReviewBlock, ReviewOutput,
};
pub use review_compact::{
    severity_rank, to_compact_review_output, CompactMultiReviewOutput, CompactReviewBlock,
    CompactReviewOutput,
};
