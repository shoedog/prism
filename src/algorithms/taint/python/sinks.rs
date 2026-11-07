//! Python sink outcomes: template rendering, SQL parametrisation, YAML loaders.

use crate::algorithms::taint::cleansing::node_contains_line;
use crate::algorithms::taint::python::sink_tables::{PY_CWE502_SINKS, PY_CWE79_SINKS, PY_CWE89_SINKS, PY_CWE918_SINKS};
use crate::algorithms::taint::python::sources::node_contains_flask_request_data_access;
use crate::algorithms::taint::sink_match::{SinkMatchOutcome, arg_is_tainted_in_path, arg_node_taints_match, call_literal_arg, call_passes_sink_semantics, line_matches_structured_sink, sink_call_path_matches};
use crate::ast::ParsedFile;
use crate::ast::tree_helpers::{call_arg_node, call_path_matches, call_path_text, collect_calls};
use crate::data_flow::FlowPath;
use crate::frameworks::{SanitizerCategory, SinkPattern};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn python_sink_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> SinkMatchOutcome {
    if let Some(outcome) = python_render_template_string_outcome(parsed, line, path) {
        return outcome;
    }

    let mut any_call_path_match = false;
    for pat in PY_CWE79_SINKS
        .iter()
        .chain(PY_CWE89_SINKS.iter())
        .chain(PY_CWE918_SINKS.iter())
        .chain(PY_CWE502_SINKS.iter())
    {
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

fn python_render_template_string_outcome(
    parsed: &ParsedFile,
    line: usize,
    path: Option<&FlowPath>,
) -> Option<SinkMatchOutcome> {
    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let pat = PY_CWE79_SINKS
        .iter()
        .find(|p| p.call_path == "render_template_string")?;
    let mut had_call = false;
    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = call_path_text(parsed, call)?;
        if !call_path_matches(parsed, &actual, "render_template_string") {
            continue;
        }
        had_call = true;
        if let Some(p) = path {
            if arg_is_tainted_in_path(parsed, call, 0, p) {
                return Some(SinkMatchOutcome::Match(pat));
            }
        }
        let unsafe_vars = python_render_unsafe_template_vars(parsed, call);
        let autoescape_disabled = python_render_autoescape_disabled(parsed, call);
        if unsafe_vars.is_empty() && !autoescape_disabled {
            continue;
        }
        if path.is_none() {
            return Some(SinkMatchOutcome::Match(pat));
        }
        if python_render_tainted_context_matches(
            parsed,
            call,
            path?,
            &unsafe_vars,
            autoescape_disabled,
        ) {
            return Some(SinkMatchOutcome::Match(pat));
        }
    }
    if had_call {
        Some(SinkMatchOutcome::SemanticallyExcluded)
    } else {
        None
    }
}

pub(super) fn python_render_unsafe_template_vars(
    parsed: &ParsedFile,
    call: &Node<'_>,
) -> BTreeSet<String> {
    let template = match call_literal_arg(parsed, call, 0) {
        Some(s) => s,
        None => return BTreeSet::new(),
    };
    let mut vars = BTreeSet::new();
    for part in template.split("{{").skip(1) {
        let expr = part.split("}}").next().unwrap_or(part);
        if !expr.contains("| safe") && !expr.contains("|safe") {
            continue;
        }
        let name = expr
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("")
            .trim();
        if !name.is_empty() {
            vars.insert(name.to_string());
        }
    }
    vars
}

pub(super) fn python_render_autoescape_disabled(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    call_literal_arg(parsed, call, 0).is_some_and(|s| {
        let compact: String = s
            .to_ascii_lowercase()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        compact.contains("{%autoescapefalse%}")
    })
}

pub(super) fn python_render_tainted_context_matches(
    parsed: &ParsedFile,
    call: &Node<'_>,
    path: &FlowPath,
    unsafe_vars: &BTreeSet<String>,
    autoescape_disabled: bool,
) -> bool {
    let args = match call.child_by_field_name("arguments") {
        Some(a) => a,
        None => return false,
    };
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        if child.kind() != "keyword_argument" {
            continue;
        }
        let key = match child.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).to_string(),
            None => continue,
        };
        if !autoescape_disabled && !unsafe_vars.contains(&key) {
            continue;
        }
        let value = child
            .child_by_field_name("value")
            .or_else(|| child.named_child(1));
        if let Some(v) = value {
            if arg_node_taints_match(parsed, &v, path) {
                return true;
            }
        }
    }
    false
}

