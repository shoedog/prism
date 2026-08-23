//! Proven resolution of named Go function types used by P5 fields.

use crate::call_graph::CallGraph;
use crate::go_concrete_receiver::{GoTypeDeclarationForm, GoTypeDeclarations};
use crate::resolution::GoOwnerIdentity;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn declaration_resolves_to_func(
    owner: &GoOwnerIdentity,
    declarations: &GoTypeDeclarations,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
) -> bool {
    resolves_owner(
        owner,
        declarations,
        imports,
        package_basenames,
        file_profiles,
        &mut BTreeSet::new(),
    )
}

fn resolves_owner(
    owner: &GoOwnerIdentity,
    declarations: &GoTypeDeclarations,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> bool {
    if !visiting.insert(owner.clone()) {
        return false;
    }
    let result = declarations.get(owner).is_some_and(|candidates| {
        let declaring_files: BTreeSet<_> = candidates
            .iter()
            .map(|declaration| declaration.defining_file.as_str())
            .collect();
        if candidates.len() != 1 || declaring_files.len() != 1 {
            return false;
        }
        let declaration = candidates.iter().next().expect("one declaration");
        match &declaration.form {
            GoTypeDeclarationForm::Defined { target } | GoTypeDeclarationForm::Alias { target } => {
                resolves_text(
                    target,
                    &declaration.defining_file,
                    declarations,
                    imports,
                    package_basenames,
                    file_profiles,
                    visiting,
                )
            }
            GoTypeDeclarationForm::Struct | GoTypeDeclarationForm::Interface => false,
        }
    });
    visiting.remove(owner);
    result
}

#[allow(clippy::too_many_arguments)]
fn resolves_text(
    text: &str,
    declaring_file: &str,
    declarations: &GoTypeDeclarations,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    file_profiles: &BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    visiting: &mut BTreeSet<GoOwnerIdentity>,
) -> bool {
    let text = text.trim();
    if text.starts_with('*') {
        return false;
    }
    if text.starts_with("func(") {
        return true;
    }
    let Some(owner) = crate::resolution::resolve_go_owner_identity(
        text,
        declaring_file,
        imports,
        package_basenames,
        file_profiles,
    ) else {
        return false;
    };
    resolves_owner(
        &owner,
        declarations,
        imports,
        package_basenames,
        file_profiles,
        visiting,
    )
}

impl CallGraph {
    pub(crate) fn go_field_type_is_func(
        &self,
        field_owner: &GoOwnerIdentity,
        field_type: &str,
    ) -> bool {
        let field_type = field_type.trim();
        if field_type.starts_with('*') {
            return false;
        }
        if field_type.starts_with("func(") {
            return true;
        }
        let Some(declaring_file) = self
            .go_declaration_kind_index
            .get(field_owner)
            .and_then(|entry| entry.declaring_file.as_deref())
        else {
            return false;
        };
        self.go_receiver_owner(field_type, declaring_file, None)
            .and_then(|owner| self.go_declaration_kind_index.get(&owner))
            .is_some_and(|entry| entry.underlying_func)
    }
}
