//! JS/TS URL and path sanitizer bindings.

use crate::algorithms::taint::{GuardControl, JsTsPathSanitizerBinding, UrlSanitizerBinding, assignment_lhs_identifiers, collect_if_statements, path_targets_var_at_line, sink_call_uses_var_in_tainted_arg};
use crate::algorithms::taint::js_ts::guards::{js_ts_allowlist_receiver_is_trusted, js_ts_guard_safely_controls_sink, js_ts_node_is_hostname_for_url_binding, js_ts_path_prefix_arg_is_trusted};
use crate::ast::ParsedFile;
use crate::ast::tree_helpers::{call_arg_node, call_path_matches, call_path_text, unwrap_parenthesized};
use crate::cpg::CodePropertyGraph;
use crate::data_flow::FlowPath;
use crate::frameworks::SinkPattern;
use crate::frameworks::js_ts::call_classify::{collect_js_ts_assignment_like_nodes, js_ts_assignment_target_and_value};
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn js_ts_ssrf_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if !cpg.has_cfg_edges() {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_js_ts_url_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.url_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.url_var) {
            continue;
        }
        if js_ts_url_guard_safely_controls_sink(parsed, cpg, path, &func_node, &binding, sink_line)
        {
            return true;
        }
    }
    false
}

pub(in crate::algorithms::taint) fn js_ts_path_traversal_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if !cpg.has_cfg_edges() {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_js_ts_path_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.result_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.result_var) {
            continue;
        }
        if js_ts_path_guard_safely_controls_sink(parsed, cpg, path, &func_node, &binding, sink_line)
        {
            return true;
        }
    }
    false
}

pub(super) fn call_or_constructor_path_text(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    if let Some(path) = call_path_text(parsed, node) {
        return Some(path);
    }
    let constructor = node
        .child_by_field_name("constructor")
        .or_else(|| node.child_by_field_name("function"))?;
    Some(parsed.node_text(&constructor).to_string())
}

fn collect_js_ts_url_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<UrlSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let (lhs, rhs) = match js_ts_assignment_target_and_value(parsed, &assignment) {
            Some(parts) => parts,
            None => continue,
        };
        let actual = match call_or_constructor_path_text(parsed, &rhs) {
            Some(s) => s,
            None => continue,
        };
        if !call_path_matches(parsed, &actual, "URL")
            && !call_path_matches(parsed, &actual, "URL.parse")
            && !call_path_matches(parsed, &actual, "url.parse")
        {
            continue;
        }
        let result_var = match assignment_lhs_identifiers(parsed, &lhs).first() {
            Some(name) => name.clone(),
            None => continue,
        };
        let url_arg = match call_arg_node(&rhs, 0) {
            Some(n) => unwrap_parenthesized(n),
            None => continue,
        };
        if url_arg.kind() != "identifier" {
            continue;
        }
        bindings.push(UrlSanitizerBinding {
            url_var: parsed.node_text(&url_arg).to_string(),
            result_var,
            call_line: rhs.start_position().row + 1,
        });
    }
    bindings
}

fn collect_js_ts_path_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<JsTsPathSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let (lhs, rhs) = match js_ts_assignment_target_and_value(parsed, &assignment) {
            Some(parts) => parts,
            None => continue,
        };
        let actual = match call_or_constructor_path_text(parsed, &rhs) {
            Some(s) => s,
            None => continue,
        };
        if !js_ts_path_sanitizer_call_path_matches(parsed, &actual) {
            continue;
        }
        let result_var = match assignment_lhs_identifiers(parsed, &lhs).first() {
            Some(name) if name != "_" => name.clone(),
            _ => continue,
        };
        bindings.push(JsTsPathSanitizerBinding {
            result_var,
            call_line: rhs.start_position().row + 1,
        });
    }
    bindings
}

fn js_ts_path_sanitizer_call_path_matches(parsed: &ParsedFile, actual: &str) -> bool {
    [
        "path.resolve",
        "path.normalize",
        "path.relative",
        "fs.realpathSync",
        "fs.promises.realpath",
        "realpath",
        "realpathSync",
    ]
    .iter()
    .any(|expected| call_path_matches(parsed, actual, expected))
}

fn js_ts_url_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    func_node: &Node<'_>,
    binding: &UrlSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_js_ts_url_guard(parsed, &condition, binding, path) {
            Some(c) => c,
            None => continue,
        };
        if js_ts_guard_safely_controls_sink(parsed, cpg, &guard, control, sink_line) {
            return true;
        }
    }
    false
}

fn js_ts_path_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: &FlowPath,
    func_node: &Node<'_>,
    binding: &JsTsPathSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_js_ts_path_guard(parsed, &condition, binding, path) {
            Some(c) => c,
            None => continue,
        };
        if js_ts_guard_safely_controls_sink(parsed, cpg, &guard, control, sink_line) {
            return true;
        }
    }
    false
}

fn classify_js_ts_url_guard(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &UrlSanitizerBinding,
    path: &FlowPath,
) -> Option<GuardControl> {
    let (call, negated) = js_ts_single_guard_call(parsed, condition)?;
    let actual = call_path_text(parsed, &call)?;
    let (receiver, method) = actual.rsplit_once('.')?;
    if !matches!(method, "includes" | "has")
        || !js_ts_allowlist_receiver_is_trusted(parsed, receiver, path, condition)
        || !call_arg_node(&call, 0)
            .is_some_and(|arg| js_ts_node_is_hostname_for_url_binding(parsed, &arg, binding))
    {
        return None;
    }
    if negated {
        Some(GuardControl::RejectBranch)
    } else {
        Some(GuardControl::AllowBranch)
    }
}

fn classify_js_ts_path_guard(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &JsTsPathSanitizerBinding,
    path: &FlowPath,
) -> Option<GuardControl> {
    let (call, negated) = js_ts_single_guard_call(parsed, condition)?;
    let actual = call_path_text(parsed, &call)?;
    let (receiver, method) = actual.rsplit_once('.')?;
    if method != "startsWith"
        || receiver != binding.result_var
        || !call_arg_node(&call, 0)
            .is_some_and(|arg| js_ts_path_prefix_arg_is_trusted(parsed, &arg, path, condition))
    {
        return None;
    }
    if negated {
        Some(GuardControl::RejectBranch)
    } else {
        Some(GuardControl::AllowBranch)
    }
}

fn js_ts_single_guard_call<'a>(
    parsed: &ParsedFile,
    condition: &Node<'a>,
) -> Option<(Node<'a>, bool)> {
    let condition = unwrap_parenthesized(*condition);
    let condition_text = parsed.node_text(&condition);
    if condition_text.contains("&&") || condition_text.contains("||") {
        return None;
    }
    if condition.kind() == "unary_expression" && condition_text.trim_start().starts_with('!') {
        let child = unwrap_parenthesized(condition.named_child(0)?);
        return (child.kind() == "call_expression").then_some((child, true));
    }
    (condition.kind() == "call_expression").then_some((condition, false))
}
