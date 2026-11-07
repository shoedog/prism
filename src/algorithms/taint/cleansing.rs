//! Language-neutral sanitizer, guard and CFG-reachability plumbing for structured sinks.

use crate::algorithms::taint::go::sanitizers::go_path_traversal_cleansed_for_sink;
use crate::algorithms::taint::go::sinks::{
    collect_go_calls, go_call_path_text, GO_CWE22_SINKS, GO_CWE78_SINKS,
};
use crate::algorithms::taint::js_ts::exec_sql::{
    js_ts_exec_file_is_literal_binary, js_ts_sql_call_is_parametrized,
    js_ts_yaml_load_uses_safe_schema,
};
use crate::algorithms::taint::js_ts::sanitizers::{
    js_ts_path_traversal_cleansed_for_sink, js_ts_ssrf_cleansed_for_sink,
};
use crate::algorithms::taint::js_ts::sinks::js_ts_safe_structured_sink_call_ranges;
use crate::algorithms::taint::python::sanitizers::{
    python_safe_structured_sink_call_ranges, python_ssrf_cleansed_for_sink,
    python_xss_cleansed_for_sink,
};
use crate::algorithms::taint::python::sinks::{
    python_sql_call_is_parametrized, python_yaml_load_uses_safe_loader,
};
use crate::algorithms::taint::sink_match::{
    call_passes_sink_semantics, sink_call_has_tainted_arg_in_path, sink_call_path_matches,
};
use crate::ast::tree_helpers::{call_path_text, collect_calls, is_js_ts_language};
use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::FlowPath;
use crate::frameworks::{SanitizerCategory, SinkPattern};
use crate::languages::Language;
use crate::sanitizers::matching::function_body_cleansed_for;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GuardControl {
    RejectBranch,
    AllowBranch,
}

pub(super) fn flow_path_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
) -> bool {
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        return go_path_traversal_cleansed_for_sink(
            parsed,
            cpg,
            Some(path),
            sink_line,
            Some(sink_pat),
            None,
        );
    }
    path.cleansed_for.contains(&sink_pat.category)
}

pub(super) fn flow_path_cleansed_for_sink_call(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    call: &Node<'_>,
) -> bool {
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        return go_path_traversal_cleansed_for_sink(
            parsed,
            cpg,
            Some(path),
            sink_line,
            Some(sink_pat),
            Some(call),
        );
    }
    if parsed.language == Language::Python {
        if sink_pat.category == SanitizerCategory::Sqli
            && python_sql_call_is_parametrized(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Deserialization
            && python_yaml_load_uses_safe_loader(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Xss
            && python_xss_cleansed_for_sink(parsed, path, sink_line, sink_pat, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Ssrf {
            return python_ssrf_cleansed_for_sink(parsed, cpg, path, sink_line, sink_pat, call);
        }
    }
    if is_js_ts_language(parsed.language) {
        if sink_pat.category == SanitizerCategory::Sqli
            && js_ts_sql_call_is_parametrized(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Deserialization
            && js_ts_yaml_load_uses_safe_schema(parsed, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::Ssrf {
            return js_ts_ssrf_cleansed_for_sink(parsed, cpg, path, sink_line, sink_pat, call);
        }
        if sink_pat.category == SanitizerCategory::PathTraversal
            && js_ts_path_traversal_cleansed_for_sink(parsed, cpg, path, sink_line, sink_pat, call)
        {
            return true;
        }
        if sink_pat.category == SanitizerCategory::OsCommand
            && js_ts_exec_file_is_literal_binary(parsed, call)
        {
            return true;
        }
    }
    path.cleansed_for.contains(&sink_pat.category)
}

pub(super) fn source_line_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
) -> bool {
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        return go_path_traversal_cleansed_for_sink(
            parsed,
            cpg,
            None,
            sink_line,
            Some(sink_pat),
            None,
        );
    }
    function_body_cleansed_for(parsed, sink_line, sink_pat.category)
}

pub(super) fn structured_sink_line_cleansed_for_path(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    line: usize,
    sink_pat: &'static SinkPattern,
) -> bool {
    if parsed.language == Language::Python {
        let mut calls = Vec::new();
        collect_calls(parsed, parsed.tree.root_node(), &mut calls);

        let mut matched = false;
        for call in &calls {
            if call.start_position().row + 1 != line {
                continue;
            }
            let actual = match call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if !sink_call_path_matches(parsed, call, &actual, sink_pat)
                || !call_passes_sink_semantics(parsed, call, sink_pat)
            {
                continue;
            }
            if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
                continue;
            }
            matched = true;
            if !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, sink_pat, call) {
                return false;
            }
        }

        return matched;
    }

    if is_js_ts_language(parsed.language) {
        let mut calls = Vec::new();
        collect_calls(parsed, parsed.tree.root_node(), &mut calls);

        let mut matched = false;
        for call in &calls {
            if !node_contains_line(call, line) {
                continue;
            }
            let actual = match call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if !sink_call_path_matches(parsed, call, &actual, sink_pat)
                || !call_passes_sink_semantics(parsed, call, sink_pat)
            {
                continue;
            }
            if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
                continue;
            }
            matched = true;
            if !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, sink_pat, call) {
                return false;
            }
        }

        return matched || path.cleansed_for.contains(&sink_pat.category);
    }

    if parsed.language != Language::Go || sink_pat.category != SanitizerCategory::PathTraversal {
        return path.cleansed_for.contains(&sink_pat.category);
    }

    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);

    let mut matched = false;
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if actual != sink_pat.call_path || !call_passes_sink_semantics(parsed, call, sink_pat) {
            continue;
        }
        if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
            continue;
        }
        matched = true;
        if !flow_path_cleansed_for_sink_call(parsed, cpg, path, line, sink_pat, call) {
            return false;
        }
    }

    matched
}

