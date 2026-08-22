//! Exact visibility for Go return-typed receiver facts.

use crate::go_owner_partition::{exact_declaration_visibility, GoOwnerReferenceMode};
use crate::go_receiver_index::{GoReturnTypes, GoTypedFact};
use crate::resolution::resolve_go_owner_identity;
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
) -> crate::go_owner_partition::GoPartitionSelection<String> {
    let Some(identity) = resolve_go_owner_identity(
        callee_text,
        caller_file,
        imports,
        package_basenames,
        go_file_profiles,
    ) else {
        return Default::default();
    };
    let Some(facts) = return_types.get(&identity) else {
        return Default::default();
    };
    unique_visible_return_type(
        &identity,
        caller_file,
        GoOwnerReferenceMode::from_type_text(callee_text),
        facts,
        go_file_profiles,
    )
}

fn unique_visible_return_type(
    owner: &crate::resolution::GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    facts: &BTreeSet<GoTypedFact>,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> crate::go_owner_partition::GoPartitionSelection<String> {
    let all_values: BTreeSet<_> = facts.iter().map(|fact| fact.ty.clone()).collect();
    let mut evidence = crate::go_owner_partition::GoPartitionEvidence::default();
    let mut tys = BTreeSet::new();
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
        tys.insert(fact.ty.clone());
    }
    evidence.distinct_visible_values = tys.len();
    evidence.conflict = tys.len() > 1;
    if evidence.conflict {
        return crate::go_owner_partition::GoPartitionSelection {
            value: None,
            evidence,
        };
    }
    evidence.recovered =
        all_values.len() > 1 && evidence.filtered_declarations > 0 && tys.len() == 1;
    crate::go_owner_partition::GoPartitionSelection {
        value: tys.into_iter().next(),
        evidence,
    }
}

pub(crate) fn unique_visible_type(
    caller_file: &str,
    facts: &BTreeSet<GoTypedFact>,
    go_file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> Option<String> {
    let caller_profile = go_file_profiles.get(caller_file);
    let mut tys = BTreeSet::new();
    for fact in facts {
        let defining_profile = go_file_profiles.get(&fact.defining_file);
        let visibility = match (caller_profile, defining_profile) {
            (Some(caller), Some(defining)) => {
                if crate::resolution::dir_of(caller_file)
                    == crate::resolution::dir_of(&fact.defining_file)
                {
                    Some(crate::go_build_profile::go_same_package_visible_detailed(
                        caller, defining,
                    ))
                } else {
                    let mut imported = caller.clone();
                    imported.package_clause = defining.package_clause.clone();
                    imported.is_test_file = false;
                    Some(crate::go_build_profile::go_same_package_visible_detailed(
                        &imported, defining,
                    ))
                }
            }
            _ => None,
        };
        let visible = visibility.as_ref().map_or(true, |vis| vis.visible);
        if visible {
            let exact_allowed = visibility.as_ref().map_or_else(
                || crate::go_build_profile::profile_allows_exact(defining_profile),
                |vis| crate::go_build_profile::visibility_allows_exact(defining_profile, vis),
            );
            if !exact_allowed {
                return None;
            }
            tys.insert(fact.ty.clone());
        }
    }
    (tys.len() == 1).then(|| tys.into_iter().next().unwrap())
}
