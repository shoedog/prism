//! JS/TS allowlist and guard checks that control a sink.

use crate::algorithms::taint::js_ts::bindings::{
    js_ts_assignment_visible_before_context, js_ts_enclosing_function_id,
};
use crate::algorithms::taint::js_ts::sanitizers::call_or_constructor_path_text;
use crate::algorithms::taint::{
    assignment_lhs_identifiers, block_ends_with_return, cfg_line_reaches, first_statement_line,
    node_contains_line, safe_successor_line, GuardControl, UrlSanitizerBinding,
};
use crate::ast::tree_helpers::{
    call_arg_node, call_path_matches, call_path_text, collect_calls, unwrap_parenthesized,
};
use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::FlowPath;
use crate::frameworks::js_ts::call_classify::{
    collect_js_ts_assignment_like_nodes, js_ts_assignment_target_and_value,
    js_ts_literal_string_value,
};
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) fn js_ts_allowlist_receiver_is_trusted(
    parsed: &ParsedFile,
    receiver: &str,
    path: &FlowPath,
    guard: &Node<'_>,
) -> bool {
    if !receiver.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return false;
    }
    let lower = receiver.to_ascii_lowercase();
    if [
        "block",
        "deny",
        "forbid",
        "reject",
        "ban",
        "blacklist",
        "disallow",
        "exclude",
        "invalid",
        "unsafe",
        "untrusted",
        "not_allow",
        "notallow",
        "not_safe",
        "notsafe",
    ]
    .iter()
    .any(|word| lower.contains(word))
    {
        return false;
    }
    if !["allow", "whitelist", "trusted", "safe"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return false;
    }
    if path
        .edges
        .iter()
        .any(|edge| edge.to.file == parsed.path && edge.to.var_name() == receiver)
    {
        return false;
    }
    js_ts_identifier_bound_to_literal_collection(parsed, receiver, guard)
}

fn js_ts_identifier_bound_to_literal_collection(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
) -> bool {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut saw_literal_collection = false;
    let mut saw_module_lifetime_binding = false;
    for assignment in assignments {
        if !js_ts_assignment_visible_before_context(parsed, context, &assignment) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == var_name)
        {
            continue;
        }
        if js_ts_node_is_literal_string_collection(parsed, &rhs) {
            saw_literal_collection = true;
            if js_ts_enclosing_function_id(parsed, &assignment).is_none() {
                saw_module_lifetime_binding = true;
            }
        } else {
            return false;
        }
    }
    saw_literal_collection
        && !js_ts_collection_has_untrusted_update(
            parsed,
            var_name,
            context,
            saw_module_lifetime_binding,
        )
}

fn js_ts_node_is_literal_string_collection(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    let node = unwrap_parenthesized(*node);
    if node.kind() == "array" {
        let mut cursor = node.walk();
        return node
            .named_children(&mut cursor)
            .all(|child| js_ts_literal_string_value(parsed, &child).is_some());
    }
    let Some(actual) = call_or_constructor_path_text(parsed, &node) else {
        return false;
    };
    if !call_path_matches(parsed, &actual, "Set") {
        return false;
    }
    match call_arg_node(&node, 0) {
        Some(arg) => js_ts_node_is_literal_string_collection(parsed, &arg),
        None => true,
    }
}

fn js_ts_collection_has_untrusted_update(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
    include_module_init_after_context: bool,
) -> bool {
    let receiver_names = js_ts_collection_aliases_visible_at_context(
        parsed,
        var_name,
        context,
        include_module_init_after_context,
    );
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    for call in calls {
        if !js_ts_effect_visible_at_context(
            parsed,
            context,
            &call,
            include_module_init_after_context,
        ) {
            continue;
        }
        let Some(actual) = call_path_text(parsed, &call) else {
            continue;
        };
        if actual == "Object.assign" {
            let Some(target) = call_arg_node(&call, 0).map(unwrap_parenthesized) else {
                continue;
            };
            if target.kind() != "identifier" || !receiver_names.contains(parsed.node_text(&target))
            {
                continue;
            }
            let mut source_idx = 1;
            while let Some(source_arg) = call_arg_node(&call, source_idx) {
                if !js_ts_node_is_literal_string_collection(parsed, &source_arg) {
                    return true;
                }
                source_idx += 1;
            }
            continue;
        }
        let Some((receiver, method)) = actual.rsplit_once('.') else {
            continue;
        };
        if !receiver_names.contains(receiver)
            || !matches!(method, "add" | "push" | "unshift" | "splice" | "set")
        {
            continue;
        }
        if !js_ts_call_args_are_literal_strings(parsed, &call) {
            return true;
        }
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);
    for assignment in assignments {
        if !js_ts_effect_visible_at_context(
            parsed,
            context,
            &assignment,
            include_module_init_after_context,
        ) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        let lhs_text = parsed.node_text(&lhs);
        if !receiver_names.iter().any(|name| {
            lhs_text.starts_with(&format!("{name}[")) || lhs_text.starts_with(&format!("{name}."))
        }) {
            continue;
        }
        if js_ts_literal_string_value(parsed, &rhs).is_none() {
            return true;
        }
    }

    false
}

