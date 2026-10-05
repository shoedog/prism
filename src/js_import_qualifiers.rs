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
    pub syntax_incomplete: bool,
    pub refusal_sources: BTreeSet<String>,
    pub refusal_source_unavailable: bool,
    pub named: BTreeMap<String, QualifierExport>,
    pub conflicted: BTreeSet<String>,
    pub locals: BTreeMap<String, Members>,
    pub written: BTreeSet<String>,
    pub called_members: BTreeMap<String, BTreeSet<String>>,
    pub import_sites: BTreeSet<(usize, usize)>,
}

impl QualifierFacts {
    pub(crate) fn insert(&mut self, name: String, target: QualifierExport) {
        if self.named.insert(name.clone(), target).is_some() {
            self.conflicted.insert(name);
        }
    }
}

pub(crate) type RefusedIdentities = BTreeSet<(String, String)>;

/// Absence is a proof outcome, never a failed positive-table lookup.
pub(crate) enum RefusalJoin {
    Joined(Vec<QualifierIdentity>),
    ProvedAbsent,
    OutOfModelUnresolvable,
    Unavailable(RefusedIdentities),
}

#[derive(Clone)]
pub(crate) enum RefusalModule {
    Resolved(String),
    ProvenUnresolved,
    ExternalBuiltin,
    Unavailable(BTreeSet<String>),
    Opaque,
}

/// Resolution failure carries its reachable module set. Only fully opaque
/// source spelling permits the global identity set.
pub(crate) fn refusal_join(
    raw: &BTreeMap<String, JsExportFacts>,
    table: &QualifierTable,
    module: &mut dyn FnMut(&str, &str) -> RefusalModule,
    target: RefusalModule,
    member: Option<&str>,
) -> RefusalJoin {
    let target = match target {
        RefusalModule::Resolved(target) => target,
        RefusalModule::ProvenUnresolved => return RefusalJoin::OutOfModelUnresolvable,
        RefusalModule::ExternalBuiltin => return RefusalJoin::ProvedAbsent,
        other => return RefusalJoin::Unavailable(module_identities(raw, table, module, other)),
    };
    // A namespace can expose exports omitted by the positive depth/name proof
    // even when some other exports joined. Retain its entire static closure.
    if member.is_none() {
        return RefusalJoin::Unavailable(possible_identities(raw, table, module, &target));
    }
    refusal_name(
        raw,
        table,
        module,
        &target,
        member.unwrap(),
        0,
        &mut BTreeSet::new(),
    )
}

fn refusal_name(
    raw: &BTreeMap<String, JsExportFacts>,
    table: &QualifierTable,
    module: &mut dyn FnMut(&str, &str) -> RefusalModule,
    target: &str,
    name: &str,
    depth: usize,
    seen: &mut BTreeSet<(String, String)>,
) -> RefusalJoin {
    let key = (target.to_owned(), name.to_owned());
    if depth > MAX_REEXPORT_DEPTH || name.contains('\\') || !seen.insert(key.clone()) {
        return RefusalJoin::Unavailable(possible_identities(raw, table, module, target));
    }
    let result = (|| {
        let Some(f) = raw
            .get(target)
            .filter(|f| f.qualifiers.complete && !f.qualifiers.conflicted.contains(name))
        else {
            return RefusalJoin::Unavailable(possible_identities(raw, table, module, target));
        };
        if let Some(identity) = table.get(target).and_then(|exports| exports.get(name)) {
            return RefusalJoin::Joined(vec![identity.clone()]);
        }
        match f.qualifiers.named.get(name) {
            Some(QualifierExport::Other | QualifierExport::Local(_)) => RefusalJoin::ProvedAbsent,
            Some(QualifierExport::Forward {
                module: spec,
                imported,
            }) => match module(target, spec) {
                RefusalModule::Resolved(next) => {
                    refusal_name(raw, table, module, &next, imported, depth + 1, seen)
                }
                RefusalModule::ProvenUnresolved => {
                    RefusalJoin::Unavailable(possible_identities(raw, table, module, target))
                }
                RefusalModule::ExternalBuiltin => RefusalJoin::ProvedAbsent,
                other => RefusalJoin::Unavailable(module_identities(raw, table, module, other)),
            },
            Some(QualifierExport::Namespace(_)) => {
                RefusalJoin::Unavailable(possible_identities(raw, table, module, target))
            }
            None => {
                let mut joined = Vec::new();
                let mut possible = BTreeSet::new();
                let mut unavailable = false;
                if name != "default" {
                    for spec in &f.star_reexports {
                        let resolution = module(target, spec);
                        let outcome = match resolution {
                            RefusalModule::Resolved(next) => {
                                refusal_name(raw, table, module, &next, name, depth + 1, seen)
                            }
                            RefusalModule::ProvenUnresolved => RefusalJoin::Unavailable(
                                possible_identities(raw, table, module, target),
                            ),
                            RefusalModule::ExternalBuiltin => RefusalJoin::ProvedAbsent,
                            other => RefusalJoin::Unavailable(module_identities(
                                raw, table, module, other,
                            )),
                        };
                        match outcome {
                            RefusalJoin::Joined(identities) => joined.extend(identities),
                            RefusalJoin::ProvedAbsent => {}
                            RefusalJoin::OutOfModelUnresolvable => unreachable!("resolved chain"),
                            RefusalJoin::Unavailable(identities) => {
                                unavailable = true;
                                possible.extend(identities);
                            }
                        }
                    }
                }
                if unavailable {
                    possible.extend(joined.into_iter().map(|i| (i.file, i.local)));
                    RefusalJoin::Unavailable(possible)
                } else if joined.is_empty() {
                    RefusalJoin::ProvedAbsent
                } else {
                    RefusalJoin::Joined(joined)
                }
            }
        }
    })();
    seen.remove(&key);
    result
}

