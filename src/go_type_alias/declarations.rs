//! Independent alias declaration records; only ordered assembly touches the resolver.
use super::*;
use rayon::prelude::*;

type Record = (GoOwnerIdentity, AliasDeclaration);
#[cfg(test)]
type JobResult = (Vec<Record>, bool);
#[cfg(not(test))]
type JobResult = Vec<Record>;

fn collect_job(
    path: &str,
    parsed: &ParsedFile,
    profile: Option<&GoBuildProfile>,
    local: Option<&str>,
    imports: Option<&BTreeMap<String, String>>,
) -> JobResult {
    let records = collect_file(path, parsed, profile, local, imports);
    #[cfg(test)]
    return (
        records,
        crate::build_pool::build_pool()
            .current_thread_index()
            .is_some(),
    );
    #[cfg(not(test))]
    records
}

pub(super) fn prepare(
    files: &BTreeMap<String, ParsedFile>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    local_paths: &BTreeMap<String, String>,
    imports: Option<&SignatureImportSource<'_>>,
) -> Vec<Vec<Record>> {
    // Resolve lazy memo entries on this thread. Neither OnceCell nor resolver
    // RefCell telemetry is captured by any worker; maps themselves are Sync.
    let jobs: Vec<_> = files
        .iter()
        .filter(|(_, parsed)| parsed.language == Language::Go)
        .map(|(path, parsed)| {
            (
                path.as_str(),
                parsed,
                profiles.get(path),
                local_paths.get(path).map(String::as_str),
                imports.and_then(|source| source.get(path)),
            )
        })
        .collect();
    if jobs.is_empty() {
        return Vec::new();
    }
    #[cfg(test)]
    let fallback_scans = jobs
        .iter()
        .filter(|(_, _, profile, _, imports)| {
            profile.is_some_and(|profile| !profile.package_clause.is_empty()) && imports.is_none()
        })
        .count();
    let rows: Vec<JobResult> = crate::build_pool::install(|| {
        crate::build_pool::per_item(jobs)
            .map(|(path, parsed, profile, local, imports)| {
                collect_job(path, parsed, profile, local, imports)
            })
            .collect()
    });
    // Attribute completed worker scans to the measuring caller, as import
    // preparation does. Resolver expansion telemetry remains untouched.
    #[cfg(test)]
    for _ in 0..fallback_scans {
        import_memo::record_scan();
    }
    #[cfg(test)]
    let rows = rows
        .into_iter()
        .map(|(records, on_pool)| {
            import_memo::parallel::work_sites::record_file(on_pool);
            records
        })
        .collect();
    rows
}

fn collect_file(
    path: &str,
    parsed: &ParsedFile,
    profile: Option<&GoBuildProfile>,
    local_path: Option<&str>,
    signature_imports_override: Option<&BTreeMap<String, String>>,
) -> Vec<Record> {
    let Some(profile) = profile.cloned() else {
        return Vec::new();
    };
    if profile.package_clause.is_empty() {
        return Vec::new();
    }
    let mut records = Vec::new();
    let imports = signature_imports_override
        .cloned()
        .unwrap_or_else(|| canon::signature_imports_unmeasured(parsed, local_path));
    let root = parsed.tree.root_node();
    let mut root_cursor = root.walk();
    for declaration in root.children(&mut root_cursor) {
        if declaration.kind() != "type_declaration" {
            continue;
        }
        let mut cursor = declaration.walk();
        for child in declaration.children(&mut cursor) {
            if !matches!(child.kind(), "type_alias" | "type_spec") {
                continue;
            }
            let Some(name_node) = child.child_by_field_name("name") else {
                continue;
            };
            let name = parsed.node_text(&name_node).trim().to_string();
            if name.is_empty() {
                continue;
            }
            let owner = GoOwnerIdentity {
                package_dir: dir_of(path).to_string(),
                package_clause: profile.package_clause.clone(),
                name: name.clone(),
            };
            let syntax = classify_alias_syntax(&child);
            let kind = if syntax != AliasSyntax::Defined {
                let (param_names, constraints_supported) = alias_parameters(parsed, &child);
                let type_params: BTreeSet<String> = param_names.iter().cloned().collect();
                let rhs = if syntax == AliasSyntax::Unresolved {
                    Err(AliasUnresolvedReason::Unresolvable)
                } else {
                    child
                        .child_by_field_name("type")
                        .ok_or(AliasUnresolvedReason::Unresolvable)
                        .and_then(|node| {
                            parse_expr(&node, parsed, &imports, path, Some(&profile), &type_params)
                                .map_err(|_| AliasUnresolvedReason::Unresolvable)
                        })
                };
                AliasDeclarationKind::Alias {
                    params: param_names,
                    rhs: if constraints_supported {
                        rhs
                    } else {
                        Err(AliasUnresolvedReason::Unresolvable)
                    },
                }
            } else {
                AliasDeclarationKind::Defined
            };
            records.push((
                owner,
                AliasDeclaration {
                    defining_file: path.to_string(),
                    kind,
                },
            ));
        }
    }
    records
}

#[cfg(test)]
mod reference;
#[cfg(test)]
mod tests;
