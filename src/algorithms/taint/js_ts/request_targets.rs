//! JS/TS request-data assignment and destructuring targets.

use crate::access_path::AccessPath;
use crate::algorithms::taint::js_ts::alias_edges::{
    js_ts_rhs_is_bare_alias, js_ts_rhs_is_koa_request_object_alias, js_ts_simple_lhs_identifier,
};
use crate::algorithms::taint::js_ts::bindings::{
    js_ts_assignment_effective_byte_scope, js_ts_assignment_effective_line_scope,
};
use crate::algorithms::taint::js_ts::framework_params::js_ts_alias_def;
use crate::algorithms::taint::js_ts::request_aliases::{
    js_ts_active_alias_names_at_range, js_ts_add_alias_def, js_ts_kill_simple_lhs_alias_defs_at,
    js_ts_request_source, js_ts_shrink_request_sources_after_unconditional_kill,
};
use crate::algorithms::taint::js_ts::source_access::{
    js_ts_normalized_property_key, js_ts_request_field_allowed,
    node_contains_js_ts_source_access_with_request_object_aliases, JS_TS_REQUEST_DATA_FIELDS,
};
use crate::algorithms::taint::js_ts::source_ranges::js_ts_source_access_ranges_for_alias_from_defs;
use crate::algorithms::taint::{JsTsAliasDef, JsTsAliasDefs, JsTsRequestDataSource};
use crate::ast::ParsedFile;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) fn collect_js_ts_request_assignment_targets(
    parsed: &ParsedFile,
    lhs: Node<'_>,
    rhs: Node<'_>,
    binding_node: Node<'_>,
    root_func_id: usize,
    framework: &str,
    request_alias_defs: &mut JsTsAliasDefs,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    let target_line = lhs.start_position().row + 1;
    let binding_scope = js_ts_assignment_effective_line_scope(parsed, &binding_node, root_func_id);
    let binding_byte_scope =
        js_ts_assignment_effective_byte_scope(parsed, &binding_node, root_func_id);
    let alias_def = js_ts_alias_def(
        target_line,
        Some(lhs.start_byte()),
        binding_scope,
        binding_byte_scope,
    );
    let request_aliases = js_ts_active_alias_names_at_range(
        request_alias_defs,
        target_line,
        rhs.start_byte(),
        rhs.end_byte(),
    );
    let koa_request_object_aliases = js_ts_active_alias_names_at_range(
        koa_request_object_alias_defs,
        target_line,
        rhs.start_byte(),
        rhs.end_byte(),
    );
    if js_ts_rhs_is_bare_alias(parsed, &rhs, &request_aliases) {
        let collected_request_data = collect_js_ts_request_destructure_targets(
            parsed,
            lhs,
            framework,
            target_line,
            binding_scope,
            binding_byte_scope,
            out,
        );
        let collected_koa_request_alias = framework == "koa"
            && collect_js_ts_koa_request_object_alias_destructure_targets(
                parsed,
                lhs,
                target_line,
                binding_scope,
                binding_byte_scope,
                koa_request_object_alias_defs,
                out,
                true,
            );
        if collected_request_data || collected_koa_request_alias {
            return;
        }
        if let Some(alias) = js_ts_simple_lhs_identifier(parsed, &lhs) {
            if alias != "_" {
                js_ts_add_alias_def(request_alias_defs, alias, alias_def);
            }
            return;
        }
    }
    if framework == "koa"
        && (js_ts_rhs_is_bare_alias(parsed, &rhs, &koa_request_object_aliases)
            || js_ts_rhs_is_koa_request_object_alias(parsed, &rhs, &request_aliases))
    {
        if collect_js_ts_request_object_destructure_targets(
            parsed,
            lhs,
            target_line,
            binding_scope,
            binding_byte_scope,
            out,
        ) {
            return;
        }
        if let Some(alias) = js_ts_simple_lhs_identifier(parsed, &lhs) {
            if alias != "_" {
                js_ts_add_alias_def(koa_request_object_alias_defs, alias, alias_def);
            }
        }
        return;
    }
    if !node_contains_js_ts_source_access_with_request_object_aliases(
        parsed,
        rhs,
        framework,
        &request_aliases,
        &koa_request_object_aliases,
    ) {
        let killed_aliases = js_ts_kill_simple_lhs_alias_defs_at(
            parsed,
            &lhs,
            &binding_node,
            target_line,
            root_func_id,
            request_alias_defs,
            koa_request_object_alias_defs,
        );
        js_ts_shrink_request_sources_after_unconditional_kill(
            parsed,
            &binding_node,
            root_func_id,
            &killed_aliases,
            target_line,
            lhs.start_byte(),
            out,
        );
        return;
    }
    collect_js_ts_lhs_targets(
        parsed,
        lhs,
        target_line,
        binding_scope,
        binding_byte_scope,
        out,
    );
}

