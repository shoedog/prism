//! Framework source detection dispatch and target-scoped seed synthesis.

use crate::algorithms::taint::cleansing::node_contains_line;
use crate::algorithms::taint::go::sinks::detect_go_framework_sources;
use crate::algorithms::taint::js_ts::alias_edges::{
    js_ts_request_source_seed_framework_and_params, js_ts_seed_is_koa_request_object_alias,
    js_ts_variable_reference_ranges_scoped, synthesize_js_ts_assignment_alias_edges,
};
use crate::algorithms::taint::js_ts::framework_params::detect_js_ts_framework_sources;
use crate::algorithms::taint::js_ts::request_aliases::{
    js_ts_seed_scope_contains, js_ts_seed_scope_contains_range,
};
use crate::algorithms::taint::js_ts::source_access::node_contains_js_ts_source_access_on_line_with_request_object_aliases;
use crate::algorithms::taint::js_ts::source_ranges::js_ts_request_source_access_ranges_for_alias_on_line;
use crate::algorithms::taint::python::sources::{
    detect_python_framework_sources, python_is_inner_decorated_function,
};
use crate::algorithms::taint::{TaintSeed, TargetSeedSynthesisContext};
use prism::access_path::AccessPath;
use prism::ast::tree_helpers::is_js_ts_language;
use prism::ast::ParsedFile;
use prism::cpg::CpgContext;
use prism::data_flow::{FlowEdge, FlowPath, VarAccessKind, VarLocation};
use prism::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

pub(super) fn detect_framework_sources(ctx: &CpgContext) -> Vec<TaintSeed> {
    let mut sources: Vec<TaintSeed> = Vec::new();
    for (file_path, parsed) in ctx.files {
        match parsed.language {
            Language::Go => detect_go_framework_sources(file_path, parsed, &mut sources),
            Language::Python => detect_python_framework_sources(file_path, parsed, &mut sources),
            Language::JavaScript | Language::TypeScript | Language::Tsx => {
                detect_js_ts_framework_sources(file_path, parsed, &mut sources)
            }
            _ => {}
        }
    }
    sources.sort();
    sources.dedup();
    sources
}

pub(super) fn synthesize_target_seed_paths(
    seeds: &[TaintSeed],
    ctx: &CpgContext,
    paths: &mut Vec<FlowPath>,
) {
    for seed in seeds {
        let target = match &seed.target {
            Some(t) => t,
            None => continue,
        };
        let parsed = match ctx.files.get(&seed.file) {
            Some(p) => p,
            None => continue,
        };
        let func = match parsed.enclosing_function(seed.line) {
            Some(f) => f,
            None => continue,
        };
        let func =
            if parsed.language == Language::Python && python_is_inner_decorated_function(&func) {
                func.parent().unwrap_or(func)
            } else {
                func
            };
        let func_name = parsed
            .language
            .function_name(&func)
            .map(|n| parsed.node_text(&n).to_string())
            .unwrap_or_else(|| "<anonymous>".to_string());
        let reachable = if ctx.cpg.has_cfg_edges() {
            Some(ctx.cpg.cfg_reachable_lines(&seed.file, seed.line))
        } else {
            None
        };
        let from = VarLocation {
            file: seed.file.clone(),
            function: func_name.clone(),
            function_start_line: func.start_position().row + 1,
            line: seed.line,
            path: target.clone(),
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Def,
        };
        let mut edges = Vec::new();
        let reachable_dfg = ctx.cpg.dfg_forward_reachable(&from);
        for target_loc in reachable_dfg {
            if !js_ts_seed_scope_contains(seed, target_loc.line) {
                continue;
            }
            if target_loc.file == seed.file
                && target_loc.function == func_name
                && target_loc.function_start_line == from.function_start_line
                && target_loc.line <= seed.line
            {
                continue;
            }
            if let Some(cfg_set) = &reachable {
                if target_loc.file == seed.file
                    && target_loc.function == func_name
                    && target_loc.function_start_line == from.function_start_line
                    && !reference_line_cfg_reachable(
                        parsed,
                        &func,
                        &seed.file,
                        target_loc.line,
                        cfg_set,
                    )
                {
                    continue;
                }
            }
            edges.push(FlowEdge {
                from: from.clone(),
                to: target_loc,
            });
        }
        let synth_ctx = TargetSeedSynthesisContext {
            seed,
            parsed,
            func,
            func_name: &func_name,
            target,
            reachable: reachable.as_ref(),
            from: &from,
        };
        synthesize_js_ts_assignment_alias_edges(&synth_ctx, &mut edges);
        synthesize_direct_target_reference_edges(&synth_ctx, &mut edges);
        if !edges.is_empty() {
            paths.push(FlowPath {
                edges,
                cleansed_for: BTreeSet::new(),
            });
        }
    }
}

