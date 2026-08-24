use super::{
    GoPromotedEmbeddedField, GoPromotedMethodSnapshot, GoPromotedProfileSnapshot, RawProfile,
};
use crate::go_build_profile::GoBuildProfile;
use crate::go_owner_partition::{
    exact_declaration_visibility, GoMethodDeclarations, GoOwnerReferenceMode,
};
use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProfiledCandidate {
    selector_path: Vec<GoPromotedEmbeddedField>,
    snapshot: GoPromotedMethodSnapshot,
    required_profiles: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ProfiledField {
    name: String,
    depth: usize,
    required_profiles: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SelectionShape {
    selector_path: Vec<GoPromotedEmbeddedField>,
    method: String,
    target_owner: GoOwnerIdentity,
    depth: usize,
    field_shadowed: bool,
    value_method_set: bool,
}

impl SelectionShape {
    fn from_candidate(candidate: &ProfiledCandidate, field_shadowed: bool) -> Self {
        Self {
            selector_path: candidate.selector_path.clone(),
            method: candidate.snapshot.method.clone(),
            target_owner: candidate.snapshot.target_owner.clone(),
            depth: candidate.snapshot.depth,
            field_shadowed,
            value_method_set: candidate.snapshot.value_method_set,
        }
    }

    fn into_snapshot(
        self,
        profile_variants: BTreeSet<crate::call_graph::FunctionId>,
    ) -> Option<GoPromotedMethodSnapshot> {
        let target = profile_variants.iter().next()?.clone();
        Some(GoPromotedMethodSnapshot {
            method: self.method,
            target,
            profile_variants,
            target_owner: self.target_owner,
            depth: self.depth,
            field_shadowed: self.field_shadowed,
            value_method_set: self.value_method_set,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutcomeKind {
    Absent,
    Ambiguous,
    Selected(BTreeSet<SelectionShape>),
    Shadowed,
    Conflict,
}

struct WorldOutcome {
    kind: OutcomeKind,
    variants: BTreeMap<SelectionShape, BTreeSet<crate::call_graph::FunctionId>>,
}

pub(super) fn promoted_for_profile(
    outer: &GoOwnerIdentity,
    outer_profile: &RawProfile,
    raw: &BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    conflicts: &BTreeMap<GoOwnerIdentity, bool>,
    methods: &GoMethodDeclarations,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> (BTreeSet<GoPromotedMethodSnapshot>, BTreeSet<String>, bool) {
    let outer_file = outer_profile.snapshot.defining_file.clone();
    let outer_required = BTreeSet::from([outer_file.clone()]);
    let mut fields = BTreeSet::new();
    record_fields(&outer_profile.snapshot, 0, &outer_required, &mut fields);
    let mut candidates = BTreeSet::new();
    let mut root_alternatives = BTreeSet::new();
    let mut uncertain = false;
    let mut path = BTreeSet::from([outer.clone()]);
    for embedded in &outer_profile.snapshot.embedded_fields {
        let mut selector_path = vec![embedded.clone()];
        walk_target(
            embedded,
            &outer_file,
            1,
            embedded.pointer,
            raw,
            conflicts,
            methods,
            profiles,
            &mut path,
            &mut selector_path,
            &outer_required,
            &mut root_alternatives,
            &mut fields,
            &mut candidates,
            &mut uncertain,
        );
    }

    candidates.retain(|candidate| {
        !outer_profile
            .snapshot
            .own_methods
            .contains(&candidate.snapshot.method)
    });
    let method_names = candidates
        .iter()
        .map(|candidate| candidate.snapshot.method.clone())
        .collect::<BTreeSet<_>>();
    if method_names.is_empty() {
        return (BTreeSet::new(), BTreeSet::new(), uncertain);
    }

    let relevant_files = candidates
        .iter()
        .flat_map(|candidate| candidate.required_profiles.iter().cloned())
        .chain(
            fields
                .iter()
                .flat_map(|field| field.required_profiles.iter().cloned()),
        )
        .chain(
            root_alternatives
                .iter()
                .flat_map(|alternative| alternative.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let Some(partitions) =
        crate::go_build_profile::active_profile_partitions(profiles, &relevant_files)
    else {
        return (BTreeSet::new(), BTreeSet::new(), true);
    };
    let worlds = partitions
        .into_iter()
        .filter(|active| {
            root_alternatives
                .iter()
                .any(|alternative| alternative.is_subset(active))
        })
        .collect::<Vec<_>>();
    if worlds.is_empty() {
        return (BTreeSet::new(), BTreeSet::new(), true);
    }

    let mut promoted = BTreeSet::new();
    let mut ambiguous = BTreeSet::new();
    let mut selection_conflict = uncertain;
    for method in method_names {
        let mut expected = None;
        let mut variants =
            BTreeMap::<SelectionShape, BTreeSet<crate::call_graph::FunctionId>>::new();
        for world in &worlds {
            let outcome = select_in_world(&method, world, &candidates, &fields);
            if outcome.kind == OutcomeKind::Conflict {
                selection_conflict = true;
                expected = None;
                break;
            }
            if expected
                .as_ref()
                .is_some_and(|prior: &OutcomeKind| prior != &outcome.kind)
            {
                selection_conflict = true;
                expected = None;
                break;
            }
            expected.get_or_insert_with(|| outcome.kind.clone());
            for (shape, targets) in outcome.variants {
                variants.entry(shape).or_default().extend(targets);
            }
        }
        match expected {
            Some(OutcomeKind::Ambiguous) => {
                ambiguous.insert(method);
            }
            Some(OutcomeKind::Selected(_)) | Some(OutcomeKind::Shadowed) => {
                promoted.extend(
                    variants
                        .into_iter()
                        .filter_map(|(shape, targets)| shape.into_snapshot(targets)),
                );
            }
            Some(OutcomeKind::Absent) | None => {}
            Some(OutcomeKind::Conflict) => unreachable!("conflict handled above"),
        }
    }
    (promoted, ambiguous, selection_conflict)
}

fn select_in_world(
    method: &str,
    active: &BTreeSet<String>,
    candidates: &BTreeSet<ProfiledCandidate>,
    fields: &BTreeSet<ProfiledField>,
) -> WorldOutcome {
    let active_candidates = candidates
        .iter()
        .filter(|candidate| {
            candidate.snapshot.method == method && candidate.required_profiles.is_subset(active)
        })
        .collect::<Vec<_>>();
    let Some(shallowest_depth) = active_candidates
        .iter()
        .map(|candidate| candidate.snapshot.depth)
        .min()
    else {
        return WorldOutcome {
            kind: OutcomeKind::Absent,
            variants: BTreeMap::new(),
        };
    };
    let shallowest = active_candidates
        .into_iter()
        .filter(|candidate| candidate.snapshot.depth == shallowest_depth)
        .collect::<Vec<_>>();
    let field_depth = fields
        .iter()
        .filter(|field| field.name == method && field.required_profiles.is_subset(active))
        .map(|field| field.depth)
        .min();
    if field_depth.is_some_and(|depth| depth <= shallowest_depth) {
        return outcome_for_shapes(shallowest, true, OutcomeKind::Shadowed);
    }

    let selector_paths = shallowest
        .iter()
        .map(|candidate| &candidate.selector_path)
        .collect::<BTreeSet<_>>();
    if selector_paths.len() > 1 {
        return WorldOutcome {
            kind: OutcomeKind::Ambiguous,
            variants: BTreeMap::new(),
        };
    }

    let outcome = outcome_for_shapes(shallowest, false, OutcomeKind::Selected(BTreeSet::new()));
    let shapes = outcome.variants.keys().cloned().collect::<BTreeSet<_>>();
    if shapes.len() != 1 {
        return WorldOutcome {
            kind: OutcomeKind::Conflict,
            variants: BTreeMap::new(),
        };
    }
    WorldOutcome {
        kind: OutcomeKind::Selected(shapes),
        variants: outcome.variants,
    }
}

fn outcome_for_shapes(
    candidates: Vec<&ProfiledCandidate>,
    field_shadowed: bool,
    kind: OutcomeKind,
) -> WorldOutcome {
    let mut variants = BTreeMap::<SelectionShape, BTreeSet<crate::call_graph::FunctionId>>::new();
    for candidate in candidates {
        variants
            .entry(SelectionShape::from_candidate(candidate, field_shadowed))
            .or_default()
            .insert(candidate.snapshot.target.clone());
    }
    WorldOutcome { kind, variants }
}

#[allow(clippy::too_many_arguments)]
fn walk_target(
    embedded: &GoPromotedEmbeddedField,
    consumer_file: &str,
    depth: usize,
    value_can_use_pointer_receiver: bool,
    raw: &BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    conflicts: &BTreeMap<GoOwnerIdentity, bool>,
    methods: &GoMethodDeclarations,
    profiles: &BTreeMap<String, GoBuildProfile>,
    path: &mut BTreeSet<GoOwnerIdentity>,
    selector_path: &mut Vec<GoPromotedEmbeddedField>,
    required_profiles: &BTreeSet<String>,
    root_alternatives: &mut BTreeSet<BTreeSet<String>>,
    fields: &mut BTreeSet<ProfiledField>,
    candidates: &mut BTreeSet<ProfiledCandidate>,
    uncertain: &mut bool,
) {
    if conflicts.get(&embedded.target).copied().unwrap_or(true)
        || !path.insert(embedded.target.clone())
    {
        return;
    }
    let mode = reference_mode(consumer_file, &embedded.target, profiles);
    let Some(target_profiles) = raw.get(&embedded.target) else {
        *uncertain = true;
        path.remove(&embedded.target);
        return;
    };
    for target_profile in target_profiles {
        let target_file = &target_profile.snapshot.defining_file;
        let (visible, exact) = exact_declaration_visibility(
            &embedded.target,
            consumer_file,
            mode,
            target_file,
            profiles,
        );
        if visible && !exact {
            *uncertain = true;
            continue;
        }
        if !exact {
            continue;
        }
        let mut next_required = required_profiles.clone();
        next_required.insert(target_file.clone());
        if depth == 1 {
            root_alternatives.insert(next_required.clone());
        }
        record_fields(&target_profile.snapshot, depth, &next_required, fields);
        let target_mode = reference_mode(target_file, &embedded.target, profiles);
        let (visible_methods, methods_uncertain) = super::visible_methods(
            &embedded.target,
            target_file,
            target_mode,
            methods,
            profiles,
        );
        *uncertain |= methods_uncertain;
        for method in visible_methods {
            let mut method_profiles = next_required.clone();
            method_profiles.insert(method.defining_file.clone());
            let target = method.function_id;
            candidates.insert(ProfiledCandidate {
                selector_path: selector_path.clone(),
                snapshot: GoPromotedMethodSnapshot {
                    method: method.method_name,
                    profile_variants: BTreeSet::from([target.clone()]),
                    target,
                    target_owner: embedded.target.clone(),
                    depth,
                    field_shadowed: false,
                    value_method_set: !method.is_pointer_receiver || value_can_use_pointer_receiver,
                },
                required_profiles: method_profiles,
            });
        }
        for child in &target_profile.snapshot.embedded_fields {
            selector_path.push(child.clone());
            walk_target(
                child,
                target_file,
                depth + 1,
                value_can_use_pointer_receiver || child.pointer,
                raw,
                conflicts,
                methods,
                profiles,
                path,
                selector_path,
                &next_required,
                root_alternatives,
                fields,
                candidates,
                uncertain,
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
    required_profiles: &BTreeSet<String>,
    fields: &mut BTreeSet<ProfiledField>,
) {
    for name in profile
        .ordinary_fields
        .iter()
        .chain(profile.embedded_fields.iter().map(|field| &field.selector))
    {
        fields.insert(ProfiledField {
            name: name.clone(),
            depth,
            required_profiles: required_profiles.clone(),
        });
    }
}
