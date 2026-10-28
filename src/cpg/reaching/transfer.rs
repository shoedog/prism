//! Edge-local transfer nodes for classified binding reuse constructs.
pub(super) mod cpp;
mod identifiers;
mod java;
mod lua;
mod route_effects;
mod statement_defs;

pub(super) use cpp::row_owns_source;
pub(super) use identifiers::{
    introduced_identifiers, normalize_c_declaration_facts, row_excludes_def,
    row_identifier_owns_def, row_introduced_identifiers,
};
pub(super) use route_effects::{apply_explicit_kills, is_route_effect_node};
pub(super) use statement_defs::materialize_def_sites_and_effects;

use super::binding_table::{select_binding_row, DeclarationKind, Role, Ruling};
use super::grammar_lint::ByteSpan;
use super::scope::{BindingFacts, BindingRelation};
use super::{CfgView, DefId, DefSite, Line};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReuseBinder {
    pub(super) node_line: Line,
    pub(super) header_span: ByteSpan,
    pub(super) body_span: Option<ByteSpan>,
    pub(super) initializer_span: Option<ByteSpan>,
    pub(super) path: AccessPath,
    pub(super) kind: DeclarationKind,
    identifier_byte: usize,
    missing_def: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HeaderSpan {
    node_line: Line,
    header_span: ByteSpan,
    body_span: Option<ByteSpan>,
    initializer_span: Option<ByteSpan>,
    path: AccessPath,
    kind: DeclarationKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HeaderUse {
    Body,
    Initializer,
}

#[derive(Debug, Default)]
pub(super) struct AppliedTransfer {
    pub(super) synthetic_nodes: BTreeMap<usize, Line>,
}

type BindingMetadata = (
    Vec<HeaderSpan>,
    BTreeMap<String, BTreeSet<Line>>,
    Vec<(String, Line, ByteSpan)>,
);

pub(super) fn discover(parsed: &ParsedFile, func: &Node<'_>, defs: &[DefSite]) -> Vec<ReuseBinder> {
    fn walk(
        parsed: &ParsedFile,
        node: Node<'_>,
        root_function_id: usize,
        defs: &[DefSite],
        out: &mut Vec<ReuseBinder>,
    ) {
        if node.id() != root_function_id
            && parsed
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
        {
            return;
        }
        if let Some(row) = select_binding_row(parsed, node).filter(|row| {
            (row.ruling == Ruling::Classified
                || (parsed.language == crate::languages::Language::Python
                    && node.kind() == "with_item"
                    && matches!(row.ruling, Ruling::Uncertain { .. })))
                && row.roles.has(Role::Binding)
                && !(parsed.language == crate::languages::Language::Go
                    && matches!(node.kind(), "for_clause" | "short_var_declaration"))
                && matches!(
                    row.declaration,
                    Some(
                        DeclarationKind::Reuse
                            | DeclarationKind::GoShort
                            | DeclarationKind::JavaScriptVar
                            | DeclarationKind::PythonAssignment
                    )
                )
        }) {
            let owner = match node.kind() {
                "range_clause" => node
                    .parent()
                    .filter(|parent| parent.kind() == "for_statement"),
                "with_item" => std::iter::successors(node.parent(), |parent| parent.parent())
                    .find(|parent| parent.kind() == "with_statement"),
                _ => Some(node),
            };
            if let Some(owner) = owner {
                for identifier in row_introduced_identifiers(parsed, node, row.fields) {
                    let name = parsed.node_text(&identifier);
                    if name == "_" {
                        continue;
                    }
                    let missing_def = !defs
                        .iter()
                        .any(|def| !def.synthetic && def.start_byte == identifier.start_byte());
                    out.push(ReuseBinder {
                        node_line: owner.start_position().row + 1,
                        header_span: byte_span(node),
                        body_span: owner.child_by_field_name("body").map(byte_span),
                        initializer_span: node
                            .child_by_field_name("right")
                            .or_else(|| node.child_by_field_name("value"))
                            .map(byte_span),
                        path: AccessPath::simple(name),
                        kind: row.declaration.expect("filtered declaration kind"),
                        identifier_byte: identifier.start_byte(),
                        missing_def,
                    });
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(parsed, child, root_function_id, defs, out);
        }
    }

    let mut binders = Vec::new();
    walk(parsed, *func, func.id(), defs, &mut binders);
    binders
}

pub(super) fn apply(
    parsed: &ParsedFile,
    binders: &[ReuseBinder],
    line_index: &BTreeMap<Line, usize>,
    successors: &mut Vec<Vec<(usize, bool)>>,
    predecessors: &mut Vec<Vec<usize>>,
    defs: &mut Vec<DefSite>,
    mapped_defs: &mut Vec<Option<usize>>,
) -> AppliedTransfer {
    let base_node_count = successors.len();
    let node_lines: BTreeMap<usize, Line> = line_index
        .iter()
        .map(|(line, node)| (*node, *line))
        .collect();
    let mut applied = AppliedTransfer::default();

    for header in 0..base_node_count {
        let original_edges = successors[header].clone();
        let Some(header_line) = node_lines.get(&header).copied() else {
            continue;
        };
        let header_binders: Vec<_> = binders
            .iter()
            .filter(|binder| {
                binder.node_line == header_line
                    && binder.missing_def
                    && binder.kind != DeclarationKind::GoShort
            })
            .collect();
        if header_binders.is_empty() {
            continue;
        }
        let mut replacement = Vec::with_capacity(original_edges.len());
        for (target, incomplete) in original_edges {
            let target_line = node_lines.get(&target).copied();
            let selected: Vec<_> = header_binders
                .iter()
                .copied()
                .filter(|binder| match (binder.body_span, target_line) {
                    (None, _) => true,
                    (Some(body), Some(line)) => {
                        line != binder.node_line && span_contains_line(parsed, body, line)
                    }
                    (Some(_), None) => false,
                })
                .collect();
            if selected.is_empty() {
                replacement.push((target, incomplete));
                continue;
            }

            let first_synthetic = successors.len();
            replacement.push((first_synthetic, incomplete));
            let mut synthetic_chain = Vec::with_capacity(selected.len());
            for binder in selected {
                let synthetic_node = successors.len();
                successors.push(Vec::new());
                synthetic_chain.push(synthetic_node);
                applied
                    .synthetic_nodes
                    .insert(synthetic_node, binder.node_line);
                let def_index = defs.len();
                defs.push(DefSite {
                    id: DefId(u32::try_from(def_index).unwrap_or(u32::MAX)),
                    path: binder.path.clone(),
                    line: binder.node_line,
                    start_byte: binder.identifier_byte,
                    alias_derived: false,
                    synthetic: true,
                });
                mapped_defs.push(Some(synthetic_node));
            }
            for (index, synthetic_node) in synthetic_chain.iter().copied().enumerate() {
                let next = synthetic_chain.get(index + 1).copied().unwrap_or(target);
                successors[synthetic_node].push((next, incomplete));
            }
        }
        successors[header] = replacement;
    }

    let mut orphaned = BTreeMap::<usize, Vec<(&ReuseBinder, usize)>>::new();
    for binder in binders
        .iter()
        .filter(|binder| binder.kind == DeclarationKind::Reuse && !binder.missing_def)
    {
        let Some(def_index) = defs
            .iter()
            .position(|def| !def.synthetic && def.start_byte == binder.identifier_byte)
        else {
            continue;
        };
        if mapped_defs[def_index].is_some() {
            continue;
        }
        let Some(body) = binder.body_span else {
            continue;
        };
        let Some(body_node) = line_index
            .iter()
            .filter(|(line, _)| span_contains_line(parsed, body, **line))
            .min_by_key(|(line, _)| **line)
            .map(|(_, node)| *node)
        else {
            continue;
        };
        orphaned
            .entry(body_node)
            .or_default()
            .push((binder, def_index));
    }
    for (body_node, binders) in orphaned {
        let body = binders[0].0.body_span.expect("grouped body span");
        let incoming: Vec<usize> = (0..successors.len())
            .filter(|from| {
                !node_lines
                    .get(from)
                    .is_some_and(|line| span_contains_line(parsed, body, *line))
                    && successors[*from].iter().any(|(to, _)| *to == body_node)
            })
            .collect();
        if incoming.is_empty() {
            continue;
        }
        let first_synthetic = successors.len();
        let mut synthetic_chain = Vec::with_capacity(binders.len());
        for (binder, def_index) in binders {
            let synthetic_node = successors.len();
            successors.push(Vec::new());
            synthetic_chain.push(synthetic_node);
            applied
                .synthetic_nodes
                .insert(synthetic_node, binder.node_line);
            mapped_defs[def_index] = Some(synthetic_node);
        }
        for from in incoming {
            for (to, _) in &mut successors[from] {
                if *to == body_node {
                    *to = first_synthetic;
                }
            }
        }
        for (index, synthetic_node) in synthetic_chain.iter().copied().enumerate() {
            let next = synthetic_chain.get(index + 1).copied().unwrap_or(body_node);
            successors[synthetic_node].push((next, false));
        }
    }

    *predecessors = vec![Vec::new(); successors.len()];
    for (from, edges) in successors.iter().enumerate() {
        for (to, _) in edges {
            predecessors[*to].push(from);
        }
    }
    applied
}

pub(super) fn materialize_existing_def_sites(
    binders: &[ReuseBinder],
    line_index: &BTreeMap<Line, usize>,
    defs: &mut Vec<DefSite>,
    mapped_defs: &mut Vec<Option<usize>>,
) -> BTreeSet<usize> {
    let mut materialized = BTreeSet::new();
    for binder in binders
        .iter()
        .filter(|binder| binder.kind == DeclarationKind::GoShort && binder.missing_def)
    {
        let Some(node) = line_index.get(&binder.node_line).copied() else {
            continue;
        };
        let def_index = defs.len();
        defs.push(DefSite {
            id: DefId(u32::try_from(def_index).unwrap_or(u32::MAX)),
            path: binder.path.clone(),
            line: binder.node_line,
            start_byte: binder.identifier_byte,
            alias_derived: false,
            synthetic: false,
        });
        mapped_defs.push(Some(node));
        materialized.insert(def_index);
    }
    materialized
}

pub(super) fn binding_metadata(
    parsed: &ParsedFile,
    binders: &[ReuseBinder],
    cfg: &CfgView<'_>,
) -> BindingMetadata {
    let node_lines: BTreeMap<usize, Line> = cfg
        .line_index
        .iter()
        .map(|(line, node)| (*node, *line))
        .collect();
    let mut header_spans = Vec::new();
    let mut no_entry_binders = BTreeMap::<String, BTreeSet<Line>>::new();
    let mut no_entry_spans = Vec::new();
    for binder in binders {
        header_spans.push(HeaderSpan {
            node_line: binder.node_line,
            header_span: binder.header_span,
            body_span: binder.body_span,
            initializer_span: binder.initializer_span,
            path: binder.path.clone(),
            kind: binder.kind,
        });
        if !binder.missing_def {
            continue;
        }
        let Some(body) = binder.body_span else {
            continue;
        };
        if parsed.line_for_byte(body.end.saturating_sub(1)) == binder.node_line {
            continue;
        }
        let has_entry = cfg
            .line_index
            .get(&binder.node_line)
            .into_iter()
            .flat_map(|node| cfg.successors[*node].iter())
            .filter_map(|(node, _)| node_lines.get(node))
            .any(|line| *line != binder.node_line && span_contains_line(parsed, body, *line));
        if !has_entry {
            no_entry_binders
                .entry(binder.path.base.clone())
                .or_default()
                .insert(binder.node_line);
            no_entry_spans.push((binder.path.base.clone(), binder.node_line, body));
        }
    }
    (header_spans, no_entry_binders, no_entry_spans)
}

impl BindingFacts {
    pub(super) fn header_use(&self, edge: &FlowEdge, use_byte: usize) -> Option<HeaderUse> {
        self.header_spans.iter().find_map(|span| {
            let in_body = span.body_span.is_some_and(|body| body.contains(use_byte));
            if span.node_line != edge.to.line
                || span.path != edge.to.path
                || (!span.header_span.contains(use_byte) && !in_body)
            {
                return None;
            }
            if span
                .initializer_span
                .is_some_and(|initializer| initializer.contains(use_byte))
            {
                return Some(HeaderUse::Initializer);
            }
            in_body.then_some(HeaderUse::Body)
        })
    }

    pub(super) fn no_entry_use(&self, name: &str, use_byte: usize) -> Option<Line> {
        self.no_entry_spans
            .iter()
            .filter(|(binder_name, _, span)| binder_name == name && span.contains(use_byte))
            .map(|(_, line, _)| *line)
            .min()
    }

    pub(super) fn header_shadow(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        use_byte: usize,
        candidates: &[usize],
    ) -> Option<Line> {
        self.header_spans
            .iter()
            .filter(|span| span.kind == DeclarationKind::GoShort)
            .filter(|span| span.path == edge.to.path)
            .filter(|span| span.body_span.is_some_and(|body| body.contains(use_byte)))
            .filter(|span| {
                candidates.iter().any(|index| {
                    self.relation(parsed, edge, *index) == BindingRelation::KilledAt(span.node_line)
                })
            })
            .map(|span| span.node_line)
            .min()
    }

    pub(super) fn no_entry_identities(&self) -> BTreeSet<(String, Line)> {
        self.no_entry_binders
            .iter()
            .flat_map(|(name, lines)| lines.iter().map(|line| (name.clone(), *line)))
            .collect()
    }

    #[cfg(test)]
    pub(super) fn with_cfg_view(
        parsed: &ParsedFile,
        func_node: &Node<'_>,
        defs: &[DefSite],
        cfg: &CfgView<'_>,
    ) -> Self {
        let binders = discover(parsed, func_node, defs);
        Self::new(parsed, func_node, defs, cfg, &binders)
    }

    #[cfg(test)]
    pub(super) fn def_binding_count(&self) -> usize {
        self.def_bindings.len()
    }
}

fn byte_span(node: Node<'_>) -> ByteSpan {
    ByteSpan {
        start: node.start_byte(),
        end: node.end_byte(),
    }
}

fn span_contains_line(parsed: &ParsedFile, span: ByteSpan, line: Line) -> bool {
    let start_line = parsed.line_for_byte(span.start);
    let end_line = parsed.line_for_byte(span.end.saturating_sub(1));
    (start_line..=end_line).contains(&line)
}

#[cfg(test)]
mod tests;
