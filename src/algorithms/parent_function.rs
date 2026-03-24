//! Algorithm 7: ParentFunction
//!
//! Includes the entire enclosing function for each diff line.
//! Provides full function context but may include irrelevant code.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::ParentFunction);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find all unique enclosing functions for the diff lines
        let mut function_ranges: BTreeSet<(usize, usize)> = BTreeSet::new();

        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                let range = parsed.node_line_range(&func_node);
                function_ranges.insert(range);
            }
        }

        // Create a block for each function
        for (func_start, func_end) in &function_ranges {
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );

            for line in *func_start..=*func_end {
                let is_diff = diff_info.diff_lines.contains(&line);
                block.add_line(&diff_info.file_path, line, is_diff);
            }

            result.blocks.push(block);
            block_id += 1;
        }

        // Handle diff lines outside any function (global scope)
        let covered: BTreeSet<usize> = function_ranges
            .iter()
            .flat_map(|(s, e)| *s..=*e)
            .collect();

        let uncovered: Vec<usize> = diff_info
            .diff_lines
            .iter()
            .filter(|l| !covered.contains(l))
            .copied()
            .collect();

        if !uncovered.is_empty() {
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );
            for line in uncovered {
                block.add_line(&diff_info.file_path, line, true);
            }
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