fn collect_js_ts_lhs_targets(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                out.insert(js_ts_request_source(
                    line,
                    AccessPath::simple(name),
                    Some(node.start_byte()),
                    scope,
                    byte_scope,
                ));
            }
        }
        "member_expression" => {
            out.insert(js_ts_request_source(
                line,
                AccessPath::from_expr(parsed.node_text(&node)),
                Some(node.start_byte()),
                None,
                None,
            ));
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_targets(parsed, left, line, scope, byte_scope, out);
            }
        }
        "pair_pattern" => {
            if let Some(value) = node.child_by_field_name("value") {
                collect_js_ts_lhs_targets(parsed, value, line, scope, byte_scope, out);
            }
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_js_ts_lhs_targets(parsed, child, line, scope, byte_scope, out);
            }
        }
    }
}

pub(super) fn collect_js_ts_request_alias_source_uses(
    parsed: &ParsedFile,
    func: Node<'_>,
    framework: &str,
    request_alias_defs: &JsTsAliasDefs,
    koa_request_object_alias_defs: &JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    for (alias, alias_defs) in request_alias_defs {
        collect_js_ts_request_alias_source_uses_for_alias(
            parsed,
            func,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            alias,
            alias_defs,
            false,
            out,
        );
    }
    for (alias, alias_defs) in koa_request_object_alias_defs {
        collect_js_ts_request_alias_source_uses_for_alias(
            parsed,
            func,
            framework,
            request_alias_defs,
            koa_request_object_alias_defs,
            alias,
            alias_defs,
            true,
            out,
        );
    }
}

fn collect_js_ts_request_alias_source_uses_for_alias(
    parsed: &ParsedFile,
    func: Node<'_>,
    framework: &str,
    request_alias_defs: &JsTsAliasDefs,
    koa_request_object_alias_defs: &JsTsAliasDefs,
    alias: &str,
    alias_defs: &[JsTsAliasDef],
    is_koa_request_object_alias: bool,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) {
    for def in alias_defs {
        let refs = parsed.find_variable_references_scoped(&func, alias, def.line);
        for ref_line in refs {
            if ref_line < def.line || !def.visible_on(ref_line) {
                continue;
            }
            let source_ranges = js_ts_source_access_ranges_for_alias_from_defs(
                parsed,
                func,
                ref_line,
                framework,
                alias,
                is_koa_request_object_alias,
                request_alias_defs,
                koa_request_object_alias_defs,
            );
            if source_ranges.is_empty() {
                continue;
            }
            out.insert(js_ts_request_source(
                ref_line,
                AccessPath::simple(alias.to_string()),
                None,
                def.scope,
                def.byte_scope,
            ));
        }
    }
}

