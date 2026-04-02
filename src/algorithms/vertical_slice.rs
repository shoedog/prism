//! Vertical Slice — end-to-end feature path tracing.
//!
//! Traces the complete path from user input to persistent output for the
//! feature being modified. Shows every architectural layer a request touches:
//! handler → service → model → database.

use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Heuristic layer patterns for directory-based layer detection.
const LAYER_PATTERNS: &[(&str, &str)] = &[
    ("handler", "Handler"),
    ("controller", "Controller"),
    ("route", "Route"),
    ("api", "API"),
    ("view", "View"),
    ("service", "Service"),
    ("usecase", "UseCase"),
    ("domain", "Domain"),
    ("model", "Model"),
    ("entity", "Entity"),
    ("repository", "Repository"),
    ("dao", "DAO"),
    ("db", "Database"),
    ("store", "Store"),
    ("migration", "Migration"),
    ("middleware", "Middleware"),
    ("util", "Utility"),
    ("helper", "Helper"),
    ("cmd", "Command"),
    ("pkg", "Package"),
];

/// Configuration for vertical slicing.
#[derive(Debug, Clone)]
pub struct VerticalConfig {
    /// Explicit layer ordering (highest to lowest). If empty, auto-detect.
    pub layers: Vec<String>,
}

impl Default for VerticalConfig {
    fn default() -> Self {
        Self { layers: Vec::new() }
    }
}

/// A layer in the vertical slice.
#[derive(Debug, Clone)]
pub struct LayerEntry {
    pub layer_name: String,
    pub file: String,
    pub function_name: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    vertical_config: &VerticalConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::VerticalSlice);
    let cpg = CodePropertyGraph::build(files);

    // Detect layers for each file
    let file_layers: BTreeMap<String, String> = if vertical_config.layers.is_empty() {
        detect_layers(files)
    } else {
        assign_layers(files, &vertical_config.layers)
    };

    // For each diff function, trace upward and downward through layers
    let mut block_id = 0;
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            if let Some((_idx, func_id)) = cpg.function_at(&diff_info.file_path, line) {
                let mut path: Vec<LayerEntry> = Vec::new();

                // Trace up: callers toward the entry point
                let callers = cpg.callers_of(&func_id.name, 10);
                for (caller_id, _depth) in callers.iter().rev() {
                    let layer = file_layers
                        .get(&caller_id.file)
                        .cloned()
                        .unwrap_or_else(|| "Unknown".to_string());
                    path.push(LayerEntry {
                        layer_name: layer,
                        file: caller_id.file.clone(),
                        function_name: caller_id.name.clone(),
                        start_line: caller_id.start_line,
                        end_line: caller_id.end_line,
                    });
                }

                // The diff function itself
                let diff_layer = file_layers
                    .get(&diff_info.file_path)
                    .cloned()
                    .unwrap_or_else(|| "Unknown".to_string());
                path.push(LayerEntry {
                    layer_name: diff_layer,
                    file: diff_info.file_path.clone(),
                    function_name: func_id.name.clone(),
                    start_line: func_id.start_line,
                    end_line: func_id.end_line,
                });

                // Trace down: callees toward persistence
                let callees = cpg.callees_of(&func_id.name, &diff_info.file_path, 10);
                for (callee_id, _depth) in &callees {
                    let layer = file_layers
                        .get(&callee_id.file)
                        .cloned()
                        .unwrap_or_else(|| "Unknown".to_string());
                    path.push(LayerEntry {
                        layer_name: layer,
                        file: callee_id.file.clone(),
                        function_name: callee_id.name.clone(),
                        start_line: callee_id.start_line,
                        end_line: callee_id.end_line,
                    });
                }

                // Deduplicate by function name
                let mut seen: BTreeSet<String> = BTreeSet::new();
                path.retain(|e| seen.insert(format!("{}:{}", e.file, e.function_name)));

                // Build block
                if !path.is_empty() {
                    let mut block =
                        DiffBlock::new(block_id, diff_info.file_path.clone(), ModifyType::Modified);

                    for entry in &path {
                        block.add_line(&entry.file, entry.start_line, false);
                        block.add_line(&entry.file, entry.end_line, false);
                        // For the diff function, include actual diff lines
                        if entry.file == diff_info.file_path && entry.function_name == func_id.name
                        {
                            for &dl in &diff_info.diff_lines {
                                if dl >= entry.start_line && dl <= entry.end_line {
                                    block.add_line(&entry.file, dl, true);
                                }
                            }
                        }
                    }

                    result.blocks.push(block);
                    block_id += 1;
                }
            }
        }
    }

    Ok(result)
}

fn detect_layers(files: &BTreeMap<String, ParsedFile>) -> BTreeMap<String, String> {
    let mut layers = BTreeMap::new();

    for file_path in files.keys() {
        let path_lower = file_path.to_lowercase();
        let mut matched = false;

        for (pattern, layer_name) in LAYER_PATTERNS {
            if path_lower.contains(pattern) {
                layers.insert(file_path.clone(), layer_name.to_string());
                matched = true;
                break;
            }
        }

        if !matched {
            layers.insert(file_path.clone(), "Application".to_string());
        }
    }

    layers
}

fn assign_layers(
    files: &BTreeMap<String, ParsedFile>,
    layer_order: &[String],
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();

    for file_path in files.keys() {
        let path_lower = file_path.to_lowercase();
        let mut matched = false;

        for layer in layer_order {
            if path_lower.contains(&layer.to_lowercase()) {
                result.insert(file_path.clone(), layer.clone());
                matched = true;
                break;
            }
        }

        if !matched {
            result.insert(file_path.clone(), "Unknown".to_string());
        }
    }

    result
}
