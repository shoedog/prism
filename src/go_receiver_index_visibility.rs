//! Exact visibility for Go cross-file receiver facts.

use crate::go_owner_partition::{exact_declaration_visibility, GoOwnerReferenceMode};
use crate::go_receiver_index::{GoReturnTypes, GoTypedFact};
use crate::resolution::resolve_go_receiver_owner_identity;
use std::collections::{BTreeMap, BTreeSet};

/// Resolve a call-RHS callee reference (`newDemux` or `pkg.New`, as written)
/// to its recorded S1 return type. Reuses `resolve_go_owner_identity`'s
/// bare/`pkg.`-qualified resolution rules and fails closed on every ambiguous
/// or inexact build-profile consult.
pub(crate) fn resolve_go_return_type_call(
    callee_text: &str,
    caller_file: &str,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    return_types: &GoReturnTypes,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> crate::go_owner_partition::GoPartitionSelection<crate::resolution::GoOwnerIdentity> {
    let Some(identity) = resolve_go_receiver_owner_identity(
        callee_text,
        caller_file,
        imports,
        package_basenames,
        go_file_profiles,
    ) else {
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence: crate::go_owner_partition::GoPartitionEvidence {
                uncertain: true,
                ..Default::default()
            },
        };
    };
    let Some(facts) = return_types.get(&identity) else {
        return Default::default();
    };
    unique_visible_return_owner(
        &identity,
        caller_file,
        GoOwnerReferenceMode::from_type_text(callee_text),
        facts,
        imports,
        package_basenames,
        go_file_profiles,
    )
}

/// Resolve one selected struct field's declared type in the declaration file's
/// package/import namespace. Returning only the raw field type is insufficient:
/// a bare `Gadget` in `factory.Widget` must not later be rebound as
/// `app.Gadget` by the caller that happens to use the widget.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GoResolvedStructField {
    pub owner: crate::resolution::GoOwnerIdentity,
    pub raw_type: String,
    pub embedded: bool,
}

