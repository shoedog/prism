use super::*;

#[derive(Clone, Copy)]
pub(super) struct IdentifierBounds {
    pub(super) scope: ScopeSpan,
    pub(super) visible_from: usize,
    pub(super) declaration_line: Line,
}

pub(super) fn def_anchors_identifier(
    parsed: &ParsedFile,
    node: Node<'_>,
    def: &DefSite,
    identifier: Node<'_>,
) -> bool {
    parsed.node_text(&identifier) == def.path.base
        && (identifier.start_byte() == def.start_byte
            || (node.kind() == "local_variable_declaration"
                && node.start_byte() <= def.start_byte
                && def.start_byte < identifier.start_byte()))
}

pub(super) fn classified_mask_applies_to_use(
    parsed: &ParsedFile,
    edge: &FlowEdge,
    use_byte: usize,
) -> bool {
    if parsed.language != Language::Java {
        return true;
    }
    let identifiers: Vec<_> = parsed
        .identifiers_on_line(edge.to.line)
        .into_iter()
        .filter(|identifier| parsed.node_text(identifier) == edge.to.path.base)
        .filter(|identifier| !is_instanceof_pattern_name(*identifier))
        .collect();
    if identifiers.is_empty()
        || !identifiers
            .iter()
            .any(|identifier| identifier.start_byte() == use_byte)
    {
        return false;
    }
    identifiers
        .iter()
        .all(|identifier| !is_qualified_field_identifier(*identifier))
}

fn is_instanceof_pattern_name(identifier: Node<'_>) -> bool {
    identifier.parent().is_some_and(|parent| {
        parent.kind() == "instanceof_expression"
            && parent
                .child_by_field_name("name")
                .is_some_and(|name| name.id() == identifier.id())
    })
}

