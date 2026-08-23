//! Profile-aware Go type-alias declaration snapshots and canonical expansion.

use crate::ast::ParsedFile;
use crate::go_build_profile::GoBuildProfile;
use crate::go_owner_partition::{exact_declaration_visibility, GoOwnerReferenceMode};
use crate::languages::Language;
use crate::resolution::{dir_of, GoOwnerIdentity};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

mod canon;
mod types;
pub(crate) use canon::signature_imports;
use canon::{alias_parameters, parse_expr, predeclared};
use types::{
    AliasDeclaration, AliasDeclarationKind, AliasExpr, AliasUnresolvedReason, NameIdentity,
    NamedType, OwnerHint,
};
pub(crate) use types::{CanonTypeError, GoAliasTelemetry};

pub(crate) struct GoAliasResolver {
    declarations: BTreeMap<GoOwnerIdentity, Vec<AliasDeclaration>>,
    profiles: BTreeMap<String, GoBuildProfile>,
    packages_by_import_path: BTreeMap<String, BTreeSet<(String, String)>>,
    telemetry: RefCell<GoAliasTelemetry>,
}

impl GoAliasResolver {
    pub(crate) fn build(
        files: &BTreeMap<String, ParsedFile>,
        package_import_paths: &BTreeMap<String, String>,
        local_import_paths: &BTreeMap<String, String>,
        profiles: &BTreeMap<String, GoBuildProfile>,
    ) -> Self {
        let mut resolver = Self {
            declarations: BTreeMap::new(),
            profiles: profiles.clone(),
            packages_by_import_path: BTreeMap::new(),
            telemetry: RefCell::new(GoAliasTelemetry::default()),
        };
        for (path, profile) in profiles {
            if profile.is_test_file || profile.package_clause.is_empty() {
                continue;
            }
            if let Some(import_path) = package_import_paths
                .get(path)
                .filter(|path| !path.is_empty())
            {
                resolver
                    .packages_by_import_path
                    .entry(import_path.clone())
                    .or_default()
                    .insert((dir_of(path).to_string(), profile.package_clause.clone()));
            }
        }
        for (path, parsed) in files {
            if parsed.language != Language::Go {
                continue;
            }
            resolver.extract_file(
                path,
                parsed,
                local_import_paths.get(path).map(String::as_str),
            );
        }
        resolver
    }

    #[cfg(test)]
    pub(crate) fn empty() -> Self {
        Self {
            declarations: BTreeMap::new(),
            profiles: BTreeMap::new(),
            packages_by_import_path: BTreeMap::new(),
            telemetry: RefCell::new(GoAliasTelemetry::default()),
        }
    }

    pub(crate) fn telemetry(&self) -> GoAliasTelemetry {
        self.telemetry.borrow().clone()
    }

    /// Resolve an embedded field's declared type to the concrete owner it names.
    /// The returned pointer bit includes pointer-ness introduced by an alias RHS,
    /// while callers retain the field's source selector separately.
    pub(crate) fn resolve_embedded_owner(
        &self,
        raw_type: &str,
        caller_file: &str,
        imports: &BTreeMap<String, String>,
    ) -> Result<(bool, GoOwnerIdentity), ()> {
        let raw = raw_type.trim();
        let (written_pointer, base) = match raw.strip_prefix('*') {
            Some(base) => (true, base.trim()),
            None => (false, raw),
        };
        if base.is_empty()
            || base.contains(|ch: char| ch.is_whitespace())
            || base.contains(['[', ']', '{', '}', '(', ')'])
        {
            return Err(());
        }
        let parts: Vec<&str> = base.split('.').collect();
        let (identity, owner_hint, name) = match parts.as_slice() {
            [name] if !name.is_empty() => {
                let profile = self.profiles.get(caller_file).ok_or(())?;
                if profile.package_clause.is_empty() {
                    return Err(());
                }
                (
                    NameIdentity::Bare,
                    OwnerHint::Local {
                        package_dir: dir_of(caller_file).to_string(),
                        clause: profile.package_clause.clone(),
                    },
                    (*name).to_string(),
                )
            }
            [qualifier, name] if !qualifier.is_empty() && !name.is_empty() => {
                let import_path = imports.get(*qualifier).cloned().ok_or(())?;
                (
                    NameIdentity::Path {
                        path: import_path.clone(),
                        qualified: true,
                    },
                    OwnerHint::ImportPath(import_path),
                    (*name).to_string(),
                )
            }
            _ => return Err(()),
        };
        let named = NamedType {
            identity,
            owner_hint,
            name,
        };
        let expanded = self
            .expand_named(&named, Vec::new(), caller_file, &mut BTreeSet::new())
            .map_err(|_| ())?
            .unwrap_or(AliasExpr::Named(named));
        let (alias_pointer, named) = match expanded {
            AliasExpr::Named(named) => (false, named),
            AliasExpr::Pointer(inner) => match *inner {
                AliasExpr::Named(named) if !written_pointer => (true, named),
                _ => return Err(()),
            },
            _ => return Err(()),
        };
        let owner = match named.owner_hint {
            OwnerHint::Local {
                package_dir,
                clause,
            } => GoOwnerIdentity {
                package_dir,
                package_clause: clause,
                name: named.name,
            },
            OwnerHint::ImportPath(path) => {
                let packages = self.packages_by_import_path.get(&path).ok_or(())?;
                if packages.len() != 1 {
                    return Err(());
                }
                let (package_dir, package_clause) =
                    packages.iter().next().expect("one resolved package");
                GoOwnerIdentity {
                    package_dir: package_dir.clone(),
                    package_clause: package_clause.clone(),
                    name: named.name,
                }
            }
            OwnerHint::None => return Err(()),
        };
        Ok((written_pointer || alias_pointer, owner))
    }

