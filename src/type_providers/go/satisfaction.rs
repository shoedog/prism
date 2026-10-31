//! Ordered parallel interface/type matching without a second GoTypeData.
use super::*;
use rayon::prelude::*;

type SatKeys = BTreeMap<String, Vec<(String, FunctionId)>>;

struct InterfaceResult {
    name: String,
    satisfying: BTreeSet<String>,
    keys: SatKeys,
    gaps: Vec<GoDispatchGap>,
    #[cfg(test)]
    pair_sites: Vec<bool>,
}

struct TypeMatch {
    value: bool,
    pointer: bool,
    #[cfg(test)]
    on_pool: bool,
}

pub(super) fn compute(data: &mut GoTypeData) {
    #[cfg(test)]
    if crate::go_type_alias::import_memo::parallel::serial_reference_enabled() {
        GoTypeProvider::compute_reference(data);
        return;
    }
    data.satisfaction.clear();
    data.sat_keys.clear();
    data.dispatch_gaps.clear();
    let interfaces = GoTypeProvider::resolve_all_interface_methods(data);
    let concrete = GoTypeProvider::resolve_all_concrete_methods(data);
    data.embedded_interface_promotions =
        GoTypeProvider::embedded_interface_promotions(data, &concrete);
    data.embedded_interface_routes = GoTypeProvider::embedded_interface_routes(data, &concrete);
    let types: Vec<_> = concrete.iter().collect();
    let jobs: Vec<_> = interfaces.iter().collect();
    if jobs.is_empty() {
        return;
    }
    let rows: Vec<_> = crate::build_pool::install(|| {
        crate::build_pool::per_item(jobs)
            .filter_map(|(name, methods)| {
                data.interfaces
                    .get(name)
                    .map(|iface| match_interface(data, name, methods, iface.generic, &types))
            })
            .collect()
    });
    // Indexed jobs preserve interface order even after absent-interface filtering.
    // Each row's vectors already have type order and value-before-pointer ties.
    for row in rows {
        #[cfg(test)]
        for site in row.pair_sites {
            crate::go_type_alias::import_memo::parallel::work_sites::record_pair(site);
        }
        data.dispatch_gaps.extend(row.gaps);
        data.satisfaction.insert(row.name.clone(), row.satisfying);
        for (method, keys) in row.keys {
            data.sat_keys.insert((row.name.clone(), method), keys);
        }
    }
}

fn match_interface(
    data: &GoTypeData,
    name: &str,
    methods: &BTreeMap<String, Result<String, GoDispatchGap>>,
    generic: bool,
    types: &[(&String, &ReceiverMethodSet)],
) -> InterfaceResult {
    let mut result = InterfaceResult {
        name: name.to_string(),
        satisfying: BTreeSet::new(),
        keys: BTreeMap::new(),
        gaps: Vec::new(),
        #[cfg(test)]
        pair_sites: Vec::new(),
    };
    if generic {
        result.gaps.push(GoDispatchGap::Generic);
        return result;
    }
    let mut canonical = BTreeMap::new();
    for (method, signature) in methods {
        match signature {
            Ok(sig) => {
                canonical.insert(method.clone(), sig.clone());
            }
            Err(gap) => result.gaps.push(*gap),
        }
    }
    if !result.gaps.is_empty() {
        return result;
    }
    // Keep the cross product implicit: O(types) small match flags per active
    // interface, rather than materializing O(interfaces * types) job records.
    let matches: Vec<_> = crate::build_pool::per_item(types)
        .map(|(type_name, set)| {
            let embedded = data.embedded_interface_promotions.get(*type_name);
            let (value, pointer) = (
                GoTypeProvider::method_set_satisfies_with_embedded(
                    &set.value, embedded, &canonical,
                ),
                GoTypeProvider::method_set_satisfies_with_embedded(
                    &set.pointer,
                    embedded,
                    &canonical,
                ),
            );
            TypeMatch {
                value,
                pointer,
                #[cfg(test)]
                on_pool: crate::build_pool::build_pool()
                    .current_thread_index()
                    .is_some(),
            }
        })
        .collect();
    for ((type_name, set), matched) in types.iter().zip(matches) {
        #[cfg(test)]
        result.pair_sites.push(matched.on_pool);
        let (value, pointer) = (matched.value, matched.pointer);
        if value || pointer {
            result.satisfying.insert((*type_name).clone());
        }
        if value {
            append_keys(&mut result.keys, &canonical, &set.value, type_name, false);
        }
        if value || pointer {
            append_keys(&mut result.keys, &canonical, &set.pointer, type_name, true);
        }
    }
    result
}

fn append_keys(
    keys: &mut SatKeys,
    methods: &BTreeMap<String, String>,
    concrete: &BTreeMap<String, CanonMethod>,
    name: &str,
    pointer: bool,
) {
    let admission = crate::resolution::admission_key(name, pointer);
    for method in methods.keys() {
        if let Some(found) = concrete.get(method) {
            keys.entry(method.clone())
                .or_default()
                .push((admission.clone(), found.func_id.clone()));
        }
    }
}

#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
