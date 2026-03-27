//! Circular Slice — data flow cycle detection across function boundaries.
//!
//! Detects cycles in the cross-function data flow graph: values that propagate
//! through a cycle of function calls and may accumulate unintended transformations.
//!
//! Bug classes caught: state management cycles, event cascades, mutual recursion
//! with state mutation.

use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::data_flow::DataFlowGraph;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::CircularSlice);
    let call_graph = CallGraph::build(files);

    // Collect function names that contain diff lines
    let mut diff_func_names: Vec<&str> = Vec::new();
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            if let Some(func_id) = call_graph.function_at(&diff_info.file_path, line) {
                diff_func_names.push(&func_id.name);
            }
        }
    }
    diff_func_names.sort();
    diff_func_names.dedup();

    // Find call graph cycles reachable from diff functions
    let cycles = call_graph.find_cycles_from(&diff_func_names);

    // For each cycle, build a block showing the cycle path
    let mut block_id = 0;
    for cycle in &cycles {
        let mut block = DiffBlock::new(block_id, String::new(), ModifyType::Modified);

        // Set the primary file to the first function in the cycle
        if let Some(first) = cycle.first() {
            block.file = first.file.clone();
        }

        for func_id in cycle {
            // Include function signature lines
            block.add_line(&func_id.file, func_id.start_line, false);
            block.add_line(&func_id.file, func_id.end_line, false);

            // Find the call site(s) to the next function in the cycle
            if let Some(sites) = call_graph.calls.get(func_id) {
                for site in sites {
                    // Check if this call is to the next function in the cycle
                    let next_idx = cycle
                        .iter()
                        .position(|f| f == func_id)
                        .map(|i| (i + 1) % cycle.len());
                    if let Some(idx) = next_idx {
                        if site.callee_name == cycle[idx].name {
                            block.add_line(&func_id.file, site.line, true);
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

    // Also check for data flow cycles within functions (variable cycles)
    let dfg = DataFlowGraph::build(files);
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            // Get all defs on this line
            let _key_prefix = (diff_info.file_path.clone(), String::new(), String::new());
            for ((file, _func, _var), def_locs) in &dfg.defs {
                if file != &diff_info.file_path {
                    continue;
                }
                for dl in def_locs {
                    if dl.line != line {
                        continue;
                    }
                    // Check if this def reaches itself (cycle in data flow)
                    let reachable = dfg.forward_reachable(dl);
                    for r in &reachable {
                        if r.var_name == dl.var_name
                            && r.file == dl.file
                            && r.function == dl.function
                            && r.line == dl.line
                            && r.kind != dl.kind
                        {
                            // Data flow cycle detected
                            let mut block =
                                DiffBlock::new(block_id, file.clone(), ModifyType::Modified);
                            block.add_line(file, line, true);
                            // Include all intermediate lines
                            for intermediate in &reachable {
                                if intermediate.file == *file {
                                    block.add_line(file, intermediate.line, false);
                                }
                            }
                            result.blocks.push(block);
                            block_id += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}
