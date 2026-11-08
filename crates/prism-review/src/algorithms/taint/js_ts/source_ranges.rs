//! JS/TS request-source access ranges, branch and switch reachability.

use crate::algorithms::taint::cleansing::node_contains_line;
use crate::algorithms::taint::js_ts::framework_params::{
    js_ts_framework_source_params, js_ts_function_params,
};
use crate::algorithms::taint::js_ts::request_aliases::{
    collect_js_ts_request_assignments, js_ts_add_alias_def, js_ts_alias_def,
};
use crate::algorithms::taint::js_ts::sinks::node_contains_range;
use crate::algorithms::taint::js_ts::source_access::{
    js_ts_field_access_text_matches, js_ts_source_access_text_matches, JS_TS_REQUEST_DATA_FIELDS,
};
use crate::algorithms::taint::{JsTsAliasDef, JsTsAliasDefs, TaintSeed};
use prism::ast::tree_helpers::is_js_ts_language;
use prism::ast::ParsedFile;
use prism::frameworks::js_ts::call_classify::js_ts_framework_receiver_names;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(super) fn js_ts_request_source_access_ranges_for_line(
    parsed: &ParsedFile,
    line: usize,
) -> Vec<(usize, usize)> {
    let Some(framework) = parsed.framework().map(|spec| spec.name) else {
        return Vec::new();
    };
    if !matches!(framework, "fastify" | "express" | "koa") {
        return Vec::new();
    }

    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);
    let mut ranges = Vec::new();
    for func in parsed.all_functions() {
        if !node_contains_line(&func, line) {
            continue;
        }
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }
        let source_params =
            js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
        if source_params.is_empty() {
            continue;
        }

        let func_start_line = func.start_position().row + 1;
        let mut request_alias_defs = BTreeMap::new();
        for param in &source_params {
            js_ts_add_alias_def(
                &mut request_alias_defs,
                param.clone(),
                js_ts_alias_def(func_start_line, None, None, None),
            );
        }
        let mut koa_request_object_alias_defs = BTreeMap::new();
        let mut assignment_sources = BTreeSet::new();
        collect_js_ts_request_assignments(
            parsed,
            func,
            func.id(),
            framework,
            &mut request_alias_defs,
            &mut koa_request_object_alias_defs,
            &mut assignment_sources,
        );
        for alias in request_alias_defs.keys() {
            ranges.extend(js_ts_source_access_ranges_for_alias_from_defs(
                parsed,
                func,
                line,
                framework,
                alias,
                false,
                &request_alias_defs,
                &koa_request_object_alias_defs,
            ));
        }
        for alias in koa_request_object_alias_defs.keys() {
            ranges.extend(js_ts_source_access_ranges_for_alias_from_defs(
                parsed,
                func,
                line,
                framework,
                alias,
                true,
                &request_alias_defs,
                &koa_request_object_alias_defs,
            ));
        }
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

