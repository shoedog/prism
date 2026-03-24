//! Gradient Slice — continuous relevance scoring with distance decay.
//!
//! **Question answered:** "How relevant is each line to the change, on a continuous scale?"
//!
//! Instead of binary include/exclude, assigns each line a relevance score that
//! decays with structural distance from the change point. Direct data deps score
//! 1.0, one hop away 0.7, two hops 0.4, etc. The consumer (LLM or human) can
//! decide their own cutoff threshold.
//!
//! All other slices produce binary output. This produces a ranked, scored slice.

use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::data_flow::DataFlowGraph;
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
    let call_graph = CallGraph::build(files);
    let _dfg = DataFlowGraph::build(files);

    // Score map: (file, line) → (score, hop_distance)
    let mut scores: BTreeMap<(String, usize), (f64, usize)> = BTreeMap::new();

    // Seed: diff lines get score 1.0 at distance 0
    let mut queue: VecDeque<(String, usize, usize)> = VecDeque::new(); // (file, line, hop)
    for diff_info in &diff.files {
        for &line in &diff_info.diff_lines {
            scores.insert((diff_info.file_path.clone(), line), (1.0, 0));
            queue.push_back((diff_info.file_path.clone(), line, 0));
        }
    }

    // BFS with decaying scores
    while let Some((file, line, hop)) = queue.pop_front() {
        if hop >= config.max_hops {
            continue;
        }

        let next_hop = hop + 1;
        let next_score = config.decay.powi(next_hop as i32);
        if next_score < config.min_score {
            continue;
        }

        let parsed = match files.get(&file) {
            Some(f) => f,
            None => continue,
        };

        // Data flow neighbors: variables on this line → their other references
        if let Some(func_node) = parsed.enclosing_function(line) {
            let lines_set = BTreeSet::from([line]);
            let (func_start, func_end) = parsed.node_line_range(&func_node);

            // L-value tracing
            let lvalues = parsed.assignment_lvalues_on_lines(&func_node, &lines_set);
            for (var_name, _) in &lvalues {
                let refs = parsed.find_variable_references(&func_node, var_name);
                for ref_line in refs {
                    if ref_line >= func_start && ref_line <= func_end {
                        let key = (file.clone(), ref_line);
                        let current = scores.get(&key).map(|s| s.0).unwrap_or(0.0);
                        if next_score > current {
                            scores.insert(key.clone(), (next_score, next_hop));
                            queue.push_back((file.clone(), ref_line, next_hop));
                        }
                    }
                }
            }

            // R-value tracing
            let rvalues = parsed.rvalue_identifiers_on_lines(&func_node, &lines_set);
            for (var_name, _) in &rvalues {
                let refs = parsed.find_variable_references(&func_node, var_name);
                for ref_line in refs {
                    if ref_line >= func_start && ref_line <= func_end {
                        let key = (file.clone(), ref_line);
                        let current = scores.get(&key).map(|s| s.0).unwrap_or(0.0);
                        if next_score > current {
                            scores.insert(key.clone(), (next_score, next_hop));
                            queue.push_back((file.clone(), ref_line, next_hop));
                        }
                    }
                }
            }

            // Call graph neighbors: callers and callees
            if let Some(func_id) = call_graph.function_at(&file, line) {
                // Callers
                let callers = call_graph.callers_of(&func_id.name, 1);
                for (caller_id, _) in &callers {
                    let key = (caller_id.file.clone(), caller_id.start_line);
                    let current = scores.get(&key).map(|s| s.0).unwrap_or(0.0);
                    if next_score > current {
                        scores.insert(key.clone(), (next_score, next_hop));
                        queue.push_back((caller_id.file.clone(), caller_id.start_line, next_hop));
                    }
                }

                // Callees
                let callees = call_graph.callees_of(&func_id.name, &file, 1);
                for (callee_id, _) in &callees {
                    let key = (callee_id.file.clone(), callee_id.start_line);
                    let current = scores.get(&key).map(|s| s.0).unwrap_or(0.0);
                    if next_score > current {
                        scores.insert(key.clone(), (next_score, next_hop));
                        queue.push_back((callee_id.file.clone(), callee_id.start_line, next_hop));
                    }
                }
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

    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

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
