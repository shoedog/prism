//! Algorithm 9: FullFlow (AnalysisRelevantCodeRHS)
//!
//! Extends LeftFlow with forward data-flow tracing: also tracks R-values
//! (variables read on the right side of assignments) and callee function
//! signatures. This provides the most comprehensive context.
//!
//! When a CPG with DFG edges is available, R-value tracing uses backward DFG
//! edge traversal for automatic field-sensitivity. Falls back to name-based
//! AST matching when DFG edges are absent.

use crate::cpg::{CpgContext, CpgNode, VarAccess};
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

use super::left_flow;

pub fn slice(ctx: &CpgContext, diff: &DiffInput, config: &SliceConfig) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::FullFlow);
    let mut block_id = 0;

    let has_dfg = !ctx.cpg.dfg.edges.is_empty();

    for diff_info in &diff.files {
        let parsed = match ctx.files.get(&diff_info.file_path) {
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
            let lf_result = left_flow::left_flow_core(
                ctx,
                &diff_info.file_path,
                &func_node,
                *func_start,
                *func_end,
                lines,
                config,
            );
            let mut slice_lines = lf_result.slice_lines;

            // === FullFlow addition: include full callee bodies if configured ===
            if config.trace_callees {
                for (func_name, _) in &lf_result.diff_line_calls {
                    for (_file_path, other_parsed) in ctx.files.iter() {
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
            //
            // DFG path: find Use nodes on diff lines, follow backward DFG edges
            // to their definitions, then include those def lines. This gives
            // field-sensitive R-value tracing (e.g., `dev->name` on the RHS
            // traces back to where `dev->name` was defined, not `dev->id`).
            //
            // Fallback: name-based AST matching (original behavior).
            let mut cross_file_lines: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();

            if has_dfg {
                let mut dfg_covered_rvalues: BTreeSet<String> = BTreeSet::new();

                for &diff_line in lines {
                    let nodes_at = ctx.cpg.nodes_at(&diff_info.file_path, diff_line);
                    for &idx in &nodes_at {
                        if let CpgNode::Variable {
                            access: VarAccess::Use,
                            path,
                            ..
                        } = ctx.cpg.node(idx)
                        {
                            dfg_covered_rvalues.insert(path.base.clone());

                            if let Some(var_loc) = ctx.cpg.to_var_location(idx) {
                                // Backward: find all definitions this use reaches
                                let defs = ctx.cpg.dfg_backward_reachable(&var_loc);
                                for def_loc in &defs {
                                    if def_loc.file == diff_info.file_path
                                        && def_loc.line >= *func_start
                                        && def_loc.line <= *func_end
                                    {
                                        slice_lines.entry(def_loc.line).or_insert(false);
                                    } else if def_loc.file != diff_info.file_path {
                                        // Cross-file reference
                                        cross_file_lines
                                            .entry(def_loc.file.clone())
                                            .or_default()
                                            .insert(def_loc.line);
                                    }
                                }

                                // Forward from each def: find all uses in this
                                // function (gives full variable reference coverage)
                                for def_loc in &defs {
                                    let uses = ctx.cpg.dfg_forward_reachable(def_loc);
                                    for use_loc in &uses {
                                        if use_loc.file == diff_info.file_path
                                            && use_loc.line >= *func_start
                                            && use_loc.line <= *func_end
                                        {
                                            slice_lines.entry(use_loc.line).or_insert(false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Fallback: R-values not covered by DFG Use nodes
                let rvalues = parsed.rvalue_identifiers_on_lines(&func_node, lines);
                for (var_name, _) in &rvalues {
                    if dfg_covered_rvalues.contains(var_name) {
                        continue;
                    }
                    let refs = parsed.find_variable_references_scoped(&func_node, var_name, 0);
                    for ref_line in refs {
                        if ref_line >= *func_start && ref_line <= *func_end {
                            slice_lines.entry(ref_line).or_insert(false);
                        }
                    }

                    // Cross-file: look for the variable as a function name
                    for (file_path, other_parsed) in ctx.files.iter() {
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
            } else {
                // No DFG — pure name-based (original behavior)
                let rvalues = parsed.rvalue_identifiers_on_lines(&func_node, lines);

                for (var_name, _) in &rvalues {
                    let refs = parsed.find_variable_references_scoped(&func_node, var_name, 0);
                    for ref_line in refs {
                        if ref_line >= *func_start && ref_line <= *func_end {
                            slice_lines.entry(ref_line).or_insert(false);
                        }
                    }

                    // Cross-file: look for the variable as a function name
                    for (file_path, other_parsed) in ctx.files.iter() {
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
