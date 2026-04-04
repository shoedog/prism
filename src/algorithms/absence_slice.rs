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
            open_patterns: vec![
                "of_node_get(",
                "of_find_node_by_name(",
                "of_find_node_by_type(",
                "of_find_compatible_node(",
            ],
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
        // Kernel slab/memory pool
        PairedPattern {
            open_patterns: vec!["kmem_cache_alloc("],
            close_patterns: vec!["kmem_cache_free("],
            description: "slab cache allocation without free",
        },
        // Kernel RCU read-side critical section
        PairedPattern {
            open_patterns: vec!["rcu_read_lock("],
            close_patterns: vec!["rcu_read_unlock("],
            description: "RCU read lock without unlock",
        },
        // === C/C++ POSIX pairs ===
        // POSIX thread mutex
        PairedPattern {
            open_patterns: vec!["pthread_mutex_lock("],
            close_patterns: vec!["pthread_mutex_unlock("],
            description: "pthread mutex lock without unlock",
        },
        // POSIX semaphore
        PairedPattern {
            open_patterns: vec!["sem_wait(", "sem_trywait(", "sem_timedwait("],
            close_patterns: vec!["sem_post("],
            description: "semaphore wait without post",
        },
        // Memory-mapped I/O
        PairedPattern {
            open_patterns: vec!["mmap("],
            close_patterns: vec!["munmap("],
            description: "mmap without munmap",
        },
        // POSIX file descriptors (distinct from C++ stream close)
        PairedPattern {
            open_patterns: vec!["=open(", "openat(", "creat("],
            close_patterns: vec!["close("],
            description: "POSIX file descriptor opened without close",
        },
        // POSIX directory stream
        PairedPattern {
            open_patterns: vec!["opendir(", "fdopendir("],
            close_patterns: vec!["closedir("],
            description: "directory opened without closedir",
        },
        // POSIX read-write lock
        PairedPattern {
            open_patterns: vec!["pthread_rwlock_rdlock(", "pthread_rwlock_wrlock("],
            close_patterns: vec!["pthread_rwlock_unlock("],
            description: "pthread rwlock without unlock",
        },
        // === Python-specific pairs ===
        PairedPattern {
            open_patterns: vec!["threading.Lock(", "threading.RLock("],
            close_patterns: vec![".release("],
            description: "Python threading lock without release",
        },
        PairedPattern {
            open_patterns: vec!["pool.apply_async(", "pool.map_async(", "Pool("],
            close_patterns: vec!["pool.close(", "pool.terminate(", "pool.join("],
            description: "Python multiprocessing pool without close/join",
        },
        PairedPattern {
            open_patterns: vec!["socket.socket(", "socket("],
            close_patterns: vec![".close(", "close("],
            description: "socket created without close",
        },
        PairedPattern {
            open_patterns: vec!["tempfile.mkstemp(", "tempfile.NamedTemporaryFile("],
            close_patterns: vec!["os.close(", "os.unlink(", "os.remove(", ".close("],
            description: "temporary file without cleanup",
        },
        // === JavaScript/TypeScript-specific pairs ===
        PairedPattern {
            open_patterns: vec!["createReadStream(", "createWriteStream("],
            close_patterns: vec![".destroy(", ".close(", ".end("],
            description: "Node.js stream without destroy/close/end",
        },
        PairedPattern {
            open_patterns: vec!["createServer("],
            close_patterns: vec!["server.close(", ".close("],
            description: "server created without close",
        },
        PairedPattern {
            open_patterns: vec!["pool.connect(", "pool.query("],
            close_patterns: vec!["client.release(", ".release(", "pool.end("],
            description: "database pool connection without release",
        },
        PairedPattern {
            open_patterns: vec!["fs.open(", "fs.openSync("],
            close_patterns: vec!["fs.close(", "fs.closeSync("],
            description: "fs.open without fs.close",
        },
        PairedPattern {
            open_patterns: vec!["acquire(", "lock("],
            close_patterns: vec!["release(", "unlock("],
            description: "lock/acquire without release/unlock",
        },
        // === Go-specific pairs ===
        PairedPattern {
            open_patterns: vec!["sql.Open("],
            close_patterns: vec!["db.Close(", ".Close("],
            description: "Go sql.Open without db.Close",
        },
        PairedPattern {
            open_patterns: vec!["os.Create(", "os.OpenFile("],
            close_patterns: vec![".Close("],
            description: "Go file created without Close",
        },
        PairedPattern {
            open_patterns: vec![
                "context.WithCancel(",
                "context.WithTimeout(",
                "context.WithDeadline(",
            ],
            close_patterns: vec!["cancel("],
            description: "Go context without cancel (may leak goroutine)",
        },
        PairedPattern {
            open_patterns: vec![".Add("],
            close_patterns: vec![".Wait("],
            description: "WaitGroup Add without Wait",
        },
        PairedPattern {
            open_patterns: vec!["http.Get(", "http.Post(", "http.Do("],
            close_patterns: vec![".Body.Close(", "Body.Close("],
            description: "Go HTTP response body not closed",
        },
        // === Rust-specific pairs ===
        PairedPattern {
            open_patterns: vec!["File::open(", "File::create(", "OpenOptions"],
            close_patterns: vec!["drop(", ".flush("],
            description: "Rust file opened without explicit flush/drop",
        },
        PairedPattern {
            open_patterns: vec![".lock()", "Mutex::lock(", "RwLock::read(", "RwLock::write("],
            close_patterns: vec!["drop("],
            description:
                "advisory: Rust mutex lock held to end of scope (explicit drop() releases sooner)",
        },
        PairedPattern {
            open_patterns: vec!["unsafe {", "unsafe{"],
            close_patterns: vec![
                "assert!", // line text scan (no trailing '(' — macros aren't call nodes)
                "debug_assert!",
                "assert_eq!",
                "assert_ne!",
                "// SAFETY",
                "// Safety",
            ],
            description: "unsafe block without safety assertion or comment",
        },
        PairedPattern {
            open_patterns: vec!["TcpListener::bind(", "TcpStream::connect("],
            close_patterns: vec![".shutdown(", "drop("],
            description: "Rust TCP connection without shutdown/drop",
        },
        PairedPattern {
            open_patterns: vec!["Command::new("],
            close_patterns: vec![".status()", ".output()", ".spawn("],
            description: "Rust Command created but never executed",
        },
        // === Lua-specific pairs ===
        PairedPattern {
            open_patterns: vec!["io.open("],
            close_patterns: vec![":close(", "io.close("],
            description: "Lua file opened without close",
        },
        PairedPattern {
            open_patterns: vec!["socket.tcp", "socket.udp", "socket.connect"],
            close_patterns: vec![":close"],
            description: "Lua socket opened without close",
        },
        PairedPattern {
            open_patterns: vec!["coroutine.create("],
            close_patterns: vec!["coroutine.resume("],
            description: "Lua coroutine created but never resumed",
        },
        // === Terraform / HCL ===
        // Resource companion patterns: resources that should have security companions
        PairedPattern {
            open_patterns: vec!["aws_s3_bucket"],
            close_patterns: vec!["aws_s3_bucket_server_side_encryption_configuration"],
            description: "S3 bucket missing encryption configuration",
        },
        PairedPattern {
            open_patterns: vec!["aws_s3_bucket"],
            close_patterns: vec!["aws_s3_bucket_public_access_block"],
            description: "S3 bucket missing public access block",
        },
        PairedPattern {
            open_patterns: vec!["aws_s3_bucket"],
            close_patterns: vec!["aws_s3_bucket_versioning"],
            description: "S3 bucket missing versioning configuration",
        },
        PairedPattern {
            open_patterns: vec!["aws_lambda_function"],
            close_patterns: vec!["aws_cloudwatch_log_group"],
            description: "Lambda function missing CloudWatch log group",
        },
        PairedPattern {
            open_patterns: vec!["aws_security_group"],
            close_patterns: vec!["aws_security_group_rule"],
            description: "Security group missing explicit rule resource",
        },
        PairedPattern {
            open_patterns: vec!["aws_db_instance"],
            close_patterns: vec!["storage_encrypted"],
            description: "RDS instance missing storage_encrypted configuration",
        },
        // === Shell / Bash ===
        PairedPattern {
            open_patterns: vec!["mktemp"],
            close_patterns: vec!["rm ", "rm -", "unlink "],
            description: "Temp file created with mktemp but never cleaned up",
        },
        PairedPattern {
            open_patterns: vec!["mount "],
            close_patterns: vec!["umount "],
            description: "Filesystem mounted but never unmounted",
        },
        PairedPattern {
            open_patterns: vec!["pushd "],
            close_patterns: vec!["popd"],
            description: "pushd without matching popd",
        },
        PairedPattern {
            open_patterns: vec!["trap "],
            close_patterns: vec!["trap -", "trap ''", "trap \"\""],
            description: "Signal trap set but never restored/cleared",
        },
        PairedPattern {
            open_patterns: vec!["exec 3>", "exec 3>>", "exec 4>", "exec 4>>"],
            close_patterns: vec!["exec 3>&-", "exec 4>&-"],
            description: "File descriptor opened but never closed",
        },
        PairedPattern {
            open_patterns: vec!["flock ", "lockfile "],
            close_patterns: vec!["flock -u", "rm -f /tmp/*.lock", "rm -f /var/lock"],
            description: "Lock file acquired but never released",
        },
        // === Busybox / Firmware shell ===
        // Flash write should be preceded by hash verification
        PairedPattern {
            open_patterns: vec!["mtd write", "mtd -r write"],
            close_patterns: vec!["sha256sum", "md5sum", "sha1sum", "verify", "hash"],
            description: "Firmware flash write (mtd) without hash verification",
        },
        // UCI config changes should be committed
        PairedPattern {
            open_patterns: vec!["uci set"],
            close_patterns: vec!["uci commit"],
            description: "UCI config set without commit",
        },
        // Kernel module load should have matching unload in cleanup
        PairedPattern {
            open_patterns: vec!["insmod ", "modprobe "],
            close_patterns: vec!["rmmod ", "modprobe -r"],
            description: "Kernel module loaded without unload in cleanup path",
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

/// Check if a call name matches a pattern base, handling both qualified and
/// unqualified forms. For example, pattern `tempfile.mkstemp` matches call
/// name `mkstemp` (method-only) or `tempfile.mkstemp` (fully qualified).
fn call_name_matches_pattern(call_name: &str, pattern_base: &str) -> bool {
    // Exact match or call name contains pattern base (original behavior)
    if call_name.contains(pattern_base) {
        return true;
    }
    // Method-only match: extract method from pattern (after last `.`)
    // e.g., pattern "tempfile.mkstemp" → method "mkstemp" matches call "mkstemp"
    if let Some(method) = pattern_base.rsplit('.').next() {
        if !method.is_empty() && call_name == method {
            return true;
        }
    }
    false
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
            // Collect AST call names on this specific diff line once (for open check).
            let line_calls = parsed.call_names_on_lines(&[diff_line]);

            // Check each pair pattern
            for pair in &pairs {
                // For patterns ending in '(': use AST call node names (avoids comments/strings).
                // For keyword patterns: use AST-aware text matching to skip comments/strings.
                let has_open = pair.open_patterns.iter().any(|p| {
                    if let Some(base) = pattern_to_call_base(p) {
                        line_calls.get(&diff_line).map_or(false, |names| {
                            names.iter().any(|n| call_name_matches_pattern(n, base))
                        })
                    } else {
                        parsed.line_has_code_text(diff_line, p)
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
                            .any(|names| names.iter().any(|n| call_name_matches_pattern(n, base)))
                    } else {
                        (func_start..=func_end).any(|l| {
                            l > 0 && l <= source_lines.len() && parsed.line_has_code_text(l, p)
                        })
                    }
                });

                // Also check for language-specific cleanup patterns and C++ RAII.
                // RAII types manage cleanup automatically on destruction, so absence
                // of an explicit close/unlock/free is not a bug in those cases.
                // Use AST-aware matching so comments like "// defer cleanup" don't suppress findings.
                const CLEANUP_KEYWORDS: &[&str] = &[
                    "defer ",
                    "finally",
                    "with ",
                    "using ",
                    // C++ RAII mutex wrappers — no explicit unlock needed
                    "std::lock_guard",
                    "std::unique_lock",
                    "std::scoped_lock",
                    // C++ RAII memory management — no explicit delete/free needed
                    "std::unique_ptr",
                    "std::shared_ptr",
                    // C++ RAII memory management via factory functions
                    "std::make_unique",
                    "std::make_shared",
                    // C++ RAII file handle — closes on destruction
                    "std::fstream",
                    "std::ifstream",
                    "std::ofstream",
                ];
                let has_defer_or_finally = (func_start..=func_end).any(|l| {
                    if l == 0 || l > source_lines.len() {
                        return false;
                    }
                    CLEANUP_KEYWORDS
                        .iter()
                        .any(|kw| parsed.line_has_code_text(l, kw))
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

                    // Include goto labels as related lines (shows cleanup paths)
                    let label_sections = parsed.label_sections(&func_node);
                    let mut related = returns.clone();
                    for (_, label_line, _) in &label_sections {
                        block.add_line(&diff_info.file_path, *label_line, false);
                        related.push(*label_line);
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
                        related_lines: related,
                        related_files: vec![],
                        category: Some("missing_counterpart".to_string()),
                    });
                    result.blocks.push(block);
                    block_id += 1;
                }

                // Double-close detection for C/C++ goto error paths.
                //
                // Detects when a close operation (free, unlock, etc.) appears both
                // inline before a goto AND in the goto target label section. This is
                // the classic kernel double-free/double-unlock bug pattern:
                //
                //   if (error) {
                //       free(buf);         // inline close
                //       goto cleanup;
                //   }
                //   ...
                //   cleanup:
                //       free(buf);         // label close — double-free!
                //
                // Known limitation: matches on function name (e.g. "free"), not
                // on which argument is freed. `free(a)` inline + `free(b)` in
                // the label will false-positive. The finding includes line numbers
                // so a human or LLM reviewer can verify.
                //
                if has_close
                    && matches!(
                        parsed.language,
                        crate::languages::Language::C | crate::languages::Language::Cpp
                    )
                {
                    let gotos = parsed.goto_statements(&func_node);
                    let label_sections = parsed.label_sections(&func_node);

                    if !gotos.is_empty() && !label_sections.is_empty() {
                        // For each close pattern, check if it appears both before
                        // a goto AND in the target label's section.
                        for close_pattern in &pair.close_patterns {
                            let close_base = match pattern_to_call_base(close_pattern) {
                                Some(b) => b,
                                None => continue,
                            };

                            for (goto_label, goto_line) in &gotos {
                                // Find close calls between the open (diff_line) and this goto
                                let inline_close_lines: Vec<usize> = (diff_line..=*goto_line)
                                    .filter(|&l| {
                                        func_calls.get(&l).map_or(false, |names| {
                                            names.iter().any(|n| n.contains(close_base))
                                        })
                                    })
                                    .collect();

                                if inline_close_lines.is_empty() {
                                    continue;
                                }

                                // Find the target label section
                                if let Some((_, label_start, label_end)) = label_sections
                                    .iter()
                                    .find(|(name, _, _)| name == goto_label)
                                {
                                    // Check if close also appears in the label section
                                    let label_close_lines: Vec<usize> = (*label_start..=*label_end)
                                        .filter(|&l| {
                                            func_calls.get(&l).map_or(false, |names| {
                                                names.iter().any(|n| n.contains(close_base))
                                            })
                                        })
                                        .collect();

                                    if !label_close_lines.is_empty() {
                                        // Double-close detected!
                                        let mut block = DiffBlock::new(
                                            block_id,
                                            diff_info.file_path.clone(),
                                            ModifyType::Modified,
                                        );

                                        // Show the open
                                        block.add_line(&diff_info.file_path, diff_line, true);
                                        // Show inline close(s)
                                        for &cl in &inline_close_lines {
                                            block.add_line(&diff_info.file_path, cl, true);
                                        }
                                        // Show goto
                                        block.add_line(&diff_info.file_path, *goto_line, false);
                                        // Show label close(s)
                                        for &cl in &label_close_lines {
                                            block.add_line(&diff_info.file_path, cl, true);
                                        }

                                        let mut related = inline_close_lines.clone();
                                        related.extend(&label_close_lines);

                                        result.findings.push(SliceFinding {
                                            algorithm: "absence".to_string(),
                                            file: diff_info.file_path.clone(),
                                            line: inline_close_lines[0],
                                            severity: "warning".to_string(),
                                            description: format!(
                                                "potential double-close in '{}': {}() at line {} and label '{}' at line {}",
                                                func_name,
                                                close_base,
                                                inline_close_lines[0],
                                                goto_label,
                                                label_close_lines[0],
                                            ),
                                            function_name: Some(func_name.clone()),
                                            related_lines: related,
                                            related_files: vec![],
                                            category: Some("double_close".to_string()),
                                        });
                                        result.blocks.push(block);
                                        block_id += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}
