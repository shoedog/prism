//! Depth-aware selector supply for proven concrete Go receivers.
//!
//! This consult intentionally walks the serialized declaration snapshots rather
//! than the flattened promotion projections. It only chooses the winning lane;
//! the existing promotion and S4 consumers remain responsible for minting edges.

use crate::call_graph::CallGraph;
use crate::go_owner_partition::{
    exact_declaration_visibility, GoOwnerReferenceMode, GoPartitionEvidence, GoPartitionSelection,
    GoStructDeclaration,
};
use crate::resolution::GoOwnerIdentity;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum GoEmbeddedSelectorSupply {
    Concrete,
    Interface(String),
    NoSelector,
}

#[derive(Clone)]
struct WalkNode {
    owner: GoOwnerIdentity,
    path: BTreeSet<GoOwnerIdentity>,
}

impl CallGraph {
    pub(crate) fn go_embedded_selector_supply(
        &self,
        owner: &GoOwnerIdentity,
        method_name: &str,
        caller_file: &str,
    ) -> GoPartitionSelection<GoEmbeddedSelectorSupply> {
        let mode = self.go_owner_reference_mode(owner, caller_file);
        let root = self.visible_struct_declaration(owner, caller_file, mode);
        let mut evidence = root.evidence;
        if evidence.conflict || evidence.uncertain {
            return GoPartitionSelection {
                value: None,
                evidence,
            };
        }
        let Some(root) = root.value else {
            return GoPartitionSelection {
                value: None,
                evidence,
            };
        };
        let mut current = self.embedded_nodes(owner, &root, BTreeSet::from([owner.clone()]));

        while !current.is_empty() {
            let mut suppliers = Vec::new();
            let mut next = Vec::new();
            for node in current {
                if self.go_interface_declarations.contains_key(&node.owner) {
                    let interface =
                        crate::go_owner_partition::select_interface_signatures_with_mode(
                            &node.owner,
                            caller_file,
                            mode,
                            &self.go_interface_declarations,
                            &self.go_file_profiles,
                        );
                    evidence.merge(interface.evidence);
                    if evidence.conflict || evidence.uncertain {
                        return GoPartitionSelection {
                            value: None,
                            evidence,
                        };
                    }
                    if interface
                        .value
                        .as_ref()
                        .is_some_and(|methods| methods.contains_key(method_name))
                    {
                        suppliers
                            .push(GoEmbeddedSelectorSupply::Interface(node.owner.name.clone()));
                    }
                    continue;
                }

                let own = crate::go_owner_partition::select_own_method_with_mode(
                    &node.owner,
                    caller_file,
                    mode,
                    method_name,
                    &self.go_field_types,
                    &self.go_method_declarations,
                    &self.go_file_profiles,
                );
                evidence.merge(own.evidence);
                let declaration = self.visible_struct_declaration(&node.owner, caller_file, mode);
                evidence.merge(declaration.evidence);
                if evidence.conflict || evidence.uncertain {
                    return GoPartitionSelection {
                        value: None,
                        evidence,
                    };
                }
                let Some(declaration) = declaration.value else {
                    continue;
                };
                if declaration.fields.contains_key(method_name) {
                    suppliers.push(GoEmbeddedSelectorSupply::NoSelector);
                }
                if own.value == Some(true) {
                    suppliers.push(GoEmbeddedSelectorSupply::Concrete);
                }
                next.extend(self.embedded_nodes(&node.owner, &declaration, node.path));
            }

            match suppliers.as_slice() {
                [] => current = next,
                [supplier] => {
                    return GoPartitionSelection {
                        value: Some(supplier.clone()),
                        evidence,
                    }
                }
                _ => {
                    evidence.conflict = true;
                    return GoPartitionSelection {
                        value: Some(GoEmbeddedSelectorSupply::NoSelector),
                        evidence,
                    };
                }
            }
        }

        GoPartitionSelection {
            value: None,
            evidence,
        }
    }

    fn visible_struct_declaration(
        &self,
        owner: &GoOwnerIdentity,
        caller_file: &str,
        mode: GoOwnerReferenceMode,
    ) -> GoPartitionSelection<GoStructDeclaration> {
        let mut evidence = GoPartitionEvidence::default();
        let mut visible = BTreeSet::new();
        for declaration in self.go_field_types.get(owner).into_iter().flatten() {
            let (is_visible, exact) = exact_declaration_visibility(
                owner,
                caller_file,
                mode,
                &declaration.defining_file,
                &self.go_file_profiles,
            );
            if !is_visible {
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
            visible.insert(declaration.clone());
        }
        evidence.distinct_visible_values = visible.len();
        evidence.conflict = visible.len() > 1;
        GoPartitionSelection {
            value: (visible.len() == 1).then(|| visible.into_iter().next().unwrap()),
            evidence,
        }
    }

    fn embedded_nodes(
        &self,
        parent: &GoOwnerIdentity,
        declaration: &GoStructDeclaration,
        path: BTreeSet<GoOwnerIdentity>,
    ) -> Vec<WalkNode> {
        declaration
            .embedded_fields
            .values()
            .filter_map(|raw| local_embedded_owner(parent, raw))
            .filter(|(owner, pointer_syntax)| {
                !self.embedded_pointer_interface(owner, *pointer_syntax)
            })
            .filter_map(|(owner, _)| {
                let mut child_path = path.clone();
                child_path.insert(owner.clone()).then_some(WalkNode {
                    owner,
                    path: child_path,
                })
            })
            .collect()
    }

    fn embedded_pointer_interface(&self, owner: &GoOwnerIdentity, pointer_syntax: bool) -> bool {
        use crate::go_concrete_receiver::GoDeclarationKind;

        self.go_declaration_kind_index
            .get(owner)
            .is_some_and(|entry| match &entry.kind {
                GoDeclarationKind::Interface { interface_of } => {
                    pointer_syntax || interface_of.is_pointer
                }
                GoDeclarationKind::AliasToInterface { target } => {
                    pointer_syntax || target.is_pointer
                }
                _ => false,
            })
    }
}

fn local_embedded_owner(parent: &GoOwnerIdentity, raw: &str) -> Option<(GoOwnerIdentity, bool)> {
    let raw = raw.trim();
    let name = raw.trim_start_matches('*').trim();
    if name.is_empty()
        || name.contains('.')
        || name.contains('[')
        || !name.chars().all(|ch| ch == '_' || ch.is_alphanumeric())
    {
        return None;
    }
    Some((
        GoOwnerIdentity {
            package_dir: parent.package_dir.clone(),
            package_clause: parent.package_clause.clone(),
            name: name.to_string(),
        },
        name.len() != raw.len(),
    ))
}
