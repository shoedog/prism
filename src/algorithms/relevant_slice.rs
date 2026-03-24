//! Relevant Slice — backward slice plus alternate branch paths.
//!
//! Extends LeftFlow by also including the first N statements of alternate
//! branches for each control flow node in the slice. This catches the
//! "one branch flip away from a bug" scenario: missing else clauses,
//! unhandled switch cases, unchecked error returns.

use crate::ast::ParsedFile;
use crate::diff::DiffInput;
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// How many lines of alternate-path code to include.
const DEFAULT_FLIP_DEPTH: usize = 3;

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
) -> Result<SliceResult> {
    // Start with LeftFlow as the base
    let mut base = crate::algorithms::left_flow::slice(files, diff, config)?;
    base.algorithm = SlicingAlgorithm::RelevantSlice;

    let flip_depth = config.max_branch_lines.max(DEFAULT_FLIP_DEPTH);

    // For each block, find control flow nodes and add alternate paths
    for block in &mut base.blocks {
        let file_path = block.file.clone();
        let parsed = match files.get(&file_path) {
            Some(f) => f,
            None => continue,
        };

        // Collect existing lines in the slice
        let existing_lines: BTreeSet<usize> = block
            .file_line_map
            .get(&file_path)
            .map(|m| m.keys().copied().collect())
            .unwrap_or_default();

        // Find control flow nodes that overlap with slice lines
        let mut alternate_lines: BTreeMap<usize, bool> = BTreeMap::new();

        for &line in &existing_lines {
            // Find if this line is a control flow statement
            let _identifiers = parsed.identifiers_on_line(line);
            if let Some(func_node) = parsed.enclosing_function(line) {
                collect_alternate_branches(
                    parsed,
                    func_node,
                    &existing_lines,
                    flip_depth,
                    &mut alternate_lines,
                );
            }
        }

        // Add alternate lines to the block
        for (line, is_diff) in &alternate_lines {
            if !existing_lines.contains(line) {
                block.add_line(&file_path, *line, *is_diff);
            }
        }
    }

    Ok(base)
}

fn collect_alternate_branches(
    parsed: &ParsedFile,
    func_node: Node<'_>,
    slice_lines: &BTreeSet<usize>,
    flip_depth: usize,
    out: &mut BTreeMap<usize, bool>,
) {
    collect_alternates_inner(parsed, func_node, slice_lines, flip_depth, out);
}

fn collect_alternates_inner(
    parsed: &ParsedFile,
    node: Node<'_>,
    slice_lines: &BTreeSet<usize>,
    flip_depth: usize,
    out: &mut BTreeMap<usize, bool>,
) {
    let line = node.start_position().row + 1;

    // If this is a control flow node with a line in the slice
    if parsed.language.is_control_flow_node(node.kind()) && slice_lines.contains(&line) {
        // Find alternative/else branches
        let mut cursor = node.walk();
        let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

        for child in &children {
            let child_kind = child.kind();
            // Look for else clauses, elif, default cases
            if child_kind == "else_clause"
                || child_kind == "elif_clause"
                || child_kind == "else"
                || child_kind == "default_case"
                || child_kind == "switch_default"
            {
                let (alt_start, alt_end) = parsed.node_line_range(child);
                // Include up to flip_depth lines of the alternate path
                let end = alt_end.min(alt_start + flip_depth - 1);
                for l in alt_start..=end {
                    out.entry(l).or_insert(false);
                }
                // Always include the closing line
                out.entry(alt_end).or_insert(false);
            }
        }

        // For if statements without else, note the absence (include the closing brace line)
        if node.kind() == "if_statement" || node.kind() == "if_expression" {
            let has_else = children
                .iter()
                .any(|c| c.kind() == "else_clause" || c.kind() == "elif_clause" || c.kind() == "else");
            if !has_else {
                // Include the line after the if block ends — the "missing else" zone
                let (_, if_end) = parsed.node_line_range(&node);
                out.entry(if_end).or_insert(false);
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_alternates_inner(parsed, child, slice_lines, flip_depth, out);
    }
}
