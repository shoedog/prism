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

/// Clear `diagrams` from every finding in place, leaving every other field
/// untouched. This is the single place `--review-no-diagrams` strips
/// per-finding diagram payloads — call it at EVERY finding-vector call site
/// (per-result `findings` inside `to_compact_review_output`, and the
/// multi-run top-level `all_findings` aggregate built independently in
/// `src/main.rs`) so the two copies cannot drift out of sync.
pub fn strip_finding_diagrams(findings: &mut [SliceFinding]) {
    for f in findings {
        f.diagrams.clear();
    }
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
///
/// `no_diagrams` implements `--review-no-diagrams`: when set, the returned
/// `diagrams` (result-level) and every retained finding's `diagrams` are
/// empty. This is a payload-size reduction only — `finalize_diagrams` (the
/// sole producer of `DiagramWarning`s) has already run by the time `result`
/// reaches this function, so `diagram_warnings` are always passed through
/// unchanged regardless of `no_diagrams`.
pub fn to_compact_review_output(
    result: &SliceResult,
    sources: &BTreeMap<String, String>,
    min_severity_rank: u8,
    full_slices: bool,
    no_diagrams: bool,
) -> CompactReviewOutput {
    let mut retained_findings: Vec<SliceFinding> = result
        .findings
        .iter()
        .filter(|f| severity_rank(&f.severity) >= min_severity_rank)
        .cloned()
        .collect();
    if no_diagrams {
        strip_finding_diagrams(&mut retained_findings);
    }

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
        diagrams: if no_diagrams {
            Vec::new()
        } else {
            result.diagrams.clone()
        },
        diagram_warnings: result.diagram_warnings.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algorithms::finalize_diagrams;
    use crate::slice::{
        DiagramWarningKind, EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceGraph,
        SlicingAlgorithm,
    };

    fn graph_with_one_node() -> SliceGraph {
        SliceGraph {
            title: Some("Data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![GraphNode {
                id: "n1".to_string(),
                label: "n1".to_string(),
                kind: NodeKind::Step,
                file: None,
                line: None,
            }],
            edges: vec![],
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    /// Same shape as `algorithms::finalize_tests::graph_with_dangling`: an
    /// edge referencing a node id absent from `nodes`. Feeding this through
    /// the real `finalize_diagrams` (not a hand-constructed `DiagramWarning`)
    /// produces a genuine bug-class `DanglingEdge` warning, so these tests
    /// exercise the actual mechanism the flag must not disturb.
    fn graph_with_dangling_edge() -> SliceGraph {
        SliceGraph {
            title: Some("Data flow".to_string()),
            shape: GraphShape::Chain,
            nodes: vec![GraphNode {
                id: "a".to_string(),
                label: "A".to_string(),
                kind: NodeKind::Source,
                file: None,
                line: None,
            }],
            edges: vec![GraphEdge {
                from: "a".to_string(),
                to: "missing".to_string(),
                label: None,
                style: EdgeStyle::Solid,
            }],
            clusters: vec![],
            mermaid: String::new(),
        }
    }

    fn finding_with_diagram() -> SliceFinding {
        SliceFinding {
            algorithm: "taint".to_string(),
            file: "a.py".to_string(),
            line: 3,
            severity: "warning".to_string(),
            description: "d".to_string(),
            function_name: None,
            related_lines: vec![],
            related_files: vec![],
            category: None,
            parse_quality: None,
            diagrams: vec![graph_with_one_node()],
        }
    }

    #[test]
    fn strip_finding_diagrams_clears_diagrams_leaves_rest_untouched() {
        let mut findings = vec![finding_with_diagram()];
        strip_finding_diagrams(&mut findings);
        assert!(
            findings[0].diagrams.is_empty(),
            "diagrams must be cleared: {:?}",
            findings[0].diagrams
        );
        assert_eq!(findings[0].file, "a.py");
        assert_eq!(findings[0].line, 3);
        assert_eq!(findings[0].description, "d");
        assert_eq!(findings[0].severity, "warning");
    }

    #[test]
    fn strip_finding_diagrams_on_empty_slice_is_a_no_op() {
        let mut findings: Vec<SliceFinding> = vec![];
        strip_finding_diagrams(&mut findings); // must not panic
        assert!(findings.is_empty());
    }

    /// Builds a `SliceResult` with a result-level diagram whose dangling
    /// edge trips a real bug-class `DanglingEdge` warning through the actual
    /// `finalize_diagrams` pass, plus a finding carrying its own diagram.
    fn result_with_diagrams_and_bug_warning() -> SliceResult {
        let mut r = SliceResult::new(SlicingAlgorithm::Taint);
        r.diagrams.push(graph_with_dangling_edge());
        r.findings.push(finding_with_diagram());
        finalize_diagrams(&mut r, 40);
        r
    }

    #[test]
    fn no_diagrams_true_strips_result_and_finding_diagrams_but_keeps_warnings() {
        let result = result_with_diagrams_and_bug_warning();
        assert!(
            !result.diagram_warnings.is_empty(),
            "sanity: finalize_diagrams must have produced a real DanglingEdge \
             warning for this fixture"
        );
        let sources = BTreeMap::new();
        let out = to_compact_review_output(&result, &sources, 0, true, true);

        assert!(
            out.diagrams.is_empty(),
            "result-level diagrams must be stripped when no_diagrams is set: {:?}",
            out.diagrams
        );
        assert!(
            out.findings.iter().all(|f| f.diagrams.is_empty()),
            "finding-level diagrams must be stripped when no_diagrams is set: {:?}",
            out.findings
        );
        assert_eq!(
            out.diagram_warnings, result.diagram_warnings,
            "diagram_warnings must survive untouched, including the bug-class \
             DanglingEdge warning"
        );
        assert!(
            out.diagram_warnings
                .iter()
                .any(|w| matches!(w.kind, DiagramWarningKind::DanglingEdge)),
            "the bug-class warning itself must be present: {:?}",
            out.diagram_warnings
        );
    }

    #[test]
    fn no_diagrams_false_keeps_result_and_finding_diagrams_unchanged() {
        let result = result_with_diagrams_and_bug_warning();
        let sources = BTreeMap::new();
        let out = to_compact_review_output(&result, &sources, 0, true, false);

        assert_eq!(
            out.diagrams.len(),
            result.diagrams.len(),
            "unflagged behavior must be unchanged: result-level diagrams preserved"
        );
        assert!(
            out.findings.iter().any(|f| !f.diagrams.is_empty()),
            "unflagged behavior must be unchanged: at least one finding keeps \
             its diagrams: {:?}",
            out.findings
        );
        assert_eq!(
            out.diagram_warnings, result.diagram_warnings,
            "diagram_warnings unaffected either way"
        );
    }
}
