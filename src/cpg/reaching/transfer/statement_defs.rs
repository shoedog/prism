//! Ordinary definitions supplied by classified statement-level bindings.

use super::super::binding_table::{select_binding_row, DeclarationKind, Role, Ruling};
use super::super::{DefId, DefSite, Line};
use super::route_effects::RouteEffects;
use super::row_introduced_identifiers;
use super::ReuseBinder;
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(in crate::cpg::reaching) fn materialize_def_sites_and_effects(
    parsed: &ParsedFile,
    func: &Node<'_>,
    binders: &[ReuseBinder],
    line_index: &BTreeMap<Line, usize>,
    defs: &mut Vec<DefSite>,
    mapped_defs: &mut Vec<Option<usize>>,
) -> (std::collections::BTreeSet<usize>, RouteEffects) {
    fn walk(
        parsed: &ParsedFile,
        node: Node<'_>,
        root_function_id: usize,
        line_index: &BTreeMap<Line, usize>,
        defs: &mut Vec<DefSite>,
        mapped_defs: &mut Vec<Option<usize>>,
    ) {
        if node.id() != root_function_id
            && parsed
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
        {
            return;
        }
        if parsed.language == Language::Python
            && parsed.language.is_statement_node(node.kind())
            && select_binding_row(parsed, node).is_some_and(|row| {
                row.ruling == Ruling::Classified
                    && row.roles.has(Role::Binding)
                    && row.declaration == Some(DeclarationKind::Other)
                    && !row.fields.is_empty()
            })
        {
            let line = node.start_position().row + 1;
            if let Some(cfg_node) = line_index.get(&line).copied() {
                let row = select_binding_row(parsed, node).expect("filtered binding row");
                for identifier in row_introduced_identifiers(parsed, node, row.fields) {
                    if defs
                        .iter()
                        .any(|def| def.start_byte == identifier.start_byte())
                    {
                        continue;
                    }
                    let index = defs.len();
                    defs.push(DefSite {
                        id: DefId(u32::try_from(index).unwrap_or(u32::MAX)),
                        path: AccessPath::simple(parsed.node_text(&identifier)),
                        line,
                        start_byte: identifier.start_byte(),
                        alias_derived: false,
                        synthetic: false,
                    });
                    mapped_defs.push(Some(cfg_node));
                }
            }
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            walk(
                parsed,
                child,
                root_function_id,
                line_index,
                defs,
                mapped_defs,
            );
        }
    }

    let materialized =
        super::materialize_existing_def_sites(binders, line_index, defs, mapped_defs);
    walk(parsed, *func, func.id(), line_index, defs, mapped_defs);
    let effects = super::route_effects::materialize_route_effects(parsed, func, line_index);
    (materialized, effects)
}
