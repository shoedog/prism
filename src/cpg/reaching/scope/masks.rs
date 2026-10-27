use super::*;

pub(super) fn collect_classified_masks(
    parsed: &ParsedFile,
    node: Node<'_>,
    root_function: Node<'_>,
    defs: &[DefSite],
    function_scope: ScopeSpan,
    declarations: &mut Vec<Declaration>,
) {
    if node.id() != root_function.id()
        && parsed
            .language
            .callable_boundary_node_types()
            .contains(&node.kind())
    {
        return;
    }
    if let Some(row) = select_binding_row(parsed, node).filter(|row| {
        row.roles.has(Role::Binding)
            && matches!(
                row.declaration,
                Some(DeclarationKind::Other | DeclarationKind::Pattern)
            )
            && !transfer::is_route_effect_node(parsed.language, node.kind())
            && row.ruling == Ruling::Classified
    }) {
        let scope = if row.roles.has(Role::Scope) {
            if parsed.language == Language::Lua
                && matches!(node.kind(), "for_numeric_clause" | "for_generic_clause")
            {
                node.parent()
                    .filter(|parent| parent.kind() == "for_statement")
                    .map(scope_span)
                    .unwrap_or_else(|| scope_span(node))
            } else {
                scope_span(node)
            }
        } else {
            declaration_scope(
                parsed,
                &root_function,
                node,
                row.declaration.expect("filtered declaration kind"),
                function_scope,
            )
        };
        for identifier in transfer::row_introduced_identifiers(parsed, node, row.fields) {
            if defs
                .iter()
                .any(|def| def.start_byte == identifier.start_byte())
            {
                continue;
            }
            let index = declarations.len();
            declarations.push(Declaration {
                binding: Binding {
                    id: BindingId::Declaration(index),
                    scope,
                    declaration_line: Some(node.start_position().row + 1),
                    classified_mask: true,
                },
                name: parsed.node_text(&identifier).to_string(),
                visible_from: match row.visibility {
                    Visibility::WholeScope => scope.start_byte,
                    Visibility::Header => identifier.end_byte(),
                    _ => node.end_byte(),
                },
                additional_visibility: Vec::new(),
            });
        }
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_classified_masks(
            parsed,
            child,
            root_function,
            defs,
            function_scope,
            declarations,
        );
    }
}

pub(in crate::cpg::reaching) fn classified_masked_reads(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    spans: &[crate::ast::PathSpan],
) -> Vec<crate::ast::PathSpan> {
    let function_scope = function_scope(parsed, func_node);
    let mut declarations = Vec::new();
    collect_classified_masks(
        parsed,
        *func_node,
        *func_node,
        &[],
        function_scope,
        &mut declarations,
    );
    spans
        .iter()
        .filter(|span| {
            declarations.iter().any(|declaration| {
                declaration.binding.classified_mask
                    && declaration.name == span.path.base
                    && declaration.binding.scope.contains(span.start_byte)
                    && declaration.is_visible_at(span.start_byte)
            })
        })
        .cloned()
        .collect()
}

pub(super) fn simple_catch_parameter_owns_def(
    parsed: &ParsedFile,
    function_id: usize,
    def: &DefSite,
) -> bool {
    if parsed.language != Language::JavaScript {
        return false;
    }
    parsed
        .tree
        .root_node()
        .descendant_for_byte_range(def.start_byte, def.start_byte.saturating_add(1))
        .into_iter()
        .flat_map(|node| std::iter::successors(Some(node), |node| node.parent()))
        .take_while(|node| node.id() != function_id)
        .filter(|node| node.kind() == "catch_clause")
        .filter_map(|node| node.child_by_field_name("parameter"))
        .any(|parameter| {
            parameter.kind() == "identifier" && parsed.node_text(&parameter) == def.path.base
        })
}
