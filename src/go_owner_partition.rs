//! Declaration-provenance snapshots for Go owner-identity lanes.
//!
//! Build profiles are intentionally not part of [`GoOwnerIdentity`]. Each
//! snapshot retains its defining file so consumers can apply the caller's
//! package/build visibility and certainty floor at consult time.

use crate::resolution::GoOwnerIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type GoOwnerPartitionSiteKey =
    (crate::call_graph::FunctionId, String, usize, usize, usize);

pub(crate) fn site_key(site: &crate::call_graph::CallSite) -> GoOwnerPartitionSiteKey {
    (
        site.caller.clone(),
        site.callee_name.clone(),
        site.line,
        site.start_byte,
        site.end_byte,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoStructDeclaration {
    pub defining_file: String,
    pub fields: BTreeMap<String, String>,
    /// Every anonymous embedded selector with its raw declared type. This is
    /// distinct from `embedded_types`, whose entries are safe S4 interface
    /// candidates (local, non-pointer names only).
    pub embedded_fields: BTreeMap<String, String>,
    /// Anonymous field syntax that has no resolvable Go selector identity.
    /// Retained so profile-safe promotion snapshots can fail closed instead of
    /// silently treating the declaration as having no embedded fields.
    #[serde(default)]
    pub unresolved_embedded_fields: BTreeSet<String>,
    pub embedded_types: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoInterfaceDeclaration {
    pub defining_file: String,
    pub methods: BTreeSet<String>,
    pub method_signatures: BTreeMap<String, String>,
    pub embedded_types: BTreeSet<String>,
    pub generic: bool,
    pub dispatchable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoMethodDeclaration {
    pub defining_file: String,
    pub method_name: String,
    pub signature: Option<String>,
    pub generic: bool,
    pub is_pointer_receiver: bool,
    pub function_id: crate::call_graph::FunctionId,
}

pub type GoStructDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoStructDeclaration>>;
pub type GoInterfaceDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoInterfaceDeclaration>>;
pub type GoMethodDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoMethodDeclaration>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoOwnerReferenceMode {
    Bare,
    Qualified,
}

impl GoOwnerReferenceMode {
    pub fn from_type_text(type_text: &str) -> Self {
        if type_text.trim().contains('.') {
            Self::Qualified
        } else {
            Self::Bare
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GoPartitionEvidence {
    pub visible_declarations: usize,
    pub filtered_declarations: usize,
    pub distinct_visible_values: usize,
    pub recovered: bool,
    pub conflict: bool,
    pub uncertain: bool,
}

impl GoPartitionEvidence {
    pub fn merge(&mut self, other: Self) {
        self.visible_declarations += other.visible_declarations;
        self.filtered_declarations += other.filtered_declarations;
        self.distinct_visible_values = self
            .distinct_visible_values
            .max(other.distinct_visible_values);
        self.recovered |= other.recovered;
        self.conflict |= other.conflict;
        self.uncertain |= other.uncertain;
    }
}

/// Whole-program or per-resolution counts for owner-partition decisions.
/// A site is recorded once even when its decision consulted several snapshot
/// lanes. `affected_edges` counts the resolved or suppressed candidate edges at
/// that site, not the legacy conflicting-owner support set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoOwnerPartitionTelemetry {
    pub drops: usize,
    pub recovered: usize,
    pub affected_edges: usize,
}

impl GoOwnerPartitionTelemetry {
    pub fn observe(&mut self, evidence: GoPartitionEvidence, affected_edges: usize) {
        if evidence.conflict {
            self.drops += 1;
        } else if evidence.recovered {
            self.recovered += 1;
        } else {
            return;
        }
        self.affected_edges += affected_edges;
    }

    pub fn merge(&mut self, other: Self) {
        self.drops += other.drops;
        self.recovered += other.recovered;
        self.affected_edges += other.affected_edges;
    }

    /// Fold multiple partition decisions made while resolving one call site
    /// into one site-level observation. A later resolution-stage decision owns
    /// the affected-edge count because it sees the final candidate set.
    pub fn coalesce_site(self, later: Self) -> Self {
        let affected_edges = if later.affected_sites() > 0 {
            later.affected_edges
        } else {
            self.affected_edges
        };
        let drops = usize::from(self.drops > 0 || later.drops > 0);
        let recovered = usize::from(drops == 0 && (self.recovered > 0 || later.recovered > 0));
        Self {
            drops,
            recovered,
            affected_edges: if drops + recovered > 0 {
                affected_edges
            } else {
                0
            },
        }
    }

    pub fn affected_sites(self) -> usize {
        self.drops + self.recovered
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoPartitionSelection<T> {
    pub value: Option<T>,
    pub evidence: GoPartitionEvidence,
}

impl<T> Default for GoPartitionSelection<T> {
    fn default() -> Self {
        Self {
            value: None,
            evidence: GoPartitionEvidence::default(),
        }
    }
}

/// Apply the one exactness floor shared by every owner-declaration consult.
/// Qualified references rewrite only the caller's package namespace to the
/// resolved target identity; test-file and build constraints remain intact.
pub(crate) fn exact_declaration_visibility(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    defining_file: &str,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> (bool, bool) {
    let (Some(caller), Some(defining)) = (profiles.get(caller_file), profiles.get(defining_file))
    else {
        return (true, false); // potentially visible but unprovable: fail closed.
    };
    if defining.package_clause != owner.package_clause
        || crate::resolution::dir_of(defining_file) != owner.package_dir
    {
        return (true, false); // corrupted/stale provenance must never mint Exact.
    }
    if mode == GoOwnerReferenceMode::Qualified && defining.is_test_file {
        return (false, true); // imported packages never include their test files.
    }
    let mut target_caller = caller.clone();
    if mode == GoOwnerReferenceMode::Qualified {
        target_caller.package_clause = owner.package_clause.clone();
    }
    let visibility =
        crate::go_build_profile::go_same_package_visible_detailed(&target_caller, defining);
    let exact = visibility.visible
        && crate::go_build_profile::profile_allows_exact(Some(caller))
        && crate::go_build_profile::visibility_allows_exact(Some(defining), &visibility);
    (visibility.visible, exact)
}

/// Exact build/test visibility for a structural implementer declaration. Test
/// declarations are visible only from the caller's own package namespace;
/// importing some other package never imports that package's test files.
pub(crate) fn exact_cross_package_visibility(
    caller_file: &str,
    defining_file: &str,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> (bool, bool) {
    let (Some(caller), Some(defining)) = (profiles.get(caller_file), profiles.get(defining_file))
    else {
        return (true, false);
    };
    let caller_owns_test_namespace = crate::resolution::dir_of(caller_file)
        == crate::resolution::dir_of(defining_file)
        && caller.package_clause == defining.package_clause;
    if defining.is_test_file && !caller_owns_test_namespace {
        return (false, true);
    }
    let mut target_caller = caller.clone();
    target_caller.package_clause = defining.package_clause.clone();
    let visibility =
        crate::go_build_profile::go_same_package_visible_detailed(&target_caller, defining);
    let exact = visibility.visible
        && crate::go_build_profile::profile_allows_exact(Some(caller))
        && crate::go_build_profile::visibility_allows_exact(Some(defining), &visibility);
    (visibility.visible, exact)
}

/// Exact build/test visibility for a P5 registration site. Registration
/// provenance belongs to the package performing the assignment, not to the
/// struct field's owner package, so only caller-to-site compatibility applies.
fn exact_registration_visibility(
    caller_file: &str,
    registration_file: &str,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> (bool, bool) {
    let (Some(caller), Some(registration)) =
        (profiles.get(caller_file), profiles.get(registration_file))
    else {
        return (true, false);
    };
    let caller_owns_test_namespace = crate::resolution::dir_of(caller_file)
        == crate::resolution::dir_of(registration_file)
        && caller.package_clause == registration.package_clause;
    if registration.is_test_file && !caller_owns_test_namespace {
        return (false, true);
    }
    let mut registration_caller = caller.clone();
    registration_caller.package_clause = registration.package_clause.clone();
    let visibility = crate::go_build_profile::go_same_package_visible_detailed(
        &registration_caller,
        registration,
    );
    let exact = visibility.visible
        && crate::go_build_profile::profile_allows_exact(Some(caller))
        && crate::go_build_profile::visibility_allows_exact(Some(registration), &visibility);
    (visibility.visible, exact)
}

/// Filter provenance-bearing values, union target sets from compatible profile
/// groups, and drop when mutually exclusive visible groups disagree. Multiple
/// registrations in files that coexist in one build remain a legitimate P5
/// fanout.
pub fn select_profiled_values<'a, T, I>(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    owner_type_text: &str,
    facts: I,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<BTreeSet<T>>
where
    T: Clone + Ord,
    I: IntoIterator<Item = (&'a str, T)>,
{
    let mode = GoOwnerReferenceMode::from_type_text(owner_type_text);
    select_profiled_values_with_mode(owner, caller_file, mode, facts, profiles)
}

pub(crate) fn select_profiled_values_with_mode<'a, T, I>(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    facts: I,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<BTreeSet<T>>
where
    T: Clone + Ord,
    I: IntoIterator<Item = (&'a str, T)>,
{
    select_values_by_visibility(
        facts,
        profiles,
        ProfileComparison::DeclarationNamespace,
        |defining_file| {
            exact_declaration_visibility(owner, caller_file, mode, defining_file, profiles)
        },
    )
}

/// Filter P5 registration targets by the invocation caller's exact build/test
/// compatibility with each registration site. The field-owner declaration is
/// intentionally irrelevant here; it was already validated by the field lane.
pub fn select_registration_values<'a, T, I>(
    caller_file: &str,
    facts: I,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<BTreeSet<T>>
where
    T: Clone + Ord,
    I: IntoIterator<Item = (&'a str, T)>,
{
    select_values_by_visibility(
        facts,
        profiles,
        ProfileComparison::BuildOnly,
        |registration_file| exact_registration_visibility(caller_file, registration_file, profiles),
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ProfileComparison {
    DeclarationNamespace,
    BuildOnly,
}

fn select_values_by_visibility<'a, T, I, F>(
    facts: I,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    comparison: ProfileComparison,
    mut visibility: F,
) -> GoPartitionSelection<BTreeSet<T>>
where
    T: Clone + Ord,
    I: IntoIterator<Item = (&'a str, T)>,
    F: FnMut(&str) -> (bool, bool),
{
    let mut evidence = GoPartitionEvidence::default();
    let mut all_groups: BTreeMap<crate::go_build_profile::GoBuildProfile, BTreeSet<T>> =
        BTreeMap::new();
    let mut visible_groups: BTreeMap<crate::go_build_profile::GoBuildProfile, BTreeSet<T>> =
        BTreeMap::new();
    for (defining_file, value) in facts {
        let Some(profile) = profiles.get(defining_file) else {
            evidence.uncertain = true;
            return GoPartitionSelection {
                value: None,
                evidence,
            };
        };
        all_groups
            .entry(profile.clone())
            .or_default()
            .insert(value.clone());
        let (visible, exact) = visibility(defining_file);
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
        visible_groups
            .entry(profile.clone())
            .or_default()
            .insert(value);
    }
    let all_sets: BTreeSet<BTreeSet<T>> = all_groups.into_values().collect();
    let visible_groups: Vec<_> = visible_groups.into_iter().collect();
    for (index, (left_profile, left_values)) in visible_groups.iter().enumerate() {
        for (right_profile, right_values) in visible_groups.iter().skip(index + 1) {
            let mut left_profile = left_profile.clone();
            let mut right_profile = right_profile.clone();
            if comparison == ProfileComparison::BuildOnly {
                left_profile.package_clause.clear();
                right_profile.package_clause.clear();
            }
            let overlap =
                crate::go_build_profile::go_same_package_visible(&left_profile, &right_profile)
                    || crate::go_build_profile::go_same_package_visible(
                        &right_profile,
                        &left_profile,
                    );
            if !overlap && left_values != right_values {
                evidence.conflict = true;
                return GoPartitionSelection {
                    value: None,
                    evidence,
                };
            }
        }
    }
    let selected: BTreeSet<T> = visible_groups
        .into_iter()
        .flat_map(|(_, values)| values)
        .collect();
    evidence.distinct_visible_values = selected.len();
    evidence.recovered =
        all_sets.len() > 1 && evidence.filtered_declarations > 0 && !selected.is_empty();
    GoPartitionSelection {
        value: (!selected.is_empty()).then_some(selected),
        evidence,
    }
}

pub fn select_struct_field(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    owner_type_text: &str,
    field_name: &str,
    declarations: &BTreeSet<GoStructDeclaration>,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<String> {
    select_struct_field_with_mode(
        owner,
        caller_file,
        GoOwnerReferenceMode::from_type_text(owner_type_text),
        field_name,
        declarations,
        profiles,
    )
}

pub fn select_struct_field_with_mode(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    field_name: &str,
    declarations: &BTreeSet<GoStructDeclaration>,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<String> {
    let all_values: BTreeSet<Option<String>> = declarations
        .iter()
        .map(|declaration| declaration.fields.get(field_name).cloned())
        .collect();
    let mut evidence = GoPartitionEvidence::default();
    let mut visible_values = BTreeSet::new();
    for declaration in declarations {
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
        visible_values.insert(declaration.fields.get(field_name).cloned());
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
        value: visible_values.into_iter().next().flatten(),
        evidence,
    }
}

pub use crate::go_owner_partition_s4::{
    select_embedded_interface_route, select_embedded_interface_route_with_mode,
    select_interface_presence_with_mode, select_interface_signatures,
    select_interface_signatures_with_mode, select_own_method, select_own_method_with_mode,
};
