//! JS/TS sink outcomes and cleansed structured-sink ranges.

use crate::algorithms::taint::{SinkMatchOutcome, arg_node_taints_match, call_passes_sink_semantics, flow_path_cleansed_for_sink_call, line_matches_structured_sink, node_contains_line, sink_call_has_tainted_arg_in_path, sink_call_path_matches};
use crate::algorithms::taint::js_ts::bindings::js_ts_identifier_binds_imported_member_at_call;
use crate::algorithms::taint::js_ts::exec_sql::{js_ts_exec_file_is_literal_binary, js_ts_prisma_tagged_template_is_safe, js_ts_sql_call_is_parametrized, js_ts_yaml_load_uses_safe_schema};
use crate::algorithms::taint::js_ts::framework_params::{js_ts_framework_source_params, js_ts_function_params};
use crate::algorithms::taint::js_ts::sink_tables::{JS_CWE22_SINKS, JS_CWE502_SINKS, JS_CWE78_SINKS, JS_CWE79_SINKS, JS_CWE89_SINKS, JS_CWE918_SINKS};
use crate::algorithms::taint::js_ts::source_access::node_contains_js_ts_source_access;
use crate::ast::ParsedFile;
use crate::ast::tree_helpers::{call_path_text, collect_calls, is_js_ts_language};
use crate::cpg::CodePropertyGraph;
use crate::data_flow::FlowPath;
use crate::frameworks::{SanitizerCategory, SinkPattern};
use crate::frameworks::js_ts::call_classify::{js_ts_assignment_target, js_ts_assignment_value, js_ts_framework_receiver_names};
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn js_ts_sink_call_has_inline_framework_source_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    sink_pat: &'static SinkPattern,
) -> bool {
    if !is_js_ts_language(parsed.language) {
        return false;
    }
    let Some(framework) = parsed.framework().map(|spec| spec.name) else {
        return false;
    };
    if !matches!(framework, "nestjs" | "fastify" | "express" | "koa") {
        return false;
    }
    let Some(func) = parsed.enclosing_function(call.start_position().row + 1) else {
        return false;
    };
    let params = js_ts_function_params(parsed, &func);
    if params.is_empty() {
        return false;
    }
    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);
    let source_params =
        js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
    if source_params.is_empty() {
        return false;
    }
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    for (idx, arg) in arguments.named_children(&mut cursor).enumerate() {
        if sink_pat.tainted_arg_indices.contains(&idx)
            && node_contains_js_ts_source_access(parsed, arg, framework, &source_params)
        {
            return true;
        }
    }
    false
}

pub(in crate::algorithms::taint) fn js_ts_inline_framework_source_yaml_sink_lines(parsed: &ParsedFile) -> BTreeSet<usize> {
    let mut lines = BTreeSet::new();
    if !is_js_ts_language(parsed.language) {
        return lines;
    }
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    for call in &calls {
        let actual = match call_path_text(parsed, call) {
            Some(actual) => actual,
            None => continue,
        };
        for pat in JS_CWE502_SINKS {
            if !matches!(pat.call_path, "yaml.load" | "load") {
                continue;
            }
            if !sink_call_path_matches(parsed, call, &actual, pat)
                || !call_passes_sink_semantics(parsed, call, pat)
            {
                continue;
            }
            if js_ts_sink_call_has_inline_framework_source_arg(parsed, call, pat) {
                lines.insert(call.start_position().row + 1);
                break;
            }
        }
    }
    lines
}

pub(in crate::algorithms::taint) fn js_ts_js_yaml_bare_load_call_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    actual: &str,
) -> bool {
    if actual.contains('.') {
        return false;
    }
    js_ts_identifier_binds_imported_member_at_call(parsed, call, actual, "js-yaml", "load")
}

