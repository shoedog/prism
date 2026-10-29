//! Per-computation module lookup memo for Python imported-class proofs.

use super::{
    file_matches_module, python_namespace_submodule_path, python_qualified_receiver_parts,
    ImportBinding, ImportBindingKind, PythonImportedClassRoute,
};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
#[path = "python_import_proofs_reference.rs"]
mod reference;
#[cfg(test)]
#[path = "python_import_proofs_tests.rs"]
mod tests;

/// The indexed-file universe is immutable and belongs to this memo, so it need
/// not be repeated in each key. Only relative matching reads the caller's
/// directory; absolute matching never reads the caller at all.
struct ModuleFileMemo<'a> {
    indexed_files: &'a BTreeSet<String>,
    results: BTreeMap<(String, Option<String>), Option<&'a String>>,
    #[cfg(test)]
    scans: usize,
}

impl<'a> ModuleFileMemo<'a> {
    fn new(indexed_files: &'a BTreeSet<String>) -> Self {
        Self {
            indexed_files,
            results: BTreeMap::new(),
            #[cfg(test)]
            scans: 0,
        }
    }

    fn resolve(&mut self, module_path: &str, caller_file: &str) -> Option<&'a String> {
        // Match file_matches_module's classification after whitespace trimming,
        // including slash-style relative paths in malformed/synthetic bindings.
        let trimmed = module_path.trim();
        let relative = if trimmed.contains('/') {
            trimmed.starts_with("./") || trimmed.starts_with("../")
        } else {
            trimmed.starts_with('.')
        };
        let directory = relative.then(|| {
            caller_file
                .rsplit_once('/')
                .map(|(directory, _)| directory)
                .unwrap_or("")
                .to_string()
        });
        *self
            .results
            .entry((module_path.to_string(), directory))
            .or_insert_with(|| {
                #[cfg(test)]
                {
                    self.scans += 1;
                }
                unique_python_module_file(module_path, caller_file, self.indexed_files)
            })
    }
}

/// Tuple ordering makes all owners of one file contiguous. The empty owner is
/// the lower bound even for synthetic empty names; take_while excludes the next
/// file without inventing an upper-bound sentinel for arbitrary strings.
fn class_owners<'a>(
    defining_file: &'a str,
    clean_class_spans: &'a BTreeMap<(String, String), (usize, usize)>,
) -> impl Iterator<Item = &'a String> {
    clean_class_spans
        .range((defining_file.to_string(), String::new())..)
        .inspect(|_| {
            #[cfg(test)]
            tests::OWNER_KEY_VISITS.with(|count| count.set(count.get() + 1));
        })
        .take_while(move |((file, _), _)| file == defining_file)
        .map(|((_, owner), _)| owner)
}

fn unique_python_module_file<'a>(
    module_path: &str,
    caller_file: &str,
    indexed_files: &'a BTreeSet<String>,
) -> Option<&'a String> {
    let mut module_files = indexed_files
        .iter()
        .filter(|file| {
            crate::languages::Language::from_path(file) == Some(crate::languages::Language::Python)
        })
        .filter(|file| file_matches_module(file, module_path, caller_file, indexed_files));
    let defining_file = module_files.next()?;
    module_files.next().is_none().then_some(defining_file)
}

pub(crate) fn python_imported_class_route(
    caller_file: &str,
    receiver_type: &str,
    bindings: Option<&[ImportBinding]>,
    indexed_files: &BTreeSet<String>,
    clean_class_spans: &BTreeMap<(String, String), (usize, usize)>,
    inert_initializers: &BTreeSet<String>,
) -> PythonImportedClassRoute {
    imported_class_route_with_lookup(
        caller_file,
        receiver_type,
        bindings,
        indexed_files,
        clean_class_spans,
        inert_initializers,
        &mut |module, caller| unique_python_module_file(module, caller, indexed_files),
    )
}

