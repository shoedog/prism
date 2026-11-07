//! JS/TS child_process shell options and SQL parametrisation checks.

use crate::algorithms::taint::{assignment_lhs_identifiers, call_literal_arg, python_sql_literal_has_named_placeholder};
use crate::algorithms::taint::js_ts::bindings::{js_ts_assignment_visible_before_context, js_ts_enclosing_function_id};
use crate::algorithms::taint::js_ts::guards::{js_ts_collection_aliases_visible_at_context, js_ts_effect_visible_at_context};
use crate::algorithms::taint::js_ts::sink_tables::{js_find_top_level_colon, js_split_top_level_commas, js_yaml_load_call_uses_unsafe_schema};
use crate::algorithms::taint::js_ts::sinks::js_ts_js_yaml_bare_load_call_matches;
use crate::ast::ParsedFile;
use crate::ast::tree_helpers::{call_arg_node, call_path_matches, call_path_text, collect_calls, unwrap_parenthesized};
use crate::frameworks::js_ts::call_classify::{collect_js_ts_assignment_like_nodes, js_ts_assignment_target_and_value, js_ts_literal_string_value};
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn js_ts_exec_file_is_literal_binary(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "execFile")
        && !call_path_matches(parsed, &actual, "execFileSync")
    {
        return false;
    }
    let Some(binary) = call_literal_arg(parsed, call, 0) else {
        return false;
    };
    match js_ts_literal_binary_kind(&binary) {
        JsTsLiteralBinaryKind::Shell => return false,
        JsTsLiteralBinaryKind::Interpreter => {
            if !js_ts_exec_file_interpreter_argv_is_inspectably_safe(parsed, call) {
                return false;
            }
        }
        JsTsLiteralBinaryKind::Other => {}
    }
    !js_ts_exec_file_shell_option_is_unsafe(parsed, call)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JsTsLiteralBinaryKind {
    Shell,
    Interpreter,
    Other,
}

fn js_ts_literal_binary_kind(binary: &str) -> JsTsLiteralBinaryKind {
    let basename = binary
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(binary)
        .to_ascii_lowercase();
    if matches!(
        basename.as_str(),
        "sh" | "bash"
            | "dash"
            | "zsh"
            | "ksh"
            | "csh"
            | "fish"
            | "cmd"
            | "cmd.exe"
            | "pwsh"
            | "powershell"
            | "powershell.exe"
    ) {
        return JsTsLiteralBinaryKind::Shell;
    }
    if matches!(
        basename.as_str(),
        "node" | "node.exe" | "python" | "python3" | "python.exe" | "perl" | "ruby" | "php"
    ) {
        return JsTsLiteralBinaryKind::Interpreter;
    }
    JsTsLiteralBinaryKind::Other
}

fn js_ts_exec_file_interpreter_argv_is_inspectably_safe(
    parsed: &ParsedFile,
    call: &Node<'_>,
) -> bool {
    let Some(argv) = call_arg_node(call, 1) else {
        return true;
    };
    let argv = unwrap_parenthesized(argv);
    if argv.kind() != "array" {
        return false;
    }
    let mut cursor = argv.walk();
    for arg in argv.named_children(&mut cursor) {
        let Some(value) = js_ts_literal_string_value(parsed, &arg) else {
            return false;
        };
        if js_ts_interpreter_eval_flag(&value) {
            return false;
        }
    }
    true
}

fn js_ts_exec_file_shell_option_is_unsafe(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    (1..=3).any(|arg_idx| match call_arg_node(call, arg_idx) {
        Some(arg) if arg_idx == 1 && js_ts_exec_file_arg_is_argv_or_callback(&arg) => false,
        Some(arg) if js_ts_exec_file_arg_is_callback(&arg) => false,
        Some(arg) => js_ts_node_has_unsafe_shell_option(parsed, &arg, call),
        None => false,
    })
}

fn js_ts_exec_file_arg_is_argv_or_callback(arg: &Node<'_>) -> bool {
    let arg = unwrap_parenthesized(*arg);
    arg.kind() == "array" || js_ts_exec_file_arg_is_callback(&arg)
}

fn js_ts_exec_file_arg_is_callback(arg: &Node<'_>) -> bool {
    let arg = unwrap_parenthesized(*arg);
    matches!(
        arg.kind(),
        "function" | "function_expression" | "arrow_function"
    )
}

fn js_ts_node_has_unsafe_shell_option(
    parsed: &ParsedFile,
    node: &Node<'_>,
    context: &Node<'_>,
) -> bool {
    let node = unwrap_parenthesized(*node);
    let text = parsed.node_text(&node);
    if text.trim_start().starts_with('{') && js_ts_object_text_has_unsafe_shell_option(text) {
        return true;
    }
    if node.kind() != "identifier" {
        return true;
    }
    js_ts_identifier_bound_to_unsafe_shell_options(parsed, parsed.node_text(&node), context)
}

