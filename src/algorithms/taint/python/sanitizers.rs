//! Python XSS and SSRF sanitizers and guards.

use crate::algorithms::taint::{SafeFormatHtmlBinding, UrlSanitizerBinding};
use crate::algorithms::taint::cleansing::{GuardControl, block_ends_with_return, cfg_line_reaches, collect_if_statements, first_statement_line, flow_path_cleansed_for_sink_call, node_contains_line, safe_successor_line};
use crate::algorithms::taint::python::sink_tables::{PY_CWE502_SINKS, PY_CWE79_SINKS, PY_CWE89_SINKS, PY_CWE918_SINKS};
use crate::algorithms::taint::python::sinks::{python_render_autoescape_disabled, python_render_tainted_context_matches, python_render_unsafe_template_vars, python_sql_call_is_parametrized, python_yaml_load_uses_safe_loader};
use crate::algorithms::taint::sink_match::{assignment_lhs_identifiers, assignment_rhs_expressions, call_literal_arg, call_passes_sink_semantics, collect_assignments, path_targets_var_at_line, sink_call_has_tainted_arg_in_path, sink_call_path_matches, sink_call_uses_var_in_tainted_arg};
use crate::ast::ParsedFile;
use crate::ast::tree_helpers::{call_arg_node, call_path_matches, call_path_text, collect_calls, unwrap_parenthesized};
use crate::cpg::CodePropertyGraph;
use crate::data_flow::FlowPath;
use crate::frameworks::{SanitizerCategory, SinkPattern};
use crate::languages::Language;
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn python_xss_cleansed_for_sink(
    parsed: &ParsedFile,
    path: &FlowPath,
    sink_line: usize,
    sink_pat: &'static SinkPattern,
    sink_call: &Node<'_>,
) -> bool {
    if parsed.language != Language::Python || sink_pat.category != SanitizerCategory::Xss {
        return false;
    }
    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };
    for binding in collect_safe_format_html_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.result_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.result_var) {
            continue;
        }
        return true;
    }
    false
}

