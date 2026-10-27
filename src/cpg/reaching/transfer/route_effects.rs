//! Python nodes whose effects must be decided by the CFG rather than textual masks.

use super::super::binding_table::{select_binding_row, DeclarationKind, Role, Ruling};
use super::super::graph::BitSet;
use super::super::{DefSite, Line};
use super::row_introduced_identifiers;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Clone, Debug)]
struct DiscoveredEffect {
    line: Line,
    path: AccessPath,
}

#[derive(Debug, Default)]
pub(in crate::cpg::reaching) struct RouteEffects {
    kills: Vec<(usize, AccessPath)>,
}

pub(in crate::cpg::reaching) fn is_route_effect_node(language: Language, kind: &str) -> bool {
    language == Language::Python
        && matches!(
            kind,
            "delete_statement" | "import_statement" | "import_from_statement" | "aliased_import"
        )
}

pub(in crate::cpg::reaching) fn materialize_route_effects(
    parsed: &ParsedFile,
    func: &Node<'_>,
    line_index: &BTreeMap<Line, usize>,
) -> RouteEffects {
    if parsed.language != Language::Python {
        return RouteEffects::default();
    }
    let discovered = discover(parsed, *func);
    let mut effects = RouteEffects::default();
    for effect in discovered {
        if let Some(node) = line_index.get(&effect.line).copied() {
            effects.kills.push((node, effect.path));
        }
    }
    effects
}

fn discover(parsed: &ParsedFile, func: Node<'_>) -> Vec<DiscoveredEffect> {
    fn walk(
        parsed: &ParsedFile,
        node: Node<'_>,
        root_function_id: usize,
        effects: &mut Vec<DiscoveredEffect>,
    ) {
        if node.id() != root_function_id
            && parsed
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
        {
            return;
        }
        if node.kind() == "delete_statement" {
            if let Some(row) = select_binding_row(parsed, node).filter(|row| {
                row.ruling == Ruling::Classified
                    && row.roles.has(Role::Binding)
                    && row.declaration == Some(DeclarationKind::Other)
            }) {
                for identifier in row_introduced_identifiers(parsed, node, row.fields) {
                    let name = parsed.node_text(&identifier);
                    if name != "_" {
                        effects.push(DiscoveredEffect {
                            line: node.start_position().row + 1,
                            path: AccessPath::simple(name),
                        });
                    }
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(parsed, child, root_function_id, effects);
        }
    }

    let mut effects = Vec::new();
    walk(parsed, func, func.id(), &mut effects);
    effects
}

pub(in crate::cpg::reaching) fn apply_explicit_kills(
    effects: &RouteEffects,
    defs: &[DefSite],
    kill: &mut [BitSet],
    flat_kill: &mut [BitSet],
) {
    for (node, path) in &effects.kills {
        for (index, def) in defs.iter().enumerate() {
            if !def.alias_derived && def.path == *path {
                kill[*node].insert(index);
                flat_kill[*node].insert(index);
            }
        }
    }
}