fn js_ts_identifier_bound_to_unsafe_shell_options(
    parsed: &ParsedFile,
    var_name: &str,
    context: &Node<'_>,
) -> bool {
    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(parsed.tree.root_node(), parsed, &mut assignments);

    let mut saw_inspectable_safe_options = false;
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
        let rhs_text = parsed.node_text(&rhs);
        if !rhs_text.trim_start().starts_with('{') {
            return true;
        }
        if js_ts_object_text_has_unsafe_shell_option(rhs_text) {
            return true;
        }
        saw_inspectable_safe_options = true;
        if js_ts_enclosing_function_id(parsed, &assignment).is_none() {
            saw_module_lifetime_binding = true;
        }
    }
    !saw_inspectable_safe_options
        || js_ts_shell_options_have_unsafe_mutation(
            parsed,
            var_name,
            context,
            saw_module_lifetime_binding,
        )
}

fn js_ts_shell_options_have_unsafe_mutation(
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
        let Some((lhs, _rhs)) = js_ts_assignment_target_and_value(parsed, &assignment) else {
            continue;
        };
        let lhs_text = parsed.node_text(&lhs);
        if receiver_names.iter().any(|name| {
            lhs_text.starts_with(&format!("{name}.shell"))
                || lhs_text.starts_with(&format!("{name}["))
        }) {
            return true;
        }
    }

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
        if actual != "Object.assign" {
            continue;
        }
        let Some(target) = call_arg_node(&call, 0) else {
            continue;
        };
        let target = unwrap_parenthesized(target);
        if target.kind() == "identifier" && receiver_names.contains(parsed.node_text(&target)) {
            return true;
        }
    }

    false
}

fn js_ts_object_text_has_unsafe_shell_option(text: &str) -> bool {
    let Some(inner) = text
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
    else {
        return true;
    };
    for prop in js_split_top_level_commas(inner) {
        let prop = prop.trim();
        if prop.is_empty() {
            continue;
        }
        if prop.starts_with("...") {
            return true;
        }
        let Some(colon) = js_find_top_level_colon(prop) else {
            if js_ts_colonless_object_property_may_define_key(prop, "shell") {
                return true;
            }
            continue;
        };
        let prop_key_text = prop[..colon].trim();
        if prop_key_text.starts_with('[') {
            return true;
        }
        let prop_key = prop_key_text.trim_matches(['"', '\'', '`']);
        if prop_key == "shell" && prop[colon + 1..].trim() != "false" {
            return true;
        }
    }
    false
}

fn js_ts_colonless_object_property_may_define_key(prop: &str, key: &str) -> bool {
    let prop = prop.trim();
    if prop.starts_with('[') || prop.contains('[') {
        return true;
    }
    let name = prop
        .split_once('(')
        .map(|(head, _)| head.trim())
        .unwrap_or(prop)
        .trim_start_matches("async ")
        .trim_start_matches('*')
        .split_whitespace()
        .last()
        .unwrap_or(prop)
        .trim_matches(['"', '\'', '`']);
    name == key
}

fn js_ts_interpreter_eval_flag(flag: &str) -> bool {
    matches!(
        flag,
        "-c" | "-e" | "-p" | "-r" | "--eval" | "--print" | "--command"
    )
}

pub(in crate::algorithms::taint) fn js_ts_yaml_load_uses_safe_schema(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "yaml.load")
        && !js_ts_js_yaml_bare_load_call_matches(parsed, call, &actual)
    {
        return false;
    }
    !js_yaml_load_call_uses_unsafe_schema(parsed, call)
}

pub(in crate::algorithms::taint) fn js_ts_sql_call_is_parametrized(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !["query", "execute"]
        .iter()
        .any(|expected| call_path_matches(parsed, &actual, expected))
    {
        return false;
    }
    let text = parsed.node_text(call);
    (text.contains("bind") || text.contains("parameters"))
        && call_literal_arg(parsed, call, 0)
            .as_deref()
            .is_some_and(js_sql_literal_has_placeholder)
}

fn js_sql_literal_has_placeholder(query: &str) -> bool {
    query.contains('?')
        || query.contains("$1")
        || query.contains("@")
        || python_sql_literal_has_named_placeholder(query)
}

pub(super) fn js_ts_prisma_tagged_template_is_safe(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    (call_path_matches(parsed, &actual, "$queryRaw")
        || call_path_matches(parsed, &actual, "$executeRaw"))
        && parsed.node_text(call).contains('`')
        && !parsed.node_text(call).contains("Prisma.raw")
}