pub(super) fn collect_if_statements<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "if_statement" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_if_statements(child, out);
    }
}

pub(super) fn first_statement_line(parsed: &ParsedFile, node: &Node<'_>) -> Option<usize> {
    if parsed.language.is_statement_node(node.kind()) {
        return Some(node.start_position().row + 1);
    }
    for child in crate::go_statements::children(*node, parsed.language) {
        if parsed.language.is_statement_node(child.kind()) {
            return Some(child.start_position().row + 1);
        }
        if parsed.language.is_scope_block(child.kind()) {
            if let Some(line) = first_statement_line(parsed, &child) {
                return Some(line);
            }
        }
    }
    None
}

pub(super) fn block_ends_with_return(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    last_statement_node(parsed, node).is_some_and(|n| parsed.language.is_return_node(n.kind()))
}

fn last_statement_node<'a>(parsed: &ParsedFile, node: &Node<'a>) -> Option<Node<'a>> {
    let mut last = None;
    for child in crate::go_statements::children(*node, parsed.language) {
        if parsed.language.is_statement_node(child.kind()) {
            last = Some(child);
        } else if parsed.language.is_scope_block(child.kind()) {
            if let Some(n) = last_statement_node(parsed, &child) {
                last = Some(n);
            }
        }
    }
    last
}

pub(super) fn safe_successor_line(
    cpg: &CodePropertyGraph,
    parsed: &ParsedFile,
    if_node: &Node<'_>,
    reject_entry: usize,
) -> Option<usize> {
    let if_line = if_node.start_position().row + 1;
    let if_idx = cpg.statement_at(&parsed.path, if_line)?;
    cpg.cfg_successors(if_idx)
        .into_iter()
        .map(|idx| cpg.node(idx).line())
        .find(|line| *line != reject_entry)
}

pub(super) fn cfg_line_reaches(
    cpg: &CodePropertyGraph,
    file: &str,
    start_line: usize,
    target_line: usize,
) -> bool {
    start_line == target_line
        || cpg
            .cfg_reachable_lines(file, start_line)
            .contains(&(file.to_string(), target_line))
}

pub(super) fn node_contains_line(node: &Node<'_>, line: usize) -> bool {
    let row = line.saturating_sub(1);
    node.start_position().row <= row && row <= node.end_position().row
}

