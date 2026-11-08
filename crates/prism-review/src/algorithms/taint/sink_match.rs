//! Language-neutral structured sink matching and tainted-argument checks.

use crate::algorithms::taint::cleansing::{node_contains_line, node_in_ranges};
use crate::algorithms::taint::go::sinks::{go_call_path_text, go_sink_outcome};
use crate::algorithms::taint::js_ts::imports_ssrf::js_ts_ssrf_sink_call_path_matches;
use crate::algorithms::taint::js_ts::sink_tables::js_yaml_load_call_uses_unsafe_schema;
use crate::algorithms::taint::js_ts::sinks::{
    is_js_yaml_load_sink_pattern, js_ts_js_yaml_bare_load_call_matches,
    js_ts_sink_call_has_inline_framework_source_arg, js_ts_sink_outcome,
};
use crate::algorithms::taint::js_ts::source_ranges::js_ts_request_source_access_ranges_for_alias_on_line;
use crate::algorithms::taint::python::clients::{
    python_is_aiohttp_client_session_method_call, python_is_aiohttp_request_call,
    python_is_urllib3_pool_manager_request_call,
};
use crate::algorithms::taint::python::sinks::python_sink_outcome;
use prism::ast::tree_helpers::{
    call_arg_node, call_path_matches, collect_calls, is_js_ts_language,
};
use prism::ast::ParsedFile;
use prism::data_flow::FlowPath;
use prism::frameworks::{CallSite, SanitizerCategory, SinkPattern};
use prism::languages::Language;
use tree_sitter::Node;

// ─────────────────────────────────────────────────────────────────────────────
// Structured sink matching (spec §3.1 / §3.2 / §3.3).
//
// A sink fires when:
//   1. The call's qualified path equals `sink_pat.call_path`, AND
//   2. The optional `semantic_check` returns true (or is `None`), AND
//   3. The taint engine has flagged the line as carrying tainted data.
//
// (3) is checked by the existing taint pass — this helper handles (1) and (2).
// ─────────────────────────────────────────────────────────────────────────────

/// Outcome of consulting the structured Go sink registry for a given call line.
///
/// Distinguishes three states that the previous `Option<&SinkPattern>` shape
/// conflated as `None`:
///
/// - `Match` — a pattern's `call_path` matched AND its `semantic_check` (if any)
///   accepted. The structured layer fires for this line; the flat-pattern catch-all
///   should also be allowed to add findings (subject to path-aware cleanser
///   suppression on the matched pattern's category).
/// - `SemanticallyExcluded` — at least one pattern's `call_path` matched but every
///   matching pattern either failed its `semantic_check` or had no relevant tainted
///   arg on this path. This outcome is not allowed to suppress the whole flat line:
///   the original PR #73 design did that, but reviewer feedback showed it hid
///   unmodeled shells and unrelated same-line sinks. Cleanser suppression, when
///   applicable, is scoped separately to identifiers inside the cleansed structured
///   call expression.
/// - `NoMatch` — no pattern's `call_path` matched. The structured layer has no
///   opinion; flat-pattern catch-all proceeds normally.
#[derive(Clone, Copy)]
pub(crate) enum SinkMatchOutcome {
    Match(&'static SinkPattern),
    SemanticallyExcluded,
    NoMatch,
}

pub(super) fn call_literal_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    arg_idx: usize,
) -> Option<String> {
    let arg = call_arg_node(call, arg_idx)?;
    let text = parsed.node_text(&arg).trim();
    if matches!(
        arg.kind(),
        "interpreted_string_literal" | "raw_string_literal" | "string"
    ) || ((text.starts_with('"') || text.starts_with('\'') || text.starts_with('`'))
        && text.len() >= 2)
    {
        if text.starts_with('`') && text.contains("${") {
            return None;
        }
        let quote_idx = text.find(['"', '\'', '`']).unwrap_or(0);
        let prefix = &text[..quote_idx];
        if prefix.chars().any(|c| c == 'f' || c == 'F') {
            return None;
        }
        let without_prefix = &text[quote_idx..];
        let trimmed = without_prefix
            .strip_prefix("\"\"\"")
            .and_then(|s| s.strip_suffix("\"\"\""))
            .or_else(|| {
                without_prefix
                    .strip_prefix("'''")
                    .and_then(|s| s.strip_suffix("'''"))
            })
            .or_else(|| {
                without_prefix
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
            })
            .or_else(|| {
                without_prefix
                    .strip_prefix('\'')
                    .and_then(|s| s.strip_suffix('\''))
            })
            .or_else(|| {
                without_prefix
                    .strip_prefix('`')
                    .and_then(|s| s.strip_suffix('`'))
            })
            .unwrap_or(without_prefix);
        return Some(trimmed.to_string());
    }
    None
}

