//! Caller-dependent selection, unchanged from the pre-memo implementation.
use super::*;

impl CallGraph {
    pub(in crate::resolution) fn go_compute_caller_scoped_satisfiers<'a>(
        &'a self,
        interface_owner: &GoOwnerIdentity,
        method_name: &str,
        caller_file: &str,
        include_promoted: bool,
    ) -> GoCallerScopedSatisfiers<'a> {
        let mut evidence = crate::go_owner_partition::GoPartitionEvidence::default();
        let mut promoted_telemetry = GoProvenPromotedSupplyTelemetry::default();
        let mode = self.go_owner_reference_mode(interface_owner, caller_file);
        let interface = crate::go_owner_partition::select_interface_signatures_with_mode(
            interface_owner,
            caller_file,
            mode,
            &self.go_interface_declarations,
            &self.go_file_profiles,
        );
        evidence.merge(interface.evidence);
        let Some(required) = interface.value else {
            return GoCallerScopedSatisfiers {
                interface_selected: false,
                selection: crate::go_owner_partition::GoPartitionSelection {
                    value: None,
                    evidence,
                },
                promoted: promoted_telemetry,
            };
        };
        let requires_interface_namespace = required
            .keys()
            .any(|name| !name.chars().next().is_some_and(char::is_uppercase));
        let mut all_satisfiers: Vec<(String, &'a FunctionId)> = Vec::new();
        let mut conflicting_owners = 0usize;
        let mut uncertain_owners = 0usize;
        let mut concrete_owners: BTreeSet<&GoOwnerIdentity> =
            self.go_method_declarations.keys().collect();
        if include_promoted {
            concrete_owners.extend(self.go_promoted_selector_snapshot().owners.keys());
        }
        for concrete_owner in concrete_owners {
            if requires_interface_namespace
                && (concrete_owner.package_dir != interface_owner.package_dir
                    || concrete_owner.package_clause != interface_owner.package_clause)
            {
                continue;
            }
            let mut visible_methods: BTreeMap<
                &str,
                Vec<&crate::go_owner_partition::GoMethodDeclaration>,
            > = BTreeMap::new();
            let mut owner_uncertain = false;
            for declaration in self
                .go_method_declarations
                .get(concrete_owner)
                .into_iter()
                .flatten()
            {
                let (visible, exact) = crate::go_owner_partition::exact_cross_package_visibility(
                    caller_file,
                    &declaration.defining_file,
                    &self.go_file_profiles,
                );
                if !visible {
                    continue;
                }
                if !exact {
                    owner_uncertain = true;
                    break;
                }
                visible_methods
                    .entry(&declaration.method_name)
                    .or_default()
                    .push(declaration);
            }
            if owner_uncertain {
                // Exactness belongs to this concrete owner's method set. An
                // uncertain owner cannot be admitted, but it also cannot
                // invalidate a separately proven structural satisfier.
                uncertain_owners += 1;
                continue;
            }
            if required.keys().any(|name| {
                visible_methods
                    .get(name.as_str())
                    .is_some_and(|methods| methods.len() > 1)
            }) {
                // Conflict belongs to this concrete owner's method set, not
                // to every independently-proven structural satisfier. Keep
                // this owner excluded; if no exact owner survives, preserve
                // the existing site-level conflict drop below.
                conflicting_owners += 1;
                continue;
            }
            let mut supplied: BTreeMap<&str, (bool, &'a FunctionId)> = BTreeMap::new();
            for (name, signature) in &required {
                if let Some(method) = visible_methods
                    .get(name.as_str())
                    .and_then(|methods| methods.first())
                {
                    if !method.generic
                        && method.signature.as_ref().is_some_and(|candidate| {
                            crate::type_providers::go::GoTypeProvider::canon_signatures_match(
                                candidate, signature,
                            )
                        })
                    {
                        supplied.insert(name, (!method.is_pointer_receiver, &method.function_id));
                    }
                    // An own method shadows a promoted method even when its
                    // signature does not satisfy this interface.
                    continue;
                }
                if !include_promoted {
                    continue;
                }
                match crate::go_promoted_snapshot::consult_promoted_snapshot_method(
                    self.go_promoted_selector_snapshot(),
                    &self.go_method_declarations,
                    concrete_owner,
                    name,
                ) {
                    crate::go_promoted_snapshot::GoPromotedSnapshotMethodConsult::Hit(hit) => {
                        if hit.declaration.signature.as_ref().is_some_and(|candidate| {
                            crate::type_providers::go::GoTypeProvider::canon_signatures_match(
                                candidate, signature,
                            )
                        }) {
                            supplied.insert(
                                name,
                                (hit.promoted.value_method_set, &hit.promoted.target),
                            );
                        } else {
                            promoted_telemetry.signature_drop += 1;
                        }
                    }
                    crate::go_promoted_snapshot::GoPromotedSnapshotMethodConsult::ConflictDrop => {
                        promoted_telemetry.conflict_drop += 1;
                    }
                    crate::go_promoted_snapshot::GoPromotedSnapshotMethodConsult::VariantDrop => {
                        promoted_telemetry.variant_drop += 1;
                    }
                    crate::go_promoted_snapshot::GoPromotedSnapshotMethodConsult::InvariantDrop => {
                        promoted_telemetry.invariant_drop += 1;
                    }
                    crate::go_promoted_snapshot::GoPromotedSnapshotMethodConsult::Miss => {}
                }
            }
            let value_matches = required
                .keys()
                .all(|name| supplied.get(name.as_str()).is_some_and(|(value, _)| *value));
            let pointer_matches = required
                .keys()
                .all(|name| supplied.contains_key(name.as_str()));
            let Some((_, target)) = supplied.get(method_name) else {
                continue;
            };
            if value_matches {
                all_satisfiers.push((concrete_owner.name.clone(), *target));
            } else if pointer_matches {
                all_satisfiers.push((admission_key(&concrete_owner.name, true), *target));
            }
        }
        if all_satisfiers.is_empty() && (conflicting_owners > 0 || uncertain_owners > 0) {
            evidence.conflict |= conflicting_owners > 0;
            evidence.uncertain |= uncertain_owners > 0;
            return GoCallerScopedSatisfiers {
                interface_selected: true,
                selection: crate::go_owner_partition::GoPartitionSelection {
                    value: None,
                    evidence,
                },
                promoted: promoted_telemetry,
            };
        }
        evidence.recovered |= conflicting_owners > 0 || uncertain_owners > 0;
        GoCallerScopedSatisfiers {
            interface_selected: true,
            selection: crate::go_owner_partition::GoPartitionSelection {
                value: Some(all_satisfiers),
                evidence,
            },
            promoted: promoted_telemetry,
        }
    }
}