fn push_cleansed_structured_sink_range(
    ranges: &mut Vec<(usize, usize)>,
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    call: &Node<'_>,
    actual: &str,
    sink_pat: &'static SinkPattern,
    path: &FlowPath,
) -> bool {
    if actual != sink_pat.call_path {
        return false;
    }
    if !call_passes_sink_semantics(parsed, call, sink_pat) {
        return false;
    }
    if parsed.language == Language::Go && sink_pat.category == SanitizerCategory::PathTraversal {
        let call_line = call.start_position().row + 1;
        if sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, path) {
            if !flow_path_cleansed_for_sink_call(parsed, cpg, path, call_line, sink_pat, call) {
                return false;
            }
        } else if path
            .cleansed_for
            .contains(&SanitizerCategory::PathTraversal)
        {
            // Flat substring matching has no per-arg precision. For diff-line
            // artifact paths whose structured sink is SemanticallyExcluded, still
            // suppress only identifiers inside this specific safely-guarded call.
            if !go_path_traversal_cleansed_for_sink(
                parsed,
                cpg,
                None,
                call_line,
                Some(sink_pat),
                Some(call),
            ) {
                return false;
            }
        } else {
            return false;
        }
    } else if !path.cleansed_for.contains(&sink_pat.category) {
        return false;
    }
    ranges.push((call.start_byte(), call.end_byte()));
    true
}

/// Returns byte ranges for structured sink calls on `line` whose own flat
/// identifier matches should be suppressed because this flow is cleansed for the
/// sink's category. Suppression is intentionally scoped to the call expression:
/// unrelated flat sinks that happen to share the same source line still run.
pub(super) fn cleansed_structured_sink_call_ranges(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)> {
    if parsed.language == Language::Python {
        return python_safe_structured_sink_call_ranges(parsed, cpg, line, path);
    }
    if is_js_ts_language(parsed.language) {
        return js_ts_safe_structured_sink_call_ranges(parsed, cpg, line, path);
    }
    if parsed.language != Language::Go {
        return Vec::new();
    }

    let mut calls = Vec::new();
    collect_go_calls(parsed.tree.root_node(), &mut calls);

    let mut ranges = Vec::new();
    for call in &calls {
        if call.start_position().row + 1 != line {
            continue;
        }
        let actual = match go_call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };

        let mut pushed = false;
        for pat in GO_CWE78_SINKS {
            if push_cleansed_structured_sink_range(
                &mut ranges,
                parsed,
                cpg,
                call,
                &actual,
                pat,
                path,
            ) {
                pushed = true;
                break;
            }
        }
        if pushed {
            continue;
        }
        for pat in GO_CWE22_SINKS {
            if push_cleansed_structured_sink_range(
                &mut ranges,
                parsed,
                cpg,
                call,
                &actual,
                pat,
                path,
            ) {
                pushed = true;
                break;
            }
        }
        if pushed {
            continue;
        }
        if let Some(spec) = parsed.framework() {
            for pat in spec.sinks {
                if push_cleansed_structured_sink_range(
                    &mut ranges,
                    parsed,
                    cpg,
                    call,
                    &actual,
                    pat,
                    path,
                ) {
                    break;
                }
            }
        }
    }
    ranges
}

pub(super) fn node_in_ranges(node: &Node<'_>, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| *start <= node.start_byte() && node.end_byte() <= *end)
}

/// Apply cleansers to a `FlowPath`, mutating `cleansed_for` in place per spec §3.6.
///
/// For each active sanitizer recognizer category, calls
/// `function_body_cleansed_for` on the flow's source line. If a recognizer in
/// that category fires (with `semantic_check` and `paired_check` satisfied per
/// §3.4 / §3.8), the category is inserted into `path.cleansed_for`.
///
/// The walk is intraprocedural (cleanser must live in same function as source);
/// cross-function cleansing is a Phase 1.5+ concern.
pub(super) fn apply_cleansers(
    path: &mut crate::data_flow::FlowPath,
    files: &BTreeMap<String, ParsedFile>,
) {
    if path.edges.is_empty() {
        return;
    }
    // The source location is the `from` of the first edge (FlowPaths are
    // single-source fans built by taint_forward_cfg).
    let src = &path.edges[0].from;
    let parsed = match files.get(&src.file) {
        Some(p) => p,
        None => return,
    };
    if !crate::sanitizers::sanitizer_supported(parsed.language) {
        return;
    }

    // Iterate distinct recognizer categories so each is checked at most once.
    let categories: BTreeSet<SanitizerCategory> = crate::sanitizers::active_recognizers()
        .map(|r| r.category)
        .collect();
    for category in categories {
        if path.cleansed_for.contains(&category) {
            continue;
        }
        if function_body_cleansed_for(parsed, src.line, category) {
            path.cleansed_for.insert(category);
        }
    }
}
