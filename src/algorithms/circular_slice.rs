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

use crate::cpg::{CpgContext, CpgEdge, CpgNode};
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::output::mermaid::safe_node_id;
use crate::slice::{
    EdgeStyle, GraphEdge, GraphNode, GraphShape, NodeKind, SliceGraph, SliceResult,
    SlicingAlgorithm,
};
use anyhow::Result;
use petgraph::visit::EdgeRef;
use std::collections::{BTreeMap, BTreeSet};

/// Emit a cycle diagram using pre-computed nodes and actual graph edges.
///
/// The caller is responsible for deriving `edges` from real graph edges
/// (filtered to SCC members) rather than inventing adjacency from node order.
fn push_cycle_diagram(
    result: &mut SliceResult,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    title: &str,
) {
    // Dedupe nodes by id, preserving first occurrence.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let nodes: Vec<GraphNode> = nodes
        .into_iter()
        .filter(|n| seen.insert(n.id.clone()))
        .collect();
    if nodes.len() < 2 || edges.is_empty() {
        return;
    }
    // Mark the edge whose target is the first node as Bold (the closing back-edge).
    // If no such edge exists, mark the last edge as Bold.
    let first_id = &nodes[0].id;
    let mut edges = edges;
    let closing_idx = edges
        .iter()
        .position(|e| &e.to == first_id)
        .unwrap_or(edges.len() - 1);
    edges[closing_idx].style = EdgeStyle::Bold;
    edges[closing_idx].label = Some("cycle".to_string());
    result.diagrams.push(SliceGraph {
        title: Some(title.to_string()),
        shape: GraphShape::Cycle,
        nodes,
        edges,
        clusters: vec![],
        mermaid: String::new(),
    });
}

