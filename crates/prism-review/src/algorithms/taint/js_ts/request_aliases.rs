//! JS/TS request-data alias definitions, kills and scopes.

use crate::algorithms::taint::js_ts::bindings::js_ts_assignment_target_byte_scope;
use crate::algorithms::taint::js_ts::request_targets::{
    collect_js_ts_request_alias_source_uses, collect_js_ts_request_assignment_targets,
};
use crate::algorithms::taint::js_ts::source_ranges::js_ts_ranges_are_in_sibling_control_flow_branches;
use crate::algorithms::taint::{
    JsTsAliasDef, JsTsAliasDefs, JsTsAliasKill, JsTsRequestDataSource, TaintSeed,
};
use prism::access_path::AccessPath;
use prism::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) fn js_ts_alias_def(
    line: usize,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
) -> JsTsAliasDef {
    JsTsAliasDef {
        line,
        start_byte,
        scope,
        byte_scope,
        kills: Vec::new(),
    }
}

pub(super) fn js_ts_request_data_assignment_sources(
    parsed: &ParsedFile,
    func: &Node<'_>,
    framework: &str,
    source_params: &BTreeSet<String>,
) -> BTreeSet<JsTsRequestDataSource> {
    let mut out = BTreeSet::new();
    let func_start_line = func.start_position().row + 1;
    let mut request_alias_defs = BTreeMap::new();
    for param in source_params {
        js_ts_add_alias_def(
            &mut request_alias_defs,
            param.clone(),
            js_ts_alias_def(func_start_line, None, None, None),
        );
    }
    let mut koa_request_object_alias_defs = BTreeMap::new();
    collect_js_ts_request_assignments(
        parsed,
        *func,
        func.id(),
        framework,
        &mut request_alias_defs,
        &mut koa_request_object_alias_defs,
        &mut out,
    );
    collect_js_ts_request_alias_source_uses(
        parsed,
        *func,
        framework,
        &request_alias_defs,
        &koa_request_object_alias_defs,
        &mut out,
    );
    out
}

pub(super) fn js_ts_add_alias_def(
    defs: &mut JsTsAliasDefs,
    alias: String,
    def: JsTsAliasDef,
) -> bool {
    let entries = defs.entry(alias).or_default();
    if entries
        .iter()
        .any(|existing| existing.same_binding_as(&def))
    {
        return false;
    }
    entries.push(def);
    true
}

fn js_ts_merge_alias_defs(into: &mut JsTsAliasDefs, from: &JsTsAliasDefs) {
    for (alias, from_defs) in from {
        let entries = into.entry(alias.clone()).or_default();
        for from_def in from_defs {
            if let Some(existing) = entries
                .iter_mut()
                .find(|existing| existing.same_binding_as(from_def))
            {
                for kill in &from_def.kills {
                    if !existing.kills.contains(kill) {
                        existing.kills.push(*kill);
                    }
                }
            } else {
                entries.push(from_def.clone());
            }
        }
    }
}

pub(super) fn js_ts_active_alias_names_at_range(
    defs: &JsTsAliasDefs,
    line: usize,
    start_byte: usize,
    end_byte: usize,
) -> BTreeSet<String> {
    defs.iter()
        .filter_map(|(alias, alias_defs)| {
            alias_defs
                .iter()
                .any(|def| def.visible_range(line, start_byte, end_byte))
                .then(|| alias.clone())
        })
        .collect()
}

fn js_ts_kill_alias_defs_at(
    defs: &mut JsTsAliasDefs,
    alias: &str,
    line: usize,
    byte: usize,
    byte_scope: Option<(usize, usize)>,
) {
    let kill = JsTsAliasKill {
        line,
        byte,
        byte_scope,
    };
    if let Some(alias_defs) = defs.get_mut(alias) {
        for def in alias_defs {
            if def.visible_range(line, byte, byte) && !def.kills.contains(&kill) {
                def.kills.push(kill);
            }
        }
    }
}

