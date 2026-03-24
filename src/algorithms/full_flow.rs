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

            let mut slice_lines: BTreeMap<usize, bool> = BTreeMap::new();

            // Function boundaries
            slice_lines.insert(*func_start, false);
            slice_lines.insert(*func_end, false);

            // Diff lines
            for &line in lines {
                slice_lines.insert(line, true);
            }

            // === LeftFlow portion ===

            // L-value tracing
            let lvalues = parsed.assignment_lvalues_on_lines(&func_node, lines);
            for (var_name, _) in &lvalues {
                let refs = parsed.find_variable_references(&func_node, var_name);
                for ref_line in refs {
                    if ref_line >= *func_start && ref_line <= *func_end {
                        slice_lines.entry(ref_line).or_insert(false);
                        if let Some((bs, be)) = parsed.enclosing_branch(ref_line) {
                            let size = be - bs + 1;
                            if size <= config.max_branch_lines {
                                for l in bs..=be {
                                    slice_lines.entry(l).or_insert(false);
                                }
                            } else {
                                slice_lines.entry(bs).or_insert(false);
                                slice_lines.entry(be).or_insert(false);
                            }
                        }
                    }
                }
            }

            // Control flow conditions
            let cond_vars = parsed.condition_variables_on_lines(&func_node, lines);
            for (var_name, _) in &cond_vars {
                let refs = parsed.find_variable_references(&func_node, var_name);
                for ref_line in refs {
                    if ref_line >= *func_start && ref_line <= *func_end {
                        slice_lines.entry(ref_line).or_insert(false);
                    }
                }
            }

            // Function calls
            let calls = parsed.function_calls_on_lines(&func_node, lines);
            for (func_name, _) in &calls {
                for (_file_path, other_parsed) in files {
                    if let Some(callee) = other_parsed.find_function_by_name(func_name) {
                        let (cs, ce) = other_parsed.node_line_range(&callee);
                        slice_lines.entry(cs).or_insert(false);
                        slice_lines.entry(ce).or_insert(false);

                        // FullFlow: if configured, include full callee body
                        if config.trace_callees {
                            for l in cs..=ce {
                                slice_lines.entry(l).or_insert(false);
                            }
                        }
                    }
                }
            }

            // Return statements
            if config.include_returns {
                let returns = parsed.return_statements(&func_node);
                for ret_line in returns {
                    slice_lines.entry(ret_line).or_insert(false);
                    if let Some((bs, be)) = parsed.enclosing_branch(ret_line) {
                        slice_lines.entry(bs).or_insert(false);
                        slice_lines.entry(be).or_insert(false);
                    }
                }
            }

            // === FullFlow additions: R-value tracing ===

            let rvalues = parsed.rvalue_identifiers_on_lines(&func_node, lines);
            for (var_name, _) in &rvalues {
                // Trace all references to R-value variables
                let refs = parsed.find_variable_references(&func_node, &var_name);
                for ref_line in refs {
                    if ref_line >= *func_start && ref_line <= *func_end {
                        slice_lines.entry(ref_line).or_insert(false);
                    }
                }

                // Cross-file: look for the variable declaration in other files
                for (file_path, other_parsed) in files {
                    if *file_path == diff_info.file_path {
                        continue;
                    }
                    // Check if this is a function name — include its signature
                    if let Some(func) = other_parsed.find_function_by_name(&var_name) {
                        let (fs, fe) = other_parsed.node_line_range(&func);
                        // Include function signature lines
                        let mut cross_block = DiffBlock::new(
                            block_id + 1000, // temporary ID for cross-file refs
                            file_path.clone(),
                            diff_info.modify_type.clone(),
                        );
                        cross_block.add_line(file_path, fs, false);
                        cross_block.add_line(file_path, fe, false);
                        // These will be merged into the main block
                    }
                }
            }

            // Build block
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
