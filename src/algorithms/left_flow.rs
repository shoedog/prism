//! Algorithm 8: LeftFlow (AnalysisRelevantCode)
//!
//! Backward data-flow tracing: for each diff line, identifies L-values
//! (assignment targets) and traces backward to find all references to those
//! variables within the enclosing function. Also includes:
//! - Control flow condition variables
//! - Function call targets
//! - Return statements
//! - Branch boundaries for small branches

use crate::ast::ParsedFile;
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
/// Uses scope-aware variable reference lookup (`find_variable_references_scoped`)
/// for L-value tracing to avoid false positives from same-named variables in
/// different scopes.
/// Result of `left_flow_core`, containing both the slice lines and the call
/// names extracted from diff lines (to avoid redundant AST queries in callers).
pub struct LeftFlowResult {
    pub slice_lines: BTreeMap<usize, bool>,
    /// Function calls found on the diff lines: `(callee_name, line)`.
    pub diff_line_calls: Vec<(String, usize)>,
}

pub fn left_flow_core(
    parsed: &ParsedFile,
    files: &BTreeMap<String, ParsedFile>,
    func_node: &tree_sitter::Node<'_>,
    func_start: usize,
    func_end: usize,
    lines: &BTreeSet<usize>,
    config: &SliceConfig,
) -> LeftFlowResult {
    let mut slice_lines: BTreeMap<usize, bool> = BTreeMap::new();

    // Always include function signature and closing
    slice_lines.insert(func_start, false);
    slice_lines.insert(func_end, false);

    // Include all diff lines
    for &line in lines {
        slice_lines.insert(line, true);
    }

    // Phase 1: Trace L-values (assignment targets)
    let lvalues = parsed.assignment_lvalues_on_lines(func_node, lines);
    for (var_name, decl_line) in &lvalues {
        let refs = parsed.find_variable_references_scoped(func_node, var_name, *decl_line);
        for ref_line in refs {
            if ref_line >= func_start && ref_line <= func_end {
                slice_lines.entry(ref_line).or_insert(false);

                // If the reference is inside a small branch, include the branch
                if let Some((branch_start, branch_end)) = parsed.enclosing_branch(ref_line) {
                    let branch_size = branch_end - branch_start + 1;
                    if branch_size <= config.max_branch_lines {
                        for l in branch_start..=branch_end {
                            slice_lines.entry(l).or_insert(false);
                        }
                    } else {
                        // Include just the branch boundaries
                        slice_lines.entry(branch_start).or_insert(false);
                        slice_lines.entry(branch_end).or_insert(false);
                    }
                }
            }
        }
    }

    // Phase 2: Control flow condition variables
    let cond_vars = parsed.condition_variables_on_lines(func_node, lines);
    for (var_name, _) in &cond_vars {
        let refs = parsed.find_variable_references(func_node, var_name);
        for ref_line in refs {
            if ref_line >= func_start && ref_line <= func_end {
                slice_lines.entry(ref_line).or_insert(false);
            }
        }
    }

    // Phase 3: Function calls on diff lines
    let diff_line_calls = parsed.function_calls_on_lines(func_node, lines);
    for (func_name, _) in &diff_line_calls {
        // Try to find the callee in all parsed files
        for (_file_path, other_parsed) in files {
            if let Some(callee) = other_parsed.find_function_by_name(func_name) {
                let (callee_start, callee_end) = other_parsed.node_line_range(&callee);
                // Include callee function signature and boundaries
                slice_lines.entry(callee_start).or_insert(false);
                slice_lines.entry(callee_end).or_insert(false);
            }
        }
    }

    // Phase 4: Return statements
    if config.include_returns {
        let returns = parsed.return_statements(func_node);
        for ret_line in returns {
            slice_lines.entry(ret_line).or_insert(false);
            // Include enclosing branch for return
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

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &SliceConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::LeftFlow);
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

        // Process each function
        for ((func_start, func_end), lines) in &func_diff_lines {
            let func_node = match parsed.enclosing_function(*lines.iter().next().unwrap()) {
                Some(f) => f,
                None => continue,
            };

            let lf_result = left_flow_core(
                parsed,
                files,
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
