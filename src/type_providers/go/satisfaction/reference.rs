//! Frozen pre-M serial satisfaction implementation.
use super::super::*;

impl GoTypeProvider {
    pub(super) fn compute_reference(data: &mut GoTypeData) {
        data.satisfaction.clear();
        data.sat_keys.clear();
        data.dispatch_gaps.clear();

        let interface_methods = Self::resolve_all_interface_methods(data);
        let concrete_methods = Self::resolve_all_concrete_methods(data);
        // P11 S4 (i): satisfaction-map MEMBERSHIP only — never feeds
        // `insert_sat_keys` (no FunctionId exists for these methods).
        data.embedded_interface_promotions =
            Self::embedded_interface_promotions(data, &concrete_methods);
        // P11 S4 (B2 fix): the package-scoped ROUTING computation, kept
        // separate from the bare-keyed membership map above.
        data.embedded_interface_routes = Self::embedded_interface_routes(data, &concrete_methods);

        for (iface_name, iface_methods) in &interface_methods {
            let Some(iface) = data.interfaces.get(iface_name) else {
                continue;
            };
            if iface.generic {
                data.dispatch_gaps.push(GoDispatchGap::Generic);
                data.satisfaction
                    .insert(iface_name.clone(), BTreeSet::new());
                continue;
            }

            let mut canonical_iface_methods = BTreeMap::new();
            let mut has_gap = false;
            for (method_name, sig) in iface_methods {
                match sig {
                    Ok(sig) => {
                        canonical_iface_methods.insert(method_name.clone(), sig.clone());
                    }
                    Err(gap) => {
                        data.dispatch_gaps.push(*gap);
                        has_gap = true;
                    }
                }
            }
            if has_gap {
                data.satisfaction
                    .insert(iface_name.clone(), BTreeSet::new());
                continue;
            }

            let mut satisfying = BTreeSet::new();
            for (type_name, method_set) in &concrete_methods {
                crate::go_type_alias::import_memo::parallel::work_sites::record_pair(
                    crate::build_pool::build_pool()
                        .current_thread_index()
                        .is_some(),
                );
                let embedded = data.embedded_interface_promotions.get(type_name);
                let value_matches = Self::method_set_satisfies_with_embedded(
                    &method_set.value,
                    embedded,
                    &canonical_iface_methods,
                );
                let pointer_matches = Self::method_set_satisfies_with_embedded(
                    &method_set.pointer,
                    embedded,
                    &canonical_iface_methods,
                );

                if value_matches || pointer_matches {
                    satisfying.insert(type_name.clone());
                }

                if value_matches {
                    Self::insert_sat_keys(
                        data,
                        iface_name,
                        &canonical_iface_methods,
                        &method_set.value,
                        type_name,
                        false,
                    );
                    Self::insert_sat_keys(
                        data,
                        iface_name,
                        &canonical_iface_methods,
                        &method_set.pointer,
                        type_name,
                        true,
                    );
                } else if pointer_matches {
                    Self::insert_sat_keys(
                        data,
                        iface_name,
                        &canonical_iface_methods,
                        &method_set.pointer,
                        type_name,
                        true,
                    );
                }
            }
            data.satisfaction.insert(iface_name.clone(), satisfying);
        }
    }

    fn insert_sat_keys(
        data: &mut GoTypeData,
        iface_name: &str,
        iface_methods: &BTreeMap<String, String>,
        concrete_methods: &BTreeMap<String, CanonMethod>,
        type_name: &str,
        is_pointer: bool,
    ) {
        let key = crate::resolution::admission_key(type_name, is_pointer);
        for method_name in iface_methods.keys() {
            if let Some(method) = concrete_methods.get(method_name) {
                data.sat_keys
                    .entry((iface_name.to_string(), method_name.clone()))
                    .or_default()
                    .push((key.clone(), method.func_id.clone()));
            }
        }
    }
}
