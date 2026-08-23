use super::{ComparisonKey, GoPromotedEmbeddedField, GoPromotedProfileSnapshot, RawProfile};
use crate::ast::ParsedFile;
use crate::go_owner_partition::{GoInterfaceDeclarations, GoStructDeclarations};
use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn extend(
    raw: &mut BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    files: &BTreeMap<String, ParsedFile>,
    package_import_paths: &BTreeMap<String, String>,
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
                        package_import_paths
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

fn selector(raw_type: &str) -> String {
    raw_type
        .trim()
        .trim_start_matches('*')
        .rsplit('.')
        .next()
        .unwrap_or(raw_type)
        .to_string()
}
