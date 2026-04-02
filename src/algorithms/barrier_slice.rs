//! Barrier Slice — interprocedural slicing with explicit depth and boundary controls.
//!
//! Traces callers and callees from diff lines up to a configurable depth,
//! stopping at barrier functions or modules. This solves the "how deep should
//! I trace?" problem by making the boundary explicit.

use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Configuration specific to barrier slicing.
#[derive(Debug, Clone)]
pub struct BarrierConfig {
    /// Maximum call depth to trace (callers up + callees down).
    pub max_depth: usize,
    /// Function names to not trace into (barriers).
    pub barrier_symbols: BTreeSet<String>,
    /// File path prefixes to not enter.
    pub barrier_modules: Vec<String>,
}

impl Default for BarrierConfig {
    fn default() -> Self {
        Self {
            max_depth: 2,
            barrier_symbols: BTreeSet::new(),
            barrier_modules: Vec::new(),
        }
    }
}

pub fn slice(
    ctx: &CpgContext,
    diff: &DiffInput,
    _config: &SliceConfig,
    barrier_config: &BarrierConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::BarrierSlice);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match ctx.files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find functions containing diff lines
        let mut diff_functions: BTreeSet<String> = BTreeSet::new();
        for &line in &diff_info.diff_lines {
            if let Some((_idx, func_id)) = ctx.cpg.function_at(&diff_info.file_path, line) {
                diff_functions.insert(func_id.name.clone());
            }
        }

        for func_name in &diff_functions {
            let mut slice_lines: BTreeMap<String, BTreeMap<usize, bool>> = BTreeMap::new();

            // Include the diff lines in the original function
            if let Some(func_node) = parsed.find_function_by_name(func_name) {
                let (start, end) = parsed.node_line_range(&func_node);
                for &line in &diff_info.diff_lines {
                    if line >= start && line <= end {
                        slice_lines
                            .entry(diff_info.file_path.clone())
                            .or_default()
                            .insert(line, true);
                    }
                }
                // Include function signature
                slice_lines
                    .entry(diff_info.file_path.clone())
                    .or_default()
                    .insert(start, false);
                slice_lines
                    .entry(diff_info.file_path.clone())
                    .or_default()
                    .insert(end, false);
            }

            // Trace callers (up)
            let callers = ctx.cpg.callers_of(func_name, barrier_config.max_depth);
            for (caller_id, _depth) in &callers {
                if barrier_config.barrier_symbols.contains(&caller_id.name) {
                    continue;
                }
                if barrier_config
                    .barrier_modules
                    .iter()
                    .any(|m| caller_id.file.starts_with(m))
                {
                    continue;
                }

                // Include caller function signature and call site
                let entry = slice_lines.entry(caller_id.file.clone()).or_default();
                entry.insert(caller_id.start_line, false);
                entry.insert(caller_id.end_line, false);

                // Find the specific call site lines
                if let Some(sites) = ctx.cpg.call_graph.callers.get(func_name) {
                    for site in sites {
                        if site.caller.name == caller_id.name {
                            entry.insert(site.line, false);
                        }
                    }
                }
            }

            // Trace callees (down)
            let callees =
                ctx.cpg
                    .callees_of(func_name, &diff_info.file_path, barrier_config.max_depth);
            for (callee_id, _depth) in &callees {
                if barrier_config.barrier_symbols.contains(&callee_id.name) {
                    continue;
                }
                if barrier_config
                    .barrier_modules
                    .iter()
                    .any(|m| callee_id.file.starts_with(m))
                {
                    continue;
                }

                let entry = slice_lines.entry(callee_id.file.clone()).or_default();
                entry.insert(callee_id.start_line, false);
                entry.insert(callee_id.end_line, false);
            }

            // Build block
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );
            for (file, lines) in &slice_lines {
                for (&line, &is_diff) in lines {
                    block.add_line(file, line, is_diff);
                }
            }
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