    pub(crate) fn canonicalize(
        &self,
        node: &tree_sitter::Node<'_>,
        parsed: &ParsedFile,
        caller_file: &str,
        imports: &BTreeMap<String, String>,
    ) -> Result<String, CanonTypeError> {
        let profile = self.profiles.get(caller_file);
        let params = BTreeSet::new();
        let expr = parse_expr(node, parsed, imports, caller_file, profile, &params)?;
        match self.expand_expr(expr, caller_file, &mut BTreeSet::new()) {
            Ok(expanded) => Ok(expanded.render(false)),
            Err(reason) => {
                *self
                    .telemetry
                    .borrow_mut()
                    .unresolved
                    .entry(reason.telemetry_key().to_string())
                    .or_default() += 1;
                Err(CanonTypeError::Alias(reason))
            }
        }
    }

    fn extract_file(&mut self, path: &str, parsed: &ParsedFile, local_path: Option<&str>) {
        let Some(profile) = self.profiles.get(path).cloned() else {
            return;
        };
        if profile.package_clause.is_empty() {
            return;
        }
        let imports = signature_imports(parsed, local_path);
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
                let parameterized_alias = child.kind() == "type_spec"
                    && child.child_by_field_name("type_parameters").is_some()
                    && parsed.node_text(&child).contains('=')
                    && child
                        .children(&mut child.walk())
                        .any(|node| node.kind() == "ERROR");
                let kind = if child.kind() == "type_alias" || parameterized_alias {
                    let (param_names, constraints_supported) =
                        alias_parameters(parsed, &child, &name);
                    let type_params: BTreeSet<String> = param_names.iter().cloned().collect();
                    let rhs = child
                        .child_by_field_name("type")
                        .ok_or(AliasUnresolvedReason::Unresolvable)
                        .and_then(|node| {
                            parse_expr(&node, parsed, &imports, path, Some(&profile), &type_params)
                                .map_err(|_| AliasUnresolvedReason::Unresolvable)
                        });
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

    fn expand_expr(
        &self,
        expr: AliasExpr,
        caller_file: &str,
        visiting: &mut BTreeSet<GoOwnerIdentity>,
    ) -> Result<AliasExpr, AliasUnresolvedReason> {
        match expr {
            AliasExpr::Named(named) => self
                .expand_named(&named, Vec::new(), caller_file, visiting)
                .map(|expanded| {
                    expanded.unwrap_or_else(|| match (&named.identity, predeclared(&named.name)) {
                        (NameIdentity::Bare, Some(normalized))
                        | (
                            NameIdentity::Path {
                                qualified: false, ..
                            },
                            Some(normalized),
                        ) => AliasExpr::Atom(normalized.to_string()),
                        _ => AliasExpr::Named(named),
                    })
                }),
            AliasExpr::Generic(base, args) => {
                let args = args
                    .into_iter()
                    .map(|arg| self.expand_expr(arg, caller_file, visiting))
                    .collect::<Result<Vec<_>, _>>()?;
                if let AliasExpr::Named(named) = *base {
                    if let Some(expanded) =
                        self.expand_named(&named, args.clone(), caller_file, visiting)?
                    {
                        return Ok(expanded);
                    }
                    return Ok(AliasExpr::Generic(Box::new(AliasExpr::Named(named)), args));
                }
                Ok(AliasExpr::Generic(
                    Box::new(self.expand_expr(*base, caller_file, visiting)?),
                    args,
                ))
            }
            AliasExpr::Pointer(inner) => Ok(AliasExpr::Pointer(Box::new(self.expand_expr(
                *inner,
                caller_file,
                visiting,
            )?))),
            AliasExpr::Slice(inner) => Ok(AliasExpr::Slice(Box::new(self.expand_expr(
                *inner,
                caller_file,
                visiting,
            )?))),
            AliasExpr::Array(length, inner) => Ok(AliasExpr::Array(
                length,
                Box::new(self.expand_expr(*inner, caller_file, visiting)?),
            )),
            AliasExpr::Map(key, value) => Ok(AliasExpr::Map(
                Box::new(self.expand_expr(*key, caller_file, visiting)?),
                Box::new(self.expand_expr(*value, caller_file, visiting)?),
            )),
            AliasExpr::Channel(direction, inner) => Ok(AliasExpr::Channel(
                direction,
                Box::new(self.expand_expr(*inner, caller_file, visiting)?),
            )),
            AliasExpr::Variadic(inner) => Ok(AliasExpr::Variadic(Box::new(self.expand_expr(
                *inner,
                caller_file,
                visiting,
            )?))),
            AliasExpr::Function(params, results) => Ok(AliasExpr::Function(
                self.expand_list(params, caller_file, visiting)?,
                self.expand_list(results, caller_file, visiting)?,
            )),
            AliasExpr::Atom(_) | AliasExpr::Param(_) => Ok(expr),
        }
    }

    fn expand_list(
        &self,
        values: Vec<AliasExpr>,
        caller_file: &str,
        visiting: &mut BTreeSet<GoOwnerIdentity>,
    ) -> Result<Vec<AliasExpr>, AliasUnresolvedReason> {
        values
            .into_iter()
            .map(|value| self.expand_expr(value, caller_file, visiting))
            .collect()
    }

    fn expand_named(
        &self,
        named: &NamedType,
        args: Vec<AliasExpr>,
        caller_file: &str,
        visiting: &mut BTreeSet<GoOwnerIdentity>,
    ) -> Result<Option<AliasExpr>, AliasUnresolvedReason> {
        let (owner, mode) = match &named.owner_hint {
            OwnerHint::Local {
                package_dir,
                clause,
            } => (
                GoOwnerIdentity {
                    package_dir: package_dir.clone(),
                    package_clause: clause.clone(),
                    name: named.name.clone(),
                },
                GoOwnerReferenceMode::Bare,
            ),
            OwnerHint::ImportPath(path) => {
                let Some(packages) = self.packages_by_import_path.get(path) else {
                    return Ok(None);
                };
                if packages.len() != 1 {
                    return Err(AliasUnresolvedReason::Unresolvable);
                }
                let (package_dir, clause) = packages.iter().next().expect("one package");
                (
                    GoOwnerIdentity {
                        package_dir: package_dir.clone(),
                        package_clause: clause.clone(),
                        name: named.name.clone(),
                    },
                    GoOwnerReferenceMode::Qualified,
                )
            }
            OwnerHint::None => return Ok(None),
        };
        let Some(declarations) = self.declarations.get(&owner) else {
            return Ok(None);
        };
        let mut visible = Vec::new();
        for declaration in declarations {
            let (potentially_visible, exact) = exact_declaration_visibility(
                &owner,
                caller_file,
                mode,
                &declaration.defining_file,
                &self.profiles,
            );
            if potentially_visible && !exact {
                return Err(AliasUnresolvedReason::ProfileUncertain);
            }
            if exact {
                visible.push(declaration);
            }
        }
        if visible.is_empty() {
            return Ok(None);
        }
        if visible
            .iter()
            .all(|decl| matches!(decl.kind, AliasDeclarationKind::Defined))
        {
            return if matches!(
                named.identity,
                NameIdentity::Bare
                    | NameIdentity::Path {
                        qualified: false,
                        ..
                    }
            ) && predeclared(&named.name).is_some()
            {
                Err(AliasUnresolvedReason::DefinedVariant)
            } else {
                Ok(None)
            };
        }
        if visible
            .iter()
            .any(|decl| matches!(decl.kind, AliasDeclarationKind::Defined))
        {
            return Err(AliasUnresolvedReason::DefinedVariant);
        }
        if !visiting.insert(owner.clone()) {
            return Err(AliasUnresolvedReason::Cycle);
        }
        let result = (|| {
            let mut expanded = Vec::new();
            for declaration in visible {
                let AliasDeclarationKind::Alias { params, rhs } = &declaration.kind else {
                    unreachable!("defined variants rejected above")
                };
                if params.len() != args.len() {
                    return Err(AliasUnresolvedReason::Arity);
                }
                let bindings: BTreeMap<String, AliasExpr> =
                    params.iter().cloned().zip(args.iter().cloned()).collect();
                let rhs = rhs.clone()?.substitute(&bindings)?;
                expanded.push(self.expand_expr(rhs, &declaration.defining_file, visiting)?);
            }
            let first = expanded
                .first()
                .cloned()
                .ok_or(AliasUnresolvedReason::Unresolvable)?;
            if expanded
                .iter()
                .any(|expr| expr.render(true) != first.render(true))
            {
                return Err(AliasUnresolvedReason::ProfileUncertain);
            }
            self.telemetry.borrow_mut().expanded += 1;
            Ok(Some(first))
        })();
        visiting.remove(&owner);
        result
    }
}