fn imported_class_route_with_lookup<'a>(
    caller_file: &str,
    receiver_type: &str,
    bindings: Option<&[ImportBinding]>,
    indexed_files: &BTreeSet<String>,
    clean_class_spans: &BTreeMap<(String, String), (usize, usize)>,
    inert_initializers: &BTreeSet<String>,
    resolve: &mut impl FnMut(&str, &str) -> Option<&'a String>,
) -> PythonImportedClassRoute {
    let Some(bindings) = bindings else {
        return PythonImportedClassRoute::NotImported;
    };
    let qualified = python_qualified_receiver_parts(receiver_type);
    let local = qualified
        .and_then(|(qualifier, _)| qualifier.split('.').next())
        .unwrap_or(receiver_type);
    let matching: Vec<_> = bindings
        .iter()
        .filter(|binding| binding.local == local)
        .collect();
    if matching.is_empty() {
        return PythonImportedClassRoute::NotImported;
    }
    if matching.len() != 1 {
        return PythonImportedClassRoute::Blocked;
    }
    let binding = matching[0];
    if !binding.eligible {
        return PythonImportedClassRoute::Blocked;
    }
    let (owner, module_path) = match qualified {
        Some((qualifier, owner))
            if matches!(binding.kind, ImportBindingKind::ModuleImport)
                && qualifier == binding.module_path =>
        {
            (owner, binding.module_path.clone())
        }
        Some((qualifier, owner))
            if matches!(binding.kind, ImportBindingKind::AliasedModuleImport)
                && qualifier == binding.local =>
        {
            (owner, binding.module_path.clone())
        }
        Some((qualifier, owner))
            if matches!(binding.kind, ImportBindingKind::MemberImport)
                && qualifier == binding.local =>
        {
            let Some(module_path) = python_namespace_submodule_path(
                binding,
                caller_file,
                indexed_files,
                inert_initializers,
            ) else {
                return PythonImportedClassRoute::Blocked;
            };
            (owner, module_path)
        }
        None if matches!(binding.kind, ImportBindingKind::MemberImport) => {
            let Some(owner) = binding.member.as_deref() else {
                return PythonImportedClassRoute::Blocked;
            };
            (owner, binding.module_path.clone())
        }
        _ => return PythonImportedClassRoute::Blocked,
    };

    // Count module candidates before consulting the requested class. This
    // prevents a single-component import from becoming Exact merely because
    // only one of several same-stem modules happens to declare `owner`.
    let Some(defining_file) = resolve(&module_path, caller_file) else {
        return PythonImportedClassRoute::Blocked;
    };
    if !clean_class_spans.contains_key(&(defining_file.to_string(), owner.to_string())) {
        return PythonImportedClassRoute::Blocked;
    }
    PythonImportedClassRoute::Proven {
        defining_file: defining_file.to_string(),
        owner: owner.to_string(),
    }
}

pub(crate) fn python_imported_class_proof_keys(
    import_bindings: &BTreeMap<String, Vec<ImportBinding>>,
    indexed_files: &BTreeSet<String>,
    clean_class_spans: &BTreeMap<(String, String), (usize, usize)>,
    inert_initializers: &BTreeSet<String>,
) -> BTreeSet<(String, String, String, String)> {
    proof_keys_with_memo(
        import_bindings,
        clean_class_spans,
        inert_initializers,
        &mut ModuleFileMemo::new(indexed_files),
    )
}

fn proof_keys_with_memo(
    import_bindings: &BTreeMap<String, Vec<ImportBinding>>,
    clean_class_spans: &BTreeMap<(String, String), (usize, usize)>,
    inert_initializers: &BTreeSet<String>,
    memo: &mut ModuleFileMemo<'_>,
) -> BTreeSet<(String, String, String, String)> {
    let indexed_files = memo.indexed_files;
    let mut proven = BTreeSet::new();
    for (caller_file, bindings) in import_bindings {
        for binding in bindings {
            match binding.kind {
                ImportBindingKind::MemberImport => {
                    let receiver_type = binding.local.as_str();
                    if let PythonImportedClassRoute::Proven {
                        defining_file,
                        owner,
                    } = imported_class_route_with_lookup(
                        caller_file,
                        receiver_type,
                        Some(bindings),
                        indexed_files,
                        clean_class_spans,
                        inert_initializers,
                        &mut |module, caller| memo.resolve(module, caller),
                    ) {
                        proven.insert((
                            caller_file.clone(),
                            receiver_type.to_string(),
                            defining_file,
                            owner,
                        ));
                    }

                    if !binding.eligible {
                        continue;
                    }
                    let Some(module_path) = python_namespace_submodule_path(
                        binding,
                        caller_file,
                        indexed_files,
                        inert_initializers,
                    ) else {
                        continue;
                    };
                    let Some(defining_file) = memo.resolve(&module_path, caller_file) else {
                        continue;
                    };
                    for owner in class_owners(defining_file, clean_class_spans) {
                        proven.insert((
                            caller_file.clone(),
                            format!("{}.{}", binding.local, owner),
                            defining_file.clone(),
                            owner.clone(),
                        ));
                    }
                }
                ImportBindingKind::ModuleImport | ImportBindingKind::AliasedModuleImport
                    if binding.eligible =>
                {
                    let Some(defining_file) = memo.resolve(&binding.module_path, caller_file)
                    else {
                        continue;
                    };
                    let receiver_prefix =
                        if matches!(binding.kind, ImportBindingKind::AliasedModuleImport) {
                            &binding.local
                        } else {
                            &binding.module_path
                        };
                    for owner in class_owners(defining_file, clean_class_spans) {
                        proven.insert((
                            caller_file.clone(),
                            format!("{receiver_prefix}.{owner}"),
                            defining_file.clone(),
                            owner.clone(),
                        ));
                    }
                }
                _ => {}
            }
        }
    }
    proven
}