pub fn slice(ctx: &CpgContext, diff: &DiffInput) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::CircularSlice);

    // Collect function names that contain diff lines
    let mut diff_func_nodes: BTreeSet<petgraph::graph::NodeIndex> = BTreeSet::new();
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            // Find function node containing this line
            for &func_idx in ctx.cpg.function_nodes().iter() {
                if let CpgNode::Function {
                    file,
                    start_line,
                    end_line,
                    ..
                } = ctx.cpg.node(func_idx)
                {
                    if file == &diff_info.file_path && line >= *start_line && line <= *end_line {
                        diff_func_nodes.insert(func_idx);
                    }
                }
            }
        }
    }

    // Find call graph cycles using petgraph's tarjan_scc
    let all_call_cycles = ctx.cpg.call_graph_cycles();

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
            if let Some(fid) = ctx.cpg.to_function_id(first_idx) {
                block.file = fid.file.clone();
            }
        }

        for &func_idx in cycle {
            if let Some(fid) = ctx.cpg.to_function_id(func_idx) {
                // Include function signature lines
                block.add_line(&fid.file, fid.start_line, false);
                block.add_line(&fid.file, fid.end_line, false);

                // Find call sites to the next function in the cycle
                let next_idx_pos = cycle.iter().position(|&f| f == func_idx);
                if let Some(pos) = next_idx_pos {
                    let next_func_idx = cycle[(pos + 1) % cycle.len()];
                    // Check all Call edges from this function
                    for edge in ctx.cpg.graph.edges(func_idx) {
                        if matches!(edge.weight(), CpgEdge::Call) && edge.target() == next_func_idx
                        {
                            // The call site is at the caller's call graph construction.
                            // We include the callee's start line as the call target indicator.
                            if let Some(callee_fid) = ctx.cpg.to_function_id(next_func_idx) {
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

            // Build a Cycle diagram for this call-graph cycle using actual
            // Call edges between SCC members — not invented adjacent-pair edges.
            let mut cycle_nodes: Vec<GraphNode> = Vec::new();
            let mut node_id_for_idx: BTreeMap<petgraph::graph::NodeIndex, String> = BTreeMap::new();
            for &func_idx in cycle {
                if let Some(fid) = ctx.cpg.to_function_id(func_idx) {
                    let id = safe_node_id(&fid.file, fid.start_line);
                    cycle_nodes.push(GraphNode {
                        id: id.clone(),
                        label: format!("{}:{}\n{}", fid.file, fid.start_line, fid.name),
                        kind: NodeKind::Step,
                        file: Some(fid.file.clone()),
                        line: Some(fid.start_line),
                    });
                    node_id_for_idx.insert(func_idx, id);
                }
            }
            // Only emit edges that actually exist in the call graph.
            let mut cycle_edges: Vec<GraphEdge> = Vec::new();
            let mut seen_edges: BTreeSet<(String, String)> = BTreeSet::new();
            for &from_idx in cycle {
                if let Some(from_id) = node_id_for_idx.get(&from_idx) {
                    for edge in ctx.cpg.graph.edges(from_idx) {
                        if matches!(edge.weight(), CpgEdge::Call)
                            && node_id_for_idx.contains_key(&edge.target())
                        {
                            if let Some(to_id) = node_id_for_idx.get(&edge.target()) {
                                let key = (from_id.clone(), to_id.clone());
                                if seen_edges.insert(key) {
                                    cycle_edges.push(GraphEdge {
                                        from: from_id.clone(),
                                        to: to_id.clone(),
                                        label: None,
                                        style: EdgeStyle::Solid,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            push_cycle_diagram(&mut result, cycle_nodes, cycle_edges, "Call cycle");
        }
    }

    // Also check for data flow cycles within functions
    let df_cycles = ctx.cpg.data_flow_cycles();
    for cycle in &df_cycles {
        // Check if any node in the cycle is on a diff line
        let has_diff_node = cycle.iter().any(|&idx| {
            let node = ctx.cpg.node(idx);
            diff.files
                .iter()
                .any(|di| di.file_path == node.file() && di.diff_lines.contains(&node.line()))
        });

        if !has_diff_node {
            continue;
        }

        let mut block = DiffBlock::new(block_id, String::new(), ModifyType::Modified);
        if let Some(&first) = cycle.first() {
            block.file = ctx.cpg.node(first).file().to_string();
        }

        for &idx in cycle {
            let node = ctx.cpg.node(idx);
            let is_diff = diff
                .files
                .iter()
                .any(|di| di.file_path == node.file() && di.diff_lines.contains(&node.line()));
            block.add_line(node.file(), node.line(), is_diff);
        }

        if !block.file_line_map.is_empty() {
            result.blocks.push(block);
            block_id += 1;

            // Build a Cycle diagram for this data-flow cycle using actual
            // DataFlow edges between SCC members — not invented adjacent-pair edges.
            let mut cycle_nodes: Vec<GraphNode> = Vec::new();
            let mut df_node_id_for_idx: BTreeMap<petgraph::graph::NodeIndex, String> =
                BTreeMap::new();
            for &idx in cycle {
                let node = ctx.cpg.node(idx);
                let id = safe_node_id(node.file(), node.line());
                cycle_nodes.push(GraphNode {
                    id: id.clone(),
                    label: format!("{}:{}", node.file(), node.line()),
                    kind: NodeKind::Step,
                    file: Some(node.file().to_string()),
                    line: Some(node.line()),
                });
                df_node_id_for_idx.insert(idx, id);
            }
            // Only emit edges that actually exist in the data-flow graph.
            let mut df_cycle_edges: Vec<GraphEdge> = Vec::new();
            let mut df_seen_edges: BTreeSet<(String, String)> = BTreeSet::new();
            for &from_idx in cycle {
                if let Some(from_id) = df_node_id_for_idx.get(&from_idx) {
                    for edge in ctx.cpg.graph.edges(from_idx) {
                        if matches!(edge.weight(), CpgEdge::DataFlow)
                            && df_node_id_for_idx.contains_key(&edge.target())
                        {
                            if let Some(to_id) = df_node_id_for_idx.get(&edge.target()) {
                                let key = (from_id.clone(), to_id.clone());
                                if df_seen_edges.insert(key) {
                                    df_cycle_edges.push(GraphEdge {
                                        from: from_id.clone(),
                                        to: to_id.clone(),
                                        label: None,
                                        style: EdgeStyle::Solid,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            push_cycle_diagram(&mut result, cycle_nodes, df_cycle_edges, "Data-flow cycle");
        }
    }

    Ok(result)
}