fn collect_safe_format_html_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<SafeFormatHtmlBinding> {
    let mut assignments = Vec::new();
    collect_assignments(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let lhs = match parsed.language.assignment_target(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let rhs = match parsed.language.assignment_value(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let lhs_items = assignment_lhs_identifiers(parsed, &lhs);
        if lhs_items.is_empty() {
            continue;
        }
        let rhs_items = if call_path_text(parsed, &rhs).is_some() {
            vec![rhs]
        } else {
            assignment_rhs_expressions(&rhs)
        };

        for (idx, expr) in rhs_items.iter().enumerate() {
            let actual = match call_path_text(parsed, expr) {
                Some(s) => s,
                None => continue,
            };
            if !call_path_matches(parsed, &actual, "format_html") {
                continue;
            }
            if call_literal_arg(parsed, expr, 0).is_none() {
                continue;
            }
            let result_var = match lhs_items.get(idx).or_else(|| {
                if rhs_items.len() == 1 {
                    lhs_items.first()
                } else {
                    None
                }
            }) {
                Some(name) if name != "_" => name.clone(),
                _ => continue,
            };
            bindings.push(SafeFormatHtmlBinding {
                result_var,
                call_line: expr.start_position().row + 1,
            });
        }
    }
    bindings
}

pub(in crate::algorithms::taint) fn python_ssrf_cleansed_for_sink(
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
    for binding in collect_url_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if !path_targets_var_at_line(parsed, path, sink_line, &binding.url_var) {
            continue;
        }
        if !sink_call_uses_var_in_tainted_arg(parsed, sink_call, sink_pat, &binding.url_var) {
            continue;
        }
        if python_url_guard_safely_controls_sink(parsed, cpg, &func_node, &binding, sink_line) {
            return true;
        }
    }
    false
}

fn collect_url_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<UrlSanitizerBinding> {
    let mut assignments = Vec::new();
    collect_assignments(*func_node, parsed, &mut assignments);

    let mut bindings = Vec::new();
    for assignment in assignments {
        let lhs = match parsed.language.assignment_target(&assignment) {
            Some(n) => n,
            None => continue,
        };
        let rhs = match parsed.language.assignment_value(&assignment) {
            Some(n) => n,
            None => continue,
        };
        if rhs.kind() != "call" && rhs.kind() != "call_expression" {
            continue;
        }
        let actual = match call_path_text(parsed, &rhs) {
            Some(s) => s,
            None => continue,
        };
        if !call_path_matches(parsed, &actual, "urlparse")
            && !call_path_matches(parsed, &actual, "urllib.parse.urlparse")
        {
            continue;
        }
        let result_var = match assignment_lhs_identifiers(parsed, &lhs).first() {
            Some(name) => name.clone(),
            None => continue,
        };
        let url_arg = match call_arg_node(&rhs, 0) {
            Some(n) => n,
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

fn python_url_guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
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
        let control = match classify_python_url_guard(parsed, &condition, binding) {
            Some(c) => c,
            None => continue,
        };
        let consequence = match guard.child_by_field_name("consequence") {
            Some(n) => n,
            None => continue,
        };
        let consequence_entry = match first_statement_line(parsed, &consequence) {
            Some(line) => line,
            None => continue,
        };

        match control {
            GuardControl::RejectBranch => {
                if !block_ends_with_return(parsed, &consequence) {
                    continue;
                }
                let safe_entry = match safe_successor_line(cpg, parsed, &guard, consequence_entry) {
                    Some(line) => line,
                    None => continue,
                };
                if cfg_line_reaches(cpg, &parsed.path, safe_entry, sink_line)
                    && !cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
                {
                    return true;
                }
            }
            GuardControl::AllowBranch => {
                if node_contains_line(&consequence, sink_line)
                    && cfg_line_reaches(cpg, &parsed.path, consequence_entry, sink_line)
                {
                    return true;
                }
            }
        }
    }

    false
}

fn classify_python_url_guard(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &UrlSanitizerBinding,
) -> Option<GuardControl> {
    let condition = unwrap_parenthesized(*condition);
    let condition_text = parsed.node_text(&condition);
    if !python_url_condition_targets_binding(condition_text, binding) {
        return None;
    }
    if condition_text.contains(" not in ") {
        Some(GuardControl::RejectBranch)
    } else if condition_text.contains(" in ") {
        Some(GuardControl::AllowBranch)
    } else {
        None
    }
}

fn python_url_condition_targets_binding(
    condition_text: &str,
    binding: &UrlSanitizerBinding,
) -> bool {
    let parsed_host = format!("{}.hostname", binding.result_var);
    let direct_urlparse = format!("urlparse({}).hostname", binding.url_var);
    let qualified_urlparse = format!("urllib.parse.urlparse({}).hostname", binding.url_var);
    condition_text.contains(&parsed_host)
        || condition_text.contains(&direct_urlparse)
        || condition_text.contains(&qualified_urlparse)
}

pub(in crate::algorithms::taint) fn python_safe_structured_sink_call_ranges(
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
            if sink_call_has_tainted_arg_in_path(parsed, call, pat, path)
                && flow_path_cleansed_for_sink_call(parsed, cpg, path, line, pat, call)
            {
                ranges.push((call.start_byte(), call.end_byte()));
                break;
            }
        }
        if call_path_matches(parsed, &actual, "render_template_string") {
            let unsafe_vars = python_render_unsafe_template_vars(parsed, call);
            let autoescape_disabled = python_render_autoescape_disabled(parsed, call);
            if (unsafe_vars.is_empty() && !autoescape_disabled)
                || !python_render_tainted_context_matches(
                    parsed,
                    call,
                    path,
                    &unsafe_vars,
                    autoescape_disabled,
                )
            {
                ranges.push((call.start_byte(), call.end_byte()));
            }
        }
        if call_path_matches(parsed, &actual, "execute")
            && python_sql_call_is_parametrized(parsed, call)
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
        if call_path_matches(parsed, &actual, "executemany")
            && python_sql_call_is_parametrized(parsed, call)
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
        if python_yaml_load_uses_safe_loader(parsed, call) {
            ranges.push((call.start_byte(), call.end_byte()));
        }
        if call_path_matches(parsed, &actual, "format_html")
            && call_literal_arg(parsed, call, 0).is_some()
        {
            ranges.push((call.start_byte(), call.end_byte()));
        }
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}
