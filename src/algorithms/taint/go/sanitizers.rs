//! Go path-traversal sanitizers and HasPrefix guards.

use crate::algorithms::taint::go::sinks::{collect_go_calls, go_call_path_text};
use crate::algorithms::taint::{
    assignment_lhs_identifiers, assignment_rhs_expressions, block_ends_with_return,
    call_passes_sink_semantics, cfg_line_reaches, collect_assignments, collect_if_statements,
    first_statement_line, node_contains_line, path_targets_var_at_line, safe_successor_line,
    sink_call_has_tainted_arg_in_path, sink_call_uses_var_in_tainted_arg, GuardControl,
    PathSanitizerBinding,
};
use crate::ast::tree_helpers::unwrap_parenthesized;
use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::FlowPath;
use crate::frameworks::SinkPattern;
use crate::languages::Language;
use tree_sitter::Node;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::algorithms::taint) enum PathSanitizerKind {
    Clean,
    Rel,
}

pub(in crate::algorithms::taint) fn go_path_traversal_cleansed_for_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    path: Option<&FlowPath>,
    sink_line: usize,
    sink_pat: Option<&'static SinkPattern>,
    sink_call: Option<&Node<'_>>,
) -> bool {
    if parsed.language != Language::Go || !cpg.has_cfg_edges() {
        return false;
    }

    let func_node = match parsed.enclosing_function(sink_line) {
        Some(n) => n,
        None => return false,
    };

    for binding in collect_path_sanitizer_bindings(parsed, &func_node) {
        if binding.call_line > sink_line {
            continue;
        }
        if let Some(p) = path {
            if !path_targets_var_at_line(parsed, p, sink_line, &binding.result_var) {
                continue;
            }
        }
        if let (Some(call), Some(pat)) = (sink_call, sink_pat) {
            if !sink_call_uses_var_in_tainted_arg(parsed, call, pat, &binding.result_var) {
                continue;
            }
        } else if let Some(pat) = sink_pat {
            if !line_has_matching_sink_call_using_var(
                parsed,
                sink_line,
                pat,
                &binding.result_var,
                path,
            ) {
                continue;
            }
        } else if path.is_none() {
            continue;
        }

        if guard_safely_controls_sink(parsed, cpg, &func_node, &binding, sink_line) {
            return true;
        }
    }

    false
}

fn collect_path_sanitizer_bindings(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
) -> Vec<PathSanitizerBinding> {
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
        let rhs_items = assignment_rhs_expressions(&rhs);

        for (idx, expr) in rhs_items.iter().enumerate() {
            if expr.kind() != "call_expression" {
                continue;
            }
            let kind = match go_call_path_text(parsed, expr).as_deref() {
                Some("filepath.Clean") => PathSanitizerKind::Clean,
                Some("filepath.Rel") => PathSanitizerKind::Rel,
                _ => continue,
            };
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
            bindings.push(PathSanitizerBinding {
                kind,
                result_var,
                call_line: expr.start_position().row + 1,
            });
        }
    }
    bindings
}

fn line_has_matching_sink_call_using_var(
    parsed: &ParsedFile,
    line: usize,
    sink_pat: &'static SinkPattern,
    var_name: &str,
    path: Option<&FlowPath>,
) -> bool {
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
        if actual != sink_pat.call_path || !call_passes_sink_semantics(parsed, call, sink_pat) {
            continue;
        }
        if let Some(p) = path {
            if !sink_call_has_tainted_arg_in_path(parsed, call, sink_pat, p) {
                continue;
            }
        }
        if sink_call_uses_var_in_tainted_arg(parsed, call, sink_pat, var_name) {
            return true;
        }
    }
    false
}

fn guard_safely_controls_sink(
    parsed: &ParsedFile,
    cpg: &CodePropertyGraph,
    func_node: &Node<'_>,
    binding: &PathSanitizerBinding,
    sink_line: usize,
) -> bool {
    let mut guards = Vec::new();
    collect_if_statements(*func_node, &mut guards);

    for guard in guards {
        let condition = match guard.child_by_field_name("condition") {
            Some(n) => n,
            None => continue,
        };
        let control = match classify_guard_control(parsed, &condition, binding) {
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

fn classify_guard_control(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    binding: &PathSanitizerBinding,
) -> Option<GuardControl> {
    let condition = unwrap_parenthesized(*condition);
    let condition_text = parsed.node_text(&condition);
    if condition_text.contains("&&") {
        return None;
    }
    if condition_text.contains("||") {
        return if binding.kind == PathSanitizerKind::Rel
            && contains_positive_hasprefix_call(parsed, &condition, &binding.result_var, false)
        {
            Some(GuardControl::RejectBranch)
        } else {
            None
        };
    }
    if is_negated_hasprefix_condition(parsed, &condition, &binding.result_var) {
        return match binding.kind {
            PathSanitizerKind::Clean => Some(GuardControl::RejectBranch),
            PathSanitizerKind::Rel => Some(GuardControl::AllowBranch),
        };
    }
    if is_bare_hasprefix_condition(parsed, &condition, &binding.result_var) {
        return match binding.kind {
            PathSanitizerKind::Clean => Some(GuardControl::AllowBranch),
            PathSanitizerKind::Rel => Some(GuardControl::RejectBranch),
        };
    }
    None
}

fn is_negated_hasprefix_condition(
    parsed: &ParsedFile,
    condition: &Node<'_>,
    var_name: &str,
) -> bool {
    let condition_text = parsed.node_text(condition).trim();
    if condition.kind() != "unary_expression" || !condition_text.starts_with('!') {
        return false;
    }
    if let Some(child) = condition.named_child(0) {
        let child = unwrap_parenthesized(child);
        is_bare_hasprefix_condition(parsed, &child, var_name)
    } else {
        false
    }
}

fn is_bare_hasprefix_condition(parsed: &ParsedFile, condition: &Node<'_>, var_name: &str) -> bool {
    let condition = unwrap_parenthesized(*condition);
    is_hasprefix_call_for_var(parsed, &condition, var_name)
}

fn contains_positive_hasprefix_call(
    parsed: &ParsedFile,
    node: &Node<'_>,
    var_name: &str,
    negated: bool,
) -> bool {
    let node_text = parsed.node_text(node).trim();
    let next_negated = negated || (node.kind() == "unary_expression" && node_text.starts_with('!'));
    if node.kind() == "call_expression"
        && !next_negated
        && is_hasprefix_call_for_var(parsed, node, var_name)
    {
        return true;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if contains_positive_hasprefix_call(parsed, &child, var_name, next_negated) {
            return true;
        }
    }
    false
}

fn is_hasprefix_call_for_var(parsed: &ParsedFile, node: &Node<'_>, var_name: &str) -> bool {
    if node.kind() != "call_expression" {
        return false;
    }
    if go_call_path_text(parsed, node).as_deref() != Some("strings.HasPrefix") {
        return false;
    }
    let arguments = match node.child_by_field_name("arguments") {
        Some(n) => n,
        None => return false,
    };
    let mut cursor = arguments.walk();
    let first_arg = match arguments.named_children(&mut cursor).next() {
        Some(n) => unwrap_parenthesized(n),
        None => return false,
    };
    first_arg.kind() == "identifier" && parsed.node_text(&first_arg) == var_name
}