pub(in crate::algorithms::taint) fn js_ts_request_source_access_ranges_for_alias_on_line(
    parsed: &ParsedFile,
    line: usize,
    alias: &str,
) -> Vec<(usize, usize)> {
    let Some(framework) = parsed.framework().map(|spec| spec.name) else {
        return Vec::new();
    };
    if !matches!(framework, "nestjs" | "fastify" | "express" | "koa") {
        return Vec::new();
    }

    let mut ranges = Vec::new();
    let framework_receivers = js_ts_framework_receiver_names(parsed, framework);
    for func in parsed.all_functions() {
        if !node_contains_line(&func, line) {
            continue;
        }
        let params = js_ts_function_params(parsed, &func);
        if params.is_empty() {
            continue;
        }
        let source_params =
            js_ts_framework_source_params(parsed, &func, framework, &params, &framework_receivers);
        if source_params.is_empty() {
            continue;
        }
        let func_start_line = func.start_position().row + 1;
        let mut request_alias_defs = BTreeMap::new();
        for param in &source_params {
            js_ts_add_alias_def(
                &mut request_alias_defs,
                param.clone(),
                js_ts_alias_def(func_start_line, None, None, None),
            );
        }
        let mut koa_request_object_alias_defs = BTreeMap::new();
        let mut assignment_sources = BTreeSet::new();
        collect_js_ts_request_assignments(
            parsed,
            func,
            func.id(),
            framework,
            &mut request_alias_defs,
            &mut koa_request_object_alias_defs,
            &mut assignment_sources,
        );
        let is_koa_request_object_alias = framework == "koa"
            && koa_request_object_alias_defs
                .get(alias)
                .is_some_and(|defs| defs.iter().any(|def| def.visible_on(line)));
        let alias_ranges = js_ts_source_access_ranges_for_alias_from_defs(
            parsed,
            func,
            line,
            framework,
            alias,
            is_koa_request_object_alias,
            &request_alias_defs,
            &koa_request_object_alias_defs,
        );
        ranges.extend(alias_ranges);
    }
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn js_ts_alias_def_can_reach_range(
    parsed: &ParsedFile,
    def: &JsTsAliasDef,
    start_byte: usize,
    end_byte: usize,
) -> bool {
    def.start_byte.is_none_or(|def_start| {
        !js_ts_ranges_are_in_sibling_control_flow_branches(
            parsed.tree.root_node(),
            def_start,
            def_start,
            start_byte,
            end_byte,
        )
    })
}

pub(super) fn js_ts_ranges_are_in_sibling_control_flow_branches(
    node: Node<'_>,
    left_start: usize,
    left_end: usize,
    right_start: usize,
    right_end: usize,
) -> bool {
    if !node_contains_range(&node, left_start, left_end)
        || !node_contains_range(&node, right_start, right_end)
    {
        return false;
    }
    if matches!(
        node.kind(),
        "if_statement" | "if_expression" | "conditional_expression" | "ternary_expression"
    ) {
        let left_branch = js_ts_conditional_branch_containing_range(&node, left_start, left_end);
        let right_branch = js_ts_conditional_branch_containing_range(&node, right_start, right_end);
        if left_branch.is_some() && right_branch.is_some() && left_branch != right_branch {
            return true;
        }
    }
    if node.kind() == "switch_statement" {
        let left_case = js_ts_switch_case_containing_range(node, left_start, left_end);
        let right_case = js_ts_switch_case_containing_range(node, right_start, right_end);
        if let (Some(left_case), Some(right_case)) = (left_case, right_case) {
            if left_case.id() != right_case.id()
                && !js_ts_switch_cases_can_fall_through(node, left_case, right_case)
            {
                return true;
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if js_ts_ranges_are_in_sibling_control_flow_branches(
            child,
            left_start,
            left_end,
            right_start,
            right_end,
        ) {
            return true;
        }
    }
    false
}

fn js_ts_conditional_branch_containing_range(
    node: &Node<'_>,
    start_byte: usize,
    end_byte: usize,
) -> Option<&'static str> {
    let consequence = node.child_by_field_name("consequence");
    if consequence.is_some_and(|branch| node_contains_range(&branch, start_byte, end_byte)) {
        return Some("consequence");
    }
    let alternative = node.child_by_field_name("alternative");
    if alternative.is_some_and(|branch| node_contains_range(&branch, start_byte, end_byte)) {
        return Some("alternative");
    }
    None
}

fn js_ts_switch_case_containing_range<'a>(
    node: Node<'a>,
    start_byte: usize,
    end_byte: usize,
) -> Option<Node<'a>> {
    if matches!(node.kind(), "switch_case" | "switch_default")
        && node_contains_range(&node, start_byte, end_byte)
    {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if !node_contains_range(&child, start_byte, end_byte) {
            continue;
        }
        if let Some(case_node) = js_ts_switch_case_containing_range(child, start_byte, end_byte) {
            return Some(case_node);
        }
    }
    None
}

