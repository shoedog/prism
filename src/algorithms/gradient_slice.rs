//! Gradient Slice — continuous relevance scoring with distance decay.
//!
//! **Question answered:** "How relevant is each line to the change, on a continuous scale?"
//!
//! Instead of binary include/exclude, assigns each line a relevance score that
//! decays with structural distance from the change point. Direct data deps score
//! 1.0, one hop away 0.7, two hops 0.4, etc. The consumer (LLM or human) can
//! decide their own cutoff threshold.
//!
//! Uses the Code Property Graph for unified traversal of data flow and call edges.
//! All other slices produce binary output. This produces a ranked, scored slice.

use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Configuration for gradient slicing.
#[derive(Debug, Clone)]
pub struct GradientConfig {
    /// Decay factor per hop (0.0-1.0). Score at hop N = decay^N.
    pub decay: f64,
    /// Minimum score threshold to include a line.
    pub min_score: f64,
    /// Maximum hops to trace.
    pub max_hops: usize,
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self {
            decay: 0.6,
            min_score: 0.1,
            max_hops: 5,
        }
    }
}

/// A line with its relevance score.
#[derive(Debug, Clone)]
pub struct ScoredLine {
    pub file: String,
    pub line: usize,
    pub score: f64,
    pub hop_distance: usize,
    pub is_diff: bool,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &GradientConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::GradientSlice);
    let cpg = CodePropertyGraph::build(files);

    // Score map: (file, line) → (score, hop_distance)
    let mut scores: BTreeMap<(String, usize), (f64, usize)> = BTreeMap::new();

    // Seed: diff lines get score 1.0 at distance 0
    // We also seed from the CPG nodes at those locations
    let mut queue: VecDeque<(petgraph::graph::NodeIndex, usize)> = VecDeque::new();

    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            scores.insert((diff_info.file_path.clone(), line), (1.0, 0));
            // Seed queue with all CPG nodes at this location
            for idx in cpg.nodes_at(&diff_info.file_path, line) {
                queue.push_back((idx, 0));
            }
            // Also seed from the enclosing function node for call graph traversal
            for &func_idx in cpg.function_nodes().iter() {
                if let CpgNode::Function {
                    file,
                    start_line,
                    end_line,
                    ..
                } = cpg.node(func_idx)
                {
                    if file == &diff_info.file_path && line >= *start_line && line <= *end_line {
                        queue.push_back((func_idx, 0));
                    }
                }
            }
        }
    }

    // BFS with decaying scores over CPG edges.
    // Edge filter: follow DataFlow, Call, and Contains edges.
    let mut visited_at_hop: BTreeMap<petgraph::graph::NodeIndex, usize> = BTreeMap::new();
    for &(idx, _) in queue.iter() {
        visited_at_hop.insert(idx, 0);
    }

    while let Some((node_idx, hop)) = queue.pop_front() {
        if hop >= config.max_hops {
            continue;
        }

        let next_hop = hop + 1;
        let next_score = config.decay.powi(next_hop as i32);
        if next_score < config.min_score {
            continue;
        }

        // Follow DataFlow, Call, and Return edges
        use petgraph::visit::EdgeRef;
        for edge in cpg.graph.edges(node_idx) {
            let follow = matches!(
                edge.weight(),
                CpgEdge::DataFlow | CpgEdge::Call | CpgEdge::Return
            );
            if !follow {
                continue;
            }

            let target = edge.target();
            let target_node = cpg.node(target);
            let target_file = target_node.file().to_string();
            let target_line = target_node.line();

            // Update score if this path is better
            let key = (target_file.clone(), target_line);
            let current_score = scores.get(&key).map(|s| s.0).unwrap_or(0.0);
            if next_score > current_score {
                scores.insert(key, (next_score, next_hop));
            }

            // Only re-queue if we found a shorter path
            let prev_hop = visited_at_hop.get(&target).copied().unwrap_or(usize::MAX);
            if next_hop < prev_hop {
                visited_at_hop.insert(target, next_hop);
                queue.push_back((target, next_hop));
            }
        }
    }

    // Collect diff lines for marking
    let diff_lines: BTreeSet<(String, usize)> = diff
        .files
        .iter()
        .flat_map(|f| f.diff_lines.iter().map(|&l| (f.file_path.clone(), l)))
        .collect();

    // Build scored lines, sort by score descending
    let mut scored: Vec<ScoredLine> = scores
        .iter()
        .map(|((file, line), (score, hop))| ScoredLine {
            file: file.clone(),
            line: *line,
            score: *score,
            hop_distance: *hop,
            is_diff: diff_lines.contains(&(file.clone(), *line)),
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Group by file for output
    let mut by_file: BTreeMap<String, Vec<&ScoredLine>> = BTreeMap::new();
    for sl in &scored {
        by_file.entry(sl.file.clone()).or_default().push(sl);
    }

    let mut block_id = 0;
    for (file, lines) in &by_file {
        let mut block = DiffBlock::new(block_id, file.clone(), ModifyType::Modified);
        for sl in lines {
            block.add_line(&sl.file, sl.line, sl.is_diff);
        }
        result.blocks.push(block);
        block_id += 1;
    }

    Ok(result)
}
