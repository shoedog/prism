//! Horizontal Slice — peer pattern consistency analysis.
//!
//! Given a change in one code construct, slices to all constructs at the same
//! abstraction level that should follow the same patterns. Enables omission
//! detection: "this handler has validation; these three don't."

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// How to identify peer constructs.
#[derive(Debug, Clone)]
pub enum PeerPattern {
    /// Match by decorator/annotation (e.g., `@app.route`, `@Test`)
    Decorator(String),
    /// Match by parent class/interface name
    ParentClass(String),
    /// Match by function name pattern (prefix/suffix)
    NamePattern(String),
    /// Match by directory siblings with similar structure
    DirectorySiblings,
    /// Auto-detect: infer the pattern from the changed construct
    Auto,
}

impl Default for PeerPattern {
    fn default() -> Self {
        Self::Auto
    }
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    peer_pattern: &PeerPattern,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::HorizontalSlice);
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        // Find the changed construct(s)
        let mut changed_funcs: Vec<(String, usize, usize)> = Vec::new(); // (name, start, end)
        for &line in &diff_info.diff_lines {
            if let Some(func_node) = parsed.enclosing_function(line) {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    let name = parsed.node_text(&name_node).to_string();
                    let (start, end) = parsed.node_line_range(&func_node);
                    if !changed_funcs.iter().any(|(n, _, _)| n == &name) {
                        changed_funcs.push((name, start, end));
                    }
                }
            }
        }

        for (changed_name, changed_start, changed_end) in &changed_funcs {
            // Determine the peer pattern
            let effective_pattern = match peer_pattern {
                PeerPattern::Auto => detect_pattern(parsed, *changed_start),
                other => other.clone(),
            };

            // Find all peers across all files
            let mut peers: Vec<(String, String, usize, usize)> = Vec::new(); // (file, name, start, end)

            for (file_path, file_parsed) in files {
                for func_node in file_parsed.all_functions() {
                    if let Some(name_node) = file_parsed.language.function_name(&func_node) {
                        let name = file_parsed.node_text(&name_node).to_string();
                        if &name == changed_name && file_path == &diff_info.file_path {
                            continue; // Skip the changed function itself
                        }

                        let (start, end) = file_parsed.node_line_range(&func_node);

                        if matches_pattern(file_parsed, &func_node, &effective_pattern) {
                            peers.push((file_path.clone(), name, start, end));
                        }
                    }
                }
            }

            // Build block: changed function first, then peers
            let mut block = DiffBlock::new(
                block_id,
                diff_info.file_path.clone(),
                diff_info.modify_type.clone(),
            );

            // Include the changed function
            for line in *changed_start..=*changed_end {
                let is_diff = diff_info.diff_lines.contains(&line);
                block.add_line(&diff_info.file_path, line, is_diff);
            }

            // Include peer functions (signatures and first few lines)
            for (peer_file, _peer_name, peer_start, peer_end) in &peers {
                let preview_end = (*peer_end).min(*peer_start + 10);
                for line in *peer_start..=preview_end {
                    block.add_line(peer_file, line, false);
                }
                if *peer_end > preview_end {
                    block.add_line(peer_file, *peer_end, false);
                }
            }

            if !block.file_line_map.is_empty() {
                result.blocks.push(block);
                block_id += 1;
            }
        }
    }

    Ok(result)
}

fn detect_pattern(parsed: &ParsedFile, func_start_line: usize) -> PeerPattern {
    // Check for decorators/annotations above the function
    let row = func_start_line.saturating_sub(1);
    if row > 0 {
        let prev_lines: Vec<&str> = parsed.source.lines().take(row).collect();
        // Look at lines just above the function for decorators
        for i in (0..prev_lines.len()).rev() {
            let line = prev_lines[i].trim();
            if line.starts_with('@') || line.starts_with("#[") {
                // Found a decorator
                let decorator = line.to_string();
                return PeerPattern::Decorator(decorator);
            }
            if !line.is_empty() && !line.starts_with('#') && !line.starts_with("//") {
                break;
            }
        }
    }

    // Fall back to directory siblings
    PeerPattern::DirectorySiblings
}

fn matches_pattern(parsed: &ParsedFile, func_node: &Node<'_>, pattern: &PeerPattern) -> bool {
    match pattern {
        PeerPattern::Decorator(dec) => {
            let func_start = func_node.start_position().row;
            if func_start == 0 {
                return false;
            }
            // Check lines above the function for the same decorator pattern
            let lines: Vec<&str> = parsed.source.lines().collect();
            for i in (0..func_start).rev() {
                let line = lines.get(i).map(|l| l.trim()).unwrap_or("");
                if line.starts_with('@') || line.starts_with("#[") {
                    // Match decorator base name (ignore arguments)
                    let dec_base = dec.split('(').next().unwrap_or(dec);
                    let line_base = line.split('(').next().unwrap_or(line);
                    return dec_base == line_base;
                }
                if !line.is_empty() && !line.starts_with('#') && !line.starts_with("//") {
                    return false;
                }
            }
            false
        }
        PeerPattern::ParentClass(class_name) => {
            // Check if the function is inside a class with the given name
            if let Some(parent) = func_node.parent() {
                if parent.kind() == "class_body" || parent.kind() == "class_declaration" {
                    let text = parsed.node_text(&parent);
                    return text.contains(class_name);
                }
            }
            false
        }
        PeerPattern::NamePattern(pattern) => {
            if let Some(name_node) = parsed.language.function_name(func_node) {
                let name = parsed.node_text(&name_node);
                if pattern.starts_with('*') {
                    name.ends_with(&pattern[1..])
                } else if pattern.ends_with('*') {
                    name.starts_with(&pattern[..pattern.len() - 1])
                } else {
                    name.contains(pattern)
                }
            } else {
                false
            }
        }
        PeerPattern::DirectorySiblings => {
            // All functions in the same file are peers by default
            true
        }
        PeerPattern::Auto => true,
    }
}