pub(super) fn js_ts_kill_simple_lhs_alias_defs_at(
    parsed: &ParsedFile,
    lhs: &Node<'_>,
    binding_node: &Node<'_>,
    line: usize,
    root_func_id: usize,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
) -> BTreeSet<String> {
    let byte_scope = js_ts_assignment_kill_byte_scope(parsed, binding_node, root_func_id);
    let mut aliases = BTreeSet::new();
    collect_js_ts_lhs_alias_identifiers(parsed, lhs, &mut aliases);
    for alias in &aliases {
        if alias == "_" {
            continue;
        }
        js_ts_kill_alias_defs_at(
            request_alias_defs,
            alias,
            line,
            lhs.start_byte(),
            byte_scope,
        );
        js_ts_kill_alias_defs_at(
            koa_request_object_alias_defs,
            alias,
            line,
            lhs.start_byte(),
            byte_scope,
        );
    }
    aliases
}

fn js_ts_assignment_kill_byte_scope(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    if binding_node.kind() == "variable_declarator" {
        return js_ts_assignment_target_byte_scope(parsed, binding_node, root_func_id);
    }
    js_ts_conditional_execution_byte_scope(parsed, binding_node, root_func_id)
        .or_else(|| js_ts_assignment_target_byte_scope(parsed, binding_node, root_func_id))
}

fn js_ts_conditional_execution_byte_scope(
    parsed: &ParsedFile,
    node: &Node<'_>,
    root_func_id: usize,
) -> Option<(usize, usize)> {
    let mut child = *node;
    let mut current = child.parent();
    while let Some(parent) = current {
        if parent.id() == root_func_id {
            return None;
        }
        if matches!(
            parent.kind(),
            "catch_clause" | "else_clause" | "finally_clause" | "switch_case" | "switch_default"
        ) {
            return Some((parent.start_byte(), parent.end_byte()));
        }
        if js_ts_is_conditional_execution_boundary(parsed, &parent) {
            return Some((child.start_byte(), child.end_byte()));
        }
        child = parent;
        current = parent.parent();
    }
    None
}

fn js_ts_is_conditional_execution_boundary(parsed: &ParsedFile, node: &Node<'_>) -> bool {
    let kind = node.kind();
    matches!(
        kind,
        "conditional_expression"
            | "do_statement"
            | "for_await_statement"
            | "for_in_statement"
            | "for_of_statement"
            | "for_statement"
            | "if_statement"
            | "switch_statement"
            | "ternary_expression"
            | "try_statement"
            | "while_statement"
    ) || (kind == "binary_expression" && {
        let text = parsed.node_text(node);
        text.contains("&&") || text.contains("||")
    })
}

pub(super) fn collect_js_ts_lhs_alias_identifiers(
    parsed: &ParsedFile,
    node: &Node<'_>,
    out: &mut BTreeSet<String>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            out.insert(parsed.node_text(node).to_string());
        }
        "assignment_pattern"
        | "object_assignment_pattern"
        | "parenthesized_expression"
        | "rest_pattern" => {
            if let Some(child) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_alias_identifiers(parsed, &child, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_js_ts_lhs_alias_identifiers(parsed, &value, out);
            } else {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "shorthand_property_identifier_pattern" {
                        collect_js_ts_lhs_alias_identifiers(parsed, &child, out);
                    }
                }
            }
        }
        "member_expression" | "subscript_expression" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_alias_identifiers(parsed, &child, out);
            }
        }
    }
}

pub(super) fn js_ts_shrink_request_sources_after_unconditional_kill(
    parsed: &ParsedFile,
    binding_node: &Node<'_>,
    root_func_id: usize,
    aliases: &BTreeSet<String>,
    line: usize,
    kill_byte: usize,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    if aliases.is_empty()
        || js_ts_conditional_execution_byte_scope(parsed, binding_node, root_func_id).is_some()
    {
        return;
    }

    let mut removals = Vec::new();
    let mut replacements = Vec::new();
    for source in out.iter() {
        if !source.target.fields.is_empty()
            || !aliases.contains(&source.target.base)
            || source.line > line
            || !js_ts_line_in_scope(source.scope, line)
        {
            continue;
        }

        let source_start = source
            .start_byte
            .or_else(|| source.byte_scope.map(|(start, _)| start))
            .unwrap_or(0);
        if source.line == line && source_start >= kill_byte {
            continue;
        }

        removals.push(source.clone());
        if source.line == line {
            if let Some(byte_scope) =
                js_ts_intersect_byte_scopes(source.byte_scope, Some((source_start, kill_byte)))
            {
                let mut replacement = source.clone();
                replacement.byte_scope = Some(byte_scope);
                replacements.push(replacement);
            }
            continue;
        }

        let scope_start = source.scope.map(|(start, _)| start).unwrap_or(source.line);
        if let Some(scope_end) = line.checked_sub(1) {
            if scope_start <= scope_end {
                let mut replacement = source.clone();
                replacement.scope = Some((scope_start, scope_end));
                replacements.push(replacement);
            }
        }
    }

    for source in removals {
        out.remove(&source);
    }
    for source in replacements {
        out.insert(source);
    }
}

