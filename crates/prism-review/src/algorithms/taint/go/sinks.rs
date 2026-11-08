//! Go structured sinks, framework sources and inline source==sink matching.

use crate::algorithms::taint::sink_match::{
    call_passes_sink_semantics, line_matches_structured_sink, SinkMatchOutcome,
};
use crate::algorithms::taint::TaintSeed;
use prism::ast::tree_helpers::call_path_text;
use prism::ast::ParsedFile;
use prism::data_flow::FlowPath;
use prism::frameworks::{CallSite, SanitizerCategory, SinkPattern};
use std::collections::BTreeSet;
use tree_sitter::Node;

// ─────────────────────────────────────────────────────────────────────────────
// Phase 1 Go CWE-78 / CWE-22 structured sinks (spec §3.2 / §3.3).
//
// These coexist with `SINK_PATTERNS` above: the flat list uses substring
// identifier matching for cross-language coverage; the structured list below
// uses qualified call-path matching with optional `semantic_check` predicates
// for argument-shape discrimination (e.g., shell-wrapper detection).
//
// Both registries are consulted independently in the analysis pass.
// ─────────────────────────────────────────────────────────────────────────────

/// Returns true if `call`'s arguments at `name_idx` and `flag_idx` form a shell-wrapper
/// invocation (e.g. `("sh", "-c", ...)`, `("pwsh", "-Command", ...)`).
///
/// Common Linux/macOS/Windows shells only; exotic absolute paths (`/usr/bin/sh`,
/// `/usr/local/bin/bash`) deliberately NOT included per spec §3.2 scope note.
fn is_shell_wrapper_at(call: &CallSite, name_idx: usize, flag_idx: usize) -> bool {
    let name = call.literal_arg(name_idx).unwrap_or("");
    let flag = call.literal_arg(flag_idx).unwrap_or("");
    match name {
        "sh" | "bash" | "/bin/sh" | "/bin/bash" => flag == "-c",
        "cmd.exe" => flag == "/c",
        "pwsh" | "powershell" | "powershell.exe" => {
            matches!(flag, "-c" | "-Command" | "-command")
        }
        _ => false,
    }
}

/// Adapter for `exec.Command("sh", "-c", X)`-shaped sinks
/// (function-pointer compatible — `semantic_check` is `Option<fn(...)>`, not a closure).
fn check_shell_wrapper(call: &CallSite) -> bool {
    is_shell_wrapper_at(call, 0, 1)
}

/// Adapter for `exec.CommandContext(ctx, "sh", "-c", X)`-shaped sinks
/// where the context arg shifts everything by one.
fn check_shell_wrapper_ctx(call: &CallSite) -> bool {
    is_shell_wrapper_at(call, 1, 2)
}

/// Cross-cutting Go CWE-78 (OS command injection) sinks. See spec §3.2.
///
/// Both `exec.Command` and `exec.CommandContext` appear twice:
/// - Once for the shell-wrapper form (`semantic_check` filters to shell calls);
///   tainted-arg index points at the `X` payload after `"-c"`.
/// - Once for the tainted-binary form; tainted-arg index is the binary-path
///   argument itself. `semantic_check: None` because per-arg taint resolution
///   at sink-eval time (see `arg_is_tainted_in_path`) is the structural gate:
///   a literal binary has no identifier and is never tainted; a variable
///   bound to a non-tainted source isn't reached by any FlowPath edge at the
///   call line.
///
/// `syscall.Exec(argv0, argv, envv)` checks both `argv0` (literal-or-tainted)
/// and the `argv` slice (DFG-conservative: any tainted slice element taints
/// the slice as a whole). Per-element tracking is out of scope for Phase 1.
pub const GO_CWE78_SINKS: &[SinkPattern] = &[
    // Shell-wrapped variants — payload is the arg after "-c".
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[2],
        semantic_check: Some(check_shell_wrapper),
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[3],
        semantic_check: Some(check_shell_wrapper_ctx),
    },
    // Tainted-binary variants — first non-ctx arg is the binary path.
    // semantic_check requires the binary arg to be non-literal so a hardcoded
    // binary like exec.Command("ffmpeg", "-i", tainted) does NOT fire here.
    SinkPattern {
        call_path: "exec.Command",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "exec.CommandContext",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[1],
        semantic_check: None,
    },
    // syscall.Exec — argv0 + argv slice.
    SinkPattern {
        call_path: "syscall.Exec",
        category: SanitizerCategory::OsCommand,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
];

