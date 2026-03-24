//! Angle Slice — cross-cutting concern tracing.
//!
//! Traces how a specific concern (error handling, logging, auth, caching)
//! cuts diagonally across the architecture. Given a diff line touching
//! a concern pattern, finds all other locations where the same concern
//! appears across all files.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

/// Built-in concern definitions.
#[derive(Debug, Clone)]
pub enum Concern {
    ErrorHandling,
    Logging,
    Authentication,
    Caching,
    /// Custom pattern: a regex-like set of keywords.
    Custom(String, Vec<String>),
}

impl Concern {
    pub fn name(&self) -> &str {
        match self {
            Self::ErrorHandling => "error_handling",
            Self::Logging => "logging",
            Self::Authentication => "authentication",
            Self::Caching => "caching",
            Self::Custom(name, _) => name,
        }
    }

    pub fn patterns(&self) -> Vec<&str> {
        match self {
            Self::ErrorHandling => vec![
                "try", "catch", "except", "raise", "throw", "Error", "Exception",
                "panic", "recover", "unwrap", "expect", "err", "error",
            ],
            Self::Logging => vec![
                "log", "logger", "logging", "console.log", "console.error",
                "console.warn", "fmt.Print", "fmt.Errorf", "log.Print",
                "Log.", "LOG", "debug", "info", "warn",
            ],
            Self::Authentication => vec![
                "auth", "token", "session", "permission", "credential",
                "login", "logout", "jwt", "oauth", "cookie", "Bearer",
                "authenticate", "authorize",
            ],
            Self::Caching => vec![
                "cache", "Cache", "redis", "memcache", "ttl", "expire",
                "invalidate", "evict", "lru", "memoize",
            ],
            Self::Custom(_, patterns) => patterns.iter().map(|s| s.as_str()).collect(),
        }
    }

    /// Parse a concern from a string.
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "error" | "error_handling" | "errors" => Self::ErrorHandling,
            "log" | "logging" | "logs" => Self::Logging,
            "auth" | "authentication" | "authorization" => Self::Authentication,
            "cache" | "caching" => Self::Caching,
            _ => {
                // Treat as a custom pattern with comma-separated keywords
                let patterns: Vec<String> = s.split(',').map(|p| p.trim().to_string()).collect();
                Self::Custom(s.to_string(), patterns)
            }
        }
    }
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    concern: &Concern,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::AngleSlice);
    let patterns = concern.patterns();

    // First, verify the concern exists on diff lines
    let mut concern_on_diff = false;
    for diff_info in &diff.files {
        if let Some(parsed) = files.get(&diff_info.file_path) {
            for &line in &diff_info.diff_lines {
                if line_matches_concern(parsed, line, &patterns) {
                    concern_on_diff = true;
                    break;
                }
            }
        }
        if concern_on_diff {
            break;
        }
    }

    // Find all locations of this concern across all files
    let mut block_id = 0;
    for (file_path, parsed) in files {
        let source_lines: Vec<&str> = parsed.source.lines().collect();
        let mut concern_lines: BTreeMap<usize, bool> = BTreeMap::new();

        for (i, line_text) in source_lines.iter().enumerate() {
            let line_num = i + 1;
            if patterns.iter().any(|p| line_text.contains(p)) {
                // Check if this line is a diff line
                let is_diff = diff.files.iter().any(|d| {
                    d.file_path == *file_path && d.diff_lines.contains(&line_num)
                });
                concern_lines.insert(line_num, is_diff);

                // Include enclosing function context
                if let Some(func_node) = parsed.enclosing_function(line_num) {
                    let (start, end) = parsed.node_line_range(&func_node);
                    concern_lines.entry(start).or_insert(false);
                    concern_lines.entry(end).or_insert(false);
                }
            }
        }

        if !concern_lines.is_empty() {
            let is_diff_file = diff
                .files
                .iter()
                .any(|d| d.file_path == *file_path);

            let mut block = DiffBlock::new(
                block_id,
                file_path.clone(),
                if is_diff_file {
                    ModifyType::Modified
                } else {
                    ModifyType::Modified // Related file
                },
            );

            for (&line, &is_diff) in &concern_lines {
                block.add_line(file_path, line, is_diff);
            }

            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}

fn line_matches_concern(parsed: &ParsedFile, line: usize, patterns: &[&str]) -> bool {
    let source_lines: Vec<&str> = parsed.source.lines().collect();
    if line == 0 || line > source_lines.len() {
        return false;
    }
    let line_text = source_lines[line - 1];
    patterns.iter().any(|p| line_text.contains(p))
}
