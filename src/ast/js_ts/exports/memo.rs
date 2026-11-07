//! Invocation-local module lookup memoization. Traversals and their telemetry
//! retain the original root/name/branch order and multiplicity.
use super::*;
use std::cell::RefCell;

#[derive(Default)]
struct Modules {
    // Nested maps permit borrowed string lookups; hits allocate only the result.
    entries: RefCell<BTreeMap<String, BTreeMap<String, Option<String>>>>,
}
impl Modules {
    fn get(
        &self,
        module: &dyn Fn(&str, &str) -> Option<String>,
        from: &str,
        spec: &str,
    ) -> Option<String> {
        if let Some(hit) = self
            .entries
            .borrow()
            .get(from)
            .and_then(|specs| specs.get(spec))
        {
            return hit.clone();
        }
        // Neither strings nor misses are normalized. Project/snapshot identity
        // is captured by the closure and isolated by this cache's lifetime.
        let result = module(from, spec);
        self.entries
            .borrow_mut()
            .entry(from.into())
            .or_default()
            .insert(spec.into(), result.clone());
        result
    }
}

pub(super) fn resolve<'a>(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    roots: impl IntoIterator<Item = &'a String>,
    projection: ExportProjection,
) -> JsExportResolution {
    let modules = Modules::default();
    let cached = |from: &str, spec: &str| modules.get(resolve_module, from, spec);
    let lookup: &dyn Fn(&str, &str) -> Option<String> = &cached;
    #[cfg(test)]
    let lookup = oracle::select(resolve_module, lookup);
    resolve_with(raw, lookup, roots, projection)
}

fn resolve_with<'a>(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    roots: impl IntoIterator<Item = &'a String>,
    projection: ExportProjection,
) -> JsExportResolution {
    let resolver = ExportResolver {
        module: resolve_module,
        projection,
    };
    let mut out = JsExportResolution::default();
    for file in roots {
        let mut visited_files = BTreeSet::new();
        let names =
            collect_candidate_names(raw, resolve_module, file, 0, &mut visited_files, &mut out);
        let mut per_file = BTreeMap::new();
        for name in names {
            let mut visited = BTreeSet::new();
            if let ExportLookup::Resolved(mut hit, false) =
                resolve_one(raw, &resolver, file, &name, 0, &mut visited, &mut out)
            {
                hit.via_unresolved_star =
                    skipped_star(raw, resolve_module, file, &name, &mut BTreeSet::new());
                per_file.insert(name, hit);
            }
        }
        if !per_file.is_empty() {
            out.resolved.insert(file.clone(), per_file);
        }
    }
    out
}

pub(super) fn namespace(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
) -> BTreeMap<String, BTreeMap<String, ResolvedJsExport>> {
    let modules = Modules::default();
    let cached = |from: &str, spec: &str| modules.get(resolve_module, from, spec);
    let lookup: &dyn Fn(&str, &str) -> Option<String> = &cached;
    #[cfg(test)]
    let lookup = oracle::select(resolve_module, lookup);
    namespace_with(raw, lookup)
}

fn namespace_with(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
) -> BTreeMap<String, BTreeMap<String, ResolvedJsExport>> {
    let mut resolved = BTreeMap::new();
    for file in raw.keys() {
        let mut telemetry = JsExportResolution::default();
        let names = collect_candidate_names(
            raw,
            resolve_module,
            file,
            0,
            &mut BTreeSet::new(),
            &mut telemetry,
        );
        for name in names {
            if let Ok(Some(target)) =
                namespace_identity(raw, resolve_module, file, &name, 0, &mut BTreeSet::new())
            {
                resolved
                    .entry(file.clone())
                    .or_insert_with(BTreeMap::new)
                    .insert(name, target);
            }
        }
    }
    resolved
}

#[cfg(test)]
mod oracle;
#[cfg(test)]
mod pipeline_tests;
#[cfg(test)]
mod tests;
