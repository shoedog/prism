//! Profile-keyed Go promoted-selector facts.
//!
//! This is a serialized foundation snapshot only. Resolution continues to use
//! the existing promotion indexes until a later slice explicitly consumes it.

use crate::ast::ParsedFile;
use crate::call_graph::FunctionId;
use crate::go_build_profile::GoBuildProfile;
use crate::go_owner_partition::{
    exact_declaration_visibility, GoInterfaceDeclarations, GoMethodDeclaration,
    GoMethodDeclarations, GoOwnerReferenceMode, GoStructDeclarations,
};
use crate::resolution::GoOwnerIdentity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod interface_profiles;
mod selector_resolution;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoPromotedSelectorSnapshot {
    pub owners: BTreeMap<GoOwnerIdentity, GoPromotedOwnerSnapshot>,
}

impl GoPromotedSelectorSnapshot {
    pub fn profile_conflicts(&self) -> usize {
        self.owners
            .values()
            .filter(|owner| owner.verdict == GoPromotedSnapshotVerdict::ProfileConflict)
            .count()
    }

    pub fn promoted_methods(&self) -> usize {
        self.owners
            .values()
            .flat_map(|owner| &owner.declarations)
            .map(|declaration| declaration.promoted_methods.len())
            .sum()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoPromotedSnapshotVerdict {
    #[default]
    ProfileUnique,
    ProfileConflict,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoPromotedOwnerSnapshot {
    pub verdict: GoPromotedSnapshotVerdict,
    pub declarations: Vec<GoPromotedProfileSnapshot>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoPromotedProfileSnapshot {
    pub defining_file: String,
    pub embedded_fields: BTreeSet<GoPromotedEmbeddedField>,
    pub unresolved_embedded_fields: BTreeSet<String>,
    pub ordinary_fields: BTreeSet<String>,
    pub own_methods: BTreeSet<String>,
    /// Fifth profile-safety axis: selector membership can change when a method
    /// keeps its name but switches between value and pointer receiver.
    pub own_method_shapes: BTreeSet<GoPromotedOwnMethodShape>,
    pub promoted_methods: BTreeSet<GoPromotedMethodSnapshot>,
    #[serde(default)]
    pub ambiguous_promoted_methods: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoPromotedEmbeddedField {
    pub pointer: bool,
    pub target: GoOwnerIdentity,
    pub selector: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoPromotedOwnMethodShape {
    pub method: String,
    pub pointer_receiver: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GoPromotedMethodSnapshot {
    pub method: String,
    pub target: FunctionId,
    pub target_owner: GoOwnerIdentity,
    pub depth: usize,
    pub field_shadowed: bool,
    pub value_method_set: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ComparisonKey {
    embedded_fields: BTreeSet<GoPromotedEmbeddedField>,
    ordinary_fields: BTreeSet<String>,
    own_methods: BTreeSet<String>,
    own_method_shapes: BTreeSet<GoPromotedOwnMethodShape>,
}

#[derive(Debug, Clone)]
struct RawProfile {
    snapshot: GoPromotedProfileSnapshot,
    comparison: ComparisonKey,
    unresolved: bool,
}

pub(crate) fn build(
    files: &BTreeMap<String, ParsedFile>,
    package_import_paths: &BTreeMap<String, String>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    type_declarations: &BTreeMap<GoOwnerIdentity, BTreeSet<String>>,
    structs: &GoStructDeclarations,
    interfaces: &GoInterfaceDeclarations,
    methods: &GoMethodDeclarations,
) -> GoPromotedSelectorSnapshot {
    // Invalid anonymous struct embedding is retained by tree-sitter under a
    // top-level ERROR node. Recover it only for this diagnostic snapshot; the
    // provider's routing inputs remain untouched.
    let mut recovered_structs = structs.clone();
    recover_error_structs(files, profiles, &mut recovered_structs);
    let structs = &recovered_structs;
    let local_import_paths = interface_profiles::local_import_paths(package_import_paths, profiles);
    let aliases = crate::go_type_alias::GoAliasResolver::build(
        files,
        package_import_paths,
        &local_import_paths,
        profiles,
    );
    let mut raw = BTreeMap::<GoOwnerIdentity, Vec<RawProfile>>::new();
    for (owner, declarations) in structs {
        for declaration in declarations {
            let mut embedded_fields = BTreeSet::new();
            let mut unresolved_embedded_fields = declaration.unresolved_embedded_fields.clone();
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
            for (selector, raw_type) in &declaration.embedded_fields {
                match aliases.resolve_embedded_owner(raw_type, &declaration.defining_file, &imports)
                {
                    Ok((pointer, target))
                        if structs.contains_key(&target)
                            || interfaces.contains_key(&target)
                            || type_declarations.contains_key(&target) =>
                    {
                        embedded_fields.insert(GoPromotedEmbeddedField {
                            pointer,
                            target,
                            selector: selector.clone(),
                        });
                    }
                    _ => {
                        unresolved_embedded_fields.insert(raw_type.clone());
                    }
                }
            }
            let ordinary_fields = declaration
                .fields
                .keys()
                .filter(|name| {
                    name.as_str() != "_" && !declaration.embedded_fields.contains_key(*name)
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            let (visible_methods, methods_uncertain) = visible_methods(
                owner,
                &declaration.defining_file,
                GoOwnerReferenceMode::Bare,
                methods,
                profiles,
            );
            let own_methods = visible_methods
                .iter()
                .map(|method| method.method_name.clone())
                .collect::<BTreeSet<_>>();
            let own_method_shapes = visible_methods
                .iter()
                .map(|method| GoPromotedOwnMethodShape {
                    method: method.method_name.clone(),
                    pointer_receiver: method.is_pointer_receiver,
                })
                .collect::<BTreeSet<_>>();
            let comparison = ComparisonKey {
                embedded_fields: embedded_fields.clone(),
                ordinary_fields: ordinary_fields.clone(),
                own_methods: own_methods.clone(),
                own_method_shapes: own_method_shapes.clone(),
            };
            raw.entry(owner.clone()).or_default().push(RawProfile {
                snapshot: GoPromotedProfileSnapshot {
                    defining_file: declaration.defining_file.clone(),
                    embedded_fields,
                    unresolved_embedded_fields: unresolved_embedded_fields.clone(),
                    ordinary_fields,
                    own_methods,
                    own_method_shapes,
                    promoted_methods: BTreeSet::new(),
                    ambiguous_promoted_methods: BTreeSet::new(),
                },
                comparison,
                unresolved: methods_uncertain || !unresolved_embedded_fields.is_empty(),
            });
        }
    }

    interface_profiles::extend(
        &mut raw,
        files,
        &local_import_paths,
        interfaces,
        structs,
        type_declarations,
        &aliases,
    );

    // Defined non-struct types can be embedded and contribute own methods.
    // Keep them as internal hop profiles; only struct owners are published.
    for (owner, declaring_files) in type_declarations {
        if raw.contains_key(owner) || interfaces.contains_key(owner) {
            continue;
        }
        for defining_file in declaring_files {
            let (visible_methods, methods_uncertain) = visible_methods(
                owner,
                defining_file,
                GoOwnerReferenceMode::Bare,
                methods,
                profiles,
            );
            let own_methods = visible_methods
                .iter()
                .map(|method| method.method_name.clone())
                .collect::<BTreeSet<_>>();
            let own_method_shapes = visible_methods
                .iter()
                .map(|method| GoPromotedOwnMethodShape {
                    method: method.method_name.clone(),
                    pointer_receiver: method.is_pointer_receiver,
                })
                .collect::<BTreeSet<_>>();
            raw.entry(owner.clone()).or_default().push(RawProfile {
                snapshot: GoPromotedProfileSnapshot {
                    defining_file: defining_file.clone(),
                    embedded_fields: BTreeSet::new(),
                    unresolved_embedded_fields: BTreeSet::new(),
                    ordinary_fields: BTreeSet::new(),
                    own_methods: own_methods.clone(),
                    own_method_shapes: own_method_shapes.clone(),
                    promoted_methods: BTreeSet::new(),
                    ambiguous_promoted_methods: BTreeSet::new(),
                },
                comparison: ComparisonKey {
                    embedded_fields: BTreeSet::new(),
                    ordinary_fields: BTreeSet::new(),
                    own_methods,
                    own_method_shapes,
                },
                unresolved: methods_uncertain,
            });
        }
    }

    let mut conflicts = BTreeMap::new();
    for owner in raw.keys() {
        owner_conflicts(owner, &raw, &mut conflicts, &mut BTreeSet::new());
    }

    let mut owners = BTreeMap::new();
    for (owner, profiles_for_owner) in &raw {
        if !structs.contains_key(owner) {
            continue;
        }
        let verdict = if conflicts.get(owner).copied().unwrap_or(true) {
            GoPromotedSnapshotVerdict::ProfileConflict
        } else {
            GoPromotedSnapshotVerdict::ProfileUnique
        };
        let mut declarations = Vec::new();
        for raw_profile in profiles_for_owner {
            let mut declaration = raw_profile.snapshot.clone();
            let (promoted_methods, ambiguous_promoted_methods) =
                selector_resolution::promoted_for_profile(
                    owner,
                    raw_profile,
                    &raw,
                    &conflicts,
                    methods,
                    profiles,
                );
            declaration.promoted_methods = promoted_methods;
            declaration.ambiguous_promoted_methods = ambiguous_promoted_methods;
            declarations.push(declaration);
        }
        declarations.sort_by(|left, right| left.defining_file.cmp(&right.defining_file));
        owners.insert(
            owner.clone(),
            GoPromotedOwnerSnapshot {
                verdict,
                declarations,
            },
        );
    }
    GoPromotedSelectorSnapshot { owners }
}

fn recover_error_structs(
    files: &BTreeMap<String, ParsedFile>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    structs: &mut GoStructDeclarations,
) {
    for (path, parsed) in files {
        let Some(profile) = profiles.get(path) else {
            continue;
        };
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.named_children(&mut cursor) {
            if child.kind() != "ERROR" {
                continue;
            }
            let mut declarations = Vec::new();
            collect_error_type_declarations(child, &mut declarations);
            for declaration in declarations {
                let mut declaration_cursor = declaration.walk();
                for spec in declaration.named_children(&mut declaration_cursor) {
                    if spec.kind() != "type_spec" {
                        continue;
                    }
                    let (Some(name_node), Some(type_node)) = (
                        spec.child_by_field_name("name"),
                        spec.child_by_field_name("type"),
                    ) else {
                        continue;
                    };
                    if type_node.kind() != "struct_type" || !type_node.has_error() {
                        continue;
                    }
                    let name = parsed.node_text(&name_node).trim();
                    if name.is_empty() {
                        continue;
                    }
                    let owner = GoOwnerIdentity {
                        package_dir: crate::resolution::dir_of(path).to_string(),
                        package_clause: profile.package_clause.clone(),
                        name: name.to_string(),
                    };
                    let already_recorded = structs.get(&owner).is_some_and(|known| {
                        known
                            .iter()
                            .any(|known| known.defining_file == path.as_str())
                    });
                    if already_recorded {
                        continue;
                    }
                    structs.entry(owner).or_default().insert(
                        crate::go_owner_partition::GoStructDeclaration {
                            defining_file: path.clone(),
                            fields: BTreeMap::new(),
                            embedded_fields: BTreeMap::new(),
                            unresolved_embedded_fields: BTreeSet::from([parsed
                                .node_text(&type_node)
                                .trim()
                                .to_string()]),
                            embedded_types: BTreeSet::new(),
                        },
                    );
                }
            }
        }
    }
}

fn collect_error_type_declarations<'a>(
    node: tree_sitter::Node<'a>,
    out: &mut Vec<tree_sitter::Node<'a>>,
) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "type_declaration" {
            out.push(child);
        } else if child.kind() == "ERROR" {
            collect_error_type_declarations(child, out);
        }
    }
}

fn owner_conflicts(
    owner: &GoOwnerIdentity,
    raw: &BTreeMap<GoOwnerIdentity, Vec<RawProfile>>,
    memo: &mut BTreeMap<GoOwnerIdentity, bool>,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> bool {
    if let Some(conflict) = memo.get(owner) {
        return *conflict;
    }
    if !visiting.insert(owner.clone()) {
        return true;
    }
    let Some(profiles) = raw.get(owner) else {
        visiting.remove(owner);
        memo.insert(owner.clone(), true);
        return true;
    };
    let keys = profiles
        .iter()
        .map(|profile| profile.comparison.clone())
        .collect::<BTreeSet<_>>();
    let direct = profiles.iter().any(|profile| profile.unresolved) || keys.len() != 1;
    let downstream = profiles
        .iter()
        .flat_map(|profile| &profile.snapshot.embedded_fields)
        .any(|embedded| owner_conflicts(&embedded.target, raw, memo, visiting));
    visiting.remove(owner);
    let conflict = direct || downstream;
    memo.insert(owner.clone(), conflict);
    conflict
}

fn visible_methods(
    owner: &GoOwnerIdentity,
    profile_file: &str,
    mode: GoOwnerReferenceMode,
    methods: &GoMethodDeclarations,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> (BTreeSet<GoMethodDeclaration>, bool) {
    let mut visible = BTreeSet::new();
    let mut uncertain = false;
    for method in methods.get(owner).into_iter().flatten() {
        let (potentially_visible, exact) = exact_declaration_visibility(
            owner,
            profile_file,
            mode,
            &method.defining_file,
            profiles,
        );
        if potentially_visible && !exact {
            uncertain = true;
        } else if exact {
            visible.insert(method.clone());
        }
    }
    (visible, uncertain)
}