/// Returns true if argument `arg_idx` of the call expression is tainted along `path`.
///
/// Resolution rules:
/// - Literal arg (string, int, bool, nil) → always false (literals can't be tainted).
/// - Bare identifier → check if any `FlowEdge` in `path` has this identifier as a `to`
///   location matching `parsed.path` (file scoping prevents cross-file collisions),
///   `call_line`, and `var_name()`. Without the file-scoping guard, an interprocedural
///   FlowEdge ending in another file at the same line/name could falsely register
///   as taint here.
/// - Complex expression (call, selector, binary, ...) → conservative recurse into
///   descendants; if ANY identifier descendant is tainted on the path (with file
///   scoping), the arg is considered tainted. Phase 1.5 keeps this conservative;
///   tightening (e.g., only considering specific positions in a selector chain) is
///   Phase 2+.
///
/// Returns false if the call has fewer than `arg_idx + 1` arguments.
pub(super) fn arg_is_tainted_in_path(
    parsed: &ParsedFile,
    call: &Node<'_>,
    arg_idx: usize,
    path: &FlowPath,
) -> bool {
    let arg_node = match call_arg_node(call, arg_idx) {
        Some(n) => n,
        None => return false,
    };
    arg_node_taints_match(parsed, &arg_node, path)
}

/// Walk `arg_node` and any descendants for identifiers that are tainted on `path` at
/// the identifier's own source line in `parsed`'s file. Returns true on first hit.
pub(super) fn arg_node_taints_match(
    parsed: &ParsedFile,
    arg_node: &Node<'_>,
    path: &FlowPath,
) -> bool {
    match arg_node.kind() {
        // Literal kinds — definitely not tainted.
        "interpreted_string_literal"
        | "raw_string_literal"
        | "rune_literal"
        | "int_literal"
        | "float_literal"
        | "number"
        | "string"
        | "imaginary_literal"
        | "true"
        | "false"
        | "null"
        | "undefined"
        | "nil" => false,

        // Bare identifier — direct check with file scoping.
        "identifier" => {
            let name = parsed.node_text(arg_node);
            let identifier_line = arg_node.start_position().row + 1;
            path.edges.iter().any(|e| {
                if e.to.file != parsed.path
                    || e.to.line != identifier_line
                    || e.to.var_name() != name
                {
                    return false;
                }
                let source_ranges = js_ts_request_source_access_ranges_for_alias_on_line(
                    parsed,
                    identifier_line,
                    name,
                );
                source_ranges.is_empty() || node_in_ranges(arg_node, &source_ranges)
            })
        }

        // Composite expression — recurse into descendants. Conservative: any
        // tainted identifier within counts.
        _ => {
            let mut cursor = arg_node.walk();
            for child in arg_node.named_children(&mut cursor) {
                if arg_node_taints_match(parsed, &child, path) {
                    return true;
                }
            }
            false
        }
    }
}

pub(super) fn assignment_lhs_identifiers(parsed: &ParsedFile, lhs: &Node<'_>) -> Vec<String> {
    if lhs.kind() == "identifier" {
        return vec![parsed.node_text(lhs).to_string()];
    }
    let mut names = Vec::new();
    let mut cursor = lhs.walk();
    for child in lhs.named_children(&mut cursor) {
        if child.kind() == "identifier" {
            names.push(parsed.node_text(&child).to_string());
        }
    }
    names
}

pub(super) fn assignment_rhs_expressions<'a>(rhs: &Node<'a>) -> Vec<Node<'a>> {
    if rhs.kind() == "call_expression" {
        return vec![*rhs];
    }
    let mut items = Vec::new();
    let mut cursor = rhs.walk();
    for child in rhs.named_children(&mut cursor) {
        items.push(child);
    }
    if items.is_empty() {
        items.push(*rhs);
    }
    items
}

pub(super) fn path_targets_var_at_line(
    parsed: &ParsedFile,
    path: &FlowPath,
    line: usize,
    var_name: &str,
) -> bool {
    path.edges
        .iter()
        .any(|e| e.to.file == parsed.path && e.to.line == line && e.to.var_name() == var_name)
}

pub(super) fn sink_call_has_tainted_arg_in_path(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
    path: &FlowPath,
) -> bool {
    sink_pat
        .tainted_arg_indices
        .iter()
        .any(|&idx| arg_is_tainted_in_path(parsed, call, idx, path))
}

pub(super) fn sink_call_uses_var_in_tainted_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
    var_name: &str,
) -> bool {
    let arguments = match call.child_by_field_name("arguments") {
        Some(n) => n,
        None => return false,
    };
    let mut cursor = arguments.walk();
    for (idx, arg) in arguments.named_children(&mut cursor).enumerate() {
        if sink_pat.tainted_arg_indices.contains(&idx)
            && node_contains_identifier(parsed, &arg, var_name)
        {
            return true;
        }
    }
    false
}

pub(super) fn node_contains_identifier(
    parsed: &ParsedFile,
    node: &Node<'_>,
    var_name: &str,
) -> bool {
    if node.kind() == "identifier" && parsed.node_text(node) == var_name {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if node_contains_identifier(parsed, &child, var_name) {
            return true;
        }
    }
    false
}

