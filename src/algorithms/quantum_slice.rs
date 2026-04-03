//! Quantum Slice — concurrent state superposition enumeration.
//!
//! For async/concurrent code, enumerates all possible states a variable could
//! hold at a given program point considering possible interleavings.
//!
//! Identifies async patterns (await, goroutines, promises, threads) and models
//! which assignments could race with the diff point.

use crate::ast::ParsedFile;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::languages::Language;
use crate::slice::{SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// A possible state for a variable at a program point.
#[derive(Debug, Clone)]
pub struct PossibleState {
    pub var_name: String,
    pub state_label: String,
    pub assignment_line: usize,
    pub assignment_file: String,
    pub is_async_dependent: bool,
}

pub fn slice(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    target_var: Option<&str>,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::QuantumSlice);
    let mut block_id = 0;

    // Pre-compute: scan all files for handler/callback registration calls
    // so we can detect functions that ARE async entry points even when they
    // don't contain async primitives themselves.
    let registered_handlers = collect_registered_handlers(files);

    for diff_info in &diff.files {
        let parsed = match files.get(&diff_info.file_path) {
            Some(f) => f,
            None => continue,
        };

        for &line in &diff_info.diff_lines {
            let func_node = match parsed.enclosing_function(line) {
                Some(f) => f,
                None => continue,
            };

            // Find variables on this diff line
            let vars_on_line: Vec<String> = if let Some(tv) = target_var {
                vec![tv.to_string()]
            } else {
                parsed
                    .identifiers_on_line(line)
                    .iter()
                    .map(|n| parsed.node_text(n).to_string())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            };

            for var_name in &vars_on_line {
                // Find all assignment points for this variable in the function
                let (func_start, func_end) = parsed.node_line_range(&func_node);
                let all_lines: BTreeSet<usize> = (func_start..=func_end).collect();
                let lvalues = parsed.assignment_lvalues_on_lines(&func_node, &all_lines);

                let assignments: Vec<usize> = lvalues
                    .iter()
                    .filter(|(name, _)| name == var_name)
                    .map(|(_, l)| *l)
                    .collect();

                if assignments.is_empty() {
                    continue;
                }

                // Detect async context
                let async_lines = find_async_points(parsed, &func_node);
                let is_async_func = is_async_function(parsed, &func_node, &registered_handlers);

                // Build possible states
                let mut states: Vec<PossibleState> = Vec::new();

                for &assign_line in &assignments {
                    let is_after_async = async_lines
                        .iter()
                        .any(|&al| al < assign_line && al > func_start);
                    let is_before_async = async_lines
                        .iter()
                        .any(|&al| al > assign_line && al < func_end);

                    let state_label = if is_after_async || is_before_async {
                        format!(
                            "line {} (async-dependent: assignment {}await boundary)",
                            assign_line,
                            if is_after_async { "after " } else { "before " }
                        )
                    } else {
                        format!("line {} (synchronous)", assign_line)
                    };

                    states.push(PossibleState {
                        var_name: var_name.clone(),
                        state_label,
                        assignment_line: assign_line,
                        assignment_file: diff_info.file_path.clone(),
                        is_async_dependent: is_after_async || is_before_async,
                    });
                }

                // If there's async context and the variable could be uninitialized
                if is_async_func && !assignments.is_empty() {
                    states.push(PossibleState {
                        var_name: var_name.clone(),
                        state_label: "undefined/uninitialized (async operation not yet completed)"
                            .to_string(),
                        assignment_line: 0,
                        assignment_file: diff_info.file_path.clone(),
                        is_async_dependent: true,
                    });
                }

                // Build block with all relevant lines
                if states.iter().any(|s| s.is_async_dependent) {
                    let mut block =
                        DiffBlock::new(block_id, diff_info.file_path.clone(), ModifyType::Modified);

                    // Include the diff line
                    block.add_line(&diff_info.file_path, line, true);

                    // Include all assignment lines
                    for state in &states {
                        if state.assignment_line > 0 {
                            block.add_line(&state.assignment_file, state.assignment_line, false);
                        }
                    }

                    // Include async boundary lines
                    for &async_line in &async_lines {
                        block.add_line(&diff_info.file_path, async_line, false);
                    }

                    // Include function boundaries
                    block.add_line(&diff_info.file_path, func_start, false);
                    block.add_line(&diff_info.file_path, func_end, false);

                    result.blocks.push(block);
                    block_id += 1;
                }
            }
        }
    }

    Ok(result)
}

