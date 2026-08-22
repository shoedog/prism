//! Declaration-provenance snapshots for Go owner-identity lanes.
//!
//! Build profiles are intentionally not part of [`GoOwnerIdentity`]. Each
//! snapshot retains its defining file so consumers can apply the caller's
//! package/build visibility and certainty floor at consult time.

use crate::resolution::GoOwnerIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoStructDeclaration {
    pub defining_file: String,
    pub fields: BTreeMap<String, String>,
    pub embedded_types: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoInterfaceDeclaration {
    pub defining_file: String,
    pub methods: BTreeSet<String>,
    pub embedded_types: BTreeSet<String>,
    pub generic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoMethodDeclaration {
    pub defining_file: String,
    pub method_name: String,
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
fn exact_declaration_visibility(
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

pub fn select_struct_field(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    owner_type_text: &str,
    field_name: &str,
    declarations: &BTreeSet<GoStructDeclaration>,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoPartitionSelection<String> {
    let mode = GoOwnerReferenceMode::from_type_text(owner_type_text);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteError {
    Conflict,
    Uncertain,
}

fn visible_own_method(
    owner: &GoOwnerIdentity,
    caller_file: &str,
    mode: GoOwnerReferenceMode,
    method_name: &str,
    declarations: &GoMethodDeclarations,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    evidence: &mut GoPartitionEvidence,
) -> Result<bool, RouteError> {
    for declaration in declarations.get(owner).into_iter().flatten() {
        if declaration.method_name != method_name {
            continue;
        }
        let (visible, exact) = exact_declaration_visibility(
            owner,
            caller_file,
            mode,
            &declaration.defining_file,
            profiles,
        );
        if !visible {
            evidence.filtered_declarations += 1;
        } else if !exact {
            return Err(RouteError::Uncertain);
        } else {
            return Ok(true);
        }
    }
    Ok(false)
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
                let embedded_owner = GoOwnerIdentity {
                    package_dir: owner.package_dir.clone(),
                    package_clause: owner.package_clause.clone(),
                    name: embedded.clone(),
                };
                has_method |= interface_has_method(
                    &embedded_owner,
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
    let own_method = match visible_own_method(
        owner,
        caller_file,
        mode,
        method_name,
        method_declarations,
        profiles,
        &mut evidence,
    ) {
        Ok(value) => value,
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
    };
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
    GoPartitionSelection {
        value: values.into_iter().next().flatten(),
        evidence,
    }
}
