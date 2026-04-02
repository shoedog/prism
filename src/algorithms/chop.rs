//! Chopping — find all statements on any data flow path between source and sink.
//!
//! Given a source location and a sink location, identifies every statement that
//! participates in data flow from source to sink. Useful for security analysis
//! (e.g., "does user input reach this SQL query?") and data flow bug detection.
//!
//! Uses the Code Property Graph for data flow chopping.

use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::diff::{DiffBlock, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

/// Configuration for a chop operation.
#[derive(Debug, Clone)]
pub struct ChopConfig {
    pub source_file: String,
    pub source_line: usize,
    pub sink_file: String,
    pub sink_line: usize,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    chop_config: &ChopConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::Chop);
    let cpg = CodePropertyGraph::build(files);

    let on_path = cpg.dfg_chop(
        &chop_config.source_file,
        chop_config.source_line,
        &chop_config.sink_file,
        chop_config.sink_line,
    );

    // Group by file
    let mut by_file: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (file, line) in &on_path {
        by_file.entry(file.clone()).or_default().push(*line);
    }

    let mut block_id = 0;
    for (file, lines) in &by_file {
        let mut block = DiffBlock::new(block_id, file.clone(), ModifyType::Modified);

        let is_source = |l: usize| *file == chop_config.source_file && l == chop_config.source_line;
        let is_sink = |l: usize| *file == chop_config.sink_file && l == chop_config.sink_line;

        for &line in lines {
            // Mark source and sink as "diff" lines for highlighting
            block.add_line(file, line, is_source(line) || is_sink(line));
        }

        result.blocks.push(block);
        block_id += 1;
    }

    Ok(result)
}
