// Frozen pre-optimization computation, compiled only by the differential tests.
use super::*;

pub(super) fn unique_python_module_file<'a>(
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
    let Some(defining_file) = unique_python_module_file(&module_path, caller_file, indexed_files)
    else {
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
    let mut proven = BTreeSet::new();
    for (caller_file, bindings) in import_bindings {
        for binding in bindings {
            match binding.kind {
                ImportBindingKind::MemberImport => {
                    let receiver_type = binding.local.as_str();
                    if let PythonImportedClassRoute::Proven {
                        defining_file,
                        owner,
                    } = python_imported_class_route(
                        caller_file,
                        receiver_type,
                        Some(bindings),
                        indexed_files,
                        clean_class_spans,
                        inert_initializers,
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
                    let Some(defining_file) =
                        unique_python_module_file(&module_path, caller_file, indexed_files)
                    else {
                        continue;
                    };
                    for (file, owner) in clean_class_spans.keys() {
                        if file == defining_file {
                            proven.insert((
                                caller_file.clone(),
                                format!("{}.{}", binding.local, owner),
                                defining_file.clone(),
                                owner.clone(),
                            ));
                        }
                    }
                }
                ImportBindingKind::ModuleImport | ImportBindingKind::AliasedModuleImport
                    if binding.eligible =>
                {
                    let Some(defining_file) =
                        unique_python_module_file(&binding.module_path, caller_file, indexed_files)
                    else {
                        continue;
                    };
                    let receiver_prefix =
                        if matches!(binding.kind, ImportBindingKind::AliasedModuleImport) {
                            &binding.local
                        } else {
                            &binding.module_path
                        };
                    for (file, owner) in clean_class_spans.keys() {
                        if file == defining_file {
                            proven.insert((
                                caller_file.clone(),
                                format!("{receiver_prefix}.{owner}"),
                                defining_file.clone(),
                                owner.clone(),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    proven
}
