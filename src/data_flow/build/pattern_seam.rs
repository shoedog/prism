//! The legacy endpoint exception for a signature closure and body on one line.

use super::{pattern_execution, pattern_formals::decoded_name};
use crate::access_path::AccessPath;
use crate::ast::member_reference_index::MemberReferenceSearch;
use crate::ast::ParsedFile;
use crate::data_flow::{BindingKey, ExactFlowEdge, FlowEdge, VarLocation};
use std::collections::BTreeMap;
use tree_sitter::Node;

#[allow(clippy::too_many_arguments)]
pub(super) fn record_relocation<'a>(
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    refs: &MemberReferenceSearch<'a>,
    from: &VarLocation,
    to: &VarLocation,
    def_byte: usize,
    body: Option<(usize, usize)>,
    refused: impl Fn(&AccessPath, usize) -> bool,
    moves: &mut BTreeMap<ExactFlowEdge, VarLocation>,
) {
    if !relocate(parsed, owner, &to.path, to.line, Some(to), body) {
        return;
    }
    let body_reads = crate::profile::search_refs(
        parsed, refs, &owner, &to.path, from.line, def_byte, body, true,
    );
    if let Some(&(start_byte, end_byte)) = body_reads.get(&to.line) {
        if !refused(&to.path, start_byte) {
            moves.insert(
                ExactFlowEdge::from_refs(from, to),
                VarLocation {
                    start_byte,
                    end_byte,
                    ..to.clone()
                },
            );
        }
    }
}

pub(super) fn apply_relocations(
    edges: &mut [FlowEdge],
    uses: &mut BTreeMap<BindingKey, Vec<VarLocation>>,
    moves: BTreeMap<ExactFlowEdge, VarLocation>,
) {
    if moves.is_empty() {
        return;
    }
    for edge in edges {
        if let Some(to) = moves.get(&ExactFlowEdge::from_legacy(edge)) {
            edge.to = to.clone();
        }
    }
    for to in moves.into_values() {
        uses.entry((
            to.file.clone(),
            to.function.clone(),
            to.function_start_line,
            to.path.clone(),
        ))
        .or_default()
        .push(to);
    }
}

pub(super) fn relocate(
    parsed: &ParsedFile,
    owner: Node<'_>,
    path: &AccessPath,
    line: usize,
    preferred: Option<&VarLocation>,
    body: Option<(usize, usize)>,
) -> bool {
    let Some((body_start, _)) = body else {
        return false;
    };
    let Some(params) = owner.child_by_field_name("parameters") else {
        return false;
    };
    let start = preferred.map_or_else(|| parsed.line_start_byte(line), |loc| loc.start_byte);
    let end = preferred.map_or(start, |loc| loc.end_byte);
    if start >= body_start
        || end > body_start
        || parsed.line_for_byte(body_start) != line
        || !path.is_simple()
    {
        return false;
    }
    let at = if start == end {
        // A fallback anchor may be indented before the closure's operand.
        start + parsed.source[start..body_start].len()
            - parsed.source[start..body_start]
                .trim_start_matches([' ', '\t'])
                .len()
    } else {
        start
    };
    if !params.byte_range().contains(&at) {
        return false;
    }
    let Some(operand) = params.descendant_for_byte_range(at, at + 1) else {
        return false;
    };
    let mut child = operand;
    while let Some(parent) = child.parent().filter(|&node| node != params) {
        if parsed
            .language
            .callable_boundary_node_types()
            .contains(&parent.kind())
        {
            return parent.child_by_field_name("body") == Some(child)
                && witness(parsed, owner, child, path, line, preferred);
        }
        child = parent;
    }
    false
}

fn witness(
    parsed: &ParsedFile,
    owner: Node<'_>,
    closure_body: Node<'_>,
    path: &AccessPath,
    line: usize,
    preferred: Option<&VarLocation>,
) -> bool {
    let mut pending = vec![closure_body];
    while let Some(node) = pending.pop() {
        if !pattern_execution::runtime_node(parsed, node)
            || parsed.line_for_byte(node.start_byte()) > line
            || parsed.line_for_byte(node.end_byte()) < line
        {
            continue;
        }
        if node.kind() == "identifier"
            && parsed.line_for_byte(node.start_byte()) == line
            && preferred.is_none_or(|loc| {
                loc.start_byte == loc.end_byte
                    || node.byte_range() == (loc.start_byte..loc.end_byte)
            })
            && decoded_name(parsed.node_text(&node)).as_deref() == Some(path.base.as_str())
            && !parsed.js_ts_identifier_is_not_a_read(node)
            && pattern_execution::binding_scope(parsed, owner, node, &path.base) == Some(owner.id())
        {
            return true;
        }
        pending.extend(node.named_children(&mut node.walk()));
    }
    false
}
