//! Quantum Slice — concurrent state superposition enumeration.
//!
//! For async/concurrent code, enumerates all possible states a variable could
//! hold at a given program point considering possible interleavings.
//!
//! Identifies async patterns (await, goroutines, promises, threads) and models
//! which assignments could race with the diff point.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// A possible state for a variable at a program point.
#[derive(Debug, Clone)]
pub struct PossibleState {
    pub var_name: String,
    pub state_label: String,
    pub assignment_line: usize,
    pub assignment_file: String,
    pub is_async_dependent: bool,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    target_var: Option<&str>,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::QuantumSlice);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        for &line in &diff_info.diff_lines {
            let func_node = match parsed.enclosing_function(line) {
                Some(f) => f,
                None => continue,
            };

            // Find variables on this diff line
            let vars_on_line: Vec<String> = if let Some(tv) = target_var {
                vec![tv.to_string()]
            } else {
                parsed
                    .identifiers_on_line(line)
                    .iter()
                    .map(|n| parsed.node_text(n).to_string())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            };

            for var_name in &vars_on_line {
                // Find all assignment points for this variable in the function
                let (func_start, func_end) = parsed.node_line_range(&func_node);
                let all_lines: BTreeSet<usize> = (func_start..=func_end).collect();
                let lvalues = parsed.assignment_lvalues_on_lines(&func_node, &all_lines);

                let assignments: Vec<usize> = lvalues
                    .iter()
                    .filter(|(name, _)| name == var_name)
                    .map(|(_, l)| *l)
                    .collect();

                if assignments.is_empty() {
                    continue;
                }

                // Detect async context
                let async_lines = find_async_points(parsed, &func_node);
                let is_async_func = is_async_function(parsed, &func_node);

                // Build possible states
                let mut states: Vec<PossibleState> = Vec::new();

                for &assign_line in &assignments {
                    let is_after_async = async_lines.iter().any(|&al| al < assign_line && al > func_start);
                    let is_before_async = async_lines.iter().any(|&al| al > assign_line && al < func_end);

                    let state_label = if is_after_async || is_before_async {
                        format!(
                            "line {} (async-dependent: assignment {}await boundary)",
                            assign_line,
                            if is_after_async { "after " } else { "before " }
                        )
                    } else {
                        format!("line {} (synchronous)", assign_line)
                    };

                    states.push(PossibleState {
                        var_name: var_name.clone(),
                        state_label,
                        assignment_line: assign_line,
                        assignment_file: diff_info.file_path.clone(),
                        is_async_dependent: is_after_async || is_before_async,
                    });
                }

                // If there's async context and the variable could be uninitialized
                if is_async_func && !assignments.is_empty() {
                    states.push(PossibleState {
                        var_name: var_name.clone(),
                        state_label: "undefined/uninitialized (async operation not yet completed)"
                            .to_string(),
                        assignment_line: 0,
                        assignment_file: diff_info.file_path.clone(),
                        is_async_dependent: true,
                    });
                }

                // Build block with all relevant lines
                if states.iter().any(|s| s.is_async_dependent) {
                    let mut block = DiffBlock::new(
                        block_id,
                        diff_info.file_path.clone(),
                        ModifyType::Modified,
                    );

                    // Include the diff line
                    block.add_line(&diff_info.file_path, line, true);

                    // Include all assignment lines
                    for state in &states {
                        if state.assignment_line > 0 {
                            block.add_line(
                                &state.assignment_file,
                                state.assignment_line,
                                false,
                            );
                        }
                    }

                    // Include async boundary lines
                    for &async_line in &async_lines {
                        block.add_line(&diff_info.file_path, async_line, false);
                    }

                    // Include function boundaries
                    block.add_line(&diff_info.file_path, func_start, false);
                    block.add_line(&diff_info.file_path, func_end, false);

                    result.blocks.push(block);
                    block_id += 1;
                }
            }
        }
    }

    Ok(result)
}

fn find_async_points(parsed: &ParsedFile, func_node: &Node<'_>) -> Vec<usize> {
    let mut points = Vec::new();
    find_async_inner(parsed, *func_node, &mut points);
    points.sort();
    points.dedup();
    points
}

fn find_async_inner(parsed: &ParsedFile, node: Node<'_>, out: &mut Vec<usize>) {
    let kind = node.kind();
    let line = node.start_position().row + 1;

    let is_async = match parsed.language {
        Language::Python => {
            kind == "await" || kind == "await_expression"
        }
        Language::JavaScript | Language::TypeScript => {
            kind == "await_expression"
                || (kind == "call_expression" && {
                    let text = parsed.node_text(&node);
                    text.contains(".then(")
                        || text.contains("setTimeout")
                        || text.contains("setInterval")
                        || text.contains("Promise")
                })
        }
        Language::Go => kind == "go_statement",
        Language::Java => {
            kind == "method_invocation" && {
                let text = parsed.node_text(&node);
                text.contains("CompletableFuture")
                    || text.contains("submit(")
                    || text.contains("execute(")
                    || text.contains(".start()")
            }
        }
    };

    if is_async {
        out.push(line);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_async_inner(parsed, child, out);
    }
}

fn is_async_function(parsed: &ParsedFile, func_node: &Node<'_>) -> bool {
    match parsed.language {
        Language::Python => {
            // Check if parent or the node itself has "async" keyword
            let text = parsed.node_text(func_node);
            text.starts_with("async ")
        }
        Language::JavaScript | Language::TypeScript => {
            let text = parsed.node_text(func_node);
            text.starts_with("async ")
        }
        Language::Go => {
            // Go functions aren't inherently async, but check if they contain goroutines
            let mut has_go = false;
            check_for_go_stmt(parsed, *func_node, &mut has_go);
            has_go
        }
        Language::Java => {
            // Check if return type involves CompletableFuture, Future, etc.
            let text = parsed.node_text(func_node);
            text.contains("CompletableFuture")
                || text.contains("Future<")
                || text.contains("Callable")
        }
    }
}

fn check_for_go_stmt(parsed: &ParsedFile, node: Node<'_>, found: &mut bool) {
    if node.kind() == "go_statement" {
        *found = true;
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !*found {
            check_for_go_stmt(parsed, child, found);
        }
    }
}