pub(in crate::algorithms::taint) fn js_ts_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    if let Some(outcome) = js_ts_dangerously_set_inner_html_outcome(parsed, line, path) {
        return outcome;
    }

    let mut any_call_path_match = false;
    for pat in JS_CWE79_SINKS
        .iter()
        .chain(JS_CWE89_SINKS.iter())
        .chain(JS_CWE918_SINKS.iter())
        .chain(JS_CWE502_SINKS.iter())
        .chain(JS_CWE78_SINKS.iter())
        .chain(JS_CWE22_SINKS.iter())
    {
        if pat.call_path == "dangerouslySetInnerHTML" {
            continue;
        }
        match line_matches_structured_sink(parsed, line, pat, path) {
            SinkMatchOutcome::Match(p) => return SinkMatchOutcome::Match(p),
            SinkMatchOutcome::SemanticallyExcluded => any_call_path_match = true,
            SinkMatchOutcome::NoMatch => {}
        }
    }
    if any_call_path_match {
        SinkMatchOutcome::SemanticallyExcluded
    } else {
        SinkMatchOutcome::NoMatch
    }
}

fn js_ts_dangerously_set_inner_html_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> Option<SinkMatchOutcome> {
    let pat = JS_CWE79_SINKS
        .iter()
        .find(|p| p.call_path == "dangerouslySetInnerHTML")?;
    let mut attrs = Vec::new();
    collect_nodes_of_kind(parsed.tree.root_node(), "jsx_attribute", &mut attrs);
    let mut had_attr = false;
    for attr in &attrs {
        if !node_contains_line(attr, line) {
            continue;
        }
        let text = parsed.node_text(attr);
        if !text.contains("dangerouslySetInnerHTML") || !text.contains("__html") {
            continue;
        }
        had_attr = true;
        if let Some(p) = path {
            if arg_node_taints_match(parsed, attr, p) {
                return Some(SinkMatchOutcome::Match(pat));
            }
        } else {
            return Some(SinkMatchOutcome::Match(pat));
        }
    }
    if had_attr {
        Some(SinkMatchOutcome::SemanticallyExcluded)
    } else {
        None
    }
}

pub(super) fn collect_nodes_of_kind<'a>(node: Node<'a>, kind: &str, out: &mut Vec<Node<'a>>) {
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_nodes_of_kind(child, kind, out);
    }
}

pub(in crate::algorithms::taint) fn is_js_yaml_load_sink_pattern(parsed: &ParsedFile, sink_pat: &'static SinkPattern) -> bool {
    is_js_ts_language(parsed.language)
        && sink_pat.category == SanitizerCategory::Deserialization
        && matches!(sink_pat.call_path, "yaml.load" | "load")
}

pub(in crate::algorithms::taint) fn js_ts_safe_structured_sink_call_ranges(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    line: usize,
    path: &FlowPath,
) -> Vec<(usize, usize)> {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let mut ranges = Vec::new();
    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = match call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        for pat in JS_CWE79_SINKS
            .iter()
            .chain(JS_CWE89_SINKS.iter())
            .chain(JS_CWE918_SINKS.iter())
            .chain(JS_CWE502_SINKS.iter())
            .chain(JS_CWE78_SINKS.iter())
            .chain(JS_CWE22_SINKS.iter())
        {
            if pat.call_path == "dangerouslySetInnerHTML" {
                continue;
            }
            if !sink_call_path_matches(parsed, call, &actual, pat) {
                continue;
            }
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
            }
            let tainted_arg = sink_call_has_tainted_arg_in_path(parsed, call, pat, path);
            if !tainted_arg {
                if let Some(function) = call.child_by_field_name("function") {
                    ranges.push((function.start_byte(), function.end_byte()));
                } else {
                    ranges.push((call.start_byte(), call.end_byte()));
                }
                break;
            }
            if flow_path_cleansed_for_sink_call(parsed, cpg, path, line, pat, call) {
                ranges.push((call.start_byte(), call.end_byte()));
                break;
            }
        }

        if js_ts_exec_file_is_literal_binary(parsed, call)
            || js_ts_yaml_load_uses_safe_schema(parsed, call)
            || js_ts_sql_call_is_parametrized(parsed, call)
            || js_ts_prisma_tagged_template_is_safe(parsed, call)
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
    }

    let mut attrs = Vec::new();
    collect_nodes_of_kind(parsed.tree.root_node(), "jsx_attribute", &mut attrs);
    for attr in &attrs {
        if node_contains_line(attr, line)
            && parsed.node_text(attr).contains("dangerouslySetInnerHTML")
            && path.cleansed_for.contains(&SanitizerCategory::Xss)
        {
            ranges.push((attr.start_byte(), attr.end_byte()));
        }
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn node_contains_node(outer: &Node<'_>, inner: &Node<'_>) -> bool {
    outer.start_byte() <= inner.start_byte() && inner.end_byte() <= outer.end_byte()
}

pub(super) fn node_contains_range(node: &Node<'_>, start_byte: usize, end_byte: usize) -> bool {
    node.start_byte() <= start_byte && end_byte <= node.end_byte()
}

fn node_contains_any_range(node: &Node<'_>, ranges: &[(usize, usize)]) -> bool {
    ranges
        .iter()
        .any(|(start, end)| node_contains_range(node, *start, *end))
}

fn call_arguments_node<'a>(call: &Node<'a>) -> Option<Node<'a>> {
    call.child_by_field_name("arguments").or_else(|| {
        let mut cursor = call.walk();
        let found = call
            .named_children(&mut cursor)
            .find(|child| child.kind() == "arguments");
        found
    })
}

