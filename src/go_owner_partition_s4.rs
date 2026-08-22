//! Build-partition-safe S4 embedded-interface routing.

use crate::go_owner_partition::{
    exact_declaration_visibility, GoInterfaceDeclarations, GoMethodDeclarations,
    GoOwnerReferenceMode, GoPartitionEvidence, GoPartitionSelection, GoStructDeclarations,
};
use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteError {
    Conflict,
    Uncertain,
}

/// Select whether the receiver has an own method in the caller's exact build
/// partition. Struct declarations provide the profile universe, making absence
/// explicit when a method exists only in another build partition.
pub fn select_own_method(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    owner_type_text: &str,
    method_name: &str,
    struct_declarations: &GoStructDeclarations,
    method_declarations: &GoMethodDeclarations,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<bool> {
    let Some(structs) = struct_declarations.get(owner) else {
        return GoPartitionSelection::default();
    };
    let named_methods: Vec<_> = method_declarations
        .get(owner)
        .into_iter()
        .flatten()
        .filter(|declaration| declaration.method_name == method_name)
        .collect();
    let mode = GoOwnerReferenceMode::from_type_text(owner_type_text);
    let mut evidence = GoPartitionEvidence::default();
    let mut all_values = BTreeSet::new();
    let mut visible_values = BTreeSet::new();
    for declaration in structs {
        let mut present = false;
        for method in &named_methods {
            let (visible, exact) = exact_declaration_visibility(
                owner,
                &declaration.defining_file,
                GoOwnerReferenceMode::Bare,
                &method.defining_file,
                profiles,
            );
            if visible && !exact {
                evidence.uncertain = true;
                return GoPartitionSelection {
                    value: None,
                    evidence,
                };
            }
            present |= visible && exact;
        }
        all_values.insert(present);

        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &declaration.defining_file,
            profiles,
        );
        if !visible {
            evidence.filtered_declarations += 1;
            continue;
        }
        evidence.visible_declarations += 1;
        if !exact {
            evidence.uncertain = true;
            return GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        visible_values.insert(present);
    }
    evidence.distinct_visible_values = visible_values.len();
    evidence.conflict = visible_values.len() > 1;
    if evidence.conflict {
        return GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    evidence.recovered =
        all_values.len() > 1 && evidence.filtered_declarations > 0 && visible_values.len() == 1;
    GoPartitionSelection {
        value: visible_values.into_iter().next(),
        evidence,
    }
}

#[allow(clippy::too_many_arguments)]
fn interface_has_method(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    method_name: &str,
    declarations: &GoInterfaceDeclarations,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    evidence: &mut GoPartitionEvidence,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> Result<bool, RouteError> {
    if !visiting.insert(owner.clone()) {
        return Err(RouteError::Uncertain);
    }
    let mut values = BTreeSet::new();
    for declaration in declarations.get(owner).into_iter().flatten() {
        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &declaration.defining_file,
            profiles,
        );
        if !visible {
            evidence.filtered_declarations += 1;
            continue;
        }
        if !exact {
            visiting.remove(owner);
            return Err(RouteError::Uncertain);
        }
        let mut has_method = !declaration.generic && declaration.methods.contains(method_name);
        if !declaration.generic {
            for embedded in &declaration.embedded_types {
                has_method |= interface_has_method(
                    &GoOwnerIdentity {
                        package_dir: owner.package_dir.clone(),
                        package_clause: owner.package_clause.clone(),
                        name: embedded.clone(),
                    },
                    caller_file,
                    mode,
                    method_name,
                    declarations,
                    profiles,
                    evidence,
                    visiting,
                )?;
            }
        }
        values.insert(has_method);
    }
    visiting.remove(owner);
    if values.len() > 1 {
        Err(RouteError::Conflict)
    } else {
        Ok(values.into_iter().next().unwrap_or(false))
    }
}

fn interface_declares_method_any(
    owner: &GoOwnerIdentity,
    method_name: &str,
    declarations: &GoInterfaceDeclarations,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> bool {
    if !visiting.insert(owner.clone()) {
        return false;
    }
    let found = declarations
        .get(owner)
        .into_iter()
        .flatten()
        .any(|declaration| {
            if declaration.generic {
                return false;
            }
            declaration.methods.contains(method_name)
                || declaration.embedded_types.iter().any(|embedded| {
                    interface_declares_method_any(
                        &GoOwnerIdentity {
                            package_dir: owner.package_dir.clone(),
                            package_clause: owner.package_clause.clone(),
                            name: embedded.clone(),
                        },
                        method_name,
                        declarations,
                        visiting,
                    )
                })
        });
    visiting.remove(owner);
    found
}

#[allow(clippy::too_many_arguments)]
fn interface_signatures(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    declarations: &GoInterfaceDeclarations,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    evidence: &mut GoPartitionEvidence,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> Result<BTreeMap<String, String>, RouteError> {
    if !visiting.insert(owner.clone()) {
        return Err(RouteError::Uncertain);
    }
    let Some(owner_declarations) = declarations.get(owner) else {
        visiting.remove(owner);
        return Err(RouteError::Uncertain);
    };
    let mut values = BTreeSet::new();
    for declaration in owner_declarations {
        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &declaration.defining_file,
            profiles,
        );
        if !visible {
            evidence.filtered_declarations += 1;
            continue;
        }
        if !exact || !declaration.dispatchable {
            visiting.remove(owner);
            return Err(RouteError::Uncertain);
        }
        let mut methods = declaration.method_signatures.clone();
        for embedded in &declaration.embedded_types {
            let embedded_methods = interface_signatures(
                &GoOwnerIdentity {
                    package_dir: owner.package_dir.clone(),
                    package_clause: owner.package_clause.clone(),
                    name: embedded.clone(),
                },
                caller_file,
                mode,
                declarations,
                profiles,
                evidence,
                visiting,
            )?;
            for (name, signature) in embedded_methods {
                if methods
                    .insert(name, signature.clone())
                    .is_some_and(|existing| existing != signature)
                {
                    visiting.remove(owner);
                    return Err(RouteError::Conflict);
                }
            }
        }
        values.insert(methods);
    }
    visiting.remove(owner);
    if values.len() > 1 {
        Err(RouteError::Conflict)
    } else {
        Ok(values.into_iter().next().unwrap_or_default())
    }
}

pub fn select_interface_signatures(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    owner_type_text: &str,
    declarations: &GoInterfaceDeclarations,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<BTreeMap<String, String>> {
    let mut evidence = GoPartitionEvidence::default();
    let result = interface_signatures(
        owner,
        caller_file,
        GoOwnerReferenceMode::from_type_text(owner_type_text),
        declarations,
        profiles,
        &mut evidence,
        &mut BTreeSet::new(),
    );
    match result {
        Ok(methods) if !methods.is_empty() => GoPartitionSelection {
            value: Some(methods),
            evidence,
        },
        Ok(_) => GoPartitionSelection {
            value: None,
            evidence,
        },
        Err(RouteError::Conflict) => {
            evidence.conflict = true;
            GoPartitionSelection {
                value: None,
                evidence,
            }
        }
        Err(RouteError::Uncertain) => {
            evidence.uncertain = true;
            GoPartitionSelection {
                value: None,
                evidence,
            }
        }
    }
}

/// Select the one embedded-interface provider visible to a caller. A route is
/// present only when every visible, exact struct declaration agrees. Field or
/// method shadowing is represented as absence, so present/absent disagreement
/// drops just like two different interface providers.
#[allow(clippy::too_many_arguments)]
pub fn select_embedded_interface_route(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    owner_type_text: &str,
    method_name: &str,
    struct_declarations: &GoStructDeclarations,
    interface_declarations: &GoInterfaceDeclarations,
    method_declarations: &GoMethodDeclarations,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<String> {
    let Some(declarations) = struct_declarations.get(owner) else {
        return GoPartitionSelection::default();
    };
    let mode = GoOwnerReferenceMode::from_type_text(owner_type_text);
    let mut evidence = GoPartitionEvidence::default();
    let all_values: BTreeSet<Option<String>> = declarations
        .iter()
        .map(|declaration| {
            if declaration.fields.contains_key(method_name) {
                return None;
            }
            let candidates: BTreeSet<String> = declaration
                .embedded_types
                .iter()
                .filter(|embedded| {
                    interface_declares_method_any(
                        &GoOwnerIdentity {
                            package_dir: owner.package_dir.clone(),
                            package_clause: owner.package_clause.clone(),
                            name: (*embedded).clone(),
                        },
                        method_name,
                        interface_declarations,
                        &mut BTreeSet::new(),
                    )
                })
                .cloned()
                .collect();
            (candidates.len() == 1)
                .then(|| candidates.into_iter().next())
                .flatten()
        })
        .collect();
    let own_method = select_own_method(
        owner,
        caller_file,
        owner_type_text,
        method_name,
        struct_declarations,
        method_declarations,
        profiles,
    );
    evidence.merge(own_method.evidence);
    if evidence.conflict || evidence.uncertain {
        return GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    let own_method = own_method.value.unwrap_or(false);
    let mut values = BTreeSet::new();
    for declaration in declarations {
        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &declaration.defining_file,
            profiles,
        );
        if !visible {
            continue;
        }
        if !exact {
            evidence.uncertain = true;
            return GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        if own_method || declaration.fields.contains_key(method_name) {
            values.insert(None);
            continue;
        }
        let mut candidates = BTreeSet::new();
        for embedded in &declaration.embedded_types {
            let interface_owner = GoOwnerIdentity {
                package_dir: owner.package_dir.clone(),
                package_clause: owner.package_clause.clone(),
                name: embedded.clone(),
            };
            match interface_has_method(
                &interface_owner,
                caller_file,
                mode,
                method_name,
                interface_declarations,
                profiles,
                &mut evidence,
                &mut BTreeSet::new(),
            ) {
                Ok(true) => {
                    candidates.insert(embedded.clone());
                }
                Ok(false) => {}
                Err(RouteError::Conflict) => {
                    evidence.conflict = true;
                    return GoPartitionSelection {
                        value: None,
                        evidence,
                    };
                }
                Err(RouteError::Uncertain) => {
                    evidence.uncertain = true;
                    return GoPartitionSelection {
                        value: None,
                        evidence,
                    };
                }
            }
        }
        if candidates.len() > 1 {
            evidence.conflict = true;
            return GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        values.insert(candidates.into_iter().next());
    }
    evidence.distinct_visible_values = values.len();
    if values.len() > 1 {
        evidence.conflict = true;
        return GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    evidence.recovered |=
        all_values.len() > 1 && evidence.filtered_declarations > 0 && values.len() == 1;
    GoPartitionSelection {
        value: values.into_iter().next().flatten(),
        evidence,
    }
}
