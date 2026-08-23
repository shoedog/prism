//! Declaration-backed routing for recovered Go receiver types.

use crate::call_graph::CallGraph;
use crate::go_owner_partition::{GoPartitionEvidence, GoPartitionSelection};
use crate::resolution::GoOwnerIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum GoTypeDeclarationForm {
    Struct,
    Interface,
    Defined { target: String },
    Alias { target: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct GoTypeDeclaration {
    pub defining_file: String,
    pub form: GoTypeDeclarationForm,
}

pub(crate) type GoTypeDeclarations = BTreeMap<GoOwnerIdentity, BTreeSet<GoTypeDeclaration>>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoCanonicalTypeTarget {
    pub owner: GoOwnerIdentity,
    pub defining_file: String,
    pub is_pointer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoDeclarationKind {
    Struct { target: GoCanonicalTypeTarget },
    DefinedNonInterface { target: GoCanonicalTypeTarget },
    Interface { interface_of: GoCanonicalTypeTarget },
    AliasToInterface { target: GoCanonicalTypeTarget },
    AliasToConcrete { target: GoCanonicalTypeTarget },
    AliasCyclicOrUnresolved,
    AmbiguousProfileConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoDeclarationKindEntry {
    pub declaring_file: Option<String>,
    pub kind: GoDeclarationKind,
    #[serde(default)]
    pub underlying_func: bool,
}

pub type GoDeclarationKindIndex = BTreeMap<GoOwnerIdentity, GoDeclarationKindEntry>;

pub(crate) fn declaration_kind_index(
    declarations: &GoTypeDeclarations,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> GoDeclarationKindIndex {
    declarations
        .iter()
        .map(|(owner, owner_declarations)| {
            let declaring_files: BTreeSet<_> = owner_declarations
                .iter()
                .map(|declaration| declaration.defining_file.clone())
                .collect();
            let (declaring_file, kind) = if declaring_files.len() > 1 {
                (None, GoDeclarationKind::AmbiguousProfileConflict)
            } else if owner_declarations.len() != 1 {
                (None, GoDeclarationKind::AliasCyclicOrUnresolved)
            } else {
                let declaration = owner_declarations.iter().next().expect("one declaration");
                let kind = match &declaration.form {
                    GoTypeDeclarationForm::Struct => GoDeclarationKind::Struct {
                        target: canonical_target(owner, declaration),
                    },
                    GoTypeDeclarationForm::Interface => GoDeclarationKind::Interface {
                        interface_of: canonical_target(owner, declaration),
                    },
                    GoTypeDeclarationForm::Defined { target } => resolve_target_text(
                        target,
                        owner,
                        &declaration.defining_file,
                        declarations,
                        imports,
                        package_basenames,
                        file_profiles,
                        &mut BTreeSet::from([owner.clone()]),
                    )
                    .map(|resolved| match resolved {
                        ResolvedDeclaration::Interface(interface_of) => {
                            GoDeclarationKind::Interface { interface_of }
                        }
                        ResolvedDeclaration::Concrete(_) => {
                            GoDeclarationKind::DefinedNonInterface {
                                target: canonical_target(owner, declaration),
                            }
                        }
                    })
                    .unwrap_or(GoDeclarationKind::AliasCyclicOrUnresolved),
                    GoTypeDeclarationForm::Alias { target } => resolve_target_text(
                        target,
                        owner,
                        &declaration.defining_file,
                        declarations,
                        imports,
                        package_basenames,
                        file_profiles,
                        &mut BTreeSet::from([owner.clone()]),
                    )
                    .map(|resolved| match resolved {
                        ResolvedDeclaration::Interface(target) => {
                            GoDeclarationKind::AliasToInterface { target }
                        }
                        ResolvedDeclaration::Concrete(target) => {
                            GoDeclarationKind::AliasToConcrete { target }
                        }
                    })
                    .unwrap_or(GoDeclarationKind::AliasCyclicOrUnresolved),
                };
                (Some(declaration.defining_file.clone()), kind)
            };
            let underlying_func = declaring_file.is_some()
                && crate::go_func_type::declaration_resolves_to_func(
                    owner,
                    declarations,
                    imports,
                    package_basenames,
                    file_profiles,
                );
            (
                owner.clone(),
                GoDeclarationKindEntry {
                    declaring_file,
                    kind,
                    underlying_func,
                },
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
enum ResolvedDeclaration {
    Interface(GoCanonicalTypeTarget),
    Concrete(GoCanonicalTypeTarget),
}

fn canonical_target(
    owner: &GoOwnerIdentity,
    declaration: &GoTypeDeclaration,
) -> GoCanonicalTypeTarget {
    GoCanonicalTypeTarget {
        owner: owner.clone(),
        defining_file: declaration.defining_file.clone(),
        is_pointer: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_target_text(
    target: &str,
    declaring_owner: &GoOwnerIdentity,
    declaring_file: &str,
    declarations: &GoTypeDeclarations,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> Option<ResolvedDeclaration> {
    let trimmed = target.trim();
    let bare = trimmed.trim_start_matches('*').trim();
    let is_pointer = bare.len() != trimmed.len();
    if is_definitely_concrete_type(bare) {
        return Some(ResolvedDeclaration::Concrete(GoCanonicalTypeTarget {
            owner: declaring_owner.clone(),
            defining_file: declaring_file.to_string(),
            is_pointer,
        }));
    }
    let target_owner = crate::resolution::resolve_go_owner_identity(
        bare,
        declaring_file,
        imports,
        package_basenames,
        file_profiles,
    )?;
    if !visiting.insert(target_owner.clone()) {
        return None;
    }
    let resolved = resolve_declared_owner(
        &target_owner,
        declarations,
        imports,
        package_basenames,
        file_profiles,
        visiting,
    );
    visiting.remove(&target_owner);
    match (is_pointer, resolved?) {
        (true, ResolvedDeclaration::Interface(mut target)) => {
            target.is_pointer = true;
            Some(ResolvedDeclaration::Interface(target))
        }
        (true, ResolvedDeclaration::Concrete(mut target)) => {
            target.is_pointer = true;
            Some(ResolvedDeclaration::Concrete(target))
        }
        (false, resolved) => Some(resolved),
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_declared_owner(
    owner: &GoOwnerIdentity,
    declarations: &GoTypeDeclarations,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> Option<ResolvedDeclaration> {
    let candidates = declarations.get(owner)?;
    let files: BTreeSet<_> = candidates
        .iter()
        .map(|declaration| declaration.defining_file.as_str())
        .collect();
    if files.len() != 1 || candidates.len() != 1 {
        return None;
    }
    let declaration = candidates.iter().next()?;
    let own_target = canonical_target(owner, declaration);
    match &declaration.form {
        GoTypeDeclarationForm::Struct => Some(ResolvedDeclaration::Concrete(own_target)),
        GoTypeDeclarationForm::Interface => Some(ResolvedDeclaration::Interface(own_target)),
        GoTypeDeclarationForm::Defined { target } => resolve_target_text(
            target,
            owner,
            &declaration.defining_file,
            declarations,
            imports,
            package_basenames,
            file_profiles,
            visiting,
        )
        .map(|resolved| match resolved {
            ResolvedDeclaration::Interface(interface_of) => {
                ResolvedDeclaration::Interface(interface_of)
            }
            ResolvedDeclaration::Concrete(_) => ResolvedDeclaration::Concrete(own_target),
        }),
        GoTypeDeclarationForm::Alias { target } => resolve_target_text(
            target,
            owner,
            &declaration.defining_file,
            declarations,
            imports,
            package_basenames,
            file_profiles,
            visiting,
        ),
    }
}

fn is_definitely_concrete_type(target: &str) -> bool {
    let target = target.trim();
    target.starts_with("struct {")
        || target.starts_with("struct{")
        || target.starts_with("[]")
        || target.starts_with('[')
        || target.starts_with("map[")
        || target.starts_with("chan ")
        || target.starts_with("<-chan ")
        || target.starts_with("func(")
        || matches!(
            target,
            "bool"
                | "byte"
                | "complex64"
                | "complex128"
                | "float32"
                | "float64"
                | "int"
                | "int8"
                | "int16"
                | "int32"
                | "int64"
                | "rune"
                | "string"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uintptr"
        )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoConcreteReceiverRoute {
    ConcreteDirect {
        owner: GoOwnerIdentity,
        selection: GoPartitionSelection<bool>,
    },
    ConcretePromoted {
        owner: GoOwnerIdentity,
    },
    ConcretePromotedDeferred {
        owner: GoOwnerIdentity,
    },
    EmbeddedInterfaceDispatch {
        owner: GoOwnerIdentity,
        interface_name: String,
        evidence: GoPartitionEvidence,
    },
    FuncValueField {
        owner: GoOwnerIdentity,
    },
    ConcreteNoSelector {
        owner: GoOwnerIdentity,
        evidence: GoPartitionEvidence,
    },
    InterfaceDispatch {
        owner: GoOwnerIdentity,
        interface_name: String,
    },
    R2OnDemandNameCollisionBail,
    ExternalNewRecoveryDrop,
    Unproven,
}

impl CallGraph {
    /// Shared resolver/manifest verdict for a recovered Go receiver.
    pub(crate) fn go_concrete_receiver_route(
        &self,
        recv_ty: &str,
        proven_owner: Option<&GoOwnerIdentity>,
        local_proof_shadowed: bool,
        newly_recovered: bool,
        method_name: &str,
        caller_file: &str,
    ) -> GoConcreteReceiverRoute {
        if newly_recovered
            && self.go_new_recovery_qualified_import_is_external(recv_ty, caller_file)
        {
            return GoConcreteReceiverRoute::ExternalNewRecoveryDrop;
        }
        let on_demand = proven_owner.is_none();
        let Some(receiver_owner) = self.go_receiver_owner(recv_ty, caller_file, proven_owner)
        else {
            return GoConcreteReceiverRoute::Unproven;
        };
        let Some(entry) = self.go_declaration_kind_index.get(&receiver_owner) else {
            return GoConcreteReceiverRoute::Unproven;
        };
        let Some(declaring_file) = entry.declaring_file.as_deref() else {
            return GoConcreteReceiverRoute::Unproven;
        };
        let mode = self.go_owner_reference_mode(&receiver_owner, caller_file);
        let (visible, exact) = crate::go_owner_partition::exact_declaration_visibility(
            &receiver_owner,
            caller_file,
            mode,
            declaring_file,
            &self.go_file_profiles,
        );
        if !visible || !exact {
            return GoConcreteReceiverRoute::Unproven;
        }
        if local_proof_shadowed {
            let collision_route = match &entry.kind {
                GoDeclarationKind::Interface { interface_of } => {
                    self.go_r2_interface_route(&interface_of.owner, on_demand)
                }
                GoDeclarationKind::AliasToInterface { target } => {
                    self.go_r2_interface_route(&target.owner, on_demand)
                }
                _ => GoConcreteReceiverRoute::Unproven,
            };
            return if matches!(
                collision_route,
                GoConcreteReceiverRoute::R2OnDemandNameCollisionBail
            ) {
                collision_route
            } else {
                GoConcreteReceiverRoute::Unproven
            };
        }

        match &entry.kind {
            GoDeclarationKind::Struct { target } => {
                self.go_concrete_route_for_target(recv_ty, target, method_name, caller_file)
            }
            GoDeclarationKind::Interface { interface_of } => {
                self.go_r2_interface_route(&interface_of.owner, on_demand)
            }
            GoDeclarationKind::DefinedNonInterface { target }
            | GoDeclarationKind::AliasToConcrete { target } => {
                self.go_concrete_route_for_target(recv_ty, target, method_name, caller_file)
            }
            GoDeclarationKind::AliasToInterface { target } => {
                self.go_r2_interface_route(&target.owner, on_demand)
            }
            GoDeclarationKind::AliasCyclicOrUnresolved
            | GoDeclarationKind::AmbiguousProfileConflict => GoConcreteReceiverRoute::Unproven,
        }
    }

    fn go_new_recovery_qualified_import_is_external(
        &self,
        recv_ty: &str,
        caller_file: &str,
    ) -> bool {
        let recv_ty = recv_ty
            .trim()
            .trim_start_matches('&')
            .trim_start_matches('*')
            .trim();
        let Some((qualifier, _)) = recv_ty.rsplit_once('.') else {
            return false;
        };
        let Some(import_path) = self
            .imports
            .get(caller_file)
            .and_then(|imports| imports.get(qualifier))
        else {
            return false;
        };
        let exact_key = crate::resolution::go_import_path_dir_key(import_path);
        if self.go_package_basenames.contains_key(&exact_key) {
            return false;
        }

        // A live effective-module graph makes absence of the exact key
        // authoritative: the import is outside the loaded module set. Without
        // module identity, retain the legacy unique-basename proof when one
        // exists and classify external only when no in-repo package can match.
        if self.go_module_graph.active > 0 {
            return true;
        }
        let basename = import_path.rsplit('/').next().unwrap_or(import_path);
        !self.go_package_basenames.contains_key(basename)
    }

    fn go_r2_interface_route(
        &self,
        interface_owner: &GoOwnerIdentity,
        on_demand: bool,
    ) -> GoConcreteReceiverRoute {
        let declaring_packages: BTreeSet<_> = self
            .go_interface_declarations
            .keys()
            .filter(|owner| owner.name == interface_owner.name)
            .map(|owner| (&owner.package_dir, &owner.package_clause))
            .collect();
        if on_demand && declaring_packages.len() > 1 {
            return GoConcreteReceiverRoute::R2OnDemandNameCollisionBail;
        }
        GoConcreteReceiverRoute::InterfaceDispatch {
            owner: interface_owner.clone(),
            interface_name: interface_owner.name.clone(),
        }
    }

    fn go_concrete_route_for_target(
        &self,
        recv_ty: &str,
        target: &GoCanonicalTypeTarget,
        method_name: &str,
        caller_file: &str,
    ) -> GoConcreteReceiverRoute {
        let Some((owner, own_method)) =
            self.go_own_method_partition(recv_ty, Some(&target.owner), method_name, caller_file)
        else {
            return GoConcreteReceiverRoute::Unproven;
        };
        if own_method.evidence.conflict || own_method.evidence.uncertain {
            return GoConcreteReceiverRoute::ConcreteNoSelector {
                owner,
                evidence: own_method.evidence,
            };
        }
        if own_method.value == Some(true) {
            return GoConcreteReceiverRoute::ConcreteDirect {
                owner,
                selection: own_method,
            };
        }

        let Some(declarations) = self.go_field_types.get(&owner) else {
            return GoConcreteReceiverRoute::ConcreteNoSelector {
                owner,
                evidence: GoPartitionEvidence::default(),
            };
        };
        let mode = self.go_owner_reference_mode(&owner, caller_file);
        let field = crate::go_owner_partition::select_struct_field_with_mode(
            &owner,
            caller_file,
            mode,
            method_name,
            declarations,
            &self.go_file_profiles,
        );
        if field.evidence.conflict || field.evidence.uncertain {
            return GoConcreteReceiverRoute::ConcreteNoSelector {
                owner,
                evidence: field.evidence,
            };
        }
        if let Some(field_type) = field.value.as_deref() {
            return if self.go_field_type_is_func(&owner, field_type) {
                GoConcreteReceiverRoute::FuncValueField { owner }
            } else {
                GoConcreteReceiverRoute::ConcreteNoSelector {
                    owner,
                    evidence: field.evidence,
                }
            };
        }

        let supply = self.go_embedded_selector_supply(&owner, method_name, caller_file);
        if supply.evidence.conflict || supply.evidence.uncertain {
            return GoConcreteReceiverRoute::ConcreteNoSelector {
                owner,
                evidence: supply.evidence,
            };
        }
        match supply.value {
            Some(crate::go_selector_supply::GoEmbeddedSelectorSupply::Concrete) => {
                if self.go_existing_embedding_promotion_hit(recv_ty, method_name) {
                    GoConcreteReceiverRoute::ConcretePromoted { owner }
                } else {
                    GoConcreteReceiverRoute::ConcretePromotedDeferred { owner }
                }
            }
            Some(crate::go_selector_supply::GoEmbeddedSelectorSupply::Interface(
                interface_name,
            )) => GoConcreteReceiverRoute::EmbeddedInterfaceDispatch {
                owner,
                interface_name,
                evidence: supply.evidence,
            },
            Some(crate::go_selector_supply::GoEmbeddedSelectorSupply::NoSelector) => {
                GoConcreteReceiverRoute::ConcreteNoSelector {
                    owner,
                    evidence: supply.evidence,
                }
            }
            None => {
                let embedded = self.go_embedded_interface_route(
                    recv_ty,
                    Some(&owner),
                    method_name,
                    caller_file,
                );
                if embedded.evidence.conflict || embedded.evidence.uncertain {
                    return GoConcreteReceiverRoute::ConcreteNoSelector {
                        owner,
                        evidence: embedded.evidence,
                    };
                }
                if let Some(interface_name) = embedded.value {
                    return GoConcreteReceiverRoute::EmbeddedInterfaceDispatch {
                        owner,
                        interface_name,
                        evidence: embedded.evidence,
                    };
                }
                GoConcreteReceiverRoute::ConcreteNoSelector {
                    owner,
                    evidence: field.evidence,
                }
            }
        }
    }
}