pub(crate) fn resolve_go_struct_field_owner(
    owner: &crate::resolution::GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    field_name: &str,
    declarations: &BTreeSet<crate::go_owner_partition::GoStructDeclaration>,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> crate::go_owner_partition::GoPartitionSelection<GoResolvedStructField> {
    let all_values: BTreeSet<Option<String>> = declarations
        .iter()
        .map(|declaration| declaration.fields.get(field_name).cloned())
        .collect();
    let mut evidence = crate::go_owner_partition::GoPartitionEvidence::default();
    let mut visible_values = BTreeSet::new();
    let mut resolved_fields = BTreeSet::new();
    for declaration in declarations {
        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &declaration.defining_file,
            go_file_profiles,
        );
        if !visible {
            evidence.filtered_declarations += 1;
            continue;
        }
        evidence.visible_declarations += 1;
        if !exact {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        let field_type = declaration.fields.get(field_name).cloned();
        visible_values.insert(field_type.clone());
        let Some(field_type) = field_type else {
            continue;
        };
        let Some(field_owner) = resolve_go_receiver_owner_identity(
            &field_type,
            &declaration.defining_file,
            imports,
            package_basenames,
            go_file_profiles,
        ) else {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        };
        resolved_fields.insert(GoResolvedStructField {
            owner: field_owner,
            raw_type: field_type,
            embedded: declaration.embedded_fields.contains_key(field_name),
        });
    }
    evidence.distinct_visible_values = visible_values.len().max(resolved_fields.len());
    evidence.conflict = visible_values.len() > 1 || resolved_fields.len() > 1;
    if evidence.conflict {
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    evidence.recovered = all_values.len() > 1
        && evidence.filtered_declarations > 0
        && visible_values.len() == 1
        && resolved_fields.len() == 1;
    crate::go_owner_partition::GoPartitionSelection {
        value: resolved_fields.into_iter().next(),
        evidence,
    }
}

fn unique_visible_return_owner(
    owner: &crate::resolution::GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    facts: &BTreeSet<GoTypedFact>,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> crate::go_owner_partition::GoPartitionSelection<crate::resolution::GoOwnerIdentity> {
    let all_values: BTreeSet<_> = facts.iter().map(|fact| fact.ty.clone()).collect();
    let mut evidence = crate::go_owner_partition::GoPartitionEvidence::default();
    let mut owners = BTreeSet::new();
    let mut visible_types = BTreeSet::new();
    for fact in facts {
        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &fact.defining_file,
            go_file_profiles,
        );
        if !visible {
            evidence.filtered_declarations += 1;
            continue;
        }
        evidence.visible_declarations += 1;
        if !exact {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        let Some(return_owner) = resolve_go_receiver_owner_identity(
            &fact.ty,
            &fact.defining_file,
            imports,
            package_basenames,
            go_file_profiles,
        ) else {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        };
        visible_types.insert(fact.ty.clone());
        owners.insert(return_owner);
    }
    evidence.distinct_visible_values = visible_types.len().max(owners.len());
    evidence.conflict = visible_types.len() > 1 || owners.len() > 1;
    if evidence.conflict {
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    evidence.recovered = all_values.len() > 1
        && evidence.filtered_declarations > 0
        && visible_types.len() == 1
        && owners.len() == 1;
    crate::go_owner_partition::GoPartitionSelection {
        value: owners.into_iter().next(),
        evidence,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GoResolvedPackageVarType {
    pub owner: crate::resolution::GoOwnerIdentity,
    pub raw_type: String,
}

pub(crate) fn unique_visible_package_var_type(
    caller_file: &str,
    facts: &BTreeSet<GoTypedFact>,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> crate::go_owner_partition::GoPartitionSelection<GoResolvedPackageVarType> {
    let all_values: BTreeSet<_> = facts.iter().map(|fact| fact.ty.clone()).collect();
    let mut evidence = crate::go_owner_partition::GoPartitionEvidence::default();
    let Some(caller_profile) = go_file_profiles.get(caller_file) else {
        evidence.uncertain = true;
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence,
        };
    };
    if !crate::go_build_profile::profile_allows_exact(Some(caller_profile)) {
        evidence.uncertain = true;
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence,
        };
    }

    let mut visible_types = BTreeSet::new();
    let mut owners = BTreeSet::new();
    let mut resolved = BTreeSet::new();
    for fact in facts {
        if crate::resolution::dir_of(caller_file) != crate::resolution::dir_of(&fact.defining_file)
        {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        let Some(defining_profile) = go_file_profiles.get(&fact.defining_file) else {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        };
        let visibility = crate::go_build_profile::go_same_package_visible_detailed(
            caller_profile,
            defining_profile,
        );
        if !visibility.visible {
            evidence.filtered_declarations += 1;
            continue;
        }
        evidence.visible_declarations += 1;
        if !crate::go_build_profile::visibility_allows_exact(Some(defining_profile), &visibility) {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        let Some(owner) = resolve_go_receiver_owner_identity(
            &fact.ty,
            &fact.defining_file,
            imports,
            package_basenames,
            go_file_profiles,
        ) else {
            evidence.uncertain = true;
            return crate::go_owner_partition::GoPartitionSelection {
                value: None,
                evidence,
            };
        };
        visible_types.insert(fact.ty.clone());
        owners.insert(owner.clone());
        resolved.insert(GoResolvedPackageVarType {
            owner,
            raw_type: fact.ty.clone(),
        });
    }
    evidence.distinct_visible_values = visible_types.len().max(owners.len());
    evidence.conflict = visible_types.len() > 1 || owners.len() > 1;
    if evidence.conflict {
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    evidence.recovered = all_values.len() > 1
        && evidence.filtered_declarations > 0
        && visible_types.len() == 1
        && owners.len() == 1;
    crate::go_owner_partition::GoPartitionSelection {
        value: resolved.into_iter().next(),
        evidence,
    }
}