/// Cross-cutting Go CWE-22 (path traversal) sinks. See spec §3.3.
///
/// `os.Rename(old, new)` checks both arguments; everything else is single-arg.
/// `filepath.Join` is *not* a sink — it's a path-construction primitive that
/// taint flows through; the downstream `os.*` call is what fires.
pub const GO_CWE22_SINKS: &[SinkPattern] = &[
    // Read sinks
    SinkPattern {
        call_path: "os.Open",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.OpenFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.ReadFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "ioutil.ReadFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    // Write sinks
    SinkPattern {
        call_path: "os.Create",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.WriteFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "ioutil.WriteFile",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    // Mutation sinks
    SinkPattern {
        call_path: "os.Remove",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.RemoveAll",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.Mkdir",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.MkdirAll",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0],
        semantic_check: None,
    },
    SinkPattern {
        call_path: "os.Rename",
        category: SanitizerCategory::PathTraversal,
        tainted_arg_indices: &[0, 1],
        semantic_check: None,
    },
];

// ─────────────────────────────────────────────────────────────────────────────
// Framework-aware source detection (spec §2.6 / §2.8 — pull model).
//
// For each Go file with a detected framework, walk every function definition
// in the file:
//   1. Collect parameter names whose type matches the framework's request type
//      (`*http.Request` for net/http and gorilla/mux; `*gin.Context` for gin).
//   2. For each `SourcePattern` in the framework spec, substitute each matched
//      parameter name into the pattern's `call_path` prefix.
//   3. Scan the function body for call expressions whose textual prefix matches
//      the substituted path. Each match's start line becomes a taint source.
//
// Patterns without a conventional prefix (like gorilla/mux's `mux.Vars`) are
// matched as-is — `mux.Vars(r)` is a free function that takes the request as
// an argument rather than living on a method receiver.
// ─────────────────────────────────────────────────────────────────────────────

/// Type strings that bind to a request-like parameter for each framework.
/// `*http.Request` covers net/http + gorilla/mux; `*gin.Context` covers gin.
fn framework_request_types(framework_name: &str) -> &'static [&'static str] {
    match framework_name {
        "gin" => &["*gin.Context"],
        "net/http" | "gorilla/mux" => &["*http.Request"],
        _ => &[],
    }
}

/// The conventional receiver-name prefixes a framework expects in its source
/// patterns. When the bound parameter name differs, we substitute these
/// prefixes textually. Patterns whose `call_path` starts with neither prefix
/// (e.g. gorilla/mux's `mux.Vars`) are matched without substitution.
fn framework_prefixes(framework_name: &str) -> &'static [&'static str] {
    match framework_name {
        "gin" => &["c."],
        "net/http" | "gorilla/mux" => &["r."],
        _ => &[],
    }
}

/// Substitute the conventional framework prefix in `call_path` with the bound
/// parameter name. If `call_path` doesn't start with any framework prefix,
/// returns it unchanged (covers free-function patterns like `mux.Vars`).
fn substitute_prefix(call_path: &str, param_name: &str, framework_name: &str) -> String {
    for prefix in framework_prefixes(framework_name) {
        if let Some(rest) = call_path.strip_prefix(prefix) {
            return format!("{}.{}", param_name, rest);
        }
    }
    call_path.to_string()
}

/// Collect names of parameters in `func_node` whose type matches one of `target_types`.
/// Per spec §2.6, ALL matching parameters bind (not just the first), to handle
/// pathological signatures like `func cmp(a, b *http.Request)`.
fn collect_request_param_names(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    target_types: &[&str],
) -> Vec<String> {
    let mut names = Vec::new();
    let params = match func_node.child_by_field_name("parameters") {
        Some(p) => p,
        None => return names,
    };
    let mut cursor = params.walk();
    for param in params.named_children(&mut cursor) {
        if param.kind() != "parameter_declaration" {
            continue;
        }
        let type_text = match param.child_by_field_name("type") {
            Some(t) => parsed.node_text(&t).trim().to_string(),
            None => continue,
        };
        if !target_types.contains(&type_text.as_str()) {
            continue;
        }
        // A single parameter_declaration may declare multiple names sharing one type
        // (Go: `func f(a, b *http.Request)`). Collect every identifier child.
        let mut name_cursor = param.walk();
        for child in param.named_children(&mut name_cursor) {
            if child.kind() == "identifier" {
                names.push(parsed.node_text(&child).to_string());
            }
        }
    }
    names
}

