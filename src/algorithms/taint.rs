//! Taint Analysis — forward trace of untrusted values through the program.
//!
//! Starting from taint sources (e.g., diff lines, function parameters, user input),
//! propagates taint forward through assignments and function calls. Reports all
//! paths from taint sources to potential sinks (SQL, exec, file ops, HTTP responses).

use crate::ast::ParsedFile;
use crate::data_flow::DataFlowGraph;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// Built-in taint sink patterns.
const SINK_PATTERNS: &[&str] = &[
    "exec",
    "eval",
    "system",
    "popen",
    "subprocess",
    "query",
    "execute",
    "raw_sql",
    "cursor",
    "open",
    "write",
    "unlink",
    "remove",
    "rmdir",
    "send",
    "respond",
    "render",
    "redirect",
    "innerHTML",
    "dangerouslySetInnerHTML",
    "Exec",
    "Command",
    // C/C++ buffer overflow / unsafe string operations
    "strcpy(",
    "strcat(",
    "strncpy(",
    "sprintf(",
    "vsprintf(",
    "gets(",
    "scanf(",
    "memcpy(",
    "memmove(",
    // C/C++ command / library injection
    "execv(",
    "execve(",
    "execvp(",
    "dlopen(",
    // C/C++ memory safety
    "free(",
    // C/C++ format string sinks
    "printf(",
    "fprintf(",
    "snprintf(",
];

/// Configuration for taint analysis.
#[derive(Debug, Clone)]
pub struct TaintConfig {
    /// Explicit taint source locations.
    pub sources: Vec<(String, usize)>,
    /// If true, auto-taint all variables assigned on diff lines.
    pub taint_from_diff: bool,
    /// Additional sink patterns to check.
    pub extra_sinks: Vec<String>,
}

impl Default for TaintConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            taint_from_diff: true,
            extra_sinks: Vec::new(),
        }
    }
}

/// A taint finding: a path from source to sink.
#[derive(Debug, Clone)]
pub struct TaintFinding {
    pub source_file: String,
    pub source_line: usize,
    pub source_var: String,
    pub sink_file: String,
    pub sink_line: usize,
    pub sink_pattern: String,
    pub path_lines: Vec<(String, usize)>,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    taint_config: &TaintConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);
    let dfg = DataFlowGraph::build(files);

    // Collect taint sources
    let mut taint_sources: Vec<(String, usize)> = taint_config.sources.clone();

    if taint_config.taint_from_diff {
        for diff_info in &diff.files {
            for &line in &diff_info.diff_lines {
                taint_sources.push((diff_info.file_path.clone(), line));
            }
        }
    }

    // Forward propagation from each source
    let paths = dfg.taint_forward(&taint_sources);

    // Collect all tainted lines and identify sinks
    let mut all_tainted: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut sink_lines: BTreeSet<(String, usize)> = BTreeSet::new();

    let all_sinks: Vec<&str> = SINK_PATTERNS
        .iter()
        .copied()
        .chain(taint_config.extra_sinks.iter().map(|s| s.as_str()))
        .collect();

    for path in &paths {
        for edge in &path.edges {
            all_tainted
                .entry(edge.from.file.clone())
                .or_default()
                .insert(edge.from.line);
            all_tainted
                .entry(edge.to.file.clone())
                .or_default()
                .insert(edge.to.line);

            // Check if the target location involves a sink
            if let Some(parsed) = files.get(&edge.to.file) {
                let ids = parsed.identifiers_on_line(edge.to.line);
                for id in &ids {
                    let text = parsed.node_text(id);
                    if all_sinks.iter().any(|s| text.contains(s)) {
                        sink_lines.insert((edge.to.file.clone(), edge.to.line));
                    }
                }
            }
        }
    }

    // Also check source lines for sinks (taint at source)
    for (file, line) in &taint_sources {
        all_tainted.entry(file.clone()).or_default().insert(*line);
    }

    // Emit findings for each taint sink reached
    for (file, line) in &sink_lines {
        // Find a source that reaches this sink (use first taint source as representative)
        let source_desc = taint_sources
            .iter()
            .find(|(sf, _)| sf == file)
            .map(|(_, sl)| format!("line {}", sl))
            .unwrap_or_else(|| "diff lines".to_string());
        result.findings.push(SliceFinding {
            algorithm: "taint".to_string(),
            file: file.clone(),
            line: *line,
            severity: "warning".to_string(),
            description: format!(
                "tainted value from {} reaches sink at line {}",
                source_desc, line
            ),
            function_name: None,
            related_lines: taint_sources
                .iter()
                .filter(|(sf, _)| sf == file)
                .map(|(_, sl)| *sl)
                .collect(),
            related_files: vec![],
            category: Some("tainted_value".to_string()),
        });
    }

    // Build output blocks
    let mut block_id = 0;
    for (file, lines) in &all_tainted {
        let mut block = DiffBlock::new(block_id, file.clone(), ModifyType::Modified);

        for &line in lines {
            let is_source = taint_sources.iter().any(|(f, l)| f == file && *l == line);
            let is_sink = sink_lines.contains(&(file.clone(), line));
            // Mark sources and sinks as diff lines for highlighting
            block.add_line(file, line, is_source || is_sink);
        }

        if !lines.is_empty() {
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
