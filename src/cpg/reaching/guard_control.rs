//! #660 S1d-2: the one-node guard `first_hazard` (#657) as the trace's control.
//!
//! The guard no longer decides availability: `cfg::js_single::single_node_cfg_is_complete` admits a
//! JS-family one-node owner with distinct recorded bases, and the census, D3, the refusal classes and
//! the capture regions label what the guard used to refuse. With `PRISM_RD_CENSUS_TRACE=1` the old
//! predicate is evaluated here, once per one-node owner, so a measurement can name the owners whose
//! availability flipped (`guard-deleted:<hazard>`). Nothing here runs with the trace off.
use super::super::{FlowConfidence, FlowDoubt};
use super::{
    census_trace, deduplicate_definitions, rd_statements, DefSite, FileCfgEdgeIndex, RdResult,
};
use crate::ast::ParsedFile;
use crate::data_flow::{ExactFlowEdge, FlowEdge};
use std::collections::BTreeMap;
use tree_sitter::Node;

/// `rd-census-owner\tguard-deleted:<hazard>\t<file>\t<owner>\t<line>` once per flipped owner, and
/// `guard-deleted:<hazard>` for every edge of a one-node owner the old guard refused and whose
/// label is no longer `NameOnly(CfgIncomplete)` (the label the refusal gave it). Silent for an owner
/// the guard admitted, for one with CFG edges, and for one the belt refuses.
pub(super) fn trace(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    (dfg_edges, exact_edges): (&[FlowEdge], &[FlowEdge]),
    cfg: &FileCfgEdgeIndex<'_>,
    head: &RdResult,
    out: &mut impl std::io::Write,
) {
    let defs = deduplicate_definitions(defs);
    let (statements, _) = rd_statements(parsed, owner);
    let lines: BTreeMap<_, _> = statements
        .iter()
        .enumerate()
        .map(|(i, (line, _))| (*line, i + 1))
        .collect();
    let bases: Vec<&str> = defs.iter().map(|d| d.path.base.as_str()).collect();
    if cfg.has_owner_edges(&lines)
        || !crate::cfg::js_single::single_node_cfg_is_complete(parsed, lines.len(), &bases)
    {
        return;
    }
    let Some(hazard) = first_hazard(parsed, owner, &bases) else {
        return;
    };
    let mechanism = format!("guard-deleted:{hazard}");
    // One line per flipped owner, moved edges or not, for the availability-counter check
    // (`dfg_rd_functions_without_cfg` falls by exactly the number of these).
    let _ = writeln!(
        out,
        "rd-census-owner\t{mechanism}\t{}\t{}\t{}",
        parsed.path,
        parsed.dfg_owner_name(owner).unwrap_or_default(),
        owner.start_position().row + 1
    );
    let refused = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    for edge in dfg_edges {
        let key = (edge.from.clone(), edge.to.clone());
        if head.labels.get(&key).is_some_and(|l| *l != refused) {
            census_trace::emit(out, &mechanism, edge);
        }
    }
    for edge in exact_edges {
        let key = ExactFlowEdge::from_legacy(edge);
        if head.exact_labels.get(&key).is_some_and(|l| *l != refused) {
            census_trace::emit(out, &mechanism, edge);
        }
    }
}

/// Conservative hazards for the one-node path. The def census records no def
/// for these, so distinct bases would admit an owner whose formal is
/// overwritten or whose read is deferred. Refusing too much only keeps main's
/// label. Pre-existing multi-statement owners are not touched (#660).
///
/// - (a) an update expression (`x++`, `--x`), anywhere in the owner, its own
///   parameter defaults included;
/// - (b) a default inside an assignment pattern (`[a = 1] = ..`,
///   `({ a = 1 } = ..)`), anywhere except the owner's own formal
///   wrappers (their default values are walked);
/// - (c) a function declaration in the owner's own scope named like a def
///   (hoisted: it replaces the formal at entry);
/// - (d) `arguments` in the owner or in an arrow nested in it (sloppy-mode
///   mapped arguments alias the formals);
/// - (e) a direct `eval(..)` call (it can write any local), found by
///   `js_ts_direct_eval_anywhere`;
/// - (f) a nested callable with a parameter default (the default is a read
///   that runs when the callable runs: a deferred capture the B8 metadata
///   does not collect, which only walks nested bodies).
pub(super) fn first_hazard(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    def_bases: &[&str],
) -> Option<char> {
    // A direct eval anywhere in the owner (own defaults and nested callables
    // included) can write any local. The shared detector handles parentheses,
    // TS wrappers, comments and escaped identifiers; indirect and optional
    // calls are not direct.
    if parsed.js_ts_direct_eval_anywhere(owner) {
        return Some('e');
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    let own_params = owner
        .child_by_field_name("parameters")
        .or_else(|| owner.child_by_field_name("parameter"))
        .map(|node| node.id());
    // (node, inside a nested callable, `arguments` is the owner's, inside the
    // owner's own parameter list)
    let mut stack = vec![(*owner, false, true, false)];
    while let Some((node, nested, own_args, in_params)) = stack.pop() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            // The owner's own formal binding/default wrappers are not hazards
            // (`b = 1` is an assignment pattern); their default values are
            // walked like any expression. Defaults make the list non-simple,
            // so `arguments` there is unmapped.
            let wrapper = in_params && !nested;
            match kind {
                "update_expression" => return Some('a'),
                "assignment_pattern" | "object_assignment_pattern" if !wrapper => {
                    return Some(if nested { 'f' } else { 'b' })
                }
                "required_parameter" | "optional_parameter"
                    if !wrapper && child.child_by_field_name("value").is_some() =>
                {
                    return Some('f');
                }
                "identifier" if own_args && parsed.node_text(&child) == "arguments" => {
                    return Some('d');
                }
                "function_declaration" | "generator_function_declaration"
                    if !nested
                        && child
                            .child_by_field_name("name")
                            .is_some_and(|name| def_bases.contains(&parsed.node_text(&name))) =>
                {
                    return Some('c');
                }
                _ => {}
            }
            let boundary = boundaries.contains(&kind);
            let params_here = Some(child.id()) == own_params && node.id() == owner.id();
            // The default value of an own formal is an ordinary expression: leave
            // the wrapper state so a pattern default written inside it counts.
            let default_value = in_params
                && !nested
                && matches!(
                    node.kind(),
                    "assignment_pattern"
                        | "object_assignment_pattern"
                        | "required_parameter"
                        | "optional_parameter"
                )
                && ["right", "value"]
                    .iter()
                    .any(|field| node.child_by_field_name(field) == Some(child));
            stack.push((
                child,
                nested || boundary,
                own_args && !params_here && (!boundary || kind == "arrow_function"),
                (in_params || params_here) && !default_value,
            ));
        }
    }
    None
}