fn collect_js_ts_request_destructure_targets(
    parsed: &ParsedFile,
    pattern: Node<'_>,
    framework: &str,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) -> bool {
    if pattern.kind() != "object_pattern" {
        return false;
    }
    let before = out.len();
    let mut cursor = pattern.walk();
    for child in pattern.named_children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                let field = parsed.node_text(&child);
                if js_ts_request_field_allowed(framework, field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(child.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                let Some(left) = child
                    .child_by_field_name("left")
                    .or_else(|| child.named_child(0))
                else {
                    continue;
                };
                if !matches!(
                    left.kind(),
                    "identifier" | "shorthand_property_identifier_pattern"
                ) {
                    continue;
                }
                let field = parsed.node_text(&left);
                if js_ts_request_field_allowed(framework, field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(left.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "pair_pattern" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let Some(field) = js_ts_normalized_property_key(parsed, &key) else {
                    continue;
                };
                if !js_ts_request_field_allowed(framework, &field) {
                    continue;
                }
                if let Some(value) = child.child_by_field_name("value") {
                    collect_js_ts_lhs_targets(parsed, value, line, scope, byte_scope, out);
                }
            }
            "object_pattern" => {
                collect_js_ts_request_destructure_targets(
                    parsed, child, framework, line, scope, byte_scope, out,
                );
            }
            _ => {}
        }
    }
    out.len() > before
}

fn collect_js_ts_koa_request_object_alias_destructure_targets(
    parsed: &ParsedFile,
    pattern: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    koa_request_object_alias_defs: &mut JsTsAliasDefs,
    out: &mut BTreeSet<JsTsRequestDataSource>,
    allow_alias_defs: bool,
) -> bool {
    if pattern.kind() != "object_pattern" {
        return false;
    }
    let before = koa_request_object_alias_defs.len();
    let source_before = out.len();
    let mut cursor = pattern.walk();
    for child in pattern.named_children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                if allow_alias_defs && parsed.node_text(&child) == "request" {
                    js_ts_add_alias_def(
                        koa_request_object_alias_defs,
                        "request".to_string(),
                        js_ts_alias_def(line, Some(child.start_byte()), scope, byte_scope),
                    );
                }
            }
            "pair_pattern" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let Some(field) = js_ts_normalized_property_key(parsed, &key) else {
                    continue;
                };
                if field != "request" {
                    continue;
                }
                if let Some(value) = child.child_by_field_name("value") {
                    if value.kind() == "object_pattern" {
                        collect_js_ts_request_object_destructure_targets(
                            parsed, value, line, scope, byte_scope, out,
                        );
                    } else {
                        if allow_alias_defs {
                            collect_js_ts_lhs_simple_alias_defs(
                                parsed,
                                value,
                                line,
                                scope,
                                byte_scope,
                                koa_request_object_alias_defs,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
    koa_request_object_alias_defs.len() > before || out.len() > source_before
}

fn collect_js_ts_lhs_simple_alias_defs(
    parsed: &ParsedFile,
    node: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    defs: &mut JsTsAliasDefs,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => {
            let name = parsed.node_text(&node);
            if name != "_" {
                js_ts_add_alias_def(
                    defs,
                    name.to_string(),
                    js_ts_alias_def(line, Some(node.start_byte()), scope, byte_scope),
                );
            }
        }
        "assignment_pattern" | "object_assignment_pattern" => {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.named_child(0))
            {
                collect_js_ts_lhs_simple_alias_defs(parsed, left, line, scope, byte_scope, defs);
            }
        }
        _ => {}
    }
}

fn collect_js_ts_request_object_destructure_targets(
    parsed: &ParsedFile,
    pattern: Node<'_>,
    line: usize,
    scope: Option<(usize, usize)>,
    byte_scope: Option<(usize, usize)>,
    out: &mut BTreeSet<JsTsRequestDataSource>,
) -> bool {
    if pattern.kind() != "object_pattern" {
        return false;
    }
    let before = out.len();
    let mut cursor = pattern.walk();
    for child in pattern.named_children(&mut cursor) {
        match child.kind() {
            "shorthand_property_identifier_pattern" | "identifier" => {
                let field = parsed.node_text(&child);
                if JS_TS_REQUEST_DATA_FIELDS.contains(&field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(child.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                let Some(left) = child
                    .child_by_field_name("left")
                    .or_else(|| child.named_child(0))
                else {
                    continue;
                };
                if !matches!(
                    left.kind(),
                    "identifier" | "shorthand_property_identifier_pattern"
                ) {
                    continue;
                }
                let field = parsed.node_text(&left);
                if JS_TS_REQUEST_DATA_FIELDS.contains(&field) {
                    out.insert(js_ts_request_source(
                        line,
                        AccessPath::simple(field.to_string()),
                        Some(left.start_byte()),
                        scope,
                        byte_scope,
                    ));
                }
            }
            "pair_pattern" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let Some(field) = js_ts_normalized_property_key(parsed, &key) else {
                    continue;
                };
                if !JS_TS_REQUEST_DATA_FIELDS.contains(&field.as_str()) {
                    continue;
                }
                if let Some(value) = child.child_by_field_name("value") {
                    collect_js_ts_lhs_targets(parsed, value, line, scope, byte_scope, out);
                }
            }
            "object_pattern" => {
                collect_js_ts_request_object_destructure_targets(
                    parsed, child, line, scope, byte_scope, out,
                );
            }
            _ => {}
        }
    }
    out.len() > before
}