fn find_async_points(parsed: &ParsedFile, func_node: &Node<'_>) -> Vec<usize> {
    let mut points = Vec::new();
    find_async_inner(parsed, *func_node, &mut points);
    points.sort();
    points.dedup();
    points
}

fn find_async_inner(parsed: &ParsedFile, node: Node<'_>, out: &mut Vec<usize>) {
    let kind = node.kind();
    let line = node.start_position().row + 1;

    let is_async = match parsed.language {
        Language::Python => {
            kind == "await"
                || kind == "await_expression"
                || (kind == "call_expression" && {
                    let text = parsed.node_text(&node);
                    text.contains("threading.Thread(")
                        || text.contains("Thread(")
                        || text.contains(".start(")
                        || text.contains("multiprocessing.Process(")
                        || text.contains("Process(")
                        || text.contains("pool.apply_async(")
                        || text.contains("pool.map_async(")
                        || text.contains("asyncio.create_task(")
                        || text.contains("asyncio.gather(")
                        || text.contains("loop.run_in_executor(")
                })
        }
        Language::JavaScript | Language::TypeScript => {
            kind == "await_expression"
                || (kind == "call_expression" && {
                    let text = parsed.node_text(&node);
                    text.contains(".then(")
                        || text.contains("setTimeout")
                        || text.contains("setInterval")
                        || text.contains("Promise")
                        || text.contains("Worker(")
                        || text.contains("process.nextTick(")
                        || text.contains("setImmediate(")
                        || text.contains("queueMicrotask(")
                })
        }
        Language::Go => {
            kind == "go_statement"
                || kind == "select_statement"
                || (kind == "send_statement")
                || (kind == "receive_statement")
        }
        Language::Java => {
            kind == "method_invocation" && {
                let text = parsed.node_text(&node);
                text.contains("CompletableFuture")
                    || text.contains("submit(")
                    || text.contains("execute(")
                    || text.contains(".start()")
            }
        }
        Language::C => {
            kind == "call_expression" && {
                let text = parsed.node_text(&node);
                text.contains("pthread_create")
                    || text.contains("fork(")
                    || text.contains("signal(")
                    || text.contains("sigaction(")
                    || text.contains("dispatch_async")
                    || text.contains("request_irq(")
                    || text.contains("request_threaded_irq(")
            }
        }
        Language::Cpp => {
            kind == "call_expression" && {
                let text = parsed.node_text(&node);
                text.contains("std::async")
                    || text.contains("std::thread")
                    || text.contains("pthread_create")
                    || text.contains("fork(")
                    || text.contains("signal(")
                    || text.contains("sigaction(")
                    || text.contains("std::jthread")
                    || text.contains("request_irq(")
                    || text.contains("request_threaded_irq(")
            }
        }
        Language::Rust => {
            kind == "call_expression" && {
                let text = parsed.node_text(&node);
                text.contains("spawn(")
                    || text.contains("tokio::spawn")
                    || text.contains("async_std::task::spawn")
                    || text.contains("thread::spawn")
                    || text.contains("rayon::spawn")
            }
        }
        Language::Lua => {
            // Lua coroutines are the primary async mechanism
            // Note: Lua's tree-sitter grammar uses "function_call", not "call_expression"
            kind == "function_call" && {
                let text = parsed.node_text(&node);
                text.contains("coroutine.create(")
                    || text.contains("coroutine.resume(")
                    || text.contains("coroutine.wrap(")
                    || text.contains("coroutine.yield(")
            }
        }
        Language::Terraform => false, // HCL is declarative, no async patterns
        Language::Bash => {
            if kind == "command" {
                let text = parsed.node_text(&node);
                // In tree-sitter-bash, `&` is a sibling of the command node,
                // not part of its text. Check next sibling for background `&`.
                let has_bg = node.next_sibling().map_or(false, |s| s.kind() == "&");
                has_bg
                    || text.starts_with("nohup ")
                    || text.starts_with("coproc ")
                    || text.starts_with("wait")
            } else {
                false
            }
        }
    };

    if is_async {
        out.push(line);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        find_async_inner(parsed, child, out);
    }
}