pub(in crate::algorithms::taint) fn js_ts_seed_scope_contains(
    seed: &TaintSeed,
    line: usize,
) -> bool {
    seed.scope
        .map(|(start, end)| start <= line && line <= end)
        .unwrap_or(true)
}

pub(in crate::algorithms::taint) fn js_ts_seed_scope_contains_range(
    parsed: &ParsedFile,
    seed: &TaintSeed,
    line: usize,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    js_ts_seed_scope_contains(seed, line)
        && (seed.line < line
            || seed
                .start_byte
                .map(|seed_start| seed_start <= start_byte)
                .unwrap_or(true))
        && seed
            .byte_scope
            .map(|(start, end)| start <= start_byte && end_byte <= end)
            .unwrap_or(true)
        && seed.start_byte.is_none_or(|seed_start| {
            !js_ts_ranges_are_in_sibling_control_flow_branches(
                parsed.tree.root_node(),
                seed_start,
                seed_start,
                start_byte,
                end_byte,
            )
        })
}

pub(super) fn js_ts_line_in_scope(scope: Option<(usize, usize)>, line: usize) -> bool {
    scope
        .map(|(start, end)| start <= line && line <= end)
        .unwrap_or(true)
}

pub(super) fn js_ts_request_source(
    line: usize,
    target: AccessPath,
    start_byte: Option<usize>,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
) -> JsTsRequestDataSource {
    JsTsRequestDataSource {
        line,
        target,
        start_byte,
        scope,
        byte_scope,
    }
}

pub(super) fn collect_js_ts_request_assignments(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    framework: &str,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return;
    }

    if node.kind() == "variable_declarator" {
        if let (Some(lhs), Some(rhs)) = (
            node.child_by_field_name("name"),
            node.child_by_field_name("value"),
        ) {
            collect_js_ts_request_assignment_targets(
                parsed,
                lhs,
                rhs,
                node,
                root_func_id,
                framework,
                request_alias_defs,
                koa_request_object_alias_defs,
                out,
            );
        }
        return;
    }

    if parsed.language.is_assignment_node(node.kind()) {
        if let (Some(lhs), Some(rhs)) = (
            parsed.language.assignment_target(&node),
            parsed.language.assignment_value(&node),
        ) {
            collect_js_ts_request_assignment_targets(
                parsed,
                lhs,
                rhs,
                node,
                root_func_id,
                framework,
                request_alias_defs,
                koa_request_object_alias_defs,
                out,
            );
        }
        return;
    }

    if matches!(node.kind(), "if_statement" | "if_expression") {
        collect_js_ts_request_assignments_in_if(
            parsed,
            node,
            root_func_id,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            out,
        );
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_request_assignments(
            parsed,
            child,
            root_func_id,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            out,
        );
    }
}

