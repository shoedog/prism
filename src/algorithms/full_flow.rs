//! Algorithm 9: FullFlow (AnalysisRelevantCodeRHS)
//!
//! Extends LeftFlow with forward data-flow tracing: also tracks R-values
//! (variables read on the right side of assignments) and callee function
//! signatures. This provides the most comprehensive context.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

use super::left_flow;

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::FullFlow);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Group diff lines by enclosing function
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

            // Reuse shared LeftFlow core (L-value, condition, call, return tracing)
            let mut slice_lines = left_flow::left_flow_core(
                parsed,
                files,
                &func_node,
                *func_start,
                *func_end,
                lines,
                config,
            );

            // === FullFlow addition: include full callee bodies if configured ===
            if config.trace_callees {
                let calls = parsed.function_calls_on_lines(&func_node, lines);
                for (func_name, _) in &calls {
                    for (_file_path, other_parsed) in files {
                        if let Some(callee) = other_parsed.find_function_by_name(func_name) {
                            let (cs, ce) = other_parsed.node_line_range(&callee);
                            for l in cs..=ce {
                                slice_lines.entry(l).or_insert(false);
                            }
                        }
                    }
                }
            }

            // === FullFlow addition: R-value tracing ===
            let rvalues = parsed.rvalue_identifiers_on_lines(&func_node, lines);

            // Track cross-file lines to add to the block separately
            // (keyed by file_path -> set of lines)
            let mut cross_file_lines: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();

            for (var_name, _) in &rvalues {
                // Trace all references to R-value variables within the function
                let refs = parsed.find_variable_references_scoped(&func_node, var_name, 0);
                for ref_line in refs {
                    if ref_line >= *func_start && ref_line <= *func_end {
                        slice_lines.entry(ref_line).or_insert(false);
                    }
                }

                // Cross-file: look for the variable as a function name in other files
                for (file_path, other_parsed) in files {
                    if *file_path == diff_info.file_path {
                        continue;
                    }
                    if let Some(func) = other_parsed.find_function_by_name(var_name) {
                        let (fs, fe) = other_parsed.node_line_range(&func);
                        let entry = cross_file_lines.entry(file_path.clone()).or_default();
                        entry.insert(fs);
                        entry.insert(fe);
                    }
                }
            }

            // Build block with same-file lines
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );
            for (line, is_diff) in &slice_lines {
                block.add_line(&diff_info.file_path, *line, *is_diff);
            }

            // Merge cross-file R-value reference lines into the block
            for (file_path, cf_lines) in &cross_file_lines {
                for &line in cf_lines {
                    block.add_line(file_path, line, false);
                }
            }

            result.blocks.push(block);
            block_id += 1;
        }

        // Global scope
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
