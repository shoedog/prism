//! Absence Slice — what's NOT in the code that should be.
//!
//! **Question answered:** "What obligations does this code have that it hasn't fulfilled?"
//!
//! Given a change, identifies expected but missing counterparts. Many operations
//! come in pairs: open/close, lock/unlock, acquire/release, connect/disconnect,
//! allocate/free, subscribe/unsubscribe, begin/commit. If one side appears
//! without the other in the enclosing scope, that's a potential resource leak,
//! deadlock, or protocol violation.
//!
//! Unlike all other slices which show what IS in the code, this shows what ISN'T.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::BTreeMap;

/// A paired operation pattern.
#[derive(Debug, Clone)]
pub struct PairedPattern {
    pub open_patterns: Vec<&'static str>,
    pub close_patterns: Vec<&'static str>,
    pub description: &'static str,
}

/// Built-in paired patterns that should appear together.
pub fn default_pairs() -> Vec<PairedPattern> {
    vec![
        PairedPattern {
            open_patterns: vec!["open(", "fopen(", "Open(", "OpenFile("],
            close_patterns: vec!["close(", "fclose(", "Close(", ".close()"],
            description: "file open without close",
        },
        PairedPattern {
            open_patterns: vec![".lock(", "Lock(", "acquire(", "mutex.lock", "RLock("],
            close_patterns: vec![".unlock(", "Unlock(", "release(", "mutex.unlock", "RUnlock("],
            description: "lock without unlock",
        },
        PairedPattern {
            open_patterns: vec!["connect(", "Connect(", "dial(", "Dial(", "createConnection"],
            close_patterns: vec!["disconnect(", "Disconnect(", "close(", "Close(", "closeConnection"],
            description: "connection opened without close",
        },
        PairedPattern {
            open_patterns: vec!["subscribe(", "addEventListener(", "on(", "addListener("],
            close_patterns: vec!["unsubscribe(", "removeEventListener(", "off(", "removeListener("],
            description: "event subscription without unsubscribe",
        },
        PairedPattern {
            open_patterns: vec!["begin(", "beginTransaction(", "startTransaction(", "BEGIN"],
            close_patterns: vec!["commit(", "rollback(", "endTransaction(", "COMMIT", "ROLLBACK"],
            description: "transaction begin without commit/rollback",
        },
        PairedPattern {
            open_patterns: vec!["malloc(", "calloc(", "realloc(", "alloc(", "new "],
            close_patterns: vec!["free(", "dealloc(", "delete ", "release("],
            description: "allocation without free",
        },
        PairedPattern {
            open_patterns: vec!["setInterval(", "setTimeout("],
            close_patterns: vec!["clearInterval(", "clearTimeout("],
            description: "timer set without clear",
        },
        PairedPattern {
            open_patterns: vec!["push(", "append(", "add(", "enqueue("],
            close_patterns: vec!["pop(", "remove(", "dequeue("],
            description: "item added without removal path",
        },
        PairedPattern {
            open_patterns: vec!["startSpan(", "beginSpan(", "startTimer("],
            close_patterns: vec!["endSpan(", "finishSpan(", "stopTimer("],
            description: "span/timer started without end",
        },
        PairedPattern {
            open_patterns: vec!["defer "], // Go-specific: if no defer, flag it
            close_patterns: vec!["defer "],
            description: "resource acquisition without defer cleanup (Go)",
        },
    ]
}

/// A finding: a missing counterpart.
#[derive(Debug, Clone)]
pub struct AbsenceFinding {
    pub file: String,
    pub line: usize,
    pub found_pattern: String,
    pub missing_description: String,
    pub function_name: String,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::AbsenceSlice);
    let pairs = default_pairs();
    let mut block_id = 0;

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        let source_lines: Vec<&str> = parsed.source.lines().collect();

        for &diff_line in &diff_info.diff_lines {
            if diff_line == 0 || diff_line > source_lines.len() {
                continue;
            }
            let line_text = source_lines[diff_line - 1];

            // Check each pair pattern
            for pair in &pairs {
                let has_open = pair.open_patterns.iter().any(|p| line_text.contains(p));
                if !has_open {
                    continue;
                }

                // Find the enclosing function
                let func_node = match parsed.enclosing_function(diff_line) {
                    Some(f) => f,
                    None => continue,
                };

                let func_name = parsed
                    .language
                    .function_name(&func_node)
                    .map(|n| parsed.node_text(&n).to_string())
                    .unwrap_or_else(|| "<anonymous>".to_string());

                let (func_start, func_end) = parsed.node_line_range(&func_node);

                // Search the entire function for the close counterpart
                let has_close = (func_start..=func_end).any(|l| {
                    if l == 0 || l > source_lines.len() {
                        return false;
                    }
                    let lt = source_lines[l - 1];
                    pair.close_patterns.iter().any(|p| lt.contains(p))
                });

                // Also check for language-specific cleanup patterns
                let has_defer_or_finally = (func_start..=func_end).any(|l| {
                    if l == 0 || l > source_lines.len() {
                        return false;
                    }
                    let lt = source_lines[l - 1];
                    lt.contains("defer ") || lt.contains("finally") || lt.contains("with ") || lt.contains("using ")
                });

                if !has_close && !has_defer_or_finally {
                    // Missing counterpart found — build a block showing the finding
                    let mut block = DiffBlock::new(
                        block_id,
                        diff_info.file_path.clone(),
                        ModifyType::Modified,
                    );

                    // Include function signature
                    block.add_line(&diff_info.file_path, func_start, false);

                    // Include the line with the open pattern (highlighted)
                    block.add_line(&diff_info.file_path, diff_line, true);

                    // Include function end (where the close should be)
                    block.add_line(&diff_info.file_path, func_end, false);

                    // Include any return statements (potential early exits missing cleanup)
                    let returns = parsed.return_statements(&func_node);
                    for ret_line in &returns {
                        block.add_line(&diff_info.file_path, *ret_line, false);
                    }

                    result.findings.push(SliceFinding {
                        algorithm: "absence".to_string(),
                        file: diff_info.file_path.clone(),
                        line: diff_line,
                        severity: "warning".to_string(),
                        description: format!(
                            "{} in function '{}' (line {})",
                            pair.description, func_name, diff_line
                        ),
                        function_name: Some(func_name.clone()),
                        related_lines: returns.clone(),
                        related_files: vec![],
                        category: Some("missing_counterpart".to_string()),
                    });
                    result.blocks.push(block);
                    block_id += 1;
                }
            }
        }
    }

    Ok(result)
}
