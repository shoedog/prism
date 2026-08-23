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
}

pub type GoDeclarationKindIndex = BTreeMap<GoOwnerIdentity, GoDeclarationKindEntry>;

pub(crate) fn basic_declaration_kind_index(
    declarations: &GoTypeDeclarations,
) -> GoDeclarationKindIndex {
    declarations
        .iter()
        .map(|(owner, declarations)| {
            let declaring_files: BTreeSet<_> = declarations
                .iter()
                .map(|declaration| declaration.defining_file.clone())
                .collect();
            let (declaring_file, kind) = if declaring_files.len() > 1 {
                (None, GoDeclarationKind::AmbiguousProfileConflict)
            } else if declarations.len() != 1 {
                (None, GoDeclarationKind::AliasCyclicOrUnresolved)
            } else {
                let declaration = declarations.iter().next().expect("one declaration");
                let target = GoCanonicalTypeTarget {
                    owner: owner.clone(),
                    defining_file: declaration.defining_file.clone(),
                    is_pointer: false,
                };
                let kind = match &declaration.form {
                    GoTypeDeclarationForm::Struct => GoDeclarationKind::Struct { target },
                    GoTypeDeclarationForm::Interface => GoDeclarationKind::Interface {
                        interface_of: target,
                    },
                    GoTypeDeclarationForm::Defined { .. } | GoTypeDeclarationForm::Alias { .. } => {
                        GoDeclarationKind::AliasCyclicOrUnresolved
                    }
                };
                (Some(declaration.defining_file.clone()), kind)
            };
            (
                owner.clone(),
                GoDeclarationKindEntry {
                    declaring_file,
                    kind,
                },
            )
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoConcreteReceiverRoute {
    ConcreteDirect {
        owner: GoOwnerIdentity,
        selection: GoPartitionSelection<bool>,
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
    },
    InterfaceDispatch {
        owner: GoOwnerIdentity,
        interface_name: String,
    },
    Unproven,
}

impl CallGraph {
    /// Shared resolver/manifest verdict for a recovered Go receiver.
    pub(crate) fn go_concrete_receiver_route(
        &self,
        recv_ty: &str,
        proven_owner: Option<&GoOwnerIdentity>,
        method_name: &str,
        caller_file: &str,
    ) -> GoConcreteReceiverRoute {
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

        match &entry.kind {
            GoDeclarationKind::Struct { target } => {
                let Some((owner, selection)) = self.go_own_method_partition(
                    recv_ty,
                    Some(&target.owner),
                    method_name,
                    caller_file,
                ) else {
                    return GoConcreteReceiverRoute::Unproven;
                };
                if selection.value == Some(true)
                    && !selection.evidence.conflict
                    && !selection.evidence.uncertain
                {
                    GoConcreteReceiverRoute::ConcreteDirect { owner, selection }
                } else {
                    GoConcreteReceiverRoute::Unproven
                }
            }
            GoDeclarationKind::Interface { interface_of } => {
                GoConcreteReceiverRoute::InterfaceDispatch {
                    owner: interface_of.owner.clone(),
                    interface_name: interface_of.owner.name.clone(),
                }
            }
            GoDeclarationKind::DefinedNonInterface { .. }
            | GoDeclarationKind::AliasToInterface { .. }
            | GoDeclarationKind::AliasToConcrete { .. }
            | GoDeclarationKind::AliasCyclicOrUnresolved
            | GoDeclarationKind::AmbiguousProfileConflict => GoConcreteReceiverRoute::Unproven,
        }
    }
}
