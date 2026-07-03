//! Compact `--format review` output (Task P1, Change 3).
//!
//! `--format json` and `--format review` historically shared
//! `ReviewOutput`/`MultiReviewOutput` byte-for-byte (compatibility tests pin
//! `--format json`, see `tests/cli/nav_compat_test.rs`). This module adds a
//! REVIEW-ONLY compact serialization used only when `cli.format == "review"`:
//!
//! a. Drops `slice_lines`/`diff_lines` from each block (keeps `slice_text`).
//! b. Filters findings (`findings` per-result and `all_findings` aggregate)
//!    to severity >= a floor (default `warning`).
//! c. Retains a block iff at least one RETAINED finding's `(file, line)` is
//!    present in the block's `file_line_map` — findings are never dropped by
//!    block filtering; only blocks are.
//!
//! `--format json` keeps using `to_review_output`/`ReviewOutput` from
//! `review.rs`, untouched.

use crate::diff::DiffBlock;
use crate::output::review::{render_review_block, taint_line_annotations, ReviewBlock};
use crate::slice::{AlgorithmError, DiagramWarning, FileParseQuality, SliceFinding, SliceResult};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Rank a finding severity for floor comparisons. Ordering:
/// `info < suggestion < warning < concern`. An unrecognized severity string
/// ranks as `concern` (never silently dropped by a floor).
pub fn severity_rank(severity: &str) -> u8 {
    match severity {
        "info" => 0,
        "suggestion" => 1,
        "warning" => 2,
        "concern" => 3,
        _ => 3,
    }
}

#[derive(Debug, Serialize)]
pub struct CompactReviewBlock {
    pub block_id: usize,
    pub file: String,
    pub modify_type: String,
    pub slice_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_lines: Option<[usize; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub lvalues: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rvalues: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub callees: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cross_file_refs: Vec<String>,
    /// Lines identified as taint sources (populated for taint algorithm only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub source_lines: Vec<usize>,
    /// Lines identified as taint sinks (populated for taint algorithm only).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sink_lines: Vec<usize>,
}

impl From<ReviewBlock> for CompactReviewBlock {
    fn from(rb: ReviewBlock) -> Self {
        Self {
            block_id: rb.block_id,
            file: rb.file,
            modify_type: rb.modify_type,
            slice_text: rb.slice_text,
            function_name: rb.function_name,
            function_lines: rb.function_lines,
            lvalues: rb.lvalues,
            rvalues: rb.rvalues,
            callees: rb.callees,
            cross_file_refs: rb.cross_file_refs,
            // `render_review_block` always leaves these empty (see
            // review.rs); the real values are derived from the full finding
            // set and assigned by `to_compact_review_output` right after
            // this `From` call. Start empty here rather than copying
            // `rb.source_lines`/`rb.sink_lines` (which are already empty) so
            // it's clear at this call site that these fields are not yet
            // populated.
            source_lines: vec![],
            sink_lines: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CompactReviewOutput {
    pub algorithm: String,
    pub slices: Vec<CompactReviewBlock>,
    pub findings: Vec<SliceFinding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parse_quality: BTreeMap<String, FileParseQuality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagrams: Vec<crate::slice::SliceGraph>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagram_warnings: Vec<DiagramWarning>,
}

#[derive(Debug, Serialize)]
pub struct CompactMultiReviewOutput {
    pub version: String,
    pub algorithms_run: Vec<String>,
    pub results: Vec<CompactReviewOutput>,
    pub all_findings: Vec<SliceFinding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<AlgorithmError>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parse_quality: BTreeMap<String, FileParseQuality>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagram_warnings: Vec<DiagramWarning>,
}

/// True iff `block`'s `file_line_map` contains at least one `(file, line)`
/// pair present in `locations` (retained-finding locations). Blocks are
/// cross-file, so this checks every file in the map, not just `block.file`.
fn block_matches_any_location(block: &DiffBlock, locations: &BTreeSet<(String, usize)>) -> bool {
    block.file_line_map.iter().any(|(file, line_map)| {
        line_map
            .keys()
            .any(|line| locations.contains(&(file.clone(), *line)))
    })
}

/// Convert a `SliceResult` into the compact review-only output: severity
/// floor on findings, `slice_lines`/`diff_lines` dropped, and (unless
/// `full_slices`) blocks with no retained-finding line dropped.
///
/// Reuses `render_review_block` for byte-identical `slice_text` rendering —
/// only the exposed shape (which fields, which blocks) differs from
/// `to_review_output`.
pub fn to_compact_review_output(
    result: &SliceResult,
    sources: &BTreeMap<String, String>,
    min_severity_rank: u8,
    full_slices: bool,
) -> CompactReviewOutput {
    let retained_findings: Vec<SliceFinding> = result
        .findings
        .iter()
        .filter(|f| severity_rank(&f.severity) >= min_severity_rank)
        .cloned()
        .collect();

    let retained_locations: BTreeSet<(String, usize)> = retained_findings
        .iter()
        .map(|f| (f.file.clone(), f.line))
        .collect();

    let slices: Vec<CompactReviewBlock> = result
        .blocks
        .iter()
        .filter(|block| full_slices || block_matches_any_location(block, &retained_locations))
        .map(|block| {
            let rb = render_review_block(block, sources);
            let mut crb = CompactReviewBlock::from(rb);
            // Re-derive source/sink line annotations from the full finding
            // set (matching `to_review_output`'s annotation semantics), not
            // just the retained subset — a hidden info taint_source still
            // marks the rendered slice_text for context.
            let (source_lines, sink_lines) = taint_line_annotations(&block.file, &result.findings);
            crb.source_lines = source_lines;
            crb.sink_lines = sink_lines;
            crb
        })
        .collect();

    CompactReviewOutput {
        algorithm: result.algorithm.name().to_string(),
        slices,
        findings: retained_findings,
        warnings: result.warnings.clone(),
        parse_quality: BTreeMap::new(),
        diagrams: result.diagrams.clone(),
        diagram_warnings: result.diagram_warnings.clone(),
    }
}
