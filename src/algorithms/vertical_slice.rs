//! Vertical Slice — end-to-end feature path tracing.
//!
//! Traces the complete path from user input to persistent output for the
//! feature being modified. Shows every architectural layer a request touches:
//! handler → service → model → database.

use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::output::mermaid::safe_node_id;
use crate::slice::{
    EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeCluster, NodeKind, SliceGraph, SliceResult,
    SlicingAlgorithm,
};
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
    ctx: &CpgContext,
    diff: &DiffInput,
    vertical_config: &VerticalConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::VerticalSlice);

    // Detect layers for each file
    let file_layers: BTreeMap<String, String> = if vertical_config.layers.is_empty() {
        detect_layers(ctx.files)
    } else {
        assign_layers(ctx.files, &vertical_config.layers)
    };

    // Diagram accumulators (one Layered diagram per result).
    let mut graph_nodes: Vec<GraphNode> = Vec::new();
    let mut graph_edges: Vec<GraphEdge> = Vec::new();
    let mut layer_to_nodes: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut layer_order: Vec<String> = Vec::new();
    let mut seen_node_ids: BTreeSet<String> = BTreeSet::new();
    let mut seen_edges: BTreeSet<(String, String)> = BTreeSet::new();

    // For each diff function, trace upward and downward through layers
    let mut block_id = 0;
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            if let Some((_idx, func_id)) = ctx.cpg.function_at(&diff_info.file_path, line) {
                let mut path: Vec<LayerEntry> = Vec::new();

                // Trace up: callers toward the entry point, scoped to the
                // correct file to disambiguate static functions
                let callers =
                    ctx.cpg
                        .callers_of_in_file(&func_id.name, 10, Some(&diff_info.file_path));
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
                let callees = ctx.cpg.callees_of(&func_id.name, &diff_info.file_path, 10);
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

                // For the diagram: each unique LayerEntry becomes a node; consecutive
                // entries within this path become an edge.
                let mut path_node_ids: Vec<String> = Vec::new();
                for entry in &path {
                    let nid = safe_node_id(&entry.file, entry.start_line);
                    path_node_ids.push(nid.clone());
                    if seen_node_ids.insert(nid.clone()) {
                        graph_nodes.push(GraphNode {
                            id: nid.clone(),
                            label: format!(
                                "{}:{}\n{}",
                                entry.file, entry.start_line, entry.function_name
                            ),
                            kind: NodeKind::Step,
                            file: Some(entry.file.clone()),
                            line: Some(entry.start_line),
                        });
                        if !layer_to_nodes.contains_key(&entry.layer_name) {
                            layer_order.push(entry.layer_name.clone());
                        }
                        layer_to_nodes
                            .entry(entry.layer_name.clone())
                            .or_default()
                            .push(nid);
                    }
                }
                for pair in path_node_ids.windows(2) {
                    let edge_key = (pair[0].clone(), pair[1].clone());
                    if seen_edges.insert(edge_key) {
                        graph_edges.push(GraphEdge {
                            from: pair[0].clone(),
                            to: pair[1].clone(),
                            label: None,
                            style: EdgeStyle::Solid,
                        });
                    }
                }

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

    if !graph_nodes.is_empty() {
        let clusters: Vec<NodeCluster> = layer_order
            .iter()
            .map(|layer| NodeCluster {
                label: layer.clone(),
                node_ids: layer_to_nodes.get(layer).cloned().unwrap_or_default(),
            })
            .collect();
        result.diagrams.push(SliceGraph {
            title: Some("Layered call graph".to_string()),
            shape: GraphShape::Layered,
            nodes: graph_nodes,
            edges: graph_edges,
            clusters,
            mermaid: String::new(),
        });
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
