//! Frozen pre-M serial alias declaration extraction.
use super::super::*;

impl GoAliasResolver {
    pub(in crate::go_type_alias) fn extract_file_reference(
        &mut self,
        path: &str,
        parsed: &ParsedFile,
        local_path: Option<&str>,
        signature_imports_override: Option<&BTreeMap<String, String>>,
    ) {
        let Some(profile) = self.profiles.get(path).cloned() else {
            return;
        };
        if profile.package_clause.is_empty() {
            return;
        }
        let imports = signature_imports_override
            .cloned()
            .unwrap_or_else(|| signature_imports(parsed, local_path));
        let root = parsed.tree.root_node();
        let mut root_cursor = root.walk();
        for declaration in root.children(&mut root_cursor) {
            if declaration.kind() != "type_declaration" {
                continue;
            }
            let mut cursor = declaration.walk();
            for child in declaration.children(&mut cursor) {
                if !matches!(child.kind(), "type_alias" | "type_spec") {
                    continue;
                }
                let Some(name_node) = child.child_by_field_name("name") else {
                    continue;
                };
                let name = parsed.node_text(&name_node).trim().to_string();
                if name.is_empty() {
                    continue;
                }
                let owner = GoOwnerIdentity {
                    package_dir: dir_of(path).to_string(),
                    package_clause: profile.package_clause.clone(),
                    name: name.clone(),
                };
                let syntax = classify_alias_syntax(&child);
                let kind = if syntax != AliasSyntax::Defined {
                    let (param_names, constraints_supported) = alias_parameters(parsed, &child);
                    let type_params: BTreeSet<String> = param_names.iter().cloned().collect();
                    let rhs = if syntax == AliasSyntax::Unresolved {
                        Err(AliasUnresolvedReason::Unresolvable)
                    } else {
                        child
                            .child_by_field_name("type")
                            .ok_or(AliasUnresolvedReason::Unresolvable)
                            .and_then(|node| {
                                parse_expr(
                                    &node,
                                    parsed,
                                    &imports,
                                    path,
                                    Some(&profile),
                                    &type_params,
                                )
                                .map_err(|_| AliasUnresolvedReason::Unresolvable)
                            })
                    };
                    AliasDeclarationKind::Alias {
                        params: param_names,
                        rhs: if constraints_supported {
                            rhs
                        } else {
                            Err(AliasUnresolvedReason::Unresolvable)
                        },
                    }
                } else {
                    AliasDeclarationKind::Defined
                };
                self.declarations
                    .entry(owner)
                    .or_default()
                    .push(AliasDeclaration {
                        defining_file: path.to_string(),
                        kind,
                    });
            }
        }
    }
}