/// Scan all parsed files for calls that register a function as a handler/callback.
/// Returns the set of function names that are registered as async entry points.
///
/// Detected registration patterns:
///   signal(SIGTERM, handler_name)
///   sigaction(SIGTERM, &sa, NULL)     — extracts from .sa_handler = name
///   pthread_create(&tid, NULL, worker, arg)
///   request_irq(irq, handler, flags, name, dev)
///   request_threaded_irq(...)
///   std::thread t(worker_func, args)
fn collect_registered_handlers(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    let mut handlers = BTreeSet::new();

    for parsed in files.values() {
        if !matches!(parsed.language, Language::C | Language::Cpp) {
            continue;
        }
        for line in parsed.source.lines() {
            let trimmed = line.trim();

            // signal(SIGxxx, handler_name) — 2nd argument
            if let Some(args) = extract_call_args(trimmed, "signal(") {
                if let Some(name) = nth_arg(&args, 1) {
                    if is_ident(name) {
                        handlers.insert(name.to_string());
                    }
                }
            }

            // pthread_create(&tid, attr, start_routine, arg) — 3rd argument
            if let Some(args) = extract_call_args(trimmed, "pthread_create(") {
                if let Some(name) = nth_arg(&args, 2) {
                    let name = name.trim_start_matches('&');
                    if is_ident(name) {
                        handlers.insert(name.to_string());
                    }
                }
            }

            // request_irq(irq, handler, flags, name, dev) — 2nd argument
            for prefix in &["request_irq(", "request_threaded_irq("] {
                if let Some(args) = extract_call_args(trimmed, prefix) {
                    if let Some(name) = nth_arg(&args, 1) {
                        let name = name.trim_start_matches('&');
                        if is_ident(name) {
                            handlers.insert(name.to_string());
                        }
                    }
                }
            }

            // .sa_handler = handler_name (sigaction struct initialization)
            if trimmed.contains(".sa_handler") {
                if let Some(eq_pos) = trimmed.find('=') {
                    let rhs = trimmed[eq_pos + 1..].trim().trim_end_matches([';', ',']);
                    let rhs = rhs.trim();
                    if is_ident(rhs) {
                        handlers.insert(rhs.to_string());
                    }
                }
            }

            // std::thread t(worker_func, ...) — 1st argument after variable name
            // Also: std::thread(worker_func, ...)
            if trimmed.contains("std::thread") && trimmed.contains('(') {
                if let Some(paren) = trimmed.find('(') {
                    let inner = &trimmed[paren + 1..];
                    if let Some(name) = nth_arg(inner, 0) {
                        let name = name.trim_start_matches('&');
                        if is_ident(name) {
                            handlers.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }

    handlers
}

/// Extract the arguments portion of a function call from a line.
/// Returns the text after the prefix up to the matching `)`.
fn extract_call_args<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let idx = line.find(prefix)?;
    let start = idx + prefix.len();
    // Find matching closing paren (handle nested parens)
    let rest = &line[start..];
    let mut depth = 1;
    for (i, ch) in rest.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&rest[..i]);
                }
            }
            _ => {}
        }
    }
    Some(rest)
}

/// Get the nth comma-separated argument from an argument list string.
fn nth_arg(args: &str, n: usize) -> Option<&str> {
    // Simple comma split — doesn't handle nested parens in args,
    // but works for the common case where handler names are plain identifiers
    args.split(',').nth(n).map(|s| s.trim())
}

