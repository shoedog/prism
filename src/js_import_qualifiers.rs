//! S2: a separate, positive member proof; the S1b-4 projection stays unchanged.
use crate::js_exports::{JsExportFacts, ResolvedJsExport, MAX_REEXPORT_DEPTH};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Members = BTreeMap<String, ResolvedJsExport>;
pub(crate) type QualifierTable = BTreeMap<String, BTreeMap<String, QualifierIdentity>>;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualifierIdentity {
    pub file: String,
    pub local: String,
    pub members: Members,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum QualifierExport {
    Local(String),
    Forward { module: String, imported: String },
    Namespace(String),
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct QualifierFacts {
    pub complete: bool,
    pub named: BTreeMap<String, QualifierExport>,
    pub conflicted: BTreeSet<String>,
    pub locals: BTreeMap<String, Members>,
    pub written: BTreeSet<String>,
    pub import_sites: BTreeSet<(usize, usize)>,
}

impl QualifierFacts {
    pub(crate) fn insert(&mut self, name: String, target: QualifierExport) {
        if self.named.insert(name.clone(), target).is_some() {
            self.conflicted.insert(name);
        }
    }
}

pub(crate) fn resolve(
    raw: &BTreeMap<String, JsExportFacts>,
    module: &dyn Fn(&str, &str) -> Option<String>,
) -> QualifierTable {
    // Reuse the landed export-chain resolver and binding-core Callable captures
    // for members of `export * as ns`. Never accept its unspanned fallback.
    let callables = crate::js_exports::resolve_js_exports_for(raw, module, raw.keys()).resolved;
    let mut out = QualifierTable::new();
    for file in raw.keys() {
        let mut names = BTreeSet::new();
        collect(raw, module, file, 0, &mut BTreeSet::new(), &mut names);
        for name in names {
            if let Ok(Some(members)) = identity(
                raw,
                module,
                &callables,
                file,
                &name,
                0,
                &mut BTreeSet::new(),
            ) {
                out.entry(file.clone()).or_default().insert(name, members);
            }
        }
    }
    out
}

fn collect(
    raw: &BTreeMap<String, JsExportFacts>,
    module: &dyn Fn(&str, &str) -> Option<String>,
    file: &str,
    depth: usize,
    seen: &mut BTreeSet<String>,
    names: &mut BTreeSet<String>,
) {
    if depth > MAX_REEXPORT_DEPTH || !seen.insert(file.into()) {
        return;
    }
    if let Some(f) = raw.get(file) {
        names.extend(f.qualifiers.named.keys().cloned());
        for spec in &f.star_reexports {
            if let Some(next) = module(file, spec) {
                collect(raw, module, &next, depth + 1, seen, names);
            }
        }
    }
    seen.remove(file);
}

fn identity(
    raw: &BTreeMap<String, JsExportFacts>,
    module: &dyn Fn(&str, &str) -> Option<String>,
    callables: &BTreeMap<String, BTreeMap<String, ResolvedJsExport>>,
    file: &str,
    name: &str,
    depth: usize,
    seen: &mut BTreeSet<(String, String)>,
) -> Result<Option<QualifierIdentity>, ()> {
    if depth > MAX_REEXPORT_DEPTH {
        return Err(());
    }
    let key = (file.into(), name.into());
    if !seen.insert(key.clone()) {
        return Err(());
    }
    let result = (|| {
        let f = raw.get(file).filter(|f| f.qualifiers.complete).ok_or(())?;
        if f.qualifiers.conflicted.contains(name) {
            return Err(());
        }
        if let Some(export) = f.qualifiers.named.get(name) {
            return match export {
                QualifierExport::Local(local) => f
                    .qualifiers
                    .locals
                    .get(local)
                    .cloned()
                    .map(|members| {
                        Some(QualifierIdentity {
                            file: file.into(),
                            local: local.clone(),
                            members,
                        })
                    })
                    .ok_or(()),
                QualifierExport::Other => Err(()),
                QualifierExport::Forward {
                    module: spec,
                    imported,
                } => {
                    let next = module(file, spec).ok_or(())?;
                    identity(raw, module, callables, &next, imported, depth + 1, seen)
                }
                QualifierExport::Namespace(spec) => {
                    if depth == MAX_REEXPORT_DEPTH {
                        return Err(());
                    }
                    let next = module(file, spec).ok_or(())?;
                    let exports = callables.get(&next).ok_or(())?;
                    let mut members = Members::new();
                    for (member, t) in exports {
                        if t.span.is_none() || t.wrapped || t.via_unresolved_star {
                            continue;
                        }
                        let captured = raw
                            .get(&t.file)
                            .and_then(|f| f.namespace_callable_locals.get(&t.local_name));
                        if captured
                            .is_some_and(|v| v.len() == 1 && v[0].span == t.span && !v[0].wrapped)
                        {
                            members.insert(member.clone(), t.clone());
                        }
                    }
                    Ok(Some(QualifierIdentity {
                        file: next,
                        local: "*namespace*".into(),
                        members,
                    }))
                }
            };
        }
        let mut unique = None;
        if name != "default" {
            for spec in &f.star_reexports {
                let next = module(file, spec).ok_or(())?;
                if let Some(members) =
                    identity(raw, module, callables, &next, name, depth + 1, seen)?
                {
                    if unique.as_ref().is_some_and(|prior| prior != &members) {
                        return Err(());
                    }
                    unique = Some(members);
                }
            }
        }
        Ok(unique)
    })();
    seen.remove(&key);
    result
}