pub(in crate::algorithms::taint) fn go_call_path_text(
    parsed: &ParsedFile,
    call_node: &Node<'_>,
) -> Option<String> {
    call_path_text(parsed, call_node)
}

/// Walk `root` collecting every Go `call_expression` node.
pub(in crate::algorithms::taint) fn collect_go_calls<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "call_expression" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_go_calls(child, out);
    }
}

pub(in crate::algorithms::taint) fn detect_go_framework_sources(
    file_path: &str,
    parsed: &ParsedFile,
    sources: &mut Vec<TaintSeed>,
) {
    let spec = match parsed.framework() {
        Some(s) => s,
        None => return,
    };
    let target_types = framework_request_types(spec.name);
    if target_types.is_empty() {
        return;
    }

    for func in parsed.all_functions() {
        let param_names = collect_request_param_names(parsed, &func, target_types);
        if param_names.is_empty() {
            continue;
        }
        let mut calls = Vec::new();
        collect_go_calls(func, &mut calls);

        for source_pat in spec.sources {
            let concrete_paths: Vec<String> = if framework_prefixes(spec.name)
                .iter()
                .any(|p| source_pat.call_path.starts_with(p))
            {
                param_names
                    .iter()
                    .map(|n| substitute_prefix(source_pat.call_path, n, spec.name))
                    .collect()
            } else {
                vec![source_pat.call_path.to_string()]
            };

            for call in &calls {
                let actual = match go_call_path_text(parsed, call) {
                    Some(s) => s,
                    None => continue,
                };
                if concrete_paths.contains(&actual) {
                    sources.push(TaintSeed::line(
                        file_path.to_string(),
                        call.start_position().row + 1,
                    ));
                }
            }
        }
    }
}

/// Returns the first structured sink pattern on `line` whose tainted_arg subtrees
/// contain a descendant call_expression matching the active framework's source
/// patterns (e.g. `c.Param`, `r.URL.Query`). Used as a secondary fallback in the
/// source==sink loop to catch inline source==sink shapes that the per-arg DFG with
/// a real FlowPath cannot resolve — inline framework-source calls don't generate
/// FlowEdges because their results are consumed inline.
///
/// Scanning is exhaustive over (sink_pat, call) on this line, not first-match.
/// First-match would miss inline shapes when an unrelated structured sink earlier
/// on the line shadows the inline-bearing one (e.g. `exec.Command("ls"); c.File(c.Param("f"))`).
///
/// Request param names are scoped to the enclosing function of `line`, mirroring
/// `detect_framework_sources`. File-wide collection would treat unrelated handlers'
/// receiver names as valid binders here. The empty-collection short-circuit
/// (return None when `request_param_names.is_empty()`) is also load-bearing: it
/// matches the `detect_framework_sources` guard at L794, preventing non-prefixed
/// sources like `mux.Vars` from being recognized in functions that don't bind a
/// `*http.Request` parameter.
///
/// Phase 1.5 limitation: only framework sources are recognized, not IPC sources.
/// IPC source==sink shapes are rare and remain a Phase 1.5.1+ refinement.
pub(in crate::algorithms::taint) fn find_sink_with_inline_framework_source(
    parsed: &ParsedFile,
    line: usize,
) -> Option<&'static SinkPattern> {
    let framework = parsed.framework()?;

    // Function-scoped request param name collection. Mirrors
    // `detect_framework_sources` — only binds receiver names that appear in the
    // enclosing function's signature.
    let func_node = parsed.enclosing_function(line)?;
    let target_types = framework_request_types(framework.name);
    if target_types.is_empty() {
        return None;
    }
    let request_param_names = collect_request_param_names(parsed, &func_node, target_types);
    // Empty-function-scope guard: mirrors the early `continue` in
    // `detect_framework_sources`. Without it, non-prefixed sources like
    // `mux.Vars` would still be inserted into `source_paths` even when the
    // enclosing function has no `*http.Request` / `*gin.Context` parameter,
    // wrongly recognizing them as framework sources for this line.
    if request_param_names.is_empty() {
        return None;
    }

    // Build the set of concrete framework-source call_paths for THIS function
    // (mirrors detect_framework_sources's prefix-substitution logic).
    let mut source_paths: BTreeSet<String> = BTreeSet::new();
    for src in framework.sources {
        if framework_prefixes(framework.name)
            .iter()
            .any(|p| src.call_path.starts_with(p))
        {
            for n in &request_param_names {
                source_paths.insert(substitute_prefix(src.call_path, n, framework.name));
            }
        } else {
            // No conventional prefix — match as-is (e.g. mux.Vars).
            source_paths.insert(src.call_path.to_string());
        }
    }
    if source_paths.is_empty() {
        return None;
    }

    // Walk all calls on `line`. For each, check against EVERY structured sink
    // pattern (priority order: GO_CWE78_SINKS, GO_CWE22_SINKS, framework SINKS).
    // First (call, sink_pat) pair where the tainted_arg subtree contains an
    // inline framework source returns its sink_pat.
    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };

        let pattern_iter = GO_CWE78_SINKS
            .iter()
            .chain(GO_CWE22_SINKS.iter())
            .chain(framework.sinks.iter());

        for pat in pattern_iter {
            if actual != pat.call_path {
                continue;
            }
            // Apply semantic_check (matches go_sink_outcome's gating). If
            // semantic_check rejects, this pattern doesn't describe THIS call —
            // skip and try the next pattern.
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
            }
            let arguments = match call.child_by_field_name("arguments") {
                Some(n) => n,
                None => continue,
            };
            let mut cursor = arguments.walk();
            let mut idx = 0usize;
            for arg in arguments.named_children(&mut cursor) {
                if pat.tainted_arg_indices.contains(&idx)
                    && subtree_has_call_in(parsed, &arg, &source_paths)
                {
                    return Some(pat);
                }
                idx += 1;
            }
        }
    }
    None
}

