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
            close_patterns: vec![
                ".unlock(",
                "Unlock(",
                "release(",
                "mutex.unlock",
                "RUnlock(",
            ],
            description: "lock without unlock",
        },
        PairedPattern {
            open_patterns: vec!["connect(", "Connect(", "dial(", "Dial(", "createConnection"],
            close_patterns: vec![
                "disconnect(",
                "Disconnect(",
                "close(",
                "Close(",
                "closeConnection",
            ],
            description: "connection opened without close",
        },
        PairedPattern {
            open_patterns: vec!["subscribe(", "addEventListener(", "on(", "addListener("],
            close_patterns: vec![
                "unsubscribe(",
                "removeEventListener(",
                "off(",
                "removeListener(",
            ],
            description: "event subscription without unsubscribe",
        },
        PairedPattern {
            open_patterns: vec!["begin(", "beginTransaction(", "startTransaction(", "BEGIN"],
            close_patterns: vec![
                "commit(",
                "rollback(",
                "endTransaction(",
                "COMMIT",
                "ROLLBACK",
            ],
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
        // Kernel memory allocation
        PairedPattern {
            open_patterns: vec!["kmalloc(", "kzalloc(", "vmalloc("],
            close_patterns: vec!["kfree(", "vfree("],
            description: "kernel allocation without free",
        },
        // DMA allocation
        PairedPattern {
            open_patterns: vec!["dma_alloc_coherent("],
            close_patterns: vec!["dma_free_coherent("],
            description: "DMA allocation without free",
        },
        // IRQ registration
        PairedPattern {
            open_patterns: vec!["request_irq(", "request_threaded_irq("],
            close_patterns: vec!["free_irq("],
            description: "IRQ registered without free",
        },
        // Kernel spinlock
        PairedPattern {
            open_patterns: vec!["spin_lock(", "spin_lock_irqsave("],
            close_patterns: vec!["spin_unlock(", "spin_unlock_irqrestore("],
            description: "spinlock without unlock",
        },
        // Clock management
        PairedPattern {
            open_patterns: vec!["clk_prepare_enable("],
            close_patterns: vec!["clk_disable_unprepare("],
            description: "clock enabled without disable",
        },
        // Platform driver registration
        PairedPattern {
            open_patterns: vec!["platform_driver_register("],
            close_patterns: vec!["platform_driver_unregister("],
            description: "platform driver registered without unregister",
        },
        // Device tree node reference counting
        PairedPattern {
            open_patterns: vec!["of_node_get(", "of_find_node_by_name(", "of_find_node_by_type(", "of_find_compatible_node("],
            close_patterns: vec!["of_node_put("],
            description: "device tree node get without put",
        },
        // Kernel mutex (distinct from pthread/userspace mutex patterns above)
        PairedPattern {
            open_patterns: vec!["mutex_lock("],
            close_patterns: vec!["mutex_unlock("],
            description: "kernel mutex lock without unlock",
        },
        // Network subsystem lock
        PairedPattern {
            open_patterns: vec!["rtnl_lock("],
            close_patterns: vec!["rtnl_unlock("],
            description: "rtnl lock without unlock",
        },
        // Kernel string duplication
        PairedPattern {
            open_patterns: vec!["kstrdup("],
            close_patterns: vec!["kfree("],
            description: "kstrdup allocation without kfree",
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

/// Extract the base function name from a call pattern like `"malloc("` or `".lock("`.
/// Returns `None` for keyword/non-call patterns (e.g. `"new "`, `"defer "`, SQL keywords).
fn pattern_to_call_base(pattern: &str) -> Option<&str> {
    if !pattern.ends_with('(') {
        return None;
    }
    let base = pattern.trim_end_matches('(');
    let base = base.trim_start_matches('.');
    if base.is_empty() {
        None
    } else {
        Some(base)
    }
}

pub fn slice(files: &BTreeMap<String, ParsedFile>, diff: &DiffInput) -> Result<SliceResult> {
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

            // Collect AST call names on this specific diff line once (for open check).
            let line_calls = parsed.call_names_on_lines(&[diff_line]);

            // Check each pair pattern
            for pair in &pairs {
                // For patterns ending in '(': use AST call node names (avoids comments/strings).
                // For keyword patterns (new, defer, SQL): fall back to substring matching.
                let has_open = pair.open_patterns.iter().any(|p| {
                    if let Some(base) = pattern_to_call_base(p) {
                        line_calls
                            .get(&diff_line)
                            .map_or(false, |names| names.iter().any(|n| n.contains(base)))
                    } else {
                        line_text.contains(p)
                    }
                });
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

                // Collect AST call names across the entire function body (for close check).
                let func_lines: Vec<usize> = (func_start..=func_end).collect();
                let func_calls = parsed.call_names_on_lines(&func_lines);

                // Search the entire function for the close counterpart.
                let has_close = pair.close_patterns.iter().any(|p| {
                    if let Some(base) = pattern_to_call_base(p) {
                        func_calls
                            .values()
                            .any(|names| names.iter().any(|n| n.contains(base)))
                    } else {
                        (func_start..=func_end).any(|l| {
                            l > 0 && l <= source_lines.len() && source_lines[l - 1].contains(p)
                        })
                    }
                });

                // Also check for language-specific cleanup patterns and C++ RAII.
                // RAII types manage cleanup automatically on destruction, so absence
                // of an explicit close/unlock/free is not a bug in those cases.
                let has_defer_or_finally = (func_start..=func_end).any(|l| {
                    if l == 0 || l > source_lines.len() {
                        return false;
                    }
                    let lt = source_lines[l - 1];
                    lt.contains("defer ")
                        || lt.contains("finally")
                        || lt.contains("with ")
                        || lt.contains("using ")
                        // C++ RAII mutex wrappers — no explicit unlock needed
                        || lt.contains("std::lock_guard")
                        || lt.contains("std::unique_lock")
                        || lt.contains("std::scoped_lock")
                        // C++ RAII memory management — no explicit delete/free needed
                        || lt.contains("std::unique_ptr")
                        || lt.contains("std::shared_ptr")
                        // C++ RAII file handle — closes on destruction
                        || lt.contains("std::fstream")
                        || lt.contains("std::ifstream")
                        || lt.contains("std::ofstream")
                });

                if !has_close && !has_defer_or_finally {
                    // Missing counterpart found — build a block showing the finding
                    let mut block =
                        DiffBlock::new(block_id, diff_info.file_path.clone(), ModifyType::Modified);

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
