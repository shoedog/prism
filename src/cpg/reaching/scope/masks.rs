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
        let default_scope = if row.roles.has(Role::Scope) {
            lua::header_scope(parsed.language, node)
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
            if defs.iter().any(|def| {
                if parsed.language == crate::languages::Language::Java {
                    java::def_anchors_identifier(parsed, node, def, identifier)
                } else {
                    transfer::row_identifier_owns_def(parsed, node, identifier, def)
                }
            }) {
                continue;
            }
            let bounds = java::identifier_bounds(parsed, node, row, identifier, default_scope);
            let index = declarations.len();
            declarations.push(Declaration {
                binding: Binding {
                    id: BindingId::Declaration(index),
                    scope: bounds.scope,
                    declaration_line: Some(bounds.declaration_line),
                    classified_mask: true,
                },
                name: parsed.node_text(&identifier).to_string(),
                visible_from: lua::header_visible_from(
                    parsed.language,
                    node,
                    bounds.visible_from,
                    bounds.scope,
                ),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpg::{AccessPath, DefId};

    #[test]
    fn astra_r2_f4_mask_suppression_is_per_identifier() {
        let parsed = ParsedFile::parse(
            "test.cpp",
            "void f() { try {} catch (int represented, int masked) { sink(masked); } }",
            Language::Cpp,
        )
        .unwrap();
        let function = parsed.all_functions()[0];
        let catch = function
            .named_descendant_for_byte_range(function.start_byte(), function.end_byte())
            .and_then(|_| {
                let mut pending = vec![function];
                while let Some(node) = pending.pop() {
                    if node.kind() == "catch_clause" {
                        return Some(node);
                    }
                    let mut cursor = node.walk();
                    pending.extend(node.named_children(&mut cursor));
                }
                None
            })
            .expect("catch clause");
        let row = select_binding_row(&parsed, catch).expect("catch row");
        let identifiers = transfer::row_introduced_identifiers(&parsed, catch, row.fields);
        assert_eq!(
            identifiers
                .iter()
                .map(|identifier| parsed.node_text(identifier))
                .collect::<Vec<_>>(),
            ["represented", "masked"]
        );
        let represented = identifiers[0];
        let defs = [DefSite {
            id: DefId(0),
            path: AccessPath::from_expr("represented"),
            line: represented.start_position().row + 1,
            start_byte: represented.start_byte(),
            alias_derived: false,
            synthetic: false,
        }];
        let mut declarations = Vec::new();
        collect_classified_masks(
            &parsed,
            function,
            function,
            &defs,
            function_scope(&parsed, &function),
            &mut declarations,
        );
        let masked: Vec<_> = declarations
            .iter()
            .filter(|declaration| declaration.binding.classified_mask)
            .map(|declaration| declaration.name.as_str())
            .collect();
        assert!(
            masked.contains(&"masked"),
            "unrepresented name needs a mask"
        );
        assert!(
            !masked.contains(&"represented"),
            "represented name must not be masked"
        );
    }
}
