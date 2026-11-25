//! Step 5b uses the least runtime position to bind elements of a JS-family rest parameter.

use super::{object_literal_args::SiteInputs, spread_args::Offsets, *};
use crate::ast::JsBindingCache;
use tree_sitter::Node;

#[path = "rest_yield.rs"]
mod rest_yield;

/// Locate the runtime implementation and verify the entry Def against the identifier's bytes.
pub(super) fn compute_rest_def_node(
    parsed: &ParsedFile,
    callee: &FunctionId,
    var_index: &VarIndex,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Option<(usize, NodeIndex)> {
    if !parsed.language.is_js_family() {
        return None;
    }
    let mut functions = parsed.all_functions().into_iter().filter(|node| {
        parsed.node_line_range(node) == (callee.start_line, callee.end_line)
            && node.child_by_field_name("body").is_some()
            && parsed.dfg_owner_name(node).as_deref() == Some(callee.name.as_str())
    });
    let implementation = functions.next()?;
    if functions.next().is_some() {
        return None;
    }
    let parameters = parsed.find_parameters_node(&implementation)?;
    if parameters.has_error() {
        return None;
    }
    let mut cursor = parameters.walk();
    let formals = parameters
        .named_children(&mut cursor)
        .filter(|node| !node.is_extra())
        .map(|node| node.child_by_field_name("pattern").unwrap_or(node))
        .filter(|node| node.kind() != "this");
    let (offset, rest) = formals.enumerate().last()?;
    let identifier = parsed.js_ts_rest_identifier(rest)?;
    let name = parsed.node_text(&identifier);
    let path = AccessPath::simple(name);
    let def = var_index.get_ref(
        graph,
        VarKeyRef {
            file: &callee.file,
            function: &callee.name,
            function_start_line: callee.start_line,
            line: callee.start_line,
            path: &path,
            access: VarAccess::Def,
        },
    )?;
    if !matches!(&graph[def], CpgNode::Variable { start_byte, end_byte, .. }
        if (*start_byte, *end_byte) == (identifier.start_byte(), identifier.end_byte()))
    {
        return None;
    }
    // This scopes writes to the formal, including captured rebinding and
    // parameter expressions. An indexed write and a write to a shadow differ.
    let mut bindings = JsBindingCache::for_sites();
    (!parsed.js_ts_direct_eval_anywhere(&implementation)
        && !parsed.js_ts_scoped_written(implementation, name, &mut bindings))
    .then_some((offset, def))
}

/// Admission for ordinary Step 5b rest edges.
pub(super) fn admitted(
    inputs: &SiteInputs<'_>,
    site: &CallSite,
    graph: &CallGraph,
    resolved: &ResolvedCallee<'_>,
    exact_singleton: bool,
) -> bool {
    inputs.caller_parsed.language.is_js_family()
        && inputs.callee_parsed.language.is_js_family()
        && !inputs.cross_file_free_single_exact
        && match resolved.confidence {
            ResolutionConfidence::Exact => {
                exact_singleton
                    && (resolved.kind != ResolutionKind::ImportQualified
                        || graph.js_ts_namespace_export_matches(site, resolved.target))
            }
            ResolutionConfidence::NameOnly => site.caller.file != resolved.target.file,
        }
}

pub(super) fn collect(
    out: &mut Vec<PendingArgumentFlow>,
    site: &SiteInputs<'_>,
    (rest_offset, to): (usize, NodeIndex),
    offsets: &Offsets,
    label: FlowConfidence,
) {
    let parsed = site.caller_parsed;
    let mut emit = |value: Node<'_>| {
        let Some(value) = parsed.js_ts_unwrap_value(value).filter(|value| {
            matches!(
                value.kind(),
                "identifier" | "this" | "member_expression" | "subscript_expression"
            )
        }) else {
            return;
        };
        for from in CodePropertyGraph::argument_nodes(
            site.caller_id,
            parsed,
            parsed.node_text(&value),
            &value.byte_range(),
            site.var_index,
            site.exact_var_index,
            site.graph,
        ) {
            if matches!(
                &site.graph[from],
                CpgNode::Variable {
                    access: VarAccess::Use,
                    ..
                }
            ) {
                out.push(PendingArgumentFlow {
                    edge: (from, to, CpgEdge::DataFlow(label)),
                    cross_file_free_single_exact: site.cross_file_free_single_exact,
                });
            }
        }
    };

    for ((text, span), &least) in site.args.iter().zip(&offsets.least) {
        if !text.starts_with("...") {
            if least >= rest_offset {
                if let Some(value) = parsed
                    .tree
                    .root_node()
                    .descendant_for_byte_range(span.start, span.end)
                    .filter(|value| value.byte_range() == *span)
                {
                    emit(value);
                }
            }
            continue;
        }
        let Some(operand) = parsed.spread_argument_operand(span) else {
            continue;
        };
        if spread_args::dense_array_length(operand).is_some() {
            let mut cursor = operand.walk();
            for value in operand
                .named_children(&mut cursor)
                .filter(|node| !node.is_extra())
                .skip(rest_offset.saturating_sub(least))
            {
                emit(value);
            }
        } else if least >= rest_offset {
            match rest_yield::at_spread(parsed, site.args, span, operand) {
                rest_yield::Yield::Nonempty => emit(operand),
                rest_yield::Yield::Unknown if label != FlowConfidence::Exact => emit(operand),
                rest_yield::Yield::Empty | rest_yield::Yield::Unknown => {}
            }
        }
    }
}
