//! Original per-name scope predicates, reachable only through the test bypass.
use super::*;

impl ParsedFile {
    pub(super) fn js_ts_scope_binds_reference(
        &self,
        scope: Node<'_>,
        boundaries: &[&str],
        name: &str,
        byte: usize,
    ) -> bool {
        if boundaries.contains(&scope.kind()) {
            let body = scope.child_by_field_name("body");
            let params = self.parameter_binding_region(&scope);
            let inside = |n: Node<'_>| n.start_byte() <= byte && byte < n.end_byte();
            let in_self_name = matches!(scope.kind(), "function_expression" | "generator_function")
                && scope.child_by_field_name("name").is_some_and(inside);
            // Legacy TS type predicates/queries can refer to formals. Preserve
            // those static-binding rows even though new synthetic Uses erase types.
            let in_return_type = scope.child_by_field_name("return_type").is_some_and(inside);
            if !body.is_some_and(inside)
                && !params.is_some_and(inside)
                && !in_self_name
                && !in_return_type
            {
                return false; // computed method key executes in the enclosing environment
            }
            if let Some(params) = self
                .find_parameters_node(&scope)
                .or_else(|| scope.child_by_field_name("parameter"))
            {
                if self.js_ts_fence_pattern_binds(params, name) {
                    return true;
                }
            }
            if matches!(scope.kind(), "function_expression" | "generator_function")
                && scope
                    .child_by_field_name("name")
                    .is_some_and(|n| self.js_ts_fence_pattern_binds(n, name))
            {
                return true;
            }
            // Function-scope (`var`, hoisted declarations) of the nested callable.
            return body.is_some_and(|body| {
                inside(body) && self.js_ts_function_scope_binds(body, boundaries, name)
            });
        }
        match scope.kind() {
            "class" | "class_declaration" | "abstract_class_declaration" => scope
                .child_by_field_name("name")
                .is_some_and(|n| self.js_ts_fence_pattern_binds(n, name)),
            "enum_body" => {
                let member_binds = |body: Node<'_>| {
                    let mut cursor = body.walk();
                    let found = body.named_children(&mut cursor).any(|member| {
                        let key = if member.kind() == "enum_assignment" {
                            member.child_by_field_name("name")
                        } else {
                            Some(member)
                        };
                        key.is_some_and(|key| {
                            let decoded = match key.kind() {
                                "string" => self.js_ts_module_export_name(key),
                                "identifier" | "property_identifier" | "type_identifier" => {
                                    Self::js_ts_decoded_identifier(self.node_text(&key))
                                }
                                _ => None,
                            };
                            decoded.as_deref() == Some(name)
                        })
                    });
                    found
                };
                if member_binds(scope) {
                    return true;
                }
                // Same-file declarations merge only within this lexical scope.
                // Namespace and cross-file merging are outside this model.
                let Some(declaration) = scope.parent() else {
                    return false;
                };
                let enum_name = declaration.child_by_field_name("name");
                let Some(parent) = declaration.parent() else {
                    return false;
                };
                let mut cursor = parent.walk();
                let found = parent.named_children(&mut cursor).any(|sibling| {
                    sibling.kind() == "enum_declaration"
                        && sibling.child_by_field_name("name").is_some_and(|n| {
                            enum_name.is_some_and(|own| {
                                Self::js_ts_decoded_identifier(self.node_text(&n))
                                    == Self::js_ts_decoded_identifier(self.node_text(&own))
                            })
                        })
                        && sibling
                            .child_by_field_name("body")
                            .is_some_and(member_binds)
                });
                found
            }
            "catch_clause" => scope
                .child_by_field_name("parameter")
                .is_some_and(|p| self.js_ts_fence_pattern_binds(p, name)),
            "class_static_block" => scope
                .child_by_field_name("body")
                .is_some_and(|body| self.js_ts_function_scope_binds(body, boundaries, name)),
            "for_in_statement" => {
                scope
                    .child_by_field_name("kind")
                    .is_some_and(|k| matches!(self.node_text(&k), "let" | "const"))
                    && scope
                        .child_by_field_name("left")
                        .is_some_and(|l| self.js_ts_fence_pattern_binds(l, name))
            }
            "for_statement" => scope
                .child_by_field_name("initializer")
                .filter(|init| init.kind() == "lexical_declaration")
                .is_some_and(|init| self.js_ts_declaration_binds(init, name)),
            "statement_block" | "switch_body" => {
                let callable_body = scope.parent().is_some_and(|p| {
                    boundaries.contains(&p.kind())
                        && p.child_by_field_name("body").map(|b| b.id()) == Some(scope.id())
                });
                let mut cursor = scope.walk();
                let found = scope
                    .named_children(&mut cursor)
                    .filter(|stmt| {
                        let mut declaration = *stmt;
                        while declaration.kind() == "labeled_statement" {
                            let Some(body) = declaration.child_by_field_name("body") else {
                                break;
                            };
                            declaration = body;
                        }
                        !callable_body
                            || !matches!(
                                declaration.kind(),
                                "function_declaration" | "generator_function_declaration"
                            )
                    })
                    .any(|stmt| self.js_ts_block_statement_binds(stmt, name));
                found
            }
            _ => false,
        }
    }

    pub(super) fn js_ts_block_statement_binds(&self, stmt: Node<'_>, name: &str) -> bool {
        match stmt.kind() {
            "labeled_statement" => stmt
                .child_by_field_name("body")
                .is_some_and(|body| self.js_ts_block_statement_binds(body, name)),
            "lexical_declaration" => self.js_ts_declaration_binds(stmt, name),
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration"
            | "enum_declaration" => stmt
                .child_by_field_name("name")
                .is_some_and(|n| self.js_ts_fence_pattern_binds(n, name)),
            "switch_case" | "switch_default" => {
                let mut cursor = stmt.walk();
                let found = stmt
                    .named_children(&mut cursor)
                    .any(|s| self.js_ts_block_statement_binds(s, name));
                found
            }
            _ => false,
        }
    }

    pub(super) fn js_ts_declaration_binds(&self, decl: Node<'_>, name: &str) -> bool {
        let mut cursor = decl.walk();
        let found = decl.named_children(&mut cursor).any(|d| {
            d.kind() == "variable_declarator"
                && d.child_by_field_name("name")
                    .is_some_and(|n| self.js_ts_fence_pattern_binds(n, name))
        });
        found
    }
}