fn is_qualified_field_identifier(identifier: Node<'_>) -> bool {
    identifier.parent().is_some_and(|parent| {
        parent.kind() == "field_access"
            && parent
                .child_by_field_name("field")
                .is_some_and(|field| field.id() == identifier.id())
    })
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
        .find(|identifier| def_anchors_identifier(parsed, node, def, *identifier))
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

    if node.kind() == "instanceof_expression" {
        match super::super::grammar_lint::visibility_span(parsed, node) {
            super::super::grammar_lint::SpanKind::Known(spans) => {
                let visibility: Vec<_> = spans
                    .into_iter()
                    .map(|span| ScopeSpan {
                        start_byte: span.start,
                        end_byte: span.end,
                        end_line: parsed
                            .tree
                            .root_node()
                            .descendant_for_byte_range(span.end.saturating_sub(1), span.end)
                            .map_or(declaration_line, |node| node.end_position().row + 1),
                    })
                    .collect();
                if let (Some(first), Some(last)) = (visibility.first(), visibility.last()) {
                    return IdentifierBounds {
                        scope: ScopeSpan {
                            start_byte: first.start_byte,
                            end_byte: last.end_byte,
                            end_line: last.end_line,
                        },
                        visible_from: usize::MAX,
                        declaration_line,
                    };
                }
            }
            super::super::grammar_lint::SpanKind::Empty => {
                return IdentifierBounds {
                    scope: ScopeSpan {
                        start_byte: node.end_byte(),
                        end_byte: node.end_byte(),
                        end_line: declaration_line,
                    },
                    visible_from: usize::MAX,
                    declaration_line,
                };
            }
            super::super::grammar_lint::SpanKind::Unknown => {}
        }
    }

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

pub(super) fn additional_visibility(parsed: &ParsedFile, node: Node<'_>) -> Vec<ScopeSpan> {
    if parsed.language != Language::Java || node.kind() != "instanceof_expression" {
        return Vec::new();
    }
    let super::super::grammar_lint::SpanKind::Known(spans) =
        super::super::grammar_lint::visibility_span(parsed, node)
    else {
        return Vec::new();
    };
    spans
        .into_iter()
        .map(|span| ScopeSpan {
            start_byte: span.start,
            end_byte: span.end,
            end_line: parsed
                .tree
                .root_node()
                .descendant_for_byte_range(span.end.saturating_sub(1), span.end)
                .map_or(node.end_position().row + 1, |end| {
                    end.end_position().row + 1
                }),
        })
        .collect()
}

pub(super) fn row_excludes_identifier(
    parsed: &ParsedFile,
    node: Node<'_>,
    row: &super::super::binding_table::BindingRow,
    def: &DefSite,
) -> bool {
    if parsed.language == Language::Java && !row.fields.is_empty() {
        return transfer::row_introduced_identifiers(parsed, node, row.fields)
            .into_iter()
            .all(|identifier| !def_anchors_identifier(parsed, node, def, identifier));
    }
    transfer::row_excludes_def(parsed, node, row.fields, def)
}

pub(super) fn same_def_binding(facts: &BindingFacts, left: usize, right: usize) -> bool {
    let left = &facts.def_bindings[left];
    let right = &facts.def_bindings[right];
    let fallback_is_field = match (&left.id, &right.id) {
        (BindingId::FlatFallback(name), _) | (_, BindingId::FlatFallback(name)) => {
            facts.java_field_names.contains(name)
        }
        _ => false,
    };
    let scoped_declaration_vs_fallback = facts.language == Language::Java
        && fallback_is_field
        && ((matches!(left.id, BindingId::Declaration(_))
            && left.scope != facts.function_scope
            && matches!(right.id, BindingId::FlatFallback(_)))
            || (matches!(right.id, BindingId::Declaration(_))
                && right.scope != facts.function_scope
                && matches!(left.id, BindingId::FlatFallback(_))));
    !scoped_declaration_vs_fallback
        && (left.id == right.id
            || matches!(left.id, BindingId::FlatFallback(_))
            || matches!(right.id, BindingId::FlatFallback(_)))
}

pub(super) fn lexical_relation_overrides_flat_fallback(
    facts: &BindingFacts,
    parsed: &ParsedFile,
    edge: &FlowEdge,
    def_index: usize,
) -> bool {
    facts.language == Language::Java
        && facts.java_field_names.contains(&edge.to.path.base)
        && facts.declarations.iter().any(|declaration| {
            declaration.name == edge.to.path.base
                && matches!(declaration.binding.id, BindingId::Declaration(_))
                && declaration.binding.scope != facts.function_scope
        })
        && facts.relation(parsed, edge, def_index) != BindingRelation::Unresolved
}

pub(super) fn enclosing_type_field_names(
    parsed: &ParsedFile,
    function_scope: ScopeSpan,
) -> BTreeSet<String> {
    let mut fields = BTreeSet::new();
    let node = parsed.tree.root_node().descendant_for_byte_range(
        function_scope.start_byte,
        function_scope.start_byte.saturating_add(1),
    );
    let Some(body) = node.and_then(|node| {
        std::iter::successors(Some(node), |node| node.parent()).find(|ancestor| {
            matches!(
                ancestor.kind(),
                "annotation_type_body" | "class_body" | "enum_body" | "interface_body"
            )
        })
    }) else {
        return fields;
    };
    let mut body_cursor = body.walk();
    for declaration in body
        .named_children(&mut body_cursor)
        .filter(|child| matches!(child.kind(), "constant_declaration" | "field_declaration"))
    {
        let mut cursor = declaration.walk();
        for identifier in declaration
            .named_children(&mut cursor)
            .filter(|declarator| declarator.kind() == "variable_declarator")
            .filter_map(|declarator| declarator.child_by_field_name("name"))
        {
            fields.insert(parsed.node_text(&identifier).to_string());
        }
    }
    fields
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
            .then(|| self.mask_on_route(parsed, edge, use_byte))
            .flatten()
    }
}
