//! Thin Slice — minimal backward slice with data dependencies only.
//!
//! Like LeftFlow but strips out all control flow context: no branch boundaries,
//! no condition variables, no return statements. Just the pure data chain:
//! assignment targets and every reference to those variables.
//!
//! Produces the most focused context, trading completeness for LLM attention.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::ThinSlice);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        let mut func_diff_lines: BTreeMap<(usize, usize), BTreeSet<usize>> = BTreeMap::new();
        let mut global_lines: BTreeSet<usize> = BTreeSet::new();

        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                let range = parsed.node_line_range(&func_node);
                func_diff_lines.entry(range).or_default().insert(line);
            } else {
                global_lines.insert(line);
            }
        }

        for ((func_start, func_end), lines) in &func_diff_lines {
            let func_node = match parsed.enclosing_function(*lines.iter().next().unwrap()) {
                Some(f) => f,
                None => continue,
            };

            let mut slice_lines: BTreeMap<usize, bool> = BTreeMap::new();

            // Include diff lines
            for &line in lines {
                slice_lines.insert(line, true);
            }

            // Data deps only: trace L-values to their references
            let lvalues = parsed.assignment_lvalues_on_lines(&func_node, lines);
            for (var_name, _) in &lvalues {
                let refs = parsed.find_variable_references(&func_node, var_name);
                for ref_line in refs {
                    if ref_line >= *func_start && ref_line <= *func_end {
                        slice_lines.entry(ref_line).or_insert(false);
                    }
                }
            }

            // No branch boundaries, no condition vars, no returns — that's the point.

            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );
            for (line, is_diff) in &slice_lines {
                block.add_line(&diff_info.file_path, *line, *is_diff);
            }
            result.blocks.push(block);
            block_id += 1;
        }

        if !global_lines.is_empty() {
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );
            for line in &global_lines {
                block.add_line(&diff_info.file_path, *line, true);
            }
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
