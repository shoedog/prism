//! Algorithm 8: LeftFlow (AnalysisRelevantCode)
//!
//! Backward data-flow tracing: for each diff line, identifies L-values
//! (assignment targets) and traces backward to find all references to those
//! variables within the enclosing function. Also includes:
//! - Control flow condition variables
//! - Function call targets
//! - Return statements
//! - Branch boundaries for small branches
//!
//! When a CPG with DFG edges is available, Phases 1 and 2 use AccessPath-based
//! DFG edge traversal for automatic field-sensitivity. Falls back to name-based
//! AST matching when DFG edges are absent for a variable.

use crate::cpg::{CpgContext, CpgNode, VarAccess};
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Shared left-flow tracing logic used by both LeftFlow (Algorithm 8) and
/// FullFlow (Algorithm 9).
///
/// For a single function block, traces L-values, condition variables, function
/// calls, and return statements. Returns the set of (line, is_diff) pairs that
/// should be included in the slice.
///
/// When the CPG has DFG edges, uses field-sensitive DFG traversal for Phases 1
/// and 2. Falls back to name-based AST matching for variables not covered by
/// the DFG.
pub struct LeftFlowResult {
    pub slice_lines: BTreeMap<usize, bool>,
    /// Function calls found on the diff lines: `(callee_name, line)`.
    pub diff_line_calls: Vec<(String, usize)>,
}

pub fn left_flow_core(
    ctx: &CpgContext,
    file_path: &str,
    func_node: &tree_sitter::Node<'_>,
    func_start: usize,
    func_end: usize,
    lines: &BTreeSet<usize>,
    config: &SliceConfig,
) -> LeftFlowResult {
    let parsed = &ctx.files[file_path];
    let mut slice_lines: BTreeMap<usize, bool> = BTreeMap::new();

    // Always include function signature and closing
    slice_lines.insert(func_start, false);
    slice_lines.insert(func_end, false);

    // Include all diff lines
    for &line in lines {
        slice_lines.insert(line, true);
    }

    // Check whether the CPG has DFG edges we can use.
    let has_dfg = !ctx.cpg.dfg.edges.is_empty();

    // Phase 1: Trace L-values (assignment targets)
    //
    // DFG path: find Def VarLocations on diff lines, follow forward DFG edges
    // to discover all downstream uses. This is field-sensitive (tracks
    // `dev->name` separately from `dev->id`).
    //
    // Fallback: name-based AST matching (original behavior).
    if has_dfg {
        let mut dfg_covered_lines: BTreeSet<usize> = BTreeSet::new();

        for &diff_line in lines {
            let nodes_at = ctx.cpg.nodes_at(file_path, diff_line);
            for &idx in &nodes_at {
                if let CpgNode::Variable {
                    access: VarAccess::Def,
                    line,
                    ..
                } = ctx.cpg.node(idx)
                {
                    if let Some(var_loc) = ctx.cpg.to_var_location(idx) {
                        dfg_covered_lines.insert(*line);
                        let reachable = ctx.cpg.dfg_forward_reachable(&var_loc);
                        for reached in &reachable {
                            if reached.file == file_path
                                && reached.line >= func_start
                                && reached.line <= func_end
                            {
                                slice_lines.entry(reached.line).or_insert(false);
                                add_branch_context(parsed, reached.line, config, &mut slice_lines);
                            }
                        }
                    }
                }
            }
        }

        // Fallback: for diff lines with L-values NOT covered by DFG Def nodes,
        // use name-based AST matching.
        let lvalues = parsed.assignment_lvalues_on_lines(func_node, lines);
        for (var_name, decl_line) in &lvalues {
            if dfg_covered_lines.contains(decl_line) {
                continue; // Already handled by DFG
            }
            let refs = parsed.find_variable_references_scoped(func_node, var_name, *decl_line);
            for ref_line in refs {
                if ref_line >= func_start && ref_line <= func_end {
                    slice_lines.entry(ref_line).or_insert(false);
                    add_branch_context(parsed, ref_line, config, &mut slice_lines);
                }
            }
        }
    } else {
        // No DFG available — pure name-based (original behavior)
        let lvalues = parsed.assignment_lvalues_on_lines(func_node, lines);
        for (var_name, decl_line) in &lvalues {
            let refs = parsed.find_variable_references_scoped(func_node, var_name, *decl_line);
            for ref_line in refs {
                if ref_line >= func_start && ref_line <= func_end {
                    slice_lines.entry(ref_line).or_insert(false);
                    add_branch_context(parsed, ref_line, config, &mut slice_lines);
                }
            }
        }
    }

    // Phase 2: Control flow condition variables
    //
    // DFG path: find Use nodes on diff lines that are condition variables,
    // follow backward DFG edges to their definitions, then forward from those
    // defs to find all downstream uses. This gives field-sensitive tracking
    // of condition variables.
    //
    // Fallback: name-based AST matching (original behavior).
    if has_dfg {
        let cond_vars = parsed.condition_variables_on_lines(func_node, lines);
        let mut dfg_covered_conds: BTreeSet<String> = BTreeSet::new();

        for (var_name, cond_line) in &cond_vars {
            // Find Use nodes at the condition line matching this variable
            let nodes_at = ctx.cpg.nodes_at(file_path, *cond_line);
            let mut found_dfg_edge = false;

            for &idx in &nodes_at {
                if let CpgNode::Variable {
                    access: VarAccess::Use,
                    path,
                    ..
                } = ctx.cpg.node(idx)
                {
                    if path.base != *var_name {
                        continue;
                    }
                    if let Some(var_loc) = ctx.cpg.to_var_location(idx) {
                        // Backward: find where this condition variable is defined
                        let defs = ctx.cpg.dfg_backward_reachable(&var_loc);
                        for def_loc in &defs {
                            if def_loc.file == file_path
                                && def_loc.line >= func_start
                                && def_loc.line <= func_end
                            {
                                slice_lines.entry(def_loc.line).or_insert(false);
                            }
                            // Forward from each def: find all uses of this variable
                            let uses = ctx.cpg.dfg_forward_reachable(def_loc);
                            for use_loc in &uses {
                                if use_loc.file == file_path
                                    && use_loc.line >= func_start
                                    && use_loc.line <= func_end
                                {
                                    slice_lines.entry(use_loc.line).or_insert(false);
                                }
                            }
                        }
                        found_dfg_edge = true;
                    }
                }
            }

            if found_dfg_edge {
                dfg_covered_conds.insert(var_name.clone());
            }
        }

        // Fallback for condition variables not covered by DFG
        for (var_name, _) in &cond_vars {
            if dfg_covered_conds.contains(var_name) {
                continue;
            }
            let refs = parsed.find_variable_references(func_node, var_name);
            for ref_line in refs {
                if ref_line >= func_start && ref_line <= func_end {
                    slice_lines.entry(ref_line).or_insert(false);
                }
            }
        }
    } else {
        let cond_vars = parsed.condition_variables_on_lines(func_node, lines);
        for (var_name, _) in &cond_vars {
            let refs = parsed.find_variable_references(func_node, var_name);
            for ref_line in refs {
                if ref_line >= func_start && ref_line <= func_end {
                    slice_lines.entry(ref_line).or_insert(false);
                }
            }
        }
    }

    // Phase 3: Function calls on diff lines (unchanged — not DFG-related)
    let diff_line_calls = parsed.function_calls_on_lines(func_node, lines);
    for (func_name, _) in &diff_line_calls {
        for (_file_path, other_parsed) in ctx.files.iter() {
            if let Some(callee) = other_parsed.find_function_by_name(func_name) {
                let (callee_start, callee_end) = other_parsed.node_line_range(&callee);
                slice_lines.entry(callee_start).or_insert(false);
                slice_lines.entry(callee_end).or_insert(false);
            }
        }
    }

    // Phase 4: Return statements (unchanged)
    if config.include_returns {
        let returns = parsed.return_statements(func_node);
        for ret_line in returns {
            slice_lines.entry(ret_line).or_insert(false);
            if let Some((branch_start, branch_end)) = parsed.enclosing_branch(ret_line) {
                slice_lines.entry(branch_start).or_insert(false);
                slice_lines.entry(branch_end).or_insert(false);
            }
        }
    }

    LeftFlowResult {
        slice_lines,
        diff_line_calls,
    }
}