fn js_ts_call_has_tainted_or_source_arg(
    parsed: &ParsedFile,
    call: &Node<'_>,
    path: &FlowPath,
    source_ranges: &[(usize, usize)],
) -> bool {
    let Some(arguments) = call_arguments_node(call) else {
        return false;
    };
    let mut cursor = arguments.walk();
    let has_tainted_arg = arguments.named_children(&mut cursor).any(|arg| {
        arg_node_taints_match(parsed, &arg, path) || node_contains_any_range(&arg, source_ranges)
    });
    has_tainted_arg
}

fn js_ts_enclosing_call_for_function_identifier<'a>(
    parsed: &ParsedFile,
    id: &Node<'a>,
) -> Option<Node<'a>> {
    let mut current = id.parent();
    while let Some(parent) = current {
        if parsed.language.is_call_node(parent.kind()) {
            return parent
                .child_by_field_name("function")
                .filter(|function| node_contains_node(function, id))
                .map(|_| parent);
        }
        current = parent.parent();
    }
    None
}

fn js_ts_enclosing_assignment_for_lhs_identifier<'a>(
    parsed: &ParsedFile,
    id: &Node<'a>,
) -> Option<Node<'a>> {
    let mut current = Some(*id);
    while let Some(node) = current {
        if node.kind() == "variable_declarator" || parsed.language.is_assignment_node(node.kind()) {
            return js_ts_assignment_target(parsed, &node)
                .filter(|lhs| node_contains_node(lhs, id))
                .map(|_| node);
        }
        current = node.parent();
    }
    None
}

fn js_ts_assignment_has_tainted_or_source_rhs(
    parsed: &ParsedFile,
    assignment: &Node<'_>,
    path: &FlowPath,
    source_ranges: &[(usize, usize)],
) -> bool {
    js_ts_assignment_value(parsed, assignment).is_some_and(|rhs| {
        arg_node_taints_match(parsed, &rhs, path) || node_contains_any_range(&rhs, source_ranges)
    })
}

pub(in crate::algorithms::taint) fn js_ts_flat_sink_identifier_has_tainted_request_source(
    parsed: &ParsedFile,
    id: &Node<'_>,
    path: &FlowPath,
    source_ranges: &[(usize, usize)],
) -> bool {
    js_ts_enclosing_call_for_function_identifier(parsed, id).is_some_and(|call| {
        js_ts_call_has_tainted_or_source_arg(parsed, &call, path, source_ranges)
    }) || js_ts_enclosing_assignment_for_lhs_identifier(parsed, id).is_some_and(|assignment| {
        js_ts_assignment_has_tainted_or_source_rhs(parsed, &assignment, path, source_ranges)
    })
}
