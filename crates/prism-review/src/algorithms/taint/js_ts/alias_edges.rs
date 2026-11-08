//! JS/TS assignment-alias flow-edge synthesis.

use crate::algorithms::taint::js_ts::bindings::{
    js_ts_assignment_effective_byte_scope, js_ts_assignment_effective_line_scope,
    js_ts_enclosing_function_id,
};
use crate::algorithms::taint::js_ts::framework_params::{
    js_ts_framework_source_params, js_ts_function_params,
};
use crate::algorithms::taint::js_ts::request_aliases::{
    js_ts_active_alias_names_at_range, js_ts_add_alias_def, js_ts_alias_def,
    js_ts_kill_simple_lhs_alias_defs_at, js_ts_line_in_scope, js_ts_seed_scope_contains,
};
use crate::algorithms::taint::js_ts::source_access::{
    js_ts_exact_field_access_text_matches,
    node_contains_js_ts_request_object_source_access_on_line,
    node_contains_js_ts_source_access_on_line,
    node_contains_js_ts_source_access_on_line_with_request_object_aliases,
    node_contains_js_ts_source_access_with_request_object_aliases,
};
use crate::algorithms::taint::js_ts::source_ranges::js_ts_request_source_access_ranges_for_alias_on_line;
use crate::algorithms::taint::seeds::reference_line_cfg_reachable;
use crate::algorithms::taint::sink_match::{assignment_lhs_identifiers, node_contains_identifier};
use crate::algorithms::taint::{JsTsAliasDefs, TargetSeedSynthesisContext};
use prism::access_path::AccessPath;
use prism::ast::tree_helpers::{is_js_ts_language, unwrap_parenthesized};
use prism::ast::ParsedFile;
use prism::data_flow::{FlowEdge, VarAccessKind, VarLocation};
use prism::frameworks::js_ts::call_classify::{
    collect_js_ts_assignment_like_nodes, js_ts_assignment_target_and_value,
    js_ts_framework_receiver_names,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

pub(in crate::algorithms::taint) fn js_ts_variable_reference_ranges_scoped(
    parsed: &ParsedFile,
    func: Node<'_>,
    var_name: &str,
    def_line: usize,
) -> Vec<(usize, usize, usize)> {
    let scoped_lines = parsed.find_variable_references_scoped(&func, var_name, def_line);
    let mut refs = Vec::new();
    collect_js_ts_identifier_reference_ranges(parsed, func, var_name, &scoped_lines, &mut refs);
    refs.sort_unstable();
    refs.dedup();
    refs
}

fn collect_js_ts_identifier_reference_ranges(
    parsed: &ParsedFile,
    node: Node<'_>,
    var_name: &str,
    scoped_lines: &BTreeSet<usize>,
    out: &mut Vec<(usize, usize, usize)>,
) {
    let line = node.start_position().row + 1;
    if scoped_lines.contains(&line)
        && node.kind() == "identifier"
        && parsed.node_text(&node) == var_name
    {
        out.push((line, node.start_byte(), node.end_byte()));
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_js_ts_identifier_reference_ranges(parsed, child, var_name, scoped_lines, out);
    }
}

pub(in crate::algorithms::taint) fn synthesize_js_ts_assignment_alias_edges(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
) {
    if !is_js_ts_language(ctx.parsed.language) {
        return;
    }

    let mut assignments = Vec::new();
    collect_js_ts_assignment_like_nodes(ctx.func, ctx.parsed, &mut assignments);
    assignments.sort_by_key(|node| node.start_byte());

    let mut request_alias_defs = BTreeMap::new();
    let mut koa_request_object_alias_defs = BTreeMap::new();
    let js_ts_request_source = js_ts_request_source_seed_framework_and_params(ctx);
    let mut alias_defs = BTreeMap::new();
    if js_ts_request_source.is_none() {
        js_ts_add_alias_def(
            &mut alias_defs,
            ctx.target.base.clone(),
            js_ts_alias_def(
                ctx.seed.line,
                ctx.seed.start_byte,
                ctx.seed.scope,
                ctx.seed.byte_scope,
            ),
        );
    }
    if let Some((framework, _)) = &js_ts_request_source {
        if *framework == "koa" && js_ts_seed_is_koa_request_object_alias(ctx) {
            js_ts_add_alias_def(
                &mut koa_request_object_alias_defs,
                ctx.target.base.clone(),
                js_ts_alias_def(
                    ctx.seed.line,
                    ctx.seed.start_byte,
                    ctx.seed.scope,
                    ctx.seed.byte_scope,
                ),
            );
        } else {
            js_ts_add_alias_def(
                &mut request_alias_defs,
                ctx.target.base.clone(),
                js_ts_alias_def(
                    ctx.seed.line,
                    ctx.seed.start_byte,
                    ctx.seed.scope,
                    ctx.seed.byte_scope,
                ),
            );
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for assignment in &assignments {
            let assignment_line = assignment.start_position().row + 1;
            if assignment_line < ctx.seed.line
                || (assignment_line == ctx.seed.line
                    && ctx
                        .seed
                        .start_byte
                        .is_some_and(|seed_start| assignment.start_byte() <= seed_start))
            {
                continue;
            }
            if !js_ts_seed_scope_contains(ctx.seed, assignment_line) {
                continue;
            }
            if js_ts_enclosing_function_id(ctx.parsed, assignment) != Some(ctx.func.id()) {
                continue;
            }
            if let Some(cfg_set) = ctx.reachable {
                if !reference_line_cfg_reachable(
                    ctx.parsed,
                    &ctx.func,
                    &ctx.seed.file,
                    assignment_line,
                    cfg_set,
                ) {
                    continue;
                }
            }
            let Some((lhs, rhs)) = js_ts_assignment_target_and_value(ctx.parsed, assignment) else {
                continue;
            };
            let alias_scope =
                js_ts_assignment_effective_line_scope(ctx.parsed, assignment, ctx.func.id());
            let alias_byte_scope =
                js_ts_assignment_effective_byte_scope(ctx.parsed, assignment, ctx.func.id());
            let alias_def = js_ts_alias_def(
                assignment_line,
                Some(lhs.start_byte()),
                alias_scope,
                alias_byte_scope,
            );
            if let Some((framework, source_params)) = &js_ts_request_source {
                let request_aliases = js_ts_active_alias_names_at_range(
                    &request_alias_defs,
                    assignment_line,
                    rhs.start_byte(),
                    rhs.end_byte(),
                )
                .into_iter()
                .chain(source_params.iter().cloned())
                .collect::<BTreeSet<_>>();
                let koa_request_object_aliases = js_ts_active_alias_names_at_range(
                    &koa_request_object_alias_defs,
                    assignment_line,
                    rhs.start_byte(),
                    rhs.end_byte(),
                );
                if js_ts_rhs_is_bare_alias(ctx.parsed, &rhs, &request_aliases) {
                    if let Some(alias) = js_ts_simple_lhs_identifier(ctx.parsed, &lhs) {
                        if alias != "_"
                            && js_ts_add_alias_def(
                                &mut request_alias_defs,
                                alias.clone(),
                                alias_def,
                            )
                        {
                            add_js_ts_request_alias_reference_edges(
                                ctx,
                                edges,
                                &alias,
                                assignment_line,
                                alias_scope,
                                framework,
                                &source_params
                                    .iter()
                                    .cloned()
                                    .chain(request_aliases)
                                    .chain(std::iter::once(alias.clone()))
                                    .collect(),
                                &koa_request_object_aliases,
                            );
                            changed = true;
                        }
                    }
                    continue;
                }
                if *framework == "koa"
                    && (js_ts_rhs_is_bare_alias(ctx.parsed, &rhs, &koa_request_object_aliases)
                        || js_ts_rhs_is_koa_request_object_alias(
                            ctx.parsed,
                            &rhs,
                            &request_aliases,
                        ))
                {
                    if let Some(alias) = js_ts_simple_lhs_identifier(ctx.parsed, &lhs) {
                        if alias != "_"
                            && js_ts_add_alias_def(
                                &mut koa_request_object_alias_defs,
                                alias.clone(),
                                alias_def,
                            )
                        {
                            add_js_ts_request_alias_reference_edges(
                                ctx,
                                edges,
                                &alias,
                                assignment_line,
                                alias_scope,
                                framework,
                                &request_aliases,
                                &source_params
                                    .iter()
                                    .cloned()
                                    .chain(koa_request_object_aliases)
                                    .chain(std::iter::once(alias.clone()))
                                    .collect(),
                            );
                            changed = true;
                        }
                    }
                    continue;
                }
                if node_contains_js_ts_source_access_with_request_object_aliases(
                    ctx.parsed,
                    rhs,
                    framework,
                    &request_aliases,
                    &koa_request_object_aliases,
                ) {
                    continue;
                }
                js_ts_kill_simple_lhs_alias_defs_at(
                    ctx.parsed,
                    &lhs,
                    assignment,
                    assignment_line,
                    ctx.func.id(),
                    &mut request_alias_defs,
                    &mut koa_request_object_alias_defs,
                );
            }
            let rhs_uses_alias = alias_defs.iter().any(|(alias, alias_defs)| {
                alias_defs.iter().any(|alias_def| {
                    alias_def.visible_range(assignment_line, rhs.start_byte(), rhs.end_byte())
                        && node_contains_identifier(ctx.parsed, &rhs, alias)
                })
            });
            if !rhs_uses_alias {
                continue;
            }
            if add_js_ts_tainted_assignment_aliases(
                ctx,
                edges,
                &lhs,
                assignment,
                assignment_line,
                &mut alias_defs,
            ) {
                changed = true;
            }
        }
    }
}

fn add_js_ts_tainted_assignment_aliases(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
    lhs: &Node<'_>,
    binding_node: &Node<'_>,
    assignment_line: usize,
    alias_defs: &mut JsTsAliasDefs,
) -> bool {
    let mut added = false;
    let alias_scope =
        js_ts_assignment_effective_line_scope(ctx.parsed, binding_node, ctx.func.id());
    let alias_byte_scope =
        js_ts_assignment_effective_byte_scope(ctx.parsed, binding_node, ctx.func.id());
    for alias in assignment_lhs_identifiers(ctx.parsed, lhs) {
        if alias == "_" {
            continue;
        }
        let alias_def = js_ts_alias_def(
            assignment_line,
            Some(lhs.start_byte()),
            alias_scope,
            alias_byte_scope,
        );
        if !js_ts_add_alias_def(alias_defs, alias.clone(), alias_def.clone()) {
            continue;
        }
        added = true;
        if !edges.iter().any(|edge| {
            edge.to.file == ctx.seed.file
                && edge.to.line == assignment_line
                && edge.to.var_name() == alias
        }) {
            edges.push(FlowEdge {
                from: ctx.from.clone(),
                to: VarLocation {
                    file: ctx.seed.file.clone(),
                    function: ctx.func_name.to_string(),
                    function_start_line: ctx.func.start_position().row + 1,
                    line: assignment_line,
                    path: AccessPath::simple(alias.clone()),
                    start_byte: 0,
                    end_byte: 0,
                    kind: VarAccessKind::Def,
                },
            });
        }
        let refs = ctx
            .parsed
            .find_variable_references_scoped(&ctx.func, &alias, assignment_line);
        for ref_line in refs {
            if ref_line <= assignment_line {
                continue;
            }
            if !js_ts_seed_scope_contains(ctx.seed, ref_line) || !alias_def.visible_on(ref_line) {
                continue;
            }
            if let Some(cfg_set) = ctx.reachable {
                if !reference_line_cfg_reachable(
                    ctx.parsed,
                    &ctx.func,
                    &ctx.seed.file,
                    ref_line,
                    cfg_set,
                ) {
                    continue;
                }
            }
            if edges.iter().any(|edge| {
                edge.to.file == ctx.seed.file
                    && edge.to.line == ref_line
                    && edge.to.var_name() == alias
            }) {
                continue;
            }
            edges.push(FlowEdge {
                from: ctx.from.clone(),
                to: VarLocation {
                    file: ctx.seed.file.clone(),
                    function: ctx.func_name.to_string(),
                    function_start_line: ctx.func.start_position().row + 1,
                    line: ref_line,
                    path: AccessPath::simple(alias.clone()),
                    start_byte: 0,
                    end_byte: 0,
                    kind: VarAccessKind::Use,
                },
            });
        }
    }
    added
}

fn add_js_ts_request_alias_reference_edges(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
    alias: &str,
    assignment_line: usize,
    alias_scope: Option<(usize, usize)>,
    framework: &str,
    request_aliases: &BTreeSet<String>,
    koa_request_object_aliases: &BTreeSet<String>,
) {
    let refs = ctx
        .parsed
        .find_variable_references_scoped(&ctx.func, alias, assignment_line);
    for ref_line in refs {
        if ref_line < assignment_line {
            continue;
        }
        if !js_ts_seed_scope_contains(ctx.seed, ref_line)
            || !js_ts_line_in_scope(alias_scope, ref_line)
        {
            continue;
        }
        if let Some(cfg_set) = ctx.reachable {
            if !reference_line_cfg_reachable(
                ctx.parsed,
                &ctx.func,
                &ctx.seed.file,
                ref_line,
                cfg_set,
            ) {
                continue;
            }
        }
        if !node_contains_js_ts_source_access_on_line_with_request_object_aliases(
            ctx.parsed,
            ctx.func,
            ctx.func.id(),
            ref_line,
            framework,
            request_aliases,
            koa_request_object_aliases,
        ) {
            continue;
        }
        let source_ranges =
            js_ts_request_source_access_ranges_for_alias_on_line(ctx.parsed, ref_line, alias);
        if source_ranges.is_empty() {
            continue;
        }
        if edges.iter().any(|edge| {
            edge.to.file == ctx.seed.file && edge.to.line == ref_line && edge.to.var_name() == alias
        }) {
            continue;
        }
        edges.push(FlowEdge {
            from: ctx.from.clone(),
            to: VarLocation {
                file: ctx.seed.file.clone(),
                function: ctx.func_name.to_string(),
                function_start_line: ctx.func.start_position().row + 1,
                line: ref_line,
                path: AccessPath::simple(alias.to_string()),
                start_byte: 0,
                end_byte: 0,
                kind: VarAccessKind::Use,
            },
        });
    }
}

pub(in crate::algorithms::taint) fn js_ts_request_source_seed_framework_and_params(
    ctx: &TargetSeedSynthesisContext<'_>,
) -> Option<(&'static str, BTreeSet<String>)> {
    if !ctx.target.is_simple() {
        return None;
    }
    let framework = ctx.parsed.framework()?.name;
    if !matches!(framework, "fastify" | "express" | "koa") {
        return None;
    }
    let params = js_ts_function_params(ctx.parsed, &ctx.func);
    if params.is_empty() {
        return None;
    }
    let framework_receivers = js_ts_framework_receiver_names(ctx.parsed, framework);
    let source_params = js_ts_framework_source_params(
        ctx.parsed,
        &ctx.func,
        framework,
        &params,
        &framework_receivers,
    );
    if source_params.contains(&ctx.target.base)
        && ctx.seed.line == ctx.func.start_position().row + 1
    {
        return Some((framework, source_params));
    }
    let target_aliases = std::iter::once(ctx.target.base.clone()).collect::<BTreeSet<_>>();
    if framework == "koa"
        && node_contains_js_ts_request_object_source_access_on_line(
            ctx.parsed,
            ctx.func,
            ctx.func.id(),
            ctx.seed.line,
            &target_aliases,
        )
    {
        return Some((framework, source_params));
    }
    if !node_contains_js_ts_source_access_on_line(
        ctx.parsed,
        ctx.func,
        ctx.func.id(),
        ctx.seed.line,
        framework,
        &target_aliases,
    ) {
        return None;
    }
    let source_aliases = source_params
        .iter()
        .cloned()
        .chain(target_aliases)
        .collect::<BTreeSet<_>>();
    Some((framework, source_aliases))
}

pub(in crate::algorithms::taint) fn js_ts_seed_is_koa_request_object_alias(
    ctx: &TargetSeedSynthesisContext<'_>,
) -> bool {
    if ctx.parsed.framework().map(|spec| spec.name) != Some("koa") {
        return false;
    }
    let target_aliases = std::iter::once(ctx.target.base.clone()).collect::<BTreeSet<_>>();
    node_contains_js_ts_request_object_source_access_on_line(
        ctx.parsed,
        ctx.func,
        ctx.func.id(),
        ctx.seed.line,
        &target_aliases,
    )
}

pub(super) fn js_ts_rhs_is_bare_alias(
    parsed: &ParsedFile,
    rhs: &Node<'_>,
    aliases: &BTreeSet<String>,
) -> bool {
    let rhs = unwrap_js_ts_alias_rhs(*rhs);
    rhs.kind() == "identifier" && aliases.contains(parsed.node_text(&rhs))
}

pub(super) fn js_ts_rhs_is_koa_request_object_alias(
    parsed: &ParsedFile,
    rhs: &Node<'_>,
    context_aliases: &BTreeSet<String>,
) -> bool {
    let rhs = unwrap_js_ts_alias_rhs(*rhs);
    let text = parsed.node_text(&rhs).trim();
    context_aliases
        .iter()
        .any(|alias| js_ts_exact_field_access_text_matches(text, alias, "request"))
}

fn unwrap_js_ts_alias_rhs(mut node: Node<'_>) -> Node<'_> {
    loop {
        let unwrapped = unwrap_parenthesized(node);
        if unwrapped.id() != node.id() {
            node = unwrapped;
            continue;
        }

        let next = match node.kind() {
            "as_expression" | "satisfies_expression" => node
                .child_by_field_name("left")
                .or_else(|| node.child_by_field_name("value"))
                .or_else(|| node.named_child(0)),
            "non_null_expression" => node
                .child_by_field_name("argument")
                .or_else(|| node.named_child(0)),
            "type_assertion" => node
                .child_by_field_name("expression")
                .or_else(|| node.child_by_field_name("value"))
                .or_else(|| node.named_child(node.named_child_count().saturating_sub(1))),
            _ => None,
        };

        match next {
            Some(next) if next.id() != node.id() => node = next,
            _ => return node,
        }
    }
}

pub(super) fn js_ts_simple_lhs_identifier(parsed: &ParsedFile, lhs: &Node<'_>) -> Option<String> {
    (lhs.kind() == "identifier").then(|| parsed.node_text(lhs).to_string())
}