/// Walk `node` and descendants; returns true if any `call_expression` node has
/// a call_path text in `paths`.
fn subtree_has_call_in(parsed: &ParsedFile, node: &Node<'_>, paths: &BTreeSet<String>) -> bool {
    if node.kind() == "call_expression" {
        if let Some(cp) = go_call_path_text(parsed, node) {
            if paths.contains(&cp) {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if subtree_has_call_in(parsed, &child, paths) {
            return true;
        }
    }
    false
}

/// Returns the structured-sink outcome for `line` across the full Go sink registry
/// (cross-cutting CWE-78/22 + framework-gated). Used during path-aware suppression.
///
/// `path` is forwarded to `line_matches_structured_sink` for per-argument taint
/// resolution (Phase 1.5 #1). Pass `Some(path)` from forward-flow callers; the
/// source==sink loop passes `None` for the no-originating-path branch (canonical
/// `c.File(c.Param("f"))` shape) so the engine falls back to call_path +
/// semantic_check matching without per-arg precision.
///
/// Aggregation rules:
/// - If any pattern returns `Match`, the first such pattern wins (priority order:
///   GO_CWE78_SINKS, GO_CWE22_SINKS, framework SINKS). The matched pattern's
///   category drives `FlowPath.cleansed_for` consultation.
/// - Else if any pattern returned `SemanticallyExcluded`, aggregate is
///   `SemanticallyExcluded`.
/// - Else `NoMatch`. Flat-pattern catch-all proceeds normally.
///
/// Note: when multiple patterns share a `call_path` (exec.Command shell-wrapper +
/// tainted-binary), the shell-wrapper variant is listed first; if it `Match`-es
/// it wins, otherwise the tainted-binary variant is checked next.
pub(in crate::algorithms::taint) fn go_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    let mut any_call_path_match = false;
    for pat in GO_CWE78_SINKS {
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    for pat in GO_CWE22_SINKS {
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    if let Some(spec) = parsed.framework() {
        for pat in spec.sinks {
            match line_matches_structured_sink(parsed, line, pat, path) {
                SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
                SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
                SinkMatchOutcome::NoMatch => {}
            }
        }
    }
    if any_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}