/// Add branch context for a reference line: if inside a small branch, include
/// all branch lines; otherwise just include branch boundaries.
fn add_branch_context(
    parsed: &crate::ast::ParsedFile,
    ref_line: usize,
    config: &SliceConfig,
    slice_lines: &mut BTreeMap<usize, bool>,
) {
    if let Some((branch_start, branch_end)) = parsed.enclosing_branch(ref_line) {
        let branch_size = branch_end - branch_start + 1;
        if branch_size <= config.max_branch_lines {
            for l in branch_start..=branch_end {
                slice_lines.entry(l).or_insert(false);
            }
        } else {
            slice_lines.entry(branch_start).or_insert(false);
            slice_lines.entry(branch_end).or_insert(false);
        }
    }
}

pub fn slice(ctx: &CpgContext, diff: &DiffInput, config: &SliceConfig) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::LeftFlow);
    let mut block_id = 0;

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

        // Process each function
        for ((func_start, func_end), lines) in &func_diff_lines {
            let func_node = match parsed.enclosing_function(*lines.iter().next().unwrap()) {
                Some(f) => f,
                None => continue,
            };

            let lf_result = left_flow_core(
                ctx,
                &diff_info.file_path,
                &func_node,
                *func_start,
                *func_end,
                lines,
                config,
            );

            // Build block
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );
            for (line, is_diff) in &lf_result.slice_lines {
                block.add_line(&diff_info.file_path, *line, *is_diff);
            }
            result.blocks.push(block);
            block_id += 1;
        }

        // Handle global scope lines
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