pub(super) fn js_ts_collection_aliases_visible_at_context(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
    include_module_init_after_context: bool,
) -> BTreeSet<String> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut aliases = BTreeSet::new();
    aliases.insert(var_name.to_string());

    let mut changed = true;
    while changed {
        changed = false;
        for assignment in &assignments {
            if !js_ts_effect_visible_at_context(
                parsed,
                context,
                assignment,
                include_module_init_after_context,
            ) {
                continue;
            }
            let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, assignment) else {
                continue;
            };
            let rhs = unwrap_parenthesized(rhs);
            if rhs.kind() != "identifier" || !aliases.contains(parsed.node_text(&rhs)) {
                continue;
            }
            for lhs_name in assignment_lhs_identifiers(parsed, &lhs) {
                if lhs_name != "_" && aliases.insert(lhs_name) {
                    changed = true;
                }
            }
        }
    }

    aliases
}

pub(super) fn js_ts_effect_visible_at_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    effect: &Node<'_>,
    include_module_init_after_context: bool,
) -> bool {
    if effect.start_byte() < context.start_byte() {
        return js_ts_effect_scope_reaches_context(parsed, context, effect);
    }

    include_module_init_after_context
        && js_ts_enclosing_function_id(parsed, effect).is_none()
        && js_ts_enclosing_function_id(parsed, context).is_some()
}

fn js_ts_effect_scope_reaches_context(
    parsed: &ParsedFile,
    context: &Node<'_>,
    effect: &Node<'_>,
) -> bool {
    let context_func_id = js_ts_enclosing_function_id(parsed, context);
    let effect_func_id = js_ts_enclosing_function_id(parsed, effect);

    match (effect_func_id, context_func_id) {
        (Some(effect_func_id), Some(context_func_id)) => effect_func_id == context_func_id,
        (None, _) => true,
        _ => false,
    }
}

fn js_ts_call_args_are_literal_strings(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let Some(args) = call.child_by_field_name("arguments") else {
        return true;
    };
    let mut cursor = args.walk();
    let all_literal = args
        .named_children(&mut cursor)
        .all(|arg| js_ts_literal_string_value(parsed, &arg).is_some());
    all_literal
}

pub(super) fn js_ts_node_is_hostname_for_url_binding(
    parsed: &ParsedFile,
    node: &Node<'_>,
    binding: &UrlSanitizerBinding,
) -> bool {
    let node = unwrap_parenthesized(*node);
    parsed.node_text(&node).trim() == format!("{}.hostname", binding.result_var)
}

pub(super) fn js_ts_path_prefix_arg_is_trusted(
    parsed: &ParsedFile,
    node: &Node<'_>,
    path: &FlowPath,
    guard: &Node<'_>,
) -> bool {
    let node = unwrap_parenthesized(*node);
    if let Some(prefix) = js_ts_literal_string_value(parsed, &node) {
        return js_ts_path_prefix_value_has_boundary(&prefix);
    }
    if node.kind() != "identifier" {
        return false;
    }
    let name = parsed.node_text(&node);
    if path
        .edges
        .iter()
        .any(|edge| edge.to.file == parsed.path && edge.to.var_name() == name)
    {
        return false;
    }
    js_ts_identifier_literal_string_before(parsed, name, guard)
        .is_some_and(|prefix| js_ts_path_prefix_value_has_boundary(&prefix))
}

fn js_ts_path_prefix_value_has_boundary(prefix: &str) -> bool {
    let prefix = prefix.trim();
    if matches!(prefix, "/" | "\\") {
        return false;
    }
    let bytes = prefix.as_bytes();
    if bytes.len() == 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
    {
        return false;
    }
    prefix.ends_with('/') || prefix.ends_with('\\')
}

fn js_ts_identifier_literal_string_before(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
) -> Option<String> {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut literal = None;
    for assignment in assignments {
        if !js_ts_assignment_visible_before_context(parsed, context, &assignment) {
            continue;
        }
        let Some((lhs, rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        if !assignment_lhs_identifiers(parsed, &lhs)
            .iter()
            .any(|name| name == var_name)
        {
            continue;
        }
        literal = Some(js_ts_literal_string_value(parsed, &rhs)?);
    }
    literal
}

pub(super) fn js_ts_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    guard: &Node<'_>,
    control: GuardControl,
    sink_line: usize,
) -> bool {
    let consequence = match guard.child_by_field_name("consequence") {
        Some(n) => n,
        None => return false,
    };
    let consequence_entry = match first_statement_line(parsed, &consequence) {
        Some(line) => line,
        None => return false,
    };

    match control {
        GuardControl::RejectBranch => {
            if !block_ends_with_return(parsed, &consequence) {
                return false;
            }
            let safe_entry = match safe_successor_line(cpg, parsed, guard, consequence_entry) {
                Some(line) => line,
                None => return false,
            };
            cfg_line_reaches(cpg, &parsed.path, safe_entry, sink_line)
                && !cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
        }
        GuardControl::AllowBranch => {
            node_contains_line(&consequence, sink_line)
                && cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
        }
    }
}