/// Check if a string is a valid C identifier.
fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn is_async_function(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    registered_handlers: &BTreeSet<String>,
) -> bool {
    // Check if this function is registered as a handler/callback elsewhere
    let func_name = parsed
        .language
        .function_name(func_node)
        .map(|n| parsed.node_text(&n).to_string())
        .unwrap_or_default();
    if registered_handlers.contains(&func_name) {
        return true;
    }

    match parsed.language {
        Language::Python => {
            let text = parsed.node_text(func_node);
            // async def or contains threading/multiprocessing usage
            text.starts_with("async ")
                || text.contains("threading.Thread(")
                || text.contains("Thread(")
                || text.contains("multiprocessing.Process(")
                || text.contains("pool.apply_async(")
                || text.contains("asyncio.create_task(")
        }
        Language::JavaScript | Language::TypeScript => {
            let text = parsed.node_text(func_node);
            text.starts_with("async ")
                || text.contains("Worker(")
                || text.contains("process.nextTick(")
                || text.contains("cluster.fork(")
        }
        Language::Go => {
            // Go functions aren't inherently async, but check if they contain
            // goroutines, select statements, or channel operations
            let mut has_go = false;
            check_for_go_stmt(parsed, *func_node, &mut has_go);
            if has_go {
                return true;
            }
            let text = parsed.node_text(func_node);
            text.contains("select {") || text.contains("<-")
        }
        Language::Java => {
            // Check if return type involves CompletableFuture, Future, etc.
            let text = parsed.node_text(func_node);
            text.contains("CompletableFuture")
                || text.contains("Future<")
                || text.contains("Callable")
        }
        Language::C => {
            // C functions aren't inherently async, but check if they contain
            // pthread_create, fork, signal handlers, or kernel IRQ registration.
            // Also treat functions whose names suggest callback/ISR semantics as async.
            // Note: registered_handlers check above catches externally registered handlers.
            let text = parsed.node_text(func_node);
            let name_is_async = func_name.contains("_handler")
                || func_name.contains("_callback")
                || func_name.contains("_isr")
                || func_name.contains("_irq");
            name_is_async
                || text.contains("pthread_create")
                || text.contains("fork(")
                || text.contains("signal(")
                || text.contains("sigaction(")
                || text.contains("request_irq(")
                || text.contains("request_threaded_irq(")
        }
        Language::Cpp => {
            // Check for C++ async patterns and callback/ISR name heuristics.
            // Note: registered_handlers check above catches externally registered handlers.
            let text = parsed.node_text(func_node);
            let name_is_async = func_name.contains("_handler")
                || func_name.contains("_callback")
                || func_name.contains("_isr")
                || func_name.contains("_irq");
            name_is_async
                || text.contains("std::async")
                || text.contains("std::thread")
                || text.contains("std::jthread")
                || text.contains("co_await")
                || text.contains("co_yield")
                || text.contains("pthread_create")
                || text.contains("fork(")
                || text.contains("signal(")
                || text.contains("sigaction(")
                || text.contains("request_irq(")
                || text.contains("request_threaded_irq(")
        }
        Language::Rust => {
            let text = parsed.node_text(func_node);
            text.starts_with("async ")
                || text.contains("tokio::spawn")
                || text.contains("thread::spawn")
                || text.contains(".await")
                || text.contains("async move")
        }
        Language::Lua => {
            let text = parsed.node_text(func_node);
            text.contains("coroutine.create(")
                || text.contains("coroutine.resume(")
                || text.contains("coroutine.wrap(")
        }
        Language::Terraform => false, // HCL is declarative, no async patterns
        Language::Bash => {
            let text = parsed.node_text(func_node);
            text.contains(" &") || text.contains("nohup ") || text.contains("coproc ")
        }
    }
}

fn check_for_go_stmt(parsed: &ParsedFile, node: Node<'_>, found: &mut bool) {
    if node.kind() == "go_statement" {
        *found = true;
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if !*found {
            check_for_go_stmt(parsed, child, found);
        }
    }
}
