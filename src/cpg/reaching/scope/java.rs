use super::*;

#[derive(Clone, Copy)]
pub(super) struct IdentifierBounds {
    pub(super) scope: ScopeSpan,
    pub(super) visible_from: usize,
    pub(super) declaration_line: Line,
}

pub(super) fn identifier_bounds_for_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    row: &super::super::binding_table::BindingRow,
    def: &DefSite,
    default_scope: ScopeSpan,
) -> Option<IdentifierBounds> {
    if parsed.language != Language::Java {
        return None;
    }
    transfer::row_introduced_identifiers(parsed, node, row.fields)
        .into_iter()
        .find(|identifier| {
            identifier.start_byte() == def.start_byte
                && parsed.node_text(identifier) == def.path.base
        })
        .map(|identifier| identifier_bounds(parsed, node, row, identifier, default_scope))
}

pub(super) fn header_identifier_bounds_for_def(
    parsed: &ParsedFile,
    node: Node<'_>,
    row: &super::super::binding_table::BindingRow,
    def: &DefSite,
    default_scope: ScopeSpan,
) -> Option<IdentifierBounds> {
    transfer::row_introduced_identifiers(parsed, node, row.fields)
        .into_iter()
        .find(|identifier| {
            identifier.start_byte() == def.start_byte
                && parsed.node_text(identifier) == def.path.base
        })
        .map(|identifier| {
            if parsed.language == Language::Java {
                identifier_bounds(parsed, node, row, identifier, default_scope)
            } else {
                IdentifierBounds {
                    scope: default_scope,
                    visible_from: identifier.end_byte(),
                    declaration_line: def.line,
                }
            }
        })
}

pub(super) fn identifier_bounds(
    parsed: &ParsedFile,
    node: Node<'_>,
    row: &super::super::binding_table::BindingRow,
    identifier: Node<'_>,
    default_scope: ScopeSpan,
) -> IdentifierBounds {
    if parsed.language != Language::Java {
        return IdentifierBounds {
            scope: default_scope,
            visible_from: match row.visibility {
                Visibility::WholeScope => default_scope.start_byte,
                Visibility::Header => identifier.end_byte(),
                _ => node.end_byte(),
            },
            declaration_line: node.start_position().row + 1,
        };
    }
    let declaration_line = identifier.start_position().row + 1;

    if node.kind() == "enhanced_for_statement" {
        if let Some(body) = node.child_by_field_name("body") {
            let scope = scope_span(body);
            return IdentifierBounds {
                scope,
                visible_from: scope.start_byte,
                declaration_line,
            };
        }
    }
    if node.kind() == "try_with_resources_statement" {
        if let Some(body) = node.child_by_field_name("body") {
            return IdentifierBounds {
                scope: ScopeSpan {
                    start_byte: identifier.end_byte(),
                    end_byte: body.end_byte(),
                    end_line: body.end_position().row + 1,
                },
                visible_from: identifier.end_byte(),
                declaration_line,
            };
        }
    }

    IdentifierBounds {
        scope: default_scope,
        visible_from: identifier.end_byte(),
        declaration_line,
    }
}

pub(super) fn row_excludes_identifier(
    parsed: &ParsedFile,
    node: Node<'_>,
    row: &super::super::binding_table::BindingRow,
    def: &DefSite,
) -> bool {
    transfer::row_excludes_identifier(parsed, node, row.fields, def.start_byte)
        || (parsed.language == Language::Java
            && !row.fields.is_empty()
            && transfer::row_introduced_identifiers(parsed, node, row.fields)
                .into_iter()
                .all(|identifier| {
                    identifier.start_byte() != def.start_byte
                        || parsed.node_text(&identifier) != def.path.base
                }))
}

impl BindingFacts {
    pub(in crate::cpg::reaching) fn java_resource_initializer_mask(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        use_byte: usize,
    ) -> Option<Line> {
        if parsed.language != Language::Java {
            return None;
        }
        let node = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(use_byte, use_byte.saturating_add(1))?;
        let resource = std::iter::successors(Some(node), |node| node.parent())
            .find(|ancestor| ancestor.kind() == "resource")?;
        let value = resource.child_by_field_name("value")?;
        (value.start_byte() <= use_byte && use_byte < value.end_byte())
            .then(|| self.mask_on_route(edge, use_byte))
            .flatten()
    }
}
