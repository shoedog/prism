//! Echo Slice — ripple effect modeling for downstream breakage.
//!
//! **Question answered:** "If this change subtly alters behavior, what downstream code would break?"
//!
//! Forward traces from changed functions through the call graph. For each caller,
//! checks whether it handles the changed function's return value, error cases,
//! and edge conditions. Flags callers that may assume old semantics.
//!
//! Catches: missing null checks after a return type change, unhandled new error
//! cases, callers that depend on side effects that were removed.

use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::output::mermaid::safe_node_id;
use crate::slice::{
    EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceFinding, SliceGraph, SliceResult,
    SlicingAlgorithm,
};
use anyhow::Result;
use std::collections::BTreeSet;

/// Patterns that suggest a caller handles the return value safely.
const SAFE_PATTERNS: &[&str] = &[
    // === Cross-language conditional checks ===
    "if ",
    "guard ",
    "switch ",
    "match ",
    // === Go nil/error checks ===
    "!= nil",
    "== nil",
    "if err != nil",
    "if err == nil",
    "errors.Is(",
    "errors.As(",
    // === Python None/exception handling ===
    "!= None",
    "== None",
    "is None",
    "is not None",
    "except",
    "with ", // context manager = safe resource handling
    "assert ",
    // === JavaScript / TypeScript null/undefined checks ===
    "!= null",
    "!== null",
    "!== undefined",
    "== null",
    "=== null",
    "=== undefined",
    "?.", // optional chaining
    "??", // nullish coalescing
    ".catch(",
    // === Rust Result/Option handling ===
    ".ok()",
    ".unwrap_or",
    ".unwrap_or_else",
    ".unwrap_or_default",
    ".map_err(",
    ".and_then(",
    ".or_else(",
    "if let Some(",
    "if let Ok(",
    "if let Err(",
    // === C/C++ return code checks ===
    "if (ret ",
    "if (rc ",
    "if (err ",
    "if (status ",
    "if (result ",
    "if (!", // if (!ptr), if (!ret)
    "== NULL)",
    "!= NULL)",
    "== nullptr)",
    "!= nullptr)",
    "errno",
    "perror(",
    "strerror(",
    "assert(",
    "ASSERT_",
    "CHECK_",
    // === Lua error handling ===
    "pcall(",
    "xpcall(",
    // === Cross-language error handling ===
    "try ",
    "catch",
    "or ",
    "|| ",
    "getOrElse",
    "orElse",
];

/// A finding: a caller that may not handle the changed semantics.
#[derive(Debug, Clone)]
pub struct EchoFinding {
    pub caller_file: String,
    pub caller_function: String,
    pub call_line: usize,
    pub callee_function: String,
    pub missing_checks: Vec<String>,
}

