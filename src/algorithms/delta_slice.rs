//! Delta Slice — minimal changes causing behavioral difference between versions.
//!
//! Compares the data flow graphs of old and new versions of changed files.
//! Identifies statements that participate in changed data flow paths —
//! new edges, removed edges, and modified assignments.

use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub fn slice(
    new_files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    old_repo: &Path,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::DeltaSlice);

    // Parse old versions of changed files
    let mut old_files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    for diff_info in &diff.files {
        let old_path = old_repo.join(&diff_info.file_path);
        if let Ok(source) = fs::read_to_string(&old_path) {
            if let Some(lang) = Language::from_path(&diff_info.file_path) {
                if let Ok(parsed) = ParsedFile::parse(&diff_info.file_path, &source, lang) {
                    old_files.insert(diff_info.file_path.clone(), parsed);
                }
            }
        }
    }

    // Build data flow graphs for both versions
    let old_dfg = DataFlowGraph::build(&old_files);
    let new_dfg = DataFlowGraph::build(new_files);

    // Find edges that differ between versions
    let old_edges: BTreeSet<(String, usize, String, usize)> = old_dfg
        .edges
        .iter()
        .map(|e| (e.from.file.clone(), e.from.line, e.to.file.clone(), e.to.line))
        .collect();

    let new_edges: BTreeSet<(String, usize, String, usize)> = new_dfg
        .edges
        .iter()
        .map(|e| (e.from.file.clone(), e.from.line, e.to.file.clone(), e.to.line))
        .collect();

    // New edges (added data flow)
    let added: BTreeSet<_> = new_edges.difference(&old_edges).collect();
    // Removed edges
    let removed: BTreeSet<_> = old_edges.difference(&new_edges).collect();

    // Collect all lines participating in changed data flow
    let mut changed_lines: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut diff_lines_set: BTreeSet<(String, usize)> = BTreeSet::new();

    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            diff_lines_set.insert((diff_info.file_path.clone(), line));
        }
    }

    for edge in &added {
        changed_lines.entry(edge.0.clone()).or_default().insert(edge.1);
        changed_lines.entry(edge.2.clone()).or_default().insert(edge.3);
    }
    for edge in &removed {
        changed_lines.entry(edge.0.clone()).or_default().insert(edge.1);
        changed_lines.entry(edge.2.clone()).or_default().insert(edge.3);
    }

    // Build output blocks
    let mut block_id = 0;
    for (file, lines) in &changed_lines {
        let mut block = DiffBlock::new(block_id, file.clone(), ModifyType::Modified);
        for &line in lines {
            let is_diff = diff_lines_set.contains(&(file.clone(), line));
            block.add_line(file, line, is_diff);
        }
        result.blocks.push(block);
        block_id += 1;
    }

    Ok(result)
}
