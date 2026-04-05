//! Spiral Slice — adaptive-depth review context through concentric rings.
//!
//! Starts narrow at the change point and progressively widens:
//! - Ring 1: Changed lines only (OriginalDiff)
//! - Ring 2: Enclosing function (ParentFunction)
//! - Ring 3: LeftFlow + direct caller/callee signatures
//! - Ring 4: Depth-2 callers/callees
//! - Ring 5: Test files exercising changed functions
//! - Ring 6: Shared utilities imported by multiple changed files
//!
//! Each line in the output is annotated with its ring level.

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceConfig, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Configuration for spiral slicing.
#[derive(Debug, Clone)]
pub struct SpiralConfig {
    /// Maximum ring to expand to (1-6).
    pub max_ring: usize,
    /// Auto-stop: stop expanding if ring N adds less than this fraction of new lines.
    pub auto_stop_threshold: f64,
}

impl Default for SpiralConfig {
    fn default() -> Self {
        Self {
            max_ring: 4,
            auto_stop_threshold: 0.05,
        }
    }
}

/// A line with its ring annotation.
#[derive(Debug, Clone)]
pub struct RingLine {
    pub file: String,
    pub line: usize,
    pub is_diff: bool,
    pub ring: usize,
}

pub fn slice(
    ctx: &CpgContext,
    diff: &DiffInput,
    config: &SliceConfig,
    spiral_config: &SpiralConfig,
) -> Result<SliceResult> {
    let result = SliceResult::new(SlicingAlgorithm::SpiralSlice);

    // Track all lines included so far, keyed by (file, line)
    let mut included: BTreeMap<(String, usize), (bool, usize)> = BTreeMap::new(); // (is_diff, ring)

    // Ring 1: Original diff lines
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            included.insert((diff_info.file_path.clone(), line), (true, 1));
        }
    }
    let mut prev_count = included.len();

    if spiral_config.max_ring < 2 {
        return build_result(result, &included, diff);
    }

    // Ring 2: Enclosing functions
    for diff_info in &diff.files {
        let parsed = match ctx.files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };
        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                let (start, end) = parsed.node_line_range(&func_node);
                for l in start..=end {
                    included
                        .entry((diff_info.file_path.clone(), l))
                        .or_insert((false, 2));
                }
            }
        }
    }

    if should_stop(prev_count, included.len(), spiral_config) || spiral_config.max_ring < 3 {
        return build_result(result, &included, diff);
    }
    prev_count = included.len();

    // Ring 3: LeftFlow + direct caller/callee signatures
    let left_flow_result = crate::algorithms::left_flow::slice(ctx, diff, config)?;
    for block in &left_flow_result.blocks {
        for (file, line_map) in &block.file_line_map {
            for (&line, &is_diff) in line_map {
                included.entry((file.clone(), line)).or_insert((is_diff, 3));
            }
        }
    }

    // Add direct callers/callees signatures
    let diff_func_names = get_diff_function_names_with_files(ctx.files, diff);
    for (file, func_name) in &diff_func_names {
        // Direct callers (file-scoped to disambiguate static functions)
        let callers = ctx.cpg.callers_of_in_file(func_name, 1, Some(file));
        for (caller_id, _) in &callers {
            included
                .entry((caller_id.file.clone(), caller_id.start_line))
                .or_insert((false, 3));
            included
                .entry((caller_id.file.clone(), caller_id.end_line))
                .or_insert((false, 3));
        }
        // Direct callees
        let callees = ctx.cpg.callees_of(func_name, file, 1);
        for (callee_id, _) in &callees {
            included
                .entry((callee_id.file.clone(), callee_id.start_line))
                .or_insert((false, 3));
            included
                .entry((callee_id.file.clone(), callee_id.end_line))
                .or_insert((false, 3));
        }
    }

    if should_stop(prev_count, included.len(), spiral_config) || spiral_config.max_ring < 4 {
        return build_result(result, &included, diff);
    }
    prev_count = included.len();

    // Ring 4: Depth-2 callers/callees
    for (file, func_name) in &diff_func_names {
        let callers = ctx.cpg.callers_of_in_file(func_name, 2, Some(file));
        for (caller_id, depth) in &callers {
            if *depth == 2 {
                included
                    .entry((caller_id.file.clone(), caller_id.start_line))
                    .or_insert((false, 4));
                included
                    .entry((caller_id.file.clone(), caller_id.end_line))
                    .or_insert((false, 4));
            }
        }
        let callees = ctx.cpg.callees_of(func_name, file, 2);
        for (callee_id, depth) in &callees {
            if *depth == 2 {
                included
                    .entry((callee_id.file.clone(), callee_id.start_line))
                    .or_insert((false, 4));
                included
                    .entry((callee_id.file.clone(), callee_id.end_line))
                    .or_insert((false, 4));
            }
        }
    }

    if should_stop(prev_count, included.len(), spiral_config) || spiral_config.max_ring < 5 {
        return build_result(result, &included, diff);
    }
    prev_count = included.len();

    // Ring 5: Test files (heuristic: files matching test patterns that reference changed functions)
    for (file_path, parsed) in ctx.files {
        let is_test = file_path.contains("test_")
            || file_path.contains("_test.")
            || file_path.contains("_spec.")
            || file_path.contains("/tests/")
            || file_path.contains("/test/")
            || file_path.contains("Test.")
            || file_path.ends_with("_test.go");

        if !is_test {
            continue;
        }

        // Check if this test file references any changed function
        for (_file, func_name) in &diff_func_names {
            let root = parsed.tree.root_node();
            let refs = parsed.find_variable_references(&root, func_name);
            if !refs.is_empty() {
                // Include the test functions that reference the changed function
                for &ref_line in &refs {
                    if let Some(test_func) = parsed.enclosing_function(ref_line) {
                        let (start, end) = parsed.node_line_range(&test_func);
                        for l in start..=end {
                            included.entry((file_path.clone(), l)).or_insert((false, 5));
                        }
                    }
                }
            }
        }
    }

    if should_stop(prev_count, included.len(), spiral_config) || spiral_config.max_ring < 6 {
        return build_result(result, &included, diff);
    }

    // Ring 6: Shared utilities (files imported by multiple changed files)
    // Heuristic: find files that are referenced from multiple changed files
    // This is approximate since we don't have full import resolution
    let changed_files: BTreeSet<&str> = diff.files.iter().map(|f| f.file_path.as_str()).collect();
    for (file_path, parsed) in ctx.files {
        if changed_files.contains(file_path.as_str()) {
            continue;
        }
        // Count how many changed files call functions in this file
        let mut ref_count = 0;
        for func_node in parsed.all_functions() {
            if let Some(name_node) = parsed.language.function_name(&func_node) {
                let fname = parsed.node_text(&name_node);
                if diff_func_names.iter().any(|(_, n)| n == fname) {
                    continue;
                }
                // Check if any changed file calls this function
                for changed_file in &changed_files {
                    if let Some(changed_parsed) = ctx.files.get(*changed_file) {
                        let root = changed_parsed.tree.root_node();
                        if !changed_parsed
                            .find_variable_references(&root, fname)
                            .is_empty()
                        {
                            ref_count += 1;
                        }
                    }
                }
            }
        }
        if ref_count >= 2 {
            // This is a shared utility — include function signatures
            for func_node in parsed.all_functions() {
                let (start, end) = parsed.node_line_range(&func_node);
                included
                    .entry((file_path.clone(), start))
                    .or_insert((false, 6));
                included
                    .entry((file_path.clone(), end))
                    .or_insert((false, 6));
            }
        }
    }

    build_result(result, &included, diff)
}