pub fn slice(ctx: &CpgContext, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::EchoSlice);
    let mut block_id = 0;

    // Diagram accumulators (one Fanout diagram per result).
    let mut graph_nodes: Vec<GraphNode> = Vec::new();
    let mut graph_edges: Vec<GraphEdge> = Vec::new();
    let mut seen_node_ids: BTreeSet<String> = BTreeSet::new();
    let mut seen_edges: BTreeSet<(String, String)> = BTreeSet::new();

    for diff_info in &diff.files {
        let parsed = match ctx.files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find changed functions
        let mut changed_functions: BTreeSet<String> = BTreeSet::new();
        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    changed_functions.insert(parsed.node_text(&name_node).to_string());
                }
            }
        }

        let source_lines: Vec<&str> = parsed.source.lines().collect();

        for func_name in &changed_functions {
            // Detect what kind of change was made
            let change_touches_return = diff_info.diff_lines.iter().any(|&l| {
                if l == 0 || l > source_lines.len() {
                    return false;
                }
                source_lines
                    .get(l - 1)
                    .map(|s| s.contains("return"))
                    .unwrap_or(false)
            });

            let change_touches_error = diff_info.diff_lines.iter().any(|&l| {
                source_lines
                    .get(l.saturating_sub(1))
                    .map(|s| {
                        s.contains("raise")
                            || s.contains("throw")
                            || s.contains("return err")
                            || s.contains("return Err(")  // Rust Result
                            || s.contains("panic")
                            || s.contains("Error(")
                            || s.contains("error(")        // Lua error()
                            || s.contains("return -1")     // C error return
                            || s.contains("return NULL")   // C null return
                            || s.contains("return nil")    // Go nil return
                            || s.contains("errno") // C errno set
                    })
                    .unwrap_or(false)
            });

            // Find all callers across all files, scoped to the correct file
            // to disambiguate static functions with the same name
            let callers = ctx
                .cpg
                .callers_of_in_file(func_name, 2, Some(&diff_info.file_path));

            for (caller_id, _depth) in &callers {
                let caller_parsed = match ctx.files.get(&caller_id.file) {
                    Some(f) => f,
                    None => continue,
                };

                let caller_source: Vec<&str> = caller_parsed.source.lines().collect();

                // Find call site lines.
                // Uses the raw callers index because caller_id already comes from
                // the file-scoped callers_of_in_file query above; the name+file
                // filter selects the exact call site within that known-correct caller.
                let call_lines: Vec<usize> =
                    if let Some(sites) = ctx.cpg.call_graph.callers.get(func_name) {
                        sites
                            .iter()
                            .filter(|s| {
                                s.caller.name == caller_id.name && s.caller.file == caller_id.file
                            })
                            .map(|s| s.line)
                            .collect()
                    } else {
                        continue;
                    };

                let mut missing_checks: Vec<String> = Vec::new();

                for &call_line in &call_lines {
                    // Check surrounding lines for safe handling patterns
                    let context_start = call_line.saturating_sub(3);
                    let context_end = (call_line + 5).min(caller_id.end_line);
                    let context_lines: Vec<&str> = (context_start..=context_end)
                        .filter_map(|l| {
                            if l > 0 && l <= caller_source.len() {
                                Some(caller_source[l - 1])
                            } else {
                                None
                            }
                        })
                        .collect();

                    let context_text = context_lines.join(" ");

                    let has_safe_pattern = SAFE_PATTERNS.iter().any(|p| context_text.contains(p))
                        // Rust ? operator (not in SAFE_PATTERNS since it's
                        // context-dependent, but valid error handling)
                        || context_text.contains(")?");

                    if change_touches_return && !has_safe_pattern {
                        missing_checks.push("return value not checked".to_string());
                    }
                    if change_touches_error && !has_safe_pattern {
                        missing_checks.push("no error handling around call".to_string());
                    }
                }

                if !missing_checks.is_empty() {
                    let first_call = call_lines.first().copied().unwrap_or(0);
                    result.findings.push(SliceFinding {
                        algorithm: "echo".to_string(),
                        file: caller_id.file.clone(),
                        line: first_call,
                        severity: "warning".to_string(),
                        description: format!(
                            "'{}' calls '{}' without handling: {}",
                            caller_id.name,
                            func_name,
                            missing_checks.join(", ")
                        ),
                        function_name: Some(caller_id.name.clone()),
                        related_lines: call_lines.clone(),
                        related_files: vec![diff_info.file_path.clone()],
                        category: Some("missing_error_handling".to_string()),
                        parse_quality: None,
                        diagrams: vec![],
                    });
                    let mut block =
                        DiffBlock::new(block_id, caller_id.file.clone(), ModifyType::Modified);

                    // Include the changed function signature
                    if let Some(func_node) = parsed.find_function_by_name(func_name) {
                        let (fs, fe) = parsed.node_line_range(&func_node);
                        block.add_line(&diff_info.file_path, fs, false);
                        // Include changed lines
                        for &l in &diff_info.diff_lines {
                            if l >= fs && l <= fe {
                                block.add_line(&diff_info.file_path, l, true);
                            }
                        }
                        block.add_line(&diff_info.file_path, fe, false);
                    }

                    // Include the caller with call site highlighted
                    block.add_line(&caller_id.file, caller_id.start_line, false);
                    for &cl in &call_lines {
                        // Highlight call site as a potential issue
                        block.add_line(&caller_id.file, cl, true);
                        // Include surrounding context
                        for l in cl.saturating_sub(1)..=(cl + 2).min(caller_id.end_line) {
                            block.add_line(&caller_id.file, l, l == cl);
                        }
                    }
                    block.add_line(&caller_id.file, caller_id.end_line, false);

                    result.blocks.push(block);
                    block_id += 1;

                    // Diagram: register the origin (changed function) and the caller as nodes,
                    // connect caller -> origin.
                    if let Some(func_node) = parsed.find_function_by_name(func_name) {
                        let (origin_start, _origin_end) = parsed.node_line_range(&func_node);
                        let origin_id = safe_node_id(&diff_info.file_path, origin_start);
                        if seen_node_ids.insert(origin_id.clone()) {
                            graph_nodes.push(GraphNode {
                                id: origin_id.clone(),
                                label: format!(
                                    "{}:{}\n{}",
                                    diff_info.file_path, origin_start, func_name
                                ),
                                kind: NodeKind::Origin,
                                file: Some(diff_info.file_path.clone()),
                                line: Some(origin_start),
                            });
                        }
                        let caller_id_str = safe_node_id(&caller_id.file, caller_id.start_line);
                        if seen_node_ids.insert(caller_id_str.clone()) {
                            graph_nodes.push(GraphNode {
                                id: caller_id_str.clone(),
                                label: format!(
                                    "{}:{}\n{}",
                                    caller_id.file, caller_id.start_line, caller_id.name
                                ),
                                kind: NodeKind::Caller,
                                file: Some(caller_id.file.clone()),
                                line: Some(caller_id.start_line),
                            });
                        }
                        let edge_key = (caller_id_str.clone(), origin_id.clone());
                        if seen_edges.insert(edge_key) {
                            graph_edges.push(GraphEdge {
                                from: caller_id_str,
                                to: origin_id,
                                label: None,
                                style: EdgeStyle::Solid,
                            });
                        }
                    }
                }
            }
        }
    }

    if !graph_nodes.is_empty() {
        result.diagrams.push(SliceGraph {
            title: Some("Caller fanout".to_string()),
            shape: GraphShape::Fanout,
            nodes: graph_nodes,
            edges: graph_edges,
            clusters: vec![],
            mermaid: String::new(),
        });
    }

    Ok(result)
}