fn collect_js_ts_request_assignments_in_if(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    framework: &str,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    let consequence = node.child_by_field_name("consequence");
    let alternative = node.child_by_field_name("alternative");
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if consequence.is_some_and(|branch| branch.id() == child.id())
            || alternative.is_some_and(|branch| branch.id() == child.id())
        {
            continue;
        }
        collect_js_ts_request_assignments(
            parsed,
            child,
            root_func_id,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            out,
        );
    }

    let base_request_alias_defs = request_alias_defs.clone();
    let base_koa_request_object_alias_defs = koa_request_object_alias_defs.clone();
    let mut branch_request_alias_defs = Vec::new();
    let mut branch_koa_request_object_alias_defs = Vec::new();
    for branch in [consequence, alternative].into_iter().flatten() {
        let mut request_defs = base_request_alias_defs.clone();
        let mut koa_defs = base_koa_request_object_alias_defs.clone();
        let branch_exits = js_ts_node_definitely_exits(branch);
        collect_js_ts_request_assignments(
            parsed,
            branch,
            root_func_id,
            framework,
            &mut request_defs,
            &mut koa_defs,
            out,
        );
        collect_js_ts_request_alias_source_uses(
            parsed,
            branch,
            framework,
            &request_defs,
            &koa_defs,
            out,
        );
        if branch_exits {
            branch_request_alias_defs.push(js_ts_branch_scoped_alias_delta(
                &request_defs,
                &base_request_alias_defs,
                branch,
            ));
            branch_koa_request_object_alias_defs.push(js_ts_branch_scoped_alias_delta(
                &koa_defs,
                &base_koa_request_object_alias_defs,
                branch,
            ));
        } else {
            branch_request_alias_defs.push(request_defs);
            branch_koa_request_object_alias_defs.push(koa_defs);
        }
    }

    for branch_defs in &branch_request_alias_defs {
        js_ts_merge_alias_defs(request_alias_defs, branch_defs);
    }
    for branch_defs in &branch_koa_request_object_alias_defs {
        js_ts_merge_alias_defs(koa_request_object_alias_defs, branch_defs);
    }
}

fn js_ts_branch_scoped_alias_delta(
    branch_defs: &JsTsAliasDefs,
    base_defs: &JsTsAliasDefs,
    branch: Node<'_>,
) -> JsTsAliasDefs {
    let branch_line_scope = Some((
        branch.start_position().row + 1,
        branch.end_position().row + 1,
    ));
    let branch_byte_scope = Some((branch.start_byte(), branch.end_byte()));
    let mut out = BTreeMap::new();
    for (alias, defs) in branch_defs {
        for def in defs {
            if base_defs
                .get(alias)
                .is_some_and(|base| base.iter().any(|base_def| base_def.same_binding_as(def)))
            {
                continue;
            }
            let mut scoped = def.clone();
            scoped.scope = js_ts_intersect_line_scopes(scoped.scope, branch_line_scope);
            scoped.byte_scope = js_ts_intersect_byte_scopes(scoped.byte_scope, branch_byte_scope);
            js_ts_add_alias_def(&mut out, alias.clone(), scoped);
        }
    }
    out
}

pub(super) fn js_ts_intersect_line_scopes(
    left: Option<(usize, usize)>,
    right: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    match (left, right) {
        (Some((left_start, left_end)), Some((right_start, right_end))) => {
            let start = left_start.max(right_start);
            let end = left_end.min(right_end);
            (start <= end).then_some((start, end))
        }
        (Some(scope), None) | (None, Some(scope)) => Some(scope),
        (None, None) => None,
    }
}

pub(super) fn js_ts_intersect_byte_scopes(
    left: Option<(usize, usize)>,
    right: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    match (left, right) {
        (Some((left_start, left_end)), Some((right_start, right_end))) => {
            let start = left_start.max(right_start);
            let end = left_end.min(right_end);
            (start <= end).then_some((start, end))
        }
        (Some(scope), None) | (None, Some(scope)) => Some(scope),
        (None, None) => None,
    }
}

pub(super) fn js_ts_node_definitely_exits(node: Node<'_>) -> bool {
    match node.kind() {
        "return_statement" | "throw_statement" => true,
        "else_clause" | "statement_block" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .is_some_and(js_ts_node_definitely_exits),
        "if_statement" | "if_expression" => {
            let Some(consequence) = node.child_by_field_name("consequence") else {
                return false;
            };
            let Some(alternative) = node.child_by_field_name("alternative") else {
                return false;
            };
            js_ts_node_definitely_exits(consequence) && js_ts_node_definitely_exits(alternative)
        }
        _ => false,
    }
}