fn should_stop(prev_count: usize, current_count: usize, config: &SpiralConfig) -> bool {
    if config.auto_stop_threshold <= 0.0 {
        return false;
    }
    if prev_count == 0 {
        return false;
    }
    let delta = current_count - prev_count;
    let ratio = delta as f64 / prev_count as f64;
    ratio < config.auto_stop_threshold
}

/// Returns (file_path, function_name) pairs for all functions containing diff lines.
fn get_diff_function_names_with_files(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> BTreeSet<(String, String)> {
    let mut names = BTreeSet::new();
    for diff_info in &diff.files {
        if let Some(parsed) = files.get(&diff_info.file_path) {
            for &line in &diff_info.diff_lines {
                if let Some(func_node) = parsed.enclosing_function(line) {
                    if let Some(name_node) = parsed.language.function_name(&func_node) {
                        names.insert((
                            diff_info.file_path.clone(),
                            parsed.node_text(&name_node).to_string(),
                        ));
                    }
                }
            }
        }
    }
    names
}

fn build_result(
    mut result: SliceResult,
    included: &BTreeMap<(String, usize), (bool, usize)>,
    diff: &DiffInput,
) -> Result<SliceResult> {
    // Group by file
    let mut by_file: BTreeMap<String, BTreeMap<usize, (bool, usize)>> = BTreeMap::new();
    for ((file, line), (is_diff, ring)) in included {
        by_file
            .entry(file.clone())
            .or_default()
            .insert(*line, (*is_diff, *ring));
    }

    let mut block_id = 0;
    for (file, lines) in &by_file {
        let modify_type = diff
            .files
            .iter()
            .find(|f| f.file_path == *file)
            .map(|f| f.modify_type.clone())
            .unwrap_or(crate::diff::ModifyType::Modified);

        let mut block = DiffBlock::new(block_id, file.clone(), modify_type);
        for (&line, &(is_diff, _ring)) in lines {
            block.add_line(file, line, is_diff);
        }
        result.blocks.push(block);
        block_id += 1;
    }

    Ok(result)
}
