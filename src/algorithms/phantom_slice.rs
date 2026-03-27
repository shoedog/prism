//! Phantom Slice — surface recently deleted code that the current diff may need.
//!
//! **Question answered:** "Is there recently deleted code that this change might unknowingly depend on?"
//!
//! If a function was removed in recent commits and the current change touches
//! code that formerly called or referenced it, the deleted code is surfaced as
//! "ghost" context. Catches the case where someone deletes a utility and the
//! current diff unknowingly depends on its absence being correct.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

/// Configuration for phantom slicing.
#[derive(Debug, Clone)]
pub struct PhantomConfig {
    /// Path to the git repository.
    pub git_dir: String,
    /// How many commits back to look for deletions.
    pub max_commits: usize,
}

impl Default for PhantomConfig {
    fn default() -> Self {
        Self {
            git_dir: ".".to_string(),
            max_commits: 50,
        }
    }
}

/// A deleted function or symbol found in git history.
#[derive(Debug, Clone)]
pub struct DeletedSymbol {
    pub name: String,
    pub file: String,
    pub deleted_code: String,
    pub commit: String,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    config: &PhantomConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::PhantomSlice);

    // Get recently deleted functions from git
    let deleted = find_recently_deleted(&config.git_dir, config.max_commits);

    // Collect all identifiers referenced on diff lines
    let mut diff_identifiers: BTreeSet<String> = BTreeSet::new();
    for diff_info in &diff.files {
        if let Some(parsed) = files.get(&diff_info.file_path) {
            for &line in &diff_info.diff_lines {
                let ids = parsed.identifiers_on_line(line);
                for id in &ids {
                    let name = parsed.node_text(id).to_string();
                    if name.len() > 2 {
                        diff_identifiers.insert(name);
                    }
                }
            }
        }
    }

    // Check if any diff identifiers match recently deleted symbols
    let mut block_id = 0;
    for symbol in &deleted {
        if diff_identifiers.contains(&symbol.name) {
            let mut block = DiffBlock::new(block_id, symbol.file.clone(), ModifyType::Deleted);

            // Include the diff lines that reference the deleted symbol
            for diff_info in &diff.files {
                if let Some(parsed) = files.get(&diff_info.file_path) {
                    for &line in &diff_info.diff_lines {
                        let ids = parsed.identifiers_on_line(line);
                        if ids.iter().any(|id| parsed.node_text(id) == symbol.name) {
                            block.add_line(&diff_info.file_path, line, true);
                        }
                    }
                }
            }

            // The deleted code itself can't be shown via line numbers since it's gone,
            // but we mark the old file location
            if !block.file_line_map.is_empty() {
                result.blocks.push(block);
                block_id += 1;
            }
        }
    }

    Ok(result)
}

/// Find recently deleted function/class definitions from git history.
fn find_recently_deleted(git_dir: &str, max_commits: usize) -> Vec<DeletedSymbol> {
    let mut deleted = Vec::new();

    // Get recent commits that deleted lines
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{}", max_commits),
            "--diff-filter=D",
            "--name-only",
            "--format=%H",
        ])
        .current_dir(git_dir)
        .output();

    let stdout = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).to_string(),
        Err(_) => return deleted,
    };

    let mut current_commit = String::new();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() == 40 && line.chars().all(|c| c.is_ascii_hexdigit()) {
            current_commit = line.to_string();
        } else {
            // This is a deleted file path
            // Try to get the content of the deleted file
            let show_output = Command::new("git")
                .args(["show", &format!("{}^:{}", current_commit, line)])
                .current_dir(git_dir)
                .output();

            if let Ok(show_out) = show_output {
                let content = String::from_utf8_lossy(&show_out.stdout);
                // Extract function names from the deleted content
                for code_line in content.lines() {
                    let trimmed = code_line.trim();
                    // Heuristic: detect function definitions
                    let name = extract_function_name(trimmed);
                    if let Some(name) = name {
                        deleted.push(DeletedSymbol {
                            name,
                            file: line.to_string(),
                            deleted_code: code_line.to_string(),
                            commit: current_commit.clone(),
                        });
                    }
                }
            }
        }
    }

    deleted
}

/// Heuristic extraction of function names from a line of code.
fn extract_function_name(line: &str) -> Option<String> {
    // Python: def function_name(
    if line.starts_with("def ") {
        return line
            .strip_prefix("def ")?
            .split('(')
            .next()
            .map(|s| s.trim().to_string());
    }
    // JS/TS: function functionName(
    if line.starts_with("function ") {
        return line
            .strip_prefix("function ")?
            .split('(')
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    // Go: func functionName(
    if line.starts_with("func ") {
        let rest = line.strip_prefix("func ")?;
        // Skip receiver: func (r *Receiver) Name(
        let rest = if rest.starts_with('(') {
            rest.split(')').nth(1)?.trim()
        } else {
            rest
        };
        return rest
            .split('(')
            .next()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
    }
    // Java: public/private/protected ... methodName(
    if (line.contains("public ") || line.contains("private ") || line.contains("protected "))
        && line.contains('(')
        && !line.contains("class ")
    {
        let before_paren = line.split('(').next()?;
        return before_paren
            .split_whitespace()
            .last()
            .map(|s| s.to_string())
            .filter(|s| {
                !s.is_empty() && s.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
            });
    }

    None
}