fn module_identities(
    raw: &BTreeMap<String, JsExportFacts>,
    table: &QualifierTable,
    module: &mut dyn FnMut(&str, &str) -> RefusalModule,
    resolution: RefusalModule,
) -> RefusedIdentities {
    match resolution {
        RefusalModule::ProvenUnresolved | RefusalModule::ExternalBuiltin => BTreeSet::new(),
        RefusalModule::Resolved(file) => possible_identities(raw, table, module, &file),
        // Every table file is visited by the original union, even when its
        // raw facts are absent. Closure cannot add an identity outside table.
        // Thus this coverage test gives exactly that union without repeated
        // whole-repository walks for unknown native package scope.
        RefusalModule::Unavailable(files) if table.keys().all(|file| files.contains(file)) => table
            .values()
            .flat_map(|exports| exports.values())
            .map(|i| (i.file.clone(), i.local.clone()))
            .collect(),
        RefusalModule::Unavailable(files) => files
            .into_iter()
            .flat_map(|file| possible_identities(raw, table, module, &file))
            .collect(),
        RefusalModule::Opaque => table
            .values()
            .flat_map(|exports| exports.values())
            .map(|i| (i.file.clone(), i.local.clone()))
            .collect(),
    }
}

pub(crate) fn possible_identities(
    raw: &BTreeMap<String, JsExportFacts>,
    table: &QualifierTable,
    module: &mut dyn FnMut(&str, &str) -> RefusalModule,
    target: &str,
) -> RefusedIdentities {
    let mut seen = BTreeSet::new();
    let mut stack = vec![target.to_string()];
    let mut opaque = false;
    while let Some(file) = stack.pop() {
        if !seen.insert(file.clone()) {
            continue;
        }
        let Some(f) = raw.get(&file) else {
            continue;
        };
        opaque |= f.qualifiers.refusal_source_unavailable;
        for spec in &f.qualifiers.refusal_sources {
            match module(&file, spec) {
                RefusalModule::ProvenUnresolved | RefusalModule::ExternalBuiltin => {}
                RefusalModule::Resolved(next) => stack.push(next),
                RefusalModule::Unavailable(files) => stack.extend(files),
                RefusalModule::Opaque => opaque = true,
            }
        }
    }
    table
        .iter()
        .flat_map(|(file, exports)| {
            exports
                .values()
                .filter(|i| opaque || seen.contains(file) || seen.contains(&i.file))
        })
        .map(|i| (i.file.clone(), i.local.clone()))
        .collect()
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
