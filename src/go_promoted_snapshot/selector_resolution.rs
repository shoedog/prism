use super::{
    GoPromotedEmbeddedField, GoPromotedMethodSnapshot, GoPromotedProfileSnapshot, RawProfile,
};
use crate::go_build_profile::GoBuildProfile;
use crate::go_owner_partition::{
    exact_declaration_visibility, GoMethodDeclarations, GoOwnerReferenceMode,
};
use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn promoted_for_profile(
    outer: &GoOwnerIdentity,
    outer_profile: &RawProfile,
    raw: &BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    conflicts: &BTreeMap<GoOwnerIdentity, bool>,
    methods: &GoMethodDeclarations,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> (BTreeSet<GoPromotedMethodSnapshot>, BTreeSet<String>) {
    let mut field_depth = BTreeMap::new();
    record_fields(&outer_profile.snapshot, 0, &mut field_depth);
    let mut candidates = BTreeSet::new();
    let mut path = BTreeSet::from([outer.clone()]);
    for embedded in &outer_profile.snapshot.embedded_fields {
        let mut selector_path = vec![embedded.clone()];
        walk_target(
            embedded,
            &outer_profile.snapshot.defining_file,
            1,
            embedded.pointer,
            raw,
            conflicts,
            methods,
            profiles,
            &mut path,
            &mut selector_path,
            &mut field_depth,
            &mut candidates,
        );
    }
    let mut candidates_by_method = BTreeMap::<String, Vec<GoPromotedMethodSnapshot>>::new();
    for (_, candidate) in candidates {
        if !outer_profile
            .snapshot
            .own_methods
            .contains(&candidate.method)
        {
            candidates_by_method
                .entry(candidate.method.clone())
                .or_default()
                .push(candidate);
        }
    }

    let mut promoted = BTreeSet::new();
    let mut ambiguous = BTreeSet::new();
    for (method, candidates) in candidates_by_method {
        let Some(shallowest_depth) = candidates.iter().map(|candidate| candidate.depth).min()
        else {
            continue;
        };
        let mut shallowest = candidates
            .into_iter()
            .filter(|candidate| candidate.depth == shallowest_depth);
        let Some(mut selected) = shallowest.next() else {
            continue;
        };
        if shallowest.next().is_some() {
            ambiguous.insert(method);
            continue;
        }
        selected.field_shadowed = field_depth
            .get(&selected.method)
            .is_some_and(|depth| *depth <= selected.depth);
        promoted.insert(selected);
    }
    (promoted, ambiguous)
}

#[allow(clippy::too_many_arguments)]
fn walk_target(
    embedded: &GoPromotedEmbeddedField,
    profile_file: &str,
    depth: usize,
    value_can_use_pointer_receiver: bool,
    raw: &BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    conflicts: &BTreeMap<GoOwnerIdentity, bool>,
    methods: &GoMethodDeclarations,
    profiles: &BTreeMap<String, GoBuildProfile>,
    path: &mut BTreeSet<GoOwnerIdentity>,
    selector_path: &mut Vec<GoPromotedEmbeddedField>,
    field_depth: &mut BTreeMap<String, usize>,
    candidates: &mut BTreeSet<(Vec<GoPromotedEmbeddedField>, GoPromotedMethodSnapshot)>,
) {
    if conflicts.get(&embedded.target).copied().unwrap_or(true)
        || !path.insert(embedded.target.clone())
    {
        return;
    }
    let mode = reference_mode(profile_file, &embedded.target, profiles);
    let Some(target_profiles) = raw.get(&embedded.target) else {
        path.remove(&embedded.target);
        return;
    };
    let visible_profiles = target_profiles.iter().filter(|target_profile| {
        exact_declaration_visibility(
            &embedded.target,
            profile_file,
            mode,
            &target_profile.snapshot.defining_file,
            profiles,
        ) == (true, true)
    });
    for target_profile in visible_profiles {
        record_fields(&target_profile.snapshot, depth, field_depth);
        let (visible_methods, uncertain) =
            super::visible_methods(&embedded.target, profile_file, mode, methods, profiles);
        if uncertain {
            continue;
        }
        for method in visible_methods {
            candidates.insert((
                selector_path.clone(),
                GoPromotedMethodSnapshot {
                    method: method.method_name,
                    target: method.function_id,
                    target_owner: embedded.target.clone(),
                    depth,
                    field_shadowed: false,
                    value_method_set: !method.is_pointer_receiver || value_can_use_pointer_receiver,
                },
            ));
        }
        for child in &target_profile.snapshot.embedded_fields {
            selector_path.push(child.clone());
            walk_target(
                child,
                profile_file,
                depth + 1,
                value_can_use_pointer_receiver || child.pointer,
                raw,
                conflicts,
                methods,
                profiles,
                path,
                selector_path,
                field_depth,
                candidates,
            );
            selector_path.pop();
        }
    }
    path.remove(&embedded.target);
}

fn reference_mode(
    profile_file: &str,
    target: &GoOwnerIdentity,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> GoOwnerReferenceMode {
    let same_package = profiles.get(profile_file).is_some_and(|profile| {
        crate::resolution::dir_of(profile_file) == target.package_dir
            && profile.package_clause == target.package_clause
    });
    if same_package {
        GoOwnerReferenceMode::Bare
    } else {
        GoOwnerReferenceMode::Qualified
    }
}

fn record_fields(
    profile: &GoPromotedProfileSnapshot,
    depth: usize,
    field_depth: &mut BTreeMap<String, usize>,
) {
    for name in profile
        .ordinary_fields
        .iter()
        .chain(profile.embedded_fields.iter().map(|field| &field.selector))
    {
        field_depth
            .entry(name.clone())
            .and_modify(|prior| *prior = (*prior).min(depth))
            .or_insert(depth);
    }
}
