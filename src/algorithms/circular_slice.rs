//! Circular Slice — data flow cycle detection across function boundaries.
//!
//! Detects cycles in the cross-function data flow graph: values that propagate
//! through a cycle of function calls and may accumulate unintended transformations.
//!
//! Bug classes caught: state management cycles, event cascades, mutual recursion
//! with state mutation.
//!
//! Uses the Code Property Graph's `tarjan_scc` (via petgraph) for cycle detection
//! instead of hand-rolled DFS.

use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet};

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::CircularSlice);
    let cpg = CodePropertyGraph::build(files);

    // Collect function names that contain diff lines
    let mut diff_func_nodes: BTreeSet<petgraph::graph::NodeIndex> = BTreeSet::new();
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            // Find function node containing this line
            for &func_idx in cpg.function_nodes().iter() {
                if let CpgNode::Function {
                    file,
                    start_line,
                    end_line,
                    ..
                } = cpg.node(func_idx)
                {
                    if file == &diff_info.file_path && line >= *start_line && line <= *end_line {
                        diff_func_nodes.insert(func_idx);
                    }
                }
            }
        }
    }

    // Find call graph cycles using petgraph's tarjan_scc
    let all_call_cycles = cpg.call_graph_cycles();

    // Filter to cycles that include at least one diff function
    let mut block_id = 0;
    for cycle in &all_call_cycles {
        let cycle_set: BTreeSet<_> = cycle.iter().copied().collect();
        if cycle_set.intersection(&diff_func_nodes).next().is_none() {
            continue; // Cycle doesn't involve any diff functions
        }

        let mut block = DiffBlock::new(block_id, String::new(), ModifyType::Modified);

        // Set the primary file to the first function in the cycle
        if let Some(&first_idx) = cycle.first() {
            if let Some(fid) = cpg.to_function_id(first_idx) {
                block.file = fid.file.clone();
            }
        }

        for &func_idx in cycle {
            if let Some(fid) = cpg.to_function_id(func_idx) {
                // Include function signature lines
                block.add_line(&fid.file, fid.start_line, false);
                block.add_line(&fid.file, fid.end_line, false);

                // Find call sites to the next function in the cycle
                let next_idx_pos = cycle.iter().position(|&f| f == func_idx);
                if let Some(pos) = next_idx_pos {
                    let next_func_idx = cycle[(pos + 1) % cycle.len()];
                    // Check all Call edges from this function
                    for edge in cpg.graph.edges(func_idx) {
                        if matches!(edge.weight(), CpgEdge::Call) && edge.target() == next_func_idx
                        {
                            // The call site is at the caller's call graph construction.
                            // We include the callee's start line as the call target indicator.
                            if let Some(callee_fid) = cpg.to_function_id(next_func_idx) {
                                block.add_line(&fid.file, fid.start_line, true);
                                // Also mark the callee
                                block.add_line(&callee_fid.file, callee_fid.start_line, false);
                            }
                        }
                    }
                }
            }
        }

        if !block.file_line_map.is_empty() {
            result.blocks.push(block);
            block_id += 1;
        }
    }

    // Also check for data flow cycles within functions
    let df_cycles = cpg.data_flow_cycles();
    for cycle in &df_cycles {
        // Check if any node in the cycle is on a diff line
        let has_diff_node = cycle.iter().any(|&idx| {
            let node = cpg.node(idx);
            diff.files
                .iter()
                .any(|di| di.file_path == node.file() && di.diff_lines.contains(&node.line()))
        });

        if !has_diff_node {
            continue;
        }

        let mut block = DiffBlock::new(block_id, String::new(), ModifyType::Modified);
        if let Some(&first) = cycle.first() {
            block.file = cpg.node(first).file().to_string();
        }

        for &idx in cycle {
            let node = cpg.node(idx);
            let is_diff = diff
                .files
                .iter()
                .any(|di| di.file_path == node.file() && di.diff_lines.contains(&node.line()));
            block.add_line(node.file(), node.line(), is_diff);
        }

        if !block.file_line_map.is_empty() {
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
