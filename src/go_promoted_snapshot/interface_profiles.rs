use super::{ComparisonKey, GoPromotedEmbeddedField, GoPromotedProfileSnapshot, RawProfile};
use crate::ast::ParsedFile;
use crate::go_owner_partition::{GoInterfaceDeclarations, GoStructDeclarations};
use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn extend(
    raw: &mut BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    files: &BTreeMap<String, ParsedFile>,
    local_import_paths: &BTreeMap<String, String>,
    interfaces: &GoInterfaceDeclarations,
    structs: &GoStructDeclarations,
    type_declarations: &BTreeMap<GoOwnerIdentity, BTreeSet<String>>,
    aliases: &crate::go_type_alias::GoAliasResolver,
) {
    for (owner, declarations) in interfaces {
        for declaration in declarations {
            let imports = files
                .get(&declaration.defining_file)
                .map(|parsed| {
                    crate::go_type_alias::signature_imports(
                        parsed,
                        local_import_paths
                            .get(&declaration.defining_file)
                            .map(String::as_str),
                    )
                })
                .unwrap_or_default();
            let mut embedded_fields = BTreeSet::new();
            let mut unresolved_embedded_fields = BTreeSet::new();
            for raw_type in &declaration.embedded_types {
                let resolved =
                    aliases.resolve_embedded_owner(raw_type, &declaration.defining_file, &imports);
                match resolved {
                    Ok((false, target)) if interfaces.contains_key(&target) => {
                        embedded_fields.insert(GoPromotedEmbeddedField {
                            pointer: false,
                            target,
                            selector: selector(raw_type),
                        });
                    }
                    _ => {
                        unresolved_embedded_fields.insert(raw_type.clone());
                    }
                }
            }
            let own_methods = declaration.methods.clone();
            let comparison = ComparisonKey {
                embedded_fields: embedded_fields.clone(),
                ordinary_fields: BTreeSet::new(),
                own_methods: own_methods.clone(),
                own_method_shapes: BTreeSet::new(),
            };
            raw.entry(owner.clone()).or_default().push(RawProfile {
                snapshot: GoPromotedProfileSnapshot {
                    defining_file: declaration.defining_file.clone(),
                    embedded_fields,
                    unresolved_embedded_fields: unresolved_embedded_fields.clone(),
                    ordinary_fields: BTreeSet::new(),
                    own_methods,
                    own_method_shapes: BTreeSet::new(),
                    promoted_methods: BTreeSet::new(),
                },
                comparison,
                unresolved: declaration.generic
                    || !unresolved_embedded_fields.is_empty()
                    || structs.contains_key(owner)
                    || !type_declarations.contains_key(owner),
            });
        }
    }
}

pub(super) fn local_import_paths(
    package_import_paths: &BTreeMap<String, String>,
    profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> BTreeMap<String, String> {
    crate::type_providers::go::GoTypeProvider::local_import_paths(package_import_paths, profiles)
}

fn selector(raw_type: &str) -> String {
    raw_type
        .trim()
        .trim_start_matches('*')
        .rsplit('.')
        .next()
        .unwrap_or(raw_type)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    #[test]
    fn snapshot_local_paths_use_the_provider_clause_filter() {
        let files = BTreeMap::from([
            (
                "mixed/a.go".to_string(),
                ParsedFile::parse("mixed/a.go", "package a\n", Language::Go).unwrap(),
            ),
            (
                "mixed/b.go".to_string(),
                ParsedFile::parse("mixed/b.go", "package b\n", Language::Go).unwrap(),
            ),
            (
                "stable/s.go".to_string(),
                ParsedFile::parse("stable/s.go", "package stable\n", Language::Go).unwrap(),
            ),
        ]);
        let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(&files);
        let package_paths = BTreeMap::from([
            ("mixed/a.go".to_string(), "example.test/mixed".to_string()),
            ("mixed/b.go".to_string(), "example.test/mixed".to_string()),
            ("stable/s.go".to_string(), "example.test/stable".to_string()),
        ]);

        assert_eq!(
            local_import_paths(&package_paths, &profiles),
            BTreeMap::from([("stable/s.go".to_string(), "example.test/stable".to_string(),)])
        );
    }
}
