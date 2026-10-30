// Frozen c84f3769 declaration loops; only shared unchanged helpers are reused.
use super::*;
use crate::go_type_alias::import_memo::reference::signature_imports_reference;

pub(super) fn build_reference(
    files: &BTreeMap<String, ParsedFile>,
    package_import_paths: &BTreeMap<String, String>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    type_declarations: &BTreeMap<GoOwnerIdentity, BTreeSet<String>>,
    structs: &GoStructDeclarations,
    interfaces: &GoInterfaceDeclarations,
    methods: &GoMethodDeclarations,
) -> GoPromotedSelectorSnapshot {
    // Invalid anonymous struct embedding is retained by tree-sitter under a
    // top-level ERROR node. Recover it only for this diagnostic snapshot; the
    // provider's routing inputs remain untouched.
    let mut recovered_structs = structs.clone();
    recover_error_structs(files, profiles, &mut recovered_structs);
    let structs = &recovered_structs;
    let local_import_paths = interface_profiles::local_import_paths(package_import_paths, profiles);
    let aliases = crate::go_type_alias::GoAliasResolver::build(
        files,
        package_import_paths,
        &local_import_paths,
        profiles,
    );
    let mut raw = BTreeMap::<GoOwnerIdentity, Vec<RawProfile>>::new();
    for (owner, declarations) in structs {
        for declaration in declarations {
            let mut embedded_fields = BTreeSet::new();
            let mut unresolved_embedded_fields = declaration.unresolved_embedded_fields.clone();
            let imports = files
                .get(&declaration.defining_file)
                .map(|parsed| {
                    signature_imports_reference(
                        parsed,
                        local_import_paths
                            .get(&declaration.defining_file)
                            .map(String::as_str),
                    )
                })
                .unwrap_or_default();
            for (selector, raw_type) in &declaration.embedded_fields {
                match aliases.resolve_embedded_owner(raw_type, &declaration.defining_file, &imports)
                {
                    Ok((pointer, target))
                        if structs.contains_key(&target)
                            || interfaces.contains_key(&target)
                            || type_declarations.contains_key(&target) =>
                    {
                        embedded_fields.insert(GoPromotedEmbeddedField {
                            pointer,
                            target,
                            selector: selector.clone(),
                        });
                    }
                    _ => {
                        unresolved_embedded_fields.insert(raw_type.clone());
                    }
                }
            }
            let ordinary_fields = declaration
                .fields
                .keys()
                .filter(|name| {
                    name.as_str() != "_" && !declaration.embedded_fields.contains_key(*name)
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            let (visible_methods, methods_uncertain) = visible_methods(
                owner,
                &declaration.defining_file,
                GoOwnerReferenceMode::Bare,
                methods,
                profiles,
            );
            let own_methods = visible_methods
                .iter()
                .map(|method| method.method_name.clone())
                .collect::<BTreeSet<_>>();
            let own_method_shapes = visible_methods
                .iter()
                .map(|method| GoPromotedOwnMethodShape {
                    method: method.method_name.clone(),
                    pointer_receiver: method.is_pointer_receiver,
                })
                .collect::<BTreeSet<_>>();
            let comparison = ComparisonKey {
                embedded_fields: embedded_fields.clone(),
                ordinary_fields: ordinary_fields.clone(),
                own_methods: own_methods.clone(),
                own_method_shapes: own_method_shapes.clone(),
                interface_method_signatures: BTreeMap::new(),
            };
            raw.entry(owner.clone()).or_default().push(RawProfile {
                snapshot: GoPromotedProfileSnapshot {
                    defining_file: declaration.defining_file.clone(),
                    embedded_fields,
                    unresolved_embedded_fields: unresolved_embedded_fields.clone(),
                    ordinary_fields,
                    own_methods,
                    own_method_shapes,
                    promoted_methods: BTreeSet::new(),
                    ambiguous_promoted_methods: BTreeSet::new(),
                },
                comparison,
                unresolved: methods_uncertain || !unresolved_embedded_fields.is_empty(),
            });
        }
    }

    extend_interfaces(
        &mut raw,
        files,
        &local_import_paths,
        interfaces,
        structs,
        type_declarations,
        &aliases,
    );

    // Defined non-struct types can be embedded and contribute own methods.
    // Keep them as internal hop profiles; only struct owners are published.
    for (owner, declaring_files) in type_declarations {
        if raw.contains_key(owner) || interfaces.contains_key(owner) {
            continue;
        }
        for defining_file in declaring_files {
            let (visible_methods, methods_uncertain) = visible_methods(
                owner,
                defining_file,
                GoOwnerReferenceMode::Bare,
                methods,
                profiles,
            );
            let own_methods = visible_methods
                .iter()
                .map(|method| method.method_name.clone())
                .collect::<BTreeSet<_>>();
            let own_method_shapes = visible_methods
                .iter()
                .map(|method| GoPromotedOwnMethodShape {
                    method: method.method_name.clone(),
                    pointer_receiver: method.is_pointer_receiver,
                })
                .collect::<BTreeSet<_>>();
            raw.entry(owner.clone()).or_default().push(RawProfile {
                snapshot: GoPromotedProfileSnapshot {
                    defining_file: defining_file.clone(),
                    embedded_fields: BTreeSet::new(),
                    unresolved_embedded_fields: BTreeSet::new(),
                    ordinary_fields: BTreeSet::new(),
                    own_methods: own_methods.clone(),
                    own_method_shapes: own_method_shapes.clone(),
                    promoted_methods: BTreeSet::new(),
                    ambiguous_promoted_methods: BTreeSet::new(),
                },
                comparison: ComparisonKey {
                    embedded_fields: BTreeSet::new(),
                    ordinary_fields: BTreeSet::new(),
                    own_methods,
                    own_method_shapes,
                    interface_method_signatures: BTreeMap::new(),
                },
                unresolved: methods_uncertain,
            });
        }
    }

    let mut conflicts = BTreeMap::new();
    for owner in raw.keys() {
        owner_conflicts(owner, &raw, &mut conflicts, &mut BTreeSet::new());
    }

    let mut owners = BTreeMap::new();
    for (owner, profiles_for_owner) in &raw {
        if !structs.contains_key(owner) {
            continue;
        }
        let mut verdict = if conflicts.get(owner).copied().unwrap_or(true) {
            GoPromotedSnapshotVerdict::ProfileConflict
        } else {
            GoPromotedSnapshotVerdict::ProfileUnique
        };
        let mut declarations = Vec::new();
        for raw_profile in profiles_for_owner {
            let mut declaration = raw_profile.snapshot.clone();
            let (promoted_methods, ambiguous_promoted_methods, selection_conflict) =
                selector_resolution::promoted_for_profile(
                    owner,
                    raw_profile,
                    &raw,
                    &conflicts,
                    methods,
                    profiles,
                );
            if selection_conflict {
                verdict = GoPromotedSnapshotVerdict::ProfileConflict;
            }
            declaration.promoted_methods = promoted_methods;
            declaration.ambiguous_promoted_methods = ambiguous_promoted_methods;
            declarations.push(declaration);
        }
        declarations.sort_by(|left, right| left.defining_file.cmp(&right.defining_file));
        owners.insert(
            owner.clone(),
            GoPromotedOwnerSnapshot {
                verdict,
                declarations,
            },
        );
    }
    GoPromotedSelectorSnapshot { owners }
}

fn extend_interfaces(
    raw: &mut BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    files: &BTreeMap<String, ParsedFile>,
    local_import_paths: &BTreeMap<String, String>,
    interfaces: &GoInterfaceDeclarations,
    structs: &GoStructDeclarations,
    type_declarations: &BTreeMap<GoOwnerIdentity, BTreeSet<String>>,
    aliases: &crate::go_type_alias::GoAliasResolver,
) {
    for (owner, declarations) in interfaces {
        for declaration in declarations {
            let imports = files
                .get(&declaration.defining_file)
                .map(|parsed| {
                    signature_imports_reference(
                        parsed,
                        local_import_paths
                            .get(&declaration.defining_file)
                            .map(String::as_str),
                    )
                })
                .unwrap_or_default();
            let mut embedded_fields = BTreeSet::new();
            let mut unresolved_embedded_fields = BTreeSet::new();
            for raw_type in &declaration.embedded_types {
                let resolved =
                    aliases.resolve_embedded_owner(raw_type, &declaration.defining_file, &imports);
                match resolved {
                    Ok((false, target)) if interfaces.contains_key(&target) => {
                        embedded_fields.insert(GoPromotedEmbeddedField {
                            pointer: false,
                            target,
                            selector: selector(raw_type),
                        });
                    }
                    _ => {
                        unresolved_embedded_fields.insert(raw_type.clone());
                    }
                }
            }
            let own_methods = declaration.methods.clone();
            let comparison = ComparisonKey {
                embedded_fields: embedded_fields.clone(),
                ordinary_fields: BTreeSet::new(),
                own_methods: own_methods.clone(),
                own_method_shapes: BTreeSet::new(),
                interface_method_signatures: declaration.method_signatures.clone(),
            };
            raw.entry(owner.clone()).or_default().push(RawProfile {
                snapshot: GoPromotedProfileSnapshot {
                    defining_file: declaration.defining_file.clone(),
                    embedded_fields,
                    unresolved_embedded_fields: unresolved_embedded_fields.clone(),
                    ordinary_fields: BTreeSet::new(),
                    own_methods,
                    own_method_shapes: BTreeSet::new(),
                    promoted_methods: BTreeSet::new(),
                    ambiguous_promoted_methods: BTreeSet::new(),
                },
                comparison,
                unresolved: declaration.generic
                    || declaration.method_signatures.len() != declaration.methods.len()
                    || !unresolved_embedded_fields.is_empty()
                    || structs.contains_key(owner)
                    || !type_declarations.contains_key(owner),
            });
        }
    }
}

fn selector(raw_type: &str) -> String {
    raw_type
        .trim()
        .trim_start_matches('*')
        .rsplit('.')
        .next()
        .unwrap_or(raw_type)
        .to_string()
}
