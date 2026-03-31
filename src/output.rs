//! Output formatting for sliced code.
//!
//! Produces the line-numbered, diff-marked format described in the paper:
//!
//! ```text
//! +linenumber|{added code line}
//! -linenumber|{deleted code line}
//!  linenumber|{context code line}
//!        ...|...
//! ```

use crate::diff::DiffBlock;
use crate::slice::{AlgorithmError, SliceFinding, SliceResult};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write;

// --- Review output types ---

#[derive(Debug, Serialize)]
pub struct ReviewBlock {
    pub block_id: usize,
    pub file: String,
    pub modify_type: String,
    pub diff_lines: Vec<usize>,
    pub slice_lines: Vec<usize>,
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
}

#[derive(Debug, Serialize)]
pub struct ReviewOutput {
    pub algorithm: String,
    pub slices: Vec<ReviewBlock>,
    pub findings: Vec<SliceFinding>,
    /// Parse quality warnings for input files (e.g. high ERROR-node rate).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MultiReviewOutput {
    pub version: String,
    pub algorithms_run: Vec<String>,
    pub results: Vec<ReviewOutput>,
    pub all_findings: Vec<SliceFinding>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<AlgorithmError>,
    /// Parse quality warnings for input files (e.g. high ERROR-node rate).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Render a DiffBlock as a line-numbered, diff-marked text snippet.
/// Non-contiguous lines get "...|..." ellipsis gaps.
pub fn render_review_block(block: &DiffBlock, sources: &BTreeMap<String, String>) -> ReviewBlock {
    let mut slice_text = String::new();
    let mut all_slice_lines: Vec<usize> = Vec::new();
    let mut cross_file_refs: Vec<String> = Vec::new();

    for (file_path, line_map) in &block.file_line_map {
        let source = match sources.get(file_path) {
            Some(s) => s,
            None => continue,
        };
        let source_lines: Vec<&str> = source.lines().collect();

        if block.file_line_map.len() > 1 {
            writeln!(slice_text, "--- {}", file_path).unwrap();
            if *file_path != block.file {
                cross_file_refs.push(file_path.clone());
            }
        }

        let lines: Vec<(&usize, &bool)> = line_map.iter().collect();
        let mut prev_line: Option<usize> = None;

        for (&line_num, &is_diff) in lines.iter() {
            all_slice_lines.push(line_num);

            if let Some(prev) = prev_line {
                if line_num > prev + 1 {
                    let width = format!("{}", line_num).len();
                    writeln!(slice_text, "{:>width$}|...", "...", width = width).unwrap();
                }
            }

            let line_content = if line_num > 0 && line_num <= source_lines.len() {
                source_lines[line_num - 1]
            } else {
                ""
            };

            let prefix = if is_diff { "+" } else { " " };
            writeln!(slice_text, "{}{:>4}|{}", prefix, line_num, line_content).unwrap();
            prev_line = Some(line_num);
        }
    }

    ReviewBlock {
        block_id: block.block_id,
        file: block.file.clone(),
        modify_type: block.modify_type.code().to_string(),
        diff_lines: block.diff_lines.iter().copied().collect(),
        slice_lines: all_slice_lines,
        slice_text,
        function_name: None,
        function_lines: None,
        lvalues: vec![],
        rvalues: vec![],
        callees: vec![],
        cross_file_refs,
    }
}

/// Convert a SliceResult into a ReviewOutput.
pub fn to_review_output(result: &SliceResult, sources: &BTreeMap<String, String>) -> ReviewOutput {
    let slices = result
        .blocks
        .iter()
        .map(|b| render_review_block(b, sources))
        .collect();
    ReviewOutput {
        algorithm: result.algorithm.name().to_string(),
        slices,
        findings: result.findings.clone(),
        warnings: result.warnings.clone(),
    }
}

/// Format a slice block as the paper's line-numbered output.
pub fn format_block(block: &DiffBlock, sources: &BTreeMap<String, String>) -> String {
    let mut output = String::new();

    for (file_path, line_map) in &block.file_line_map {
        let source = match sources.get(file_path) {
            Some(s) => s,
            None => continue,
        };
        let source_lines: Vec<&str> = source.lines().collect();

        if block.file_line_map.len() > 1 {
            writeln!(output, "--- {}", file_path).unwrap();
        }

        let lines: Vec<(&usize, &bool)> = line_map.iter().collect();
        let mut prev_line: Option<usize> = None;

        for (&line_num, &is_diff) in lines.iter() {
            // Insert ellipsis for gaps
            if let Some(prev) = prev_line {
                if line_num > prev + 1 {
                    let width = format!("{}", line_num).len();
                    writeln!(output, "{:>width$}|...", "...", width = width).unwrap();
                }
            }

            let line_content = if line_num > 0 && line_num <= source_lines.len() {
                source_lines[line_num - 1]
            } else {
                ""
            };

            let prefix = if is_diff { "+" } else { " " };
            writeln!(output, "{}{:>4}|{}", prefix, line_num, line_content).unwrap();
            prev_line = Some(line_num);
        }
    }

    output
}

/// Format all blocks in a slice result.
pub fn format_slice_result(blocks: &[DiffBlock], sources: &BTreeMap<String, String>) -> String {
    let mut output = String::new();

    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            output.push_str("\n---\n\n");
        }
        writeln!(
            output,
            "# Block {} [{}] {}",
            block.block_id,
            block.modify_type.code(),
            block.file
        )
        .unwrap();
        output.push_str(&format_block(block, sources));
    }

    output
}

/// Produce a JSON output compatible with the paper's diff_outputs.json format.
pub fn to_paper_format(blocks: &[DiffBlock]) -> serde_json::Value {
    let mut output = Vec::new();

    for block in blocks {
        let mut diff_list = serde_json::Map::new();
        for (file_path, line_map) in &block.file_line_map {
            let mut file_lines = serde_json::Map::new();
            for (line, is_diff) in line_map {
                file_lines.insert(
                    line.to_string(),
                    serde_json::Value::Number(if *is_diff { 1.into() } else { 0.into() }),
                );
            }
            diff_list.insert(file_path.clone(), serde_json::Value::Object(file_lines));
        }

        output.push(serde_json::json!({
            "block_id": block.block_id,
            "file": block.file,
            "modify_type": block.modify_type.code(),
            "diff_lines": block.diff_lines.iter().collect::<Vec<_>>(),
            "diff_list": diff_list,
        }));
    }

    serde_json::Value::Array(output)
}