fn js_ts_switch_cases_can_fall_through(
    switch_node: Node<'_>,
    left_case: Node<'_>,
    right_case: Node<'_>,
) -> bool {
    if left_case.start_byte() >= right_case.start_byte() {
        return false;
    }

    let mut cases = Vec::new();
    collect_js_ts_switch_cases(switch_node, &mut cases);
    let Some(left_index) = cases
        .iter()
        .position(|case_node| case_node.id() == left_case.id())
    else {
        return false;
    };
    let Some(right_index) = cases
        .iter()
        .position(|case_node| case_node.id() == right_case.id())
    else {
        return false;
    };
    if left_index >= right_index {
        return false;
    }

    cases[left_index..right_index]
        .iter()
        .all(|case_node| js_ts_switch_case_can_fall_through(*case_node))
}

fn collect_js_ts_switch_cases<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if matches!(node.kind(), "switch_case" | "switch_default") {
        out.push(node);
        return;
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_switch_cases(child, out);
    }
}

fn js_ts_switch_case_can_fall_through(case_node: Node<'_>) -> bool {
    !case_node
        .named_child(case_node.named_child_count().saturating_sub(1))
        .is_some_and(js_ts_node_stops_switch_fallthrough)
}

fn js_ts_node_stops_switch_fallthrough(node: Node<'_>) -> bool {
    match node.kind() {
        "break_statement" | "continue_statement" | "return_statement" | "throw_statement" => true,
        "statement_block" => node
            .named_child(node.named_child_count().saturating_sub(1))
            .is_some_and(js_ts_node_stops_switch_fallthrough),
        "if_statement" | "if_expression" => {
            let Some(consequence) = node.child_by_field_name("consequence") else {
                return false;
            };
            let Some(alternative) = node.child_by_field_name("alternative") else {
                return false;
            };
            js_ts_node_stops_switch_fallthrough(consequence)
                && js_ts_node_stops_switch_fallthrough(alternative)
        }
        _ => false,
    }
}

pub(super) fn js_ts_source_access_ranges_for_alias_from_defs(
    parsed: &ParsedFile,
    func: Node<'_>,
    line: usize,
    framework: &str,
    alias: &str,
    is_koa_request_object_alias: bool,
    request_alias_defs: &JsTsAliasDefs,
    koa_request_object_alias_defs: &JsTsAliasDefs,
) -> Vec<(usize, usize)> {
    let mut request_aliases = BTreeSet::new();
    if request_alias_defs
        .get(alias)
        .is_some_and(|defs| defs.iter().any(|def| def.visible_on(line)))
    {
        request_aliases.insert(alias.to_string());
    }
    let mut koa_request_object_aliases = BTreeSet::new();
    if is_koa_request_object_alias
        && koa_request_object_alias_defs
            .get(alias)
            .is_some_and(|defs| defs.iter().any(|def| def.visible_on(line)))
    {
        koa_request_object_aliases.insert(alias.to_string());
    }
    if request_aliases.is_empty() && koa_request_object_aliases.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    collect_js_ts_source_access_ranges_on_line(
        parsed,
        func,
        func.id(),
        line,
        framework,
        &request_aliases,
        &koa_request_object_aliases,
        &mut ranges,
    );
    ranges.retain(|(start, end)| {
        request_alias_defs.get(alias).is_some_and(|defs| {
            defs.iter().any(|def| {
                def.visible_range(line, *start, *end)
                    && js_ts_alias_def_can_reach_range(parsed, def, *start, *end)
            })
        }) || (is_koa_request_object_alias
            && koa_request_object_alias_defs
                .get(alias)
                .is_some_and(|defs| {
                    defs.iter().any(|def| {
                        def.visible_range(line, *start, *end)
                            && js_ts_alias_def_can_reach_range(parsed, def, *start, *end)
                    })
                }))
    });
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn collect_js_ts_source_access_ranges_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_func_id: usize,
    line: usize,
    framework: &str,
    request_aliases: &BTreeSet<String>,
    koa_request_object_aliases: &BTreeSet<String>,
    out: &mut Vec<(usize, usize)>,
) {
    if !node_contains_line(&node, line) {
        return;
    }
    if node.id() != root_func_id && parsed.language.function_node_types().contains(&node.kind()) {
        return;
    }

    let start_line = node.start_position().row + 1;
    let is_source_access = if start_line == line {
        let text = parsed.node_text(&node);
        request_aliases
            .iter()
            .any(|alias| js_ts_source_access_text_matches(text, framework, alias))
            || (framework == "koa"
                && koa_request_object_aliases.iter().any(|alias| {
                    JS_TS_REQUEST_DATA_FIELDS
                        .iter()
                        .any(|field| js_ts_field_access_text_matches(text, alias, field))
                }))
    } else {
        false
    };
    if is_source_access && js_ts_source_access_range_node_kind(node.kind()) {
        out.push((node.start_byte(), node.end_byte()));
        return;
    }

    let before_children = out.len();
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_source_access_ranges_on_line(
            parsed,
            child,
            root_func_id,
            line,
            framework,
            request_aliases,
            koa_request_object_aliases,
            out,
        );
    }
    if out.len() > before_children {
        return;
    }

    if start_line == line {
        if is_source_access {
            out.push((node.start_byte(), node.end_byte()));
        }
    }
}