fn synthesize_direct_target_reference_edges(
    ctx: &TargetSeedSynthesisContext<'_>,
    edges: &mut Vec<FlowEdge>,
) {
    let js_ts_request_source =
        js_ts_request_source_seed_framework_and_params(ctx).map(|(framework, params)| {
            let koa_request_object_aliases =
                if framework == "koa" && js_ts_seed_is_koa_request_object_alias(ctx) {
                    std::iter::once(ctx.target.base.clone()).collect()
                } else {
                    BTreeSet::new()
                };
            (
                framework,
                std::iter::once(ctx.target.base.clone())
                    .chain(params)
                    .collect::<BTreeSet<_>>(),
                koa_request_object_aliases,
            )
        });
    let allow_same_line_refs = is_js_ts_language(ctx.parsed.language);
    let refs = if allow_same_line_refs {
        js_ts_variable_reference_ranges_scoped(
            ctx.parsed,
            ctx.func,
            &ctx.target.base,
            ctx.seed.line,
        )
    } else {
        ctx.parsed
            .find_variable_references_scoped(&ctx.func, &ctx.target.base, ctx.seed.line)
            .into_iter()
            .map(|line| (line, 0, usize::MAX))
            .collect()
    };
    for (ref_line, ref_start, ref_end) in refs {
        if ref_line < ctx.seed.line || (!allow_same_line_refs && ref_line == ctx.seed.line) {
            continue;
        }
        if allow_same_line_refs
            && ref_line == ctx.seed.line
            && ctx.seed.start_byte == Some(ref_start)
        {
            continue;
        }
        if !js_ts_seed_scope_contains_range(ctx.parsed, ctx.seed, ref_line, ref_start, ref_end) {
            continue;
        }
        if ref_line != ctx.seed.line {
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
        }
        if let Some((framework, source_params, koa_request_object_aliases)) = &js_ts_request_source
        {
            let source_ranges = js_ts_request_source_access_ranges_for_alias_on_line(
                ctx.parsed,
                ref_line,
                &ctx.target.base,
            );
            if source_ranges.is_empty() {
                continue;
            }
            if !node_contains_js_ts_source_access_on_line_with_request_object_aliases(
                ctx.parsed,
                ctx.func,
                ctx.func.id(),
                ref_line,
                framework,
                source_params,
                koa_request_object_aliases,
            ) {
                continue;
            }
        }
        if edges.iter().any(|edge| {
            edge.to.file == ctx.seed.file
                && edge.to.line == ref_line
                && edge.to.var_name() == ctx.target.base
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
                path: AccessPath::simple(ctx.target.base.clone()),
                start_byte: 0,
                end_byte: 0,
                kind: VarAccessKind::Use,
            },
        });
    }
}

pub(super) fn reference_line_cfg_reachable(
    parsed: &ParsedFile,
    func: &Node<'_>,
    file: &str,
    ref_line: usize,
    cfg_set: &BTreeSet<(String, usize)>,
) -> bool {
    if cfg_set.contains(&(file.to_string(), ref_line)) {
        return true;
    }
    reachable_multiline_node_contains_line(parsed, *func, file, ref_line, cfg_set)
}

fn reachable_multiline_node_contains_line(
    parsed: &ParsedFile,
    node: Node<'_>,
    file: &str,
    ref_line: usize,
    cfg_set: &BTreeSet<(String, usize)>,
) -> bool {
    if !node_contains_line(&node, ref_line) {
        return false;
    }

    let start_line = node.start_position().row + 1;
    if start_line < ref_line
        && cfg_set.contains(&(file.to_string(), start_line))
        && (parsed.language.is_call_node(node.kind())
            || parsed.language.is_assignment_node(node.kind())
            || matches!(node.kind(), "return_statement" | "expression_statement"))
    {
        return true;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if reachable_multiline_node_contains_line(parsed, child, file, ref_line, cfg_set) {
            return true;
        }
    }
    false
}