/// Returns the structured-sink outcome for `sink_pat` on `line` of `parsed`,
/// using `path` to resolve per-argument taint via `arg_is_tainted_in_path`.
///
/// Outcome rules:
/// - `Match(sink_pat)` — call_path matches, semantic_check (if any) passes,
///   AND at least one arg in `sink_pat.tainted_arg_indices` is tainted on `path`.
/// - `SemanticallyExcluded` — call_path matches but EITHER `semantic_check`
///   rejects OR no arg in `tainted_arg_indices` is tainted on this path.
/// - `NoMatch` — no call expression on `line` matches `sink_pat.call_path`.
///
/// `path == None` is the source==sink no-originating-path fallback (see design
/// note §2.3); in that branch we trust call_path + semantic_check without per-arg
/// precision, preserving today's source==sink behavior for shapes like
/// `c.File(c.Param("f"))`.
///
/// Caller is responsible for confirming `parsed.language == Language::Go`
/// (the function returns `NoMatch` for non-Go files).
pub(crate) fn line_matches_structured_sink(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let mut had_call_path_match = false;
    for call in &calls {
        let call_on_line = if matches!(
            parsed.language,
            Language::Python | Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            node_contains_line(call, line)
        } else {
            call.start_position().row + 1 == line
        };
        if !call_on_line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if !sink_call_path_matches(parsed, call, &actual, sink_pat) {
            continue;
        }
        had_call_path_match = true;
        if !call_passes_sink_semantics(parsed, call, sink_pat) {
            continue;
        }

        // Per-arg taint check — only when a FlowPath is provided. `path == None`
        // is the source==sink no-originating-path fallback (see design note §2.3);
        // in that case we trust the existing call_path + semantic_check gate
        // without per-arg precision, preserving today's source==sink behavior for
        // shapes like `c.File(c.Param("f"))`.
        if let Some(p) = path {
            let any_arg_tainted = sink_pat
                .tainted_arg_indices
                .iter()
                .any(|&idx| arg_is_tainted_in_path(parsed, call, idx, p))
                || js_ts_sink_call_has_inline_framework_source_arg(parsed, call, sink_pat);
            if !any_arg_tainted {
                // call_path + semantic_check passed, but the relevant args
                // aren't tainted on this path. Mark as a structural match-but-
                // not-actually-firing; subsequent iterations may find a
                // different call on this line that DOES have tainted args.
                continue;
            }
        }

        return SinkMatchOutcome::Match(sink_pat);
    }
    if had_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}

pub(super) fn sink_call_path_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
    sink_pat: &'static SinkPattern,
) -> bool {
    if is_js_ts_language(parsed.language) && sink_pat.category == SanitizerCategory::Ssrf {
        return js_ts_ssrf_sink_call_path_matches(parsed, call, actual, sink_pat.call_path);
    }
    if is_js_ts_language(parsed.language)
        && sink_pat.category == SanitizerCategory::Deserialization
        && sink_pat.call_path == "load"
    {
        return js_ts_js_yaml_bare_load_call_matches(parsed, call, actual);
    }
    if call_path_matches(parsed, actual, sink_pat.call_path) {
        return true;
    }
    if parsed.language != Language::Python || sink_pat.category != SanitizerCategory::Ssrf {
        return false;
    }
    if sink_pat.call_path == "urllib3.PoolManager.request" {
        return python_is_urllib3_pool_manager_request_call(parsed, call);
    }
    if sink_pat.call_path == "aiohttp.request" {
        return python_is_aiohttp_request_call(parsed, call);
    }
    let Some(method) = sink_pat.call_path.strip_prefix("aiohttp.ClientSession.") else {
        return false;
    };
    python_is_aiohttp_client_session_method_call(parsed, call, method)
}

pub(super) fn structured_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    match parsed.language {
        Language::Go => go_sink_outcome(parsed, line, path),
        Language::Python => python_sink_outcome(parsed, line, path),
        Language::JavaScript | Language::TypeScript | Language::Tsx => {
            js_ts_sink_outcome(parsed, line, path)
        }
        _ => SinkMatchOutcome::NoMatch,
    }
}

pub(super) fn collect_assignments<'a>(
    node: Node<'a>,
    parsed: &ParsedFile,
    out: &mut Vec<Node<'a>>,
) {
    if parsed.language.is_assignment_node(node.kind()) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_assignments(child, parsed, out);
    }
}

pub(super) fn call_passes_sink_semantics(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
) -> bool {
    if is_js_yaml_load_sink_pattern(parsed, sink_pat) {
        return js_yaml_load_call_uses_unsafe_schema(parsed, call);
    }
    if let Some(check) = sink_pat.semantic_check {
        let cs = CallSite {
            call_node: *call,
            source: parsed.source.as_str(),
        };
        check(&cs)
    } else {
        true
    }
}