pub(in crate::algorithms::taint) fn python_sink_with_inline_flask_source(
    parsed: &ParsedFile,
    line: usize,
) -> Option<&'static SinkPattern> {
    if parsed.language != Language::Python {
        return None;
    }

    let mut calls = Vec::new();
    collect_calls(parsed, parsed.tree.root_node(), &mut calls);
    let render_pat = PY_CWE79_SINKS
        .iter()
        .find(|p| p.call_path == "render_template_string");

    for call in &calls {
        if !node_contains_line(call, line) {
            continue;
        }
        let actual = match call_path_text(parsed, call) {
            Some(s) => s,
            None => continue,
        };
        if call_path_matches(parsed, &actual, "render_template_string") {
            if let Some(pat) = render_pat {
                if python_render_inline_flask_source_matches(parsed, call) {
                    return Some(pat);
                }
            }
            continue;
        }

        for pat in PY_CWE79_SINKS
            .iter()
            .chain(PY_CWE89_SINKS.iter())
            .chain(PY_CWE918_SINKS.iter())
            .chain(PY_CWE502_SINKS.iter())
        {
            if !sink_call_path_matches(parsed, call, &actual, pat) {
                continue;
            }
            if !call_passes_sink_semantics(parsed, call, pat) {
                continue;
            }
            if pat.category == SanitizerCategory::Sqli
                && python_sql_call_is_parametrized(parsed, call)
            {
                continue;
            }
            if pat.category == SanitizerCategory::Deserialization
                && python_yaml_load_uses_safe_loader(parsed, call)
            {
                continue;
            }
            if pat.tainted_arg_indices.iter().any(|&idx| {
                call_arg_node(call, idx)
                    .is_some_and(|arg| node_contains_flask_request_data_access(parsed, arg))
            }) {
                return Some(pat);
            }
        }
    }
    None
}

fn python_render_inline_flask_source_matches(parsed: &ParsedFile, call: &Node<'_>) -> bool {
    let unsafe_vars = python_render_unsafe_template_vars(parsed, call);
    let autoescape_disabled = python_render_autoescape_disabled(parsed, call);
    if unsafe_vars.is_empty() && !autoescape_disabled {
        return false;
    }

    let args = match call.child_by_field_name("arguments") {
        Some(a) => a,
        None => return false,
    };
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        if child.kind() != "keyword_argument" {
            continue;
        }
        let key = match child.child_by_field_name("name") {
            Some(n) => parsed.node_text(&n).to_string(),
            None => continue,
        };
        if !autoescape_disabled && !unsafe_vars.contains(&key) {
            continue;
        }
        let value = child
            .child_by_field_name("value")
            .or_else(|| child.named_child(1));
        if value.is_some_and(|v| node_contains_flask_request_data_access(parsed, v)) {
            return true;
        }
    }
    false
}

pub(in crate::algorithms::taint) fn python_sql_call_is_parametrized(
    parsed: &ParsedFile,
    call: &Node<'_>,
) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "execute")
        && !call_path_matches(parsed, &actual, "executemany")
    {
        return false;
    }

    if let Some(query) = call_literal_arg(parsed, call, 0) {
        return python_sql_literal_has_placeholder(&query) && call_has_arg_after(call, 0);
    }

    let arg0 = match call_arg_node(call, 0) {
        Some(n) => n,
        None => return false,
    };
    let arg0_text = parsed.node_text(&arg0);
    if !(arg0_text.contains(".bindparams(") || arg0_text.contains(".params(")) {
        return false;
    }
    let Some(query) = first_string_literal_text(parsed, &arg0) else {
        return false;
    };
    arg0_text.contains("text(") && python_sql_literal_has_named_placeholder(&query)
}

fn python_sql_literal_has_placeholder(query: &str) -> bool {
    query.contains("%s") || query.contains('?') || python_sql_literal_has_named_placeholder(query)
}

pub(in crate::algorithms::taint) fn python_sql_literal_has_named_placeholder(query: &str) -> bool {
    let bytes = query.as_bytes();
    bytes.iter().enumerate().any(|(idx, b)| {
        *b == b':'
            && bytes
                .get(idx + 1)
                .is_some_and(|next| (*next as char).is_ascii_alphabetic() || *next == b'_')
    })
}

fn call_has_arg_after(call: &Node<'_>, arg_idx: usize) -> bool {
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    arguments.named_children(&mut cursor).count() > arg_idx + 1
}

fn first_string_literal_text(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let text = parsed.node_text(node).trim();
    if node.kind() == "string"
        || node.kind() == "interpreted_string_literal"
        || node.kind() == "raw_string_literal"
        || text.starts_with('"')
        || text.starts_with('\'')
        || text.starts_with('r')
        || text.starts_with('R')
        || text.starts_with('u')
        || text.starts_with('U')
        || text.starts_with('b')
        || text.starts_with('B')
    {
        let quote_idx = text.find(['"', '\'']).unwrap_or(0);
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
            .unwrap_or(without_prefix);
        return Some(trimmed.to_string());
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if let Some(text) = first_string_literal_text(parsed, &child) {
            return Some(text);
        }
    }
    None
}

pub(in crate::algorithms::taint) fn python_yaml_load_uses_safe_loader(
    parsed: &ParsedFile,
    call: &Node<'_>,
) -> bool {
    let actual = match call_path_text(parsed, call) {
        Some(s) => s,
        None => return false,
    };
    if !call_path_matches(parsed, &actual, "yaml.load")
        && !call_path_matches(parsed, &actual, "load")
    {
        return false;
    }
    let Some(arguments) = call.child_by_field_name("arguments") else {
        return false;
    };
    let mut cursor = arguments.walk();
    for (idx, arg) in arguments.named_children(&mut cursor).enumerate() {
        if idx == 0 {
            continue;
        }
        let text = parsed.node_text(&arg);
        if text.contains("SafeLoader") || text.contains("CSafeLoader") {
            return true;
        }
    }
    false
}