fn js_ts_source_access_range_node_kind(kind: &str) -> bool {
    matches!(kind, "member_expression" | "subscript_expression")
}

pub(in crate::algorithms::taint) fn js_ts_framework_source_target_ranges_by_line(
    files: &BTreeMap<String, ParsedFile>,
    framework_sources: &[TaintSeed],
) -> BTreeMap<(String, usize), Vec<(usize, usize)>> {
    let mut targets_by_line = BTreeMap::<(String, usize), BTreeSet<String>>::new();
    for source in framework_sources {
        let Some(target) = source.target.as_ref() else {
            continue;
        };
        let Some(parsed) = files.get(&source.file) else {
            continue;
        };
        if !is_js_ts_language(parsed.language) {
            continue;
        }
        targets_by_line
            .entry((source.file.clone(), source.line))
            .or_default()
            .insert(target.base.clone());
    }

    let mut out = BTreeMap::new();
    for ((file, line), targets) in targets_by_line {
        let Some(parsed) = files.get(&file) else {
            continue;
        };
        let ranges = js_ts_lhs_binding_ranges_for_line(parsed, line, &targets);
        if !ranges.is_empty() {
            out.insert((file, line), ranges);
        }
    }
    out
}

fn js_ts_lhs_binding_ranges_for_line(
    parsed: &ParsedFile,
    line: usize,
    targets: &BTreeSet<String>,
) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    collect_js_ts_lhs_binding_ranges_on_line(
        parsed,
        parsed.tree.root_node(),
        line,
        targets,
        &mut ranges,
    );
    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

fn collect_js_ts_lhs_binding_ranges_on_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    targets: &BTreeSet<String>,
    ranges: &mut Vec<(usize, usize)>,
) {
    if !node_contains_line(&node, line) {
        return;
    }
    if node.kind() == "variable_declarator" {
        if let Some(lhs) = node.child_by_field_name("name") {
            collect_js_ts_lhs_identifier_ranges(parsed, lhs, targets, ranges);
        }
        return;
    }
    if parsed.language.is_assignment_node(node.kind()) {
        if let Some(lhs) = parsed.language.assignment_target(&node) {
            collect_js_ts_lhs_identifier_ranges(parsed, lhs, targets, ranges);
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_lhs_binding_ranges_on_line(parsed, child, line, targets, ranges);
    }
}

fn collect_js_ts_lhs_identifier_ranges(
    parsed: &ParsedFile,
    node: Node<'_>,
    targets: &BTreeSet<String>,
    ranges: &mut Vec<(usize, usize)>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            if targets.contains(parsed.node_text(&node)) {
                ranges.push((node.start_byte(), node.end_byte()));
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_identifier_ranges(parsed, left, targets, ranges);
            }
        }
        "member_expression" => {}
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_identifier_ranges(parsed, child, targets, ranges);
            }
        }
    }
}
