//! Exact Go callback signatures for the bounded Level-3 B1 resolver.

use crate::ast::ParsedFile;
use crate::call_graph::{CallKind, CallSite, CallSiteOrigin, FunctionId};
use crate::go_build_profile::GoBuildProfile;
use crate::languages::Language;
use crate::resolution::{dir_of, GoOwnerIdentity};
use crate::type_providers::go::GoTypeProvider;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct GoCallbackParameterFact {
    pub slot: usize,
    pub name: String,
    pub signature: String,
}

pub(crate) type GoCallbackParameterIndex = BTreeMap<FunctionId, Vec<GoCallbackParameterFact>>;
pub(crate) type GoFreeFunctionSignatureIndex = BTreeMap<FunctionId, String>;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GoCallbackIndices {
    pub parameters: GoCallbackParameterIndex,
    pub free_functions: GoFreeFunctionSignatureIndex,
}

pub(crate) fn extract_go_callback_indices(
    files: &BTreeMap<String, ParsedFile>,
    package_import_paths: &BTreeMap<String, String>,
) -> GoCallbackIndices {
    let (profiles, _) = crate::go_build_profile::extract_go_file_profiles(files);
    let clean_package_dirs = clean_package_dirs(files, &profiles);
    let packages_by_import_path =
        packages_by_import_path(package_import_paths, &profiles, &clean_package_dirs);
    let imports = strict_signature_imports(
        files,
        package_import_paths,
        &profiles,
        &packages_by_import_path,
        &clean_package_dirs,
    );
    let alias_resolver = crate::go_type_alias::GoAliasResolver::build_callback_strict(
        files,
        package_import_paths,
        package_import_paths,
        &profiles,
        &imports,
    );
    let declarations = named_type_declarations(files, &profiles, &clean_package_dirs);
    let context = CallbackExtractionContext {
        files,
        profiles: &profiles,
        imports: &imports,
        packages_by_import_path: &packages_by_import_path,
        declarations: &declarations,
        alias_resolver: &alias_resolver,
    };
    let mut indices = GoCallbackIndices::default();

    for (path, parsed) in files {
        if !admissible_file(path, parsed, &profiles) || !clean_package_dirs.contains(dir_of(path)) {
            continue;
        }
        let file_imports = imports.get(path).cloned().unwrap_or_default();
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for function in root.children(&mut cursor) {
            if function.kind() != "function_declaration"
                || function.child_by_field_name("type_parameters").is_some()
            {
                continue;
            }
            let Some(name_node) = function.child_by_field_name("name") else {
                continue;
            };
            let name = parsed.node_text(&name_node).trim();
            if name.is_empty() {
                continue;
            }
            let function_id = FunctionId {
                name: name.to_string(),
                file: path.clone(),
                start_line: function.start_position().row + 1,
                end_line: function.end_position().row + 1,
            };
            if let Ok(signature) = GoTypeProvider::canon_sig_with_imports(
                function.child_by_field_name("parameters").as_ref(),
                function.child_by_field_name("result").as_ref(),
                parsed,
                path,
                &file_imports,
                &alias_resolver,
            ) {
                indices
                    .free_functions
                    .insert(function_id.clone(), signature);
            }

            let Some(slots) = parsed.function_parameter_slot_occurrences(&function) else {
                continue;
            };
            let mut facts = Vec::new();
            for (slot, (name, start, end)) in slots.into_iter().enumerate() {
                let Some(binding) = root.descendant_for_byte_range(start, end) else {
                    continue;
                };
                let Some(declaration) = enclosing_parameter_declaration(binding, function) else {
                    continue;
                };
                if declaration.kind() == "variadic_parameter_declaration" {
                    continue;
                }
                let Some(type_node) = declaration.child_by_field_name("type") else {
                    continue;
                };
                if let Some(signature) =
                    context.callable_signature(type_node, path, &mut BTreeSet::new())
                {
                    facts.push(GoCallbackParameterFact {
                        slot,
                        name,
                        signature,
                    });
                }
            }
            if !facts.is_empty() {
                indices.parameters.insert(function_id, facts);
            }
        }
    }

    indices
}

#[derive(Debug, Clone)]
struct NamedTypeDeclaration {
    file: String,
    start_byte: usize,
    end_byte: usize,
}

struct CallbackExtractionContext<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    profiles: &'a BTreeMap<String, GoBuildProfile>,
    imports: &'a BTreeMap<String, BTreeMap<String, String>>,
    packages_by_import_path: &'a BTreeMap<String, BTreeSet<(String, String)>>,
    declarations: &'a BTreeMap<GoOwnerIdentity, Vec<NamedTypeDeclaration>>,
    alias_resolver: &'a crate::go_type_alias::GoAliasResolver,
}

impl CallbackExtractionContext<'_> {
    fn callable_signature(
        &self,
        type_node: tree_sitter::Node<'_>,
        file: &str,
        visiting: &mut BTreeSet<GoOwnerIdentity>,
    ) -> Option<String> {
        match type_node.kind() {
            "function_type" => {
                let parsed = self.files.get(file)?;
                let imports = self.imports.get(file)?;
                GoTypeProvider::canon_sig_with_imports(
                    type_node.child_by_field_name("parameters").as_ref(),
                    type_node.child_by_field_name("result").as_ref(),
                    parsed,
                    file,
                    imports,
                    self.alias_resolver,
                )
                .ok()
            }
            "parenthesized_type" => type_node
                .named_child(0)
                .and_then(|inner| self.callable_signature(inner, file, visiting)),
            "type_identifier" | "qualified_type" => {
                let owner = self.owner_for_type(type_node, file)?;
                if !visiting.insert(owner.clone()) {
                    return None;
                }
                let result = (|| {
                    let declarations = self.declarations.get(&owner)?;
                    if declarations.len() != 1 {
                        return None;
                    }
                    let declaration = &declarations[0];
                    let parsed = self.files.get(&declaration.file)?;
                    let node = parsed
                        .tree
                        .root_node()
                        .descendant_for_byte_range(declaration.start_byte, declaration.end_byte)?;
                    if !matches!(node.kind(), "type_spec" | "type_alias")
                        || node.child_by_field_name("type_parameters").is_some()
                    {
                        return None;
                    }
                    let target = node.child_by_field_name("type")?;
                    self.callable_signature(target, &declaration.file, visiting)
                })();
                visiting.remove(&owner);
                result
            }
            _ => None,
        }
    }

    fn owner_for_type(
        &self,
        type_node: tree_sitter::Node<'_>,
        file: &str,
    ) -> Option<GoOwnerIdentity> {
        let parsed = self.files.get(file)?;
        match type_node.kind() {
            "type_identifier" => {
                let profile = self.profiles.get(file)?;
                let name = parsed.node_text(&type_node).trim();
                (!profile.is_test_file && !profile.package_clause.is_empty() && !name.is_empty())
                    .then(|| GoOwnerIdentity {
                        package_dir: dir_of(file).to_string(),
                        package_clause: profile.package_clause.clone(),
                        name: name.to_string(),
                    })
            }
            "qualified_type" => {
                let package = type_node.child_by_field_name("package")?;
                let name = type_node.child_by_field_name("name")?;
                let qualifier = parsed.node_text(&package).trim();
                let name = parsed.node_text(&name).trim();
                let import_path = self.imports.get(file)?.get(qualifier)?;
                let packages = self.packages_by_import_path.get(import_path)?;
                if packages.len() != 1 || name.is_empty() {
                    return None;
                }
                let (package_dir, package_clause) = packages.iter().next()?;
                Some(GoOwnerIdentity {
                    package_dir: package_dir.clone(),
                    package_clause: package_clause.clone(),
                    name: name.to_string(),
                })
            }
            _ => None,
        }
    }
}

fn enclosing_parameter_declaration<'a>(
    mut node: tree_sitter::Node<'a>,
    function: tree_sitter::Node<'a>,
) -> Option<tree_sitter::Node<'a>> {
    loop {
        if matches!(
            node.kind(),
            "parameter_declaration" | "variadic_parameter_declaration"
        ) {
            return Some(node);
        }
        node = node.parent()?;
        if node.start_byte() < function.start_byte() || node.end_byte() > function.end_byte() {
            return None;
        }
    }
}

fn admissible_file(
    path: &str,
    parsed: &ParsedFile,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> bool {
    parsed.language == Language::Go
        && profiles
            .get(path)
            .is_some_and(|profile| !profile.is_test_file && !profile.package_clause.is_empty())
        && !contains_recovery(parsed.tree.root_node())
}

fn contains_recovery(node: tree_sitter::Node<'_>) -> bool {
    if node.is_error() || node.is_missing() {
        return true;
    }
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(contains_recovery);
    found
}

fn clean_package_dirs(
    files: &BTreeMap<String, ParsedFile>,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> BTreeSet<String> {
    let mut clauses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut poisoned = BTreeSet::new();
    for (path, parsed) in files {
        if parsed.language != Language::Go || path.ends_with("_test.go") {
            continue;
        }
        let dir = dir_of(path).to_string();
        let Some(profile) = profiles.get(path) else {
            poisoned.insert(dir);
            continue;
        };
        if profile.package_clause.is_empty()
            || profile.build_unparsed
            || contains_recovery(parsed.tree.root_node())
        {
            poisoned.insert(dir);
            continue;
        }
        clauses
            .entry(dir)
            .or_default()
            .insert(profile.package_clause.clone());
    }
    clauses
        .into_iter()
        .filter_map(|(dir, clauses)| {
            (clauses.len() == 1 && !poisoned.contains(&dir)).then_some(dir)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum GoLevel3DropReason {
    NonGoOrTest,
    PackagePoison,
    DotImport,
    StrictImportNameUnavailable,
    MissingOccurrence,
    NonBareArgument,
    LocalBindingOrMutation,
    FileOrPackageNamespaceCollision,
    TargetUnresolvedOrAmbiguous,
    TargetNotFreeOrTest,
    HofNotSingletonExact,
    MissingSlotOrArgumentIdentity,
    SignatureUnprovenOrMismatch,
    MissingCallbackParameter,
    CallbackParameterShadowOrMutation,
    CallbackParameterAddressEscape,
    NestedCallableInvocation,
    NoDirectParameterInvocation,
}

impl GoLevel3DropReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NonGoOrTest => "non_go_or_test",
            Self::PackagePoison => "package_poison",
            Self::DotImport => "dot_import",
            Self::StrictImportNameUnavailable => "strict_import_name_unavailable",
            Self::MissingOccurrence => "missing_occurrence",
            Self::NonBareArgument => "non_bare_argument",
            Self::LocalBindingOrMutation => "local_binding_or_mutation",
            Self::FileOrPackageNamespaceCollision => "file_or_package_namespace_collision",
            Self::TargetUnresolvedOrAmbiguous => "target_unresolved_or_ambiguous",
            Self::TargetNotFreeOrTest => "target_not_free_or_test",
            Self::HofNotSingletonExact => "hof_not_singleton_exact",
            Self::MissingSlotOrArgumentIdentity => "missing_slot_or_argument_identity",
            Self::SignatureUnprovenOrMismatch => "signature_unproven_or_mismatch",
            Self::MissingCallbackParameter => "missing_callback_parameter",
            Self::CallbackParameterShadowOrMutation => "callback_parameter_shadow_or_mutation",
            Self::CallbackParameterAddressEscape => "callback_parameter_address_escape",
            Self::NestedCallableInvocation => "nested_callable_invocation",
            Self::NoDirectParameterInvocation => "no_direct_parameter_invocation",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GoLevel3Telemetry {
    pub candidates: usize,
    pub exact_inbound_sites: usize,
    pub accepted_inbound_sites: usize,
    pub unique_targets: usize,
    pub edges: usize,
    pub drops: BTreeMap<String, usize>,
    #[serde(default)]
    pub sites: Vec<GoLevel3SiteRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GoLevel3InvocationSpan {
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GoLevel3SiteRecord {
    pub inbound_caller: FunctionId,
    pub inbound_line: usize,
    pub inbound_start_byte: usize,
    pub inbound_end_byte: usize,
    pub hof_name: String,
    pub slot: usize,
    pub argument: Option<String>,
    pub hof: Option<FunctionId>,
    pub target: Option<FunctionId>,
    pub callback_parameter: Option<String>,
    pub invocation_spans: Vec<GoLevel3InvocationSpan>,
    pub accepted: bool,
    pub drop_reason: Option<String>,
}

pub(crate) struct GoCallbackProofContext<'a> {
    files: &'a BTreeMap<String, ParsedFile>,
    functions: &'a BTreeMap<String, Vec<FunctionId>>,
    method_owners: &'a BTreeMap<FunctionId, String>,
    profiles: &'a BTreeMap<String, GoBuildProfile>,
    dot_import_files: &'a BTreeSet<String>,
    package_import_paths: &'a BTreeMap<String, String>,
    clean_package_dirs: BTreeSet<String>,
    strict_imports: BTreeMap<String, BTreeMap<String, String>>,
    unproven_default_import_files: BTreeSet<String>,
}

impl<'a> GoCallbackProofContext<'a> {
    pub(crate) fn new(
        files: &'a BTreeMap<String, ParsedFile>,
        functions: &'a BTreeMap<String, Vec<FunctionId>>,
        method_owners: &'a BTreeMap<FunctionId, String>,
        profiles: &'a BTreeMap<String, GoBuildProfile>,
        dot_import_files: &'a BTreeSet<String>,
        package_import_paths: &'a BTreeMap<String, String>,
    ) -> Self {
        let clean_package_dirs = clean_package_dirs(files, profiles);
        let packages_by_import_path =
            packages_by_import_path(package_import_paths, profiles, &clean_package_dirs);
        let strict_imports = strict_signature_imports(
            files,
            package_import_paths,
            profiles,
            &packages_by_import_path,
            &clean_package_dirs,
        );
        let unproven_default_import_files = files
            .iter()
            .filter_map(|(path, parsed)| {
                has_unproven_default_import(parsed, &packages_by_import_path)
                    .then_some(path.clone())
            })
            .collect();
        Self {
            files,
            functions,
            method_owners,
            profiles,
            dot_import_files,
            package_import_paths,
            clean_package_dirs,
            strict_imports,
            unproven_default_import_files,
        }
    }

    pub(crate) fn bare_free_function_at(
        &self,
        caller: &FunctionId,
        occurrence: Range<usize>,
        name: &str,
    ) -> Result<FunctionId, GoLevel3DropReason> {
        let parsed = self
            .files
            .get(&caller.file)
            .ok_or(GoLevel3DropReason::NonGoOrTest)?;
        let profile = self
            .profiles
            .get(&caller.file)
            .ok_or(GoLevel3DropReason::NonGoOrTest)?;
        if parsed.language != Language::Go || profile.is_test_file {
            return Err(GoLevel3DropReason::NonGoOrTest);
        }
        if !self.clean_package_dirs.contains(dir_of(&caller.file)) {
            return Err(GoLevel3DropReason::PackagePoison);
        }
        if self.dot_import_files.contains(&caller.file) {
            return Err(GoLevel3DropReason::DotImport);
        }
        if self.unproven_default_import_files.contains(&caller.file) {
            return Err(GoLevel3DropReason::StrictImportNameUnavailable);
        }

        let occurrence_node = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(occurrence.start, occurrence.end)
            .ok_or(GoLevel3DropReason::MissingOccurrence)?;
        if occurrence_node.kind() != "identifier"
            || occurrence_node.start_byte() != occurrence.start
            || occurrence_node.end_byte() != occurrence.end
            || parsed.node_text(&occurrence_node).trim() != name
        {
            return Err(GoLevel3DropReason::NonBareArgument);
        }

        let function =
            named_function_node(parsed, caller).ok_or(GoLevel3DropReason::MissingOccurrence)?;
        if function_has_binding_or_mutation(parsed, function, name, None) {
            return Err(GoLevel3DropReason::LocalBindingOrMutation);
        }
        if self
            .strict_imports
            .get(&caller.file)
            .is_some_and(|imports| imports.contains_key(name))
            || package_has_non_function_name(self.files, &caller.file, name)
        {
            return Err(GoLevel3DropReason::FileOrPackageNamespaceCollision);
        }

        let mut ambiguous = 0;
        let target = crate::resolution::resolve_go_bare_value_ref(
            self.functions,
            self.method_owners,
            self.profiles,
            &mut ambiguous,
            &caller.file,
            name,
        )
        .ok_or(GoLevel3DropReason::TargetUnresolvedOrAmbiguous)?;
        let target_profile = self
            .profiles
            .get(&target.file)
            .ok_or(GoLevel3DropReason::TargetNotFreeOrTest)?;
        if target_profile.is_test_file
            || self.method_owners.contains_key(&target)
            || !self.clean_package_dirs.contains(dir_of(&target.file))
        {
            return Err(GoLevel3DropReason::TargetNotFreeOrTest);
        }
        Ok(target)
    }

    pub(crate) fn hof_at(
        &self,
        caller: &FunctionId,
        site: &CallSite,
    ) -> Result<FunctionId, GoLevel3DropReason> {
        if site.origin != CallSiteOrigin::Source || site.kind != CallKind::Call {
            return Err(GoLevel3DropReason::MissingOccurrence);
        }
        let parsed = self
            .files
            .get(&caller.file)
            .ok_or(GoLevel3DropReason::NonGoOrTest)?;
        let call = call_node_at(parsed, site.start_byte, &site.callee_name)
            .ok_or(GoLevel3DropReason::MissingOccurrence)?;
        let function = call
            .child_by_field_name("function")
            .ok_or(GoLevel3DropReason::MissingOccurrence)?;
        if site.qualifier.is_none() {
            return self.bare_free_function_at(
                caller,
                function.start_byte()..function.end_byte(),
                &site.callee_name,
            );
        }
        if !self.clean_package_dirs.contains(dir_of(&caller.file)) {
            return Err(GoLevel3DropReason::PackagePoison);
        }
        if self.dot_import_files.contains(&caller.file) {
            return Err(GoLevel3DropReason::DotImport);
        }
        if self.unproven_default_import_files.contains(&caller.file) {
            return Err(GoLevel3DropReason::StrictImportNameUnavailable);
        }
        let qualifier = site.qualifier.as_deref().unwrap_or_default();
        if function.kind() != "selector_expression" {
            return Err(GoLevel3DropReason::MissingOccurrence);
        }
        let operand = function
            .child_by_field_name("operand")
            .ok_or(GoLevel3DropReason::MissingOccurrence)?;
        let field = function
            .child_by_field_name("field")
            .ok_or(GoLevel3DropReason::MissingOccurrence)?;
        if operand.kind() != "identifier"
            || parsed.node_text(&operand).trim() != qualifier
            || parsed.node_text(&field).trim() != site.callee_name
        {
            return Err(GoLevel3DropReason::MissingOccurrence);
        }
        let caller_function =
            named_function_node(parsed, caller).ok_or(GoLevel3DropReason::MissingOccurrence)?;
        if function_has_binding_or_mutation(parsed, caller_function, qualifier, None) {
            return Err(GoLevel3DropReason::LocalBindingOrMutation);
        }
        let import_path = self
            .strict_imports
            .get(&caller.file)
            .and_then(|imports| imports.get(qualifier))
            .ok_or(GoLevel3DropReason::StrictImportNameUnavailable)?;
        let candidates = self
            .functions
            .get(&site.callee_name)
            .into_iter()
            .flatten()
            .filter(|candidate| {
                !self.method_owners.contains_key(*candidate)
                    && self.package_import_paths.get(&candidate.file) == Some(import_path)
                    && self
                        .profiles
                        .get(&candidate.file)
                        .is_some_and(|profile| !profile.is_test_file)
                    && self.clean_package_dirs.contains(dir_of(&candidate.file))
            })
            .cloned()
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            return Err(GoLevel3DropReason::TargetUnresolvedOrAmbiguous);
        }
        Ok(candidates[0].clone())
    }

    pub(crate) fn direct_parameter_invocations(
        &self,
        calls: &BTreeMap<FunctionId, BTreeSet<CallSite>>,
        hof: &FunctionId,
        parameter: &GoCallbackParameterFact,
    ) -> Result<Vec<CallSite>, GoLevel3DropReason> {
        let parsed = self
            .files
            .get(&hof.file)
            .ok_or(GoLevel3DropReason::NonGoOrTest)?;
        if !self.clean_package_dirs.contains(dir_of(&hof.file)) {
            return Err(GoLevel3DropReason::PackagePoison);
        }
        let function =
            free_function_node(parsed, hof).ok_or(GoLevel3DropReason::TargetNotFreeOrTest)?;
        let slots = parsed
            .function_parameter_slot_occurrences(&function)
            .ok_or(GoLevel3DropReason::MissingCallbackParameter)?;
        let Some((slot_name, start, end)) = slots.get(parameter.slot) else {
            return Err(GoLevel3DropReason::MissingCallbackParameter);
        };
        if slot_name != &parameter.name
            || slots
                .iter()
                .filter(|(name, _, _)| name == &parameter.name)
                .count()
                != 1
        {
            return Err(GoLevel3DropReason::MissingCallbackParameter);
        }
        let parameter_span = Some(*start..*end);
        if function_has_binding_or_mutation(parsed, function, &parameter.name, parameter_span) {
            return Err(GoLevel3DropReason::CallbackParameterShadowOrMutation);
        }
        if contains_address_escape(parsed, function, &parameter.name) {
            return Err(GoLevel3DropReason::CallbackParameterAddressEscape);
        }

        let mut direct = Vec::new();
        let mut nested = false;
        for site in calls.get(hof).into_iter().flatten() {
            if site.origin != CallSiteOrigin::Source
                || site.kind != CallKind::Call
                || site.qualifier.is_some()
                || site.callee_name != parameter.name
            {
                continue;
            }
            let Some(call) = call_node_at(parsed, site.start_byte, &parameter.name) else {
                continue;
            };
            if call_is_nested_in_callable(call, function) {
                nested = true;
                continue;
            }
            direct.push(site.clone());
        }
        if direct.is_empty() {
            return Err(if nested {
                GoLevel3DropReason::NestedCallableInvocation
            } else {
                GoLevel3DropReason::NoDirectParameterInvocation
            });
        }
        Ok(direct)
    }
}

fn has_unproven_default_import(
    parsed: &ParsedFile,
    packages_by_import_path: &BTreeMap<String, BTreeSet<(String, String)>>,
) -> bool {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let found = root
        .children(&mut cursor)
        .filter(|node| node.kind() == "import_declaration")
        .flat_map(|node| import_specs(parsed, node))
        .any(|(alias, path)| {
            alias.is_none()
                && packages_by_import_path
                    .get(&path)
                    .is_none_or(|packages| packages.len() != 1)
        });
    found
}

fn named_function_node<'a>(
    parsed: &'a ParsedFile,
    function: &FunctionId,
) -> Option<tree_sitter::Node<'a>> {
    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let matches = root
        .children(&mut cursor)
        .filter(|node| matches!(node.kind(), "function_declaration" | "method_declaration"))
        .filter(|node| {
            node.start_position().row + 1 == function.start_line
                && node.end_position().row + 1 == function.end_line
                && node
                    .child_by_field_name("name")
                    .is_some_and(|name| parsed.node_text(&name).trim() == function.name)
        })
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| matches[0])
}

fn free_function_node<'a>(
    parsed: &'a ParsedFile,
    function: &FunctionId,
) -> Option<tree_sitter::Node<'a>> {
    named_function_node(parsed, function).filter(|node| node.kind() == "function_declaration")
}

fn function_has_binding_or_mutation(
    parsed: &ParsedFile,
    function: tree_sitter::Node<'_>,
    name: &str,
    exempt: Option<Range<usize>>,
) -> bool {
    fn matches_name(
        parsed: &ParsedFile,
        node: tree_sitter::Node<'_>,
        name: &str,
        exempt: &Option<Range<usize>>,
    ) -> bool {
        node.kind() == "identifier"
            && parsed.node_text(&node).trim() == name
            && !exempt
                .as_ref()
                .is_some_and(|span| span.start == node.start_byte() && span.end == node.end_byte())
    }

    fn descendant_matches(
        parsed: &ParsedFile,
        node: tree_sitter::Node<'_>,
        name: &str,
        exempt: &Option<Range<usize>>,
    ) -> bool {
        if matches_name(parsed, node, name, exempt) {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| descendant_matches(parsed, child, name, exempt));
        found
    }

    fn walk(
        parsed: &ParsedFile,
        node: tree_sitter::Node<'_>,
        name: &str,
        exempt: &Option<Range<usize>>,
    ) -> bool {
        let bound = match node.kind() {
            "parameter_declaration"
            | "variadic_parameter_declaration"
            | "var_spec"
            | "const_spec" => {
                let mut cursor = node.walk();
                let found = node
                    .children(&mut cursor)
                    .any(|child| matches_name(parsed, child, name, exempt));
                found
            }
            "type_spec" | "type_alias" => node.child_by_field_name("name").is_some_and(|child| {
                parsed.node_text(&child).trim() == name
                    && !exempt.as_ref().is_some_and(|span| {
                        span.start == child.start_byte() && span.end == child.end_byte()
                    })
            }),
            "short_var_declaration" | "assignment_statement" | "range_clause" | "inc_statement" => {
                node.child_by_field_name("left")
                    .or_else(|| node.named_child(0))
                    .is_some_and(|left| descendant_matches(parsed, left, name, exempt))
            }
            "type_switch_statement" | "communication_case" => parsed
                .node_text(&node)
                .split_once(":=")
                .is_some_and(|(left, _)| identifier_words(left).any(|word| word == name)),
            _ => false,
        };
        if bound {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| walk(parsed, child, name, exempt));
        found
    }

    walk(parsed, function, name, &exempt)
}

fn identifier_words(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(|character: char| character != '_' && !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
}

fn package_has_non_function_name(
    files: &BTreeMap<String, ParsedFile>,
    caller_file: &str,
    name: &str,
) -> bool {
    files.iter().any(|(path, parsed)| {
        parsed.language == Language::Go
            && !path.ends_with("_test.go")
            && dir_of(path) == dir_of(caller_file)
            && top_level_non_function_binds(parsed, name)
    })
}

fn top_level_non_function_binds(parsed: &ParsedFile, name: &str) -> bool {
    fn declaration_binds(parsed: &ParsedFile, node: tree_sitter::Node<'_>, name: &str) -> bool {
        if matches!(node.kind(), "var_spec" | "const_spec") {
            let mut cursor = node.walk();
            if node.children(&mut cursor).any(|child| {
                child.kind() == "identifier" && parsed.node_text(&child).trim() == name
            }) {
                return true;
            }
        }
        if matches!(node.kind(), "type_spec" | "type_alias")
            && node
                .child_by_field_name("name")
                .is_some_and(|child| parsed.node_text(&child).trim() == name)
        {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .any(|child| declaration_binds(parsed, child, name));
        found
    }

    let root = parsed.tree.root_node();
    let mut cursor = root.walk();
    let found = root
        .children(&mut cursor)
        .filter(|node| {
            matches!(
                node.kind(),
                "var_declaration" | "const_declaration" | "type_declaration"
            )
        })
        .any(|node| declaration_binds(parsed, node, name));
    found
}

fn contains_address_escape(parsed: &ParsedFile, node: tree_sitter::Node<'_>, name: &str) -> bool {
    if node.kind() == "unary_expression" && parsed.node_text(&node).trim_start().starts_with('&') {
        let mut cursor = node.walk();
        if node
            .children(&mut cursor)
            .any(|child| child.kind() == "identifier" && parsed.node_text(&child).trim() == name)
        {
            return true;
        }
    }
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|child| contains_address_escape(parsed, child, name));
    found
}

fn call_node_at<'a>(
    parsed: &'a ParsedFile,
    start_byte: usize,
    callee_name: &str,
) -> Option<tree_sitter::Node<'a>> {
    fn find<'a>(
        parsed: &'a ParsedFile,
        node: tree_sitter::Node<'a>,
        start_byte: usize,
        callee_name: &str,
    ) -> Option<tree_sitter::Node<'a>> {
        if node.kind() == "call_expression" && node.start_byte() == start_byte {
            let name = parsed.language.call_function_name(&node)?;
            if parsed.node_text(&name).trim() == callee_name {
                return Some(node);
            }
        }
        if start_byte < node.start_byte() || start_byte >= node.end_byte() {
            return None;
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .find_map(|child| find(parsed, child, start_byte, callee_name));
        found
    }
    find(parsed, parsed.tree.root_node(), start_byte, callee_name)
}

fn call_is_nested_in_callable(
    call: tree_sitter::Node<'_>,
    function: tree_sitter::Node<'_>,
) -> bool {
    let function_key = (
        function.start_byte(),
        function.end_byte(),
        function.kind_id(),
    );
    let mut current = call.parent();
    while let Some(node) = current {
        if (node.start_byte(), node.end_byte(), node.kind_id()) == function_key {
            return false;
        }
        if matches!(
            node.kind(),
            "func_literal" | "function_declaration" | "method_declaration"
        ) {
            return true;
        }
        current = node.parent();
    }
    true
}

fn packages_by_import_path(
    package_import_paths: &BTreeMap<String, String>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    clean_package_dirs: &BTreeSet<String>,
) -> BTreeMap<String, BTreeSet<(String, String)>> {
    let mut packages = BTreeMap::new();
    for (file, import_path) in package_import_paths {
        let Some(profile) = profiles.get(file) else {
            continue;
        };
        if import_path.is_empty()
            || profile.is_test_file
            || profile.package_clause.is_empty()
            || !clean_package_dirs.contains(dir_of(file))
        {
            continue;
        }
        packages
            .entry(import_path.clone())
            .or_insert_with(BTreeSet::new)
            .insert((dir_of(file).to_string(), profile.package_clause.clone()));
    }
    packages
}

fn strict_signature_imports(
    files: &BTreeMap<String, ParsedFile>,
    package_import_paths: &BTreeMap<String, String>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    packages_by_import_path: &BTreeMap<String, BTreeSet<(String, String)>>,
    clean_package_dirs: &BTreeSet<String>,
) -> BTreeMap<String, BTreeMap<String, String>> {
    files
        .iter()
        .filter(|(path, parsed)| {
            admissible_file(path, parsed, profiles) && clean_package_dirs.contains(dir_of(path))
        })
        .map(|(path, parsed)| {
            let mut imports = BTreeMap::new();
            let mut ambiguous = BTreeSet::new();
            if let Some(local_path) = package_import_paths
                .get(path)
                .filter(|path| !path.is_empty())
            {
                imports.insert(String::new(), local_path.clone());
            }
            let root = parsed.tree.root_node();
            let mut cursor = root.walk();
            for declaration in root.children(&mut cursor) {
                if declaration.kind() != "import_declaration" {
                    continue;
                }
                for (alias, import_path) in import_specs(parsed, declaration) {
                    match alias.as_deref() {
                        Some("_") => continue,
                        Some(".") => {
                            imports.insert(".".to_string(), String::new());
                            continue;
                        }
                        Some(alias) => insert_import(
                            &mut imports,
                            &mut ambiguous,
                            alias.to_string(),
                            import_path,
                        ),
                        None => {
                            let Some(packages) = packages_by_import_path.get(&import_path) else {
                                continue;
                            };
                            if packages.len() != 1 {
                                continue;
                            }
                            let (_, clause) = packages.iter().next().expect("one package");
                            insert_import(
                                &mut imports,
                                &mut ambiguous,
                                clause.clone(),
                                import_path,
                            );
                        }
                    }
                }
            }
            (path.clone(), imports)
        })
        .collect()
}

fn insert_import(
    imports: &mut BTreeMap<String, String>,
    ambiguous: &mut BTreeSet<String>,
    alias: String,
    path: String,
) {
    if ambiguous.contains(&alias) {
        return;
    }
    match imports.get(&alias) {
        None => {
            imports.insert(alias, path);
        }
        Some(existing) if existing == &path => {}
        Some(_) => {
            imports.remove(&alias);
            ambiguous.insert(alias);
        }
    }
}

fn import_specs(
    parsed: &ParsedFile,
    declaration: tree_sitter::Node<'_>,
) -> Vec<(Option<String>, String)> {
    fn collect<'a>(node: tree_sitter::Node<'a>, out: &mut Vec<tree_sitter::Node<'a>>) {
        if node.kind() == "import_spec" {
            out.push(node);
            return;
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            collect(child, out);
        }
    }

    let mut specs = Vec::new();
    collect(declaration, &mut specs);
    specs
        .into_iter()
        .filter_map(|spec| {
            let mut alias = None;
            let mut path = None;
            let mut cursor = spec.walk();
            for child in spec.children(&mut cursor) {
                match child.kind() {
                    "package_identifier" | "blank_identifier" | "dot" => {
                        alias = Some(parsed.node_text(&child).trim().to_string());
                    }
                    "interpreted_string_literal" | "raw_string_literal" => {
                        path = Some(
                            parsed
                                .node_text(&child)
                                .trim_matches(['\"', '`'])
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
            path.filter(|path| !path.is_empty())
                .map(|path| (alias, path))
        })
        .collect()
}

fn named_type_declarations(
    files: &BTreeMap<String, ParsedFile>,
    profiles: &BTreeMap<String, GoBuildProfile>,
    clean_package_dirs: &BTreeSet<String>,
) -> BTreeMap<GoOwnerIdentity, Vec<NamedTypeDeclaration>> {
    let mut declarations = BTreeMap::new();
    for (path, parsed) in files {
        if !admissible_file(path, parsed, profiles) || !clean_package_dirs.contains(dir_of(path)) {
            continue;
        }
        let profile = &profiles[path];
        let root = parsed.tree.root_node();
        let mut root_cursor = root.walk();
        for declaration in root.children(&mut root_cursor) {
            if declaration.kind() != "type_declaration" {
                continue;
            }
            let mut cursor = declaration.walk();
            for spec in declaration.children(&mut cursor) {
                if !matches!(spec.kind(), "type_spec" | "type_alias") {
                    continue;
                }
                let Some(name_node) = spec.child_by_field_name("name") else {
                    continue;
                };
                let name = parsed.node_text(&name_node).trim();
                if name.is_empty() {
                    continue;
                }
                declarations
                    .entry(GoOwnerIdentity {
                        package_dir: dir_of(path).to_string(),
                        package_clause: profile.package_clause.clone(),
                        name: name.to_string(),
                    })
                    .or_insert_with(Vec::new)
                    .push(NamedTypeDeclaration {
                        file: path.clone(),
                        start_byte: spec.start_byte(),
                        end_byte: spec.end_byte(),
                    });
            }
        }
    }
    declarations
}

#[derive(Debug, PartialEq, Eq)]
enum SignatureNameKind {
    Bare,
    Local,
    Qualified,
}

#[derive(Debug, PartialEq, Eq)]
enum SignatureToken {
    Symbol(char),
    Name {
        kind: SignatureNameKind,
        path: String,
        name: String,
    },
}

pub(crate) fn signatures_match_strict(left: &str, right: &str) -> bool {
    let (Some(left), Some(right)) = (signature_tokens(left), signature_tokens(right)) else {
        return false;
    };
    left.len() == right.len()
        && left
            .iter()
            .zip(&right)
            .all(|(left, right)| match (left, right) {
                (SignatureToken::Symbol(left), SignatureToken::Symbol(right)) => left == right,
                (
                    SignatureToken::Name {
                        kind: left_kind,
                        path: left_path,
                        name: left_name,
                    },
                    SignatureToken::Name {
                        kind: right_kind,
                        path: right_path,
                        name: right_name,
                    },
                ) => {
                    if left_name != right_name {
                        return false;
                    }
                    match (left_kind, right_kind) {
                        (SignatureNameKind::Bare, SignatureNameKind::Bare) => {
                            predeclared(left_name)
                        }
                        (SignatureNameKind::Bare, _) | (_, SignatureNameKind::Bare) => false,
                        _ => left_path == right_path,
                    }
                }
                _ => false,
            })
}

fn signature_tokens(signature: &str) -> Option<Vec<SignatureToken>> {
    let mut tokens = Vec::new();
    let mut rest = signature;
    while !rest.is_empty() {
        let first = rest.chars().next()?;
        if matches!(first, '@' | '~') {
            let after_marker = &rest[first.len_utf8()..];
            let separator = after_marker.find("::")?;
            let path = &after_marker[..separator];
            if path.is_empty() {
                return None;
            }
            let after_separator = &after_marker[separator + 2..];
            let name_len = identifier_prefix_len(after_separator)?;
            tokens.push(SignatureToken::Name {
                kind: if first == '@' {
                    SignatureNameKind::Qualified
                } else {
                    SignatureNameKind::Local
                },
                path: path.to_string(),
                name: after_separator[..name_len].to_string(),
            });
            rest = &after_separator[name_len..];
        } else if first == '_' || first.is_alphabetic() {
            let name_len = identifier_prefix_len(rest)?;
            tokens.push(SignatureToken::Name {
                kind: SignatureNameKind::Bare,
                path: String::new(),
                name: rest[..name_len].to_string(),
            });
            rest = &rest[name_len..];
        } else {
            tokens.push(SignatureToken::Symbol(first));
            rest = &rest[first.len_utf8()..];
        }
    }
    Some(tokens)
}

fn identifier_prefix_len(value: &str) -> Option<usize> {
    value
        .char_indices()
        .take_while(|(_, ch)| *ch == '_' || ch.is_alphanumeric())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
}

fn predeclared(name: &str) -> bool {
    matches!(
        name,
        "any"
            | "bool"
            | "comparable"
            | "complex64"
            | "complex128"
            | "error"
            | "float32"
            | "float64"
            | "int"
            | "int8"
            | "int16"
            | "int32"
            | "int64"
            | "string"
            | "uint"
            | "uint8"
            | "uint16"
            | "uint32"
            | "uint64"
            | "uintptr"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::call_graph::CallGraph;
    use crate::languages::Language;

    fn parsed_files(entries: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
        entries
            .iter()
            .map(|(path, source)| {
                (
                    (*path).to_string(),
                    ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
                )
            })
            .collect()
    }

    fn function<'a>(
        index: &'a BTreeMap<FunctionId, impl Sized>,
        file: &str,
        name: &str,
    ) -> &'a FunctionId {
        index
            .keys()
            .find(|id| id.file == file && id.name == name)
            .unwrap_or_else(|| panic!("missing {file}::{name}"))
    }

    fn graph_function<'a>(graph: &'a CallGraph, file: &str, name: &str) -> &'a FunctionId {
        graph.functions[name]
            .iter()
            .find(|id| id.file == file)
            .unwrap_or_else(|| panic!("missing {file}::{name}"))
    }

    fn source_site<'a>(graph: &'a CallGraph, caller: &FunctionId, callee: &str) -> &'a CallSite {
        graph.calls[caller]
            .iter()
            .find(|site| site.origin == CallSiteOrigin::Source && site.callee_name == callee)
            .unwrap_or_else(|| panic!("missing source site {} -> {callee}", caller.name))
    }

    #[test]
    fn literal_and_named_function_parameters_share_strict_signature() {
        let files = parsed_files(&[(
            "p/callback.go",
            r#"package p
type Handler func(int) error
func invoke(prefix string, cb Handler, direct func(int) error) {}
func safe(value int) error { return nil }
"#,
        )]);
        let paths = BTreeMap::from([(
            "p/callback.go".to_string(),
            "example.com/project/p".to_string(),
        )]);

        let indices = extract_go_callback_indices(&files, &paths);
        let invoke = function(&indices.parameters, "p/callback.go", "invoke");
        let safe = function(&indices.free_functions, "p/callback.go", "safe");
        let facts = &indices.parameters[invoke];

        assert_eq!(facts.len(), 2);
        assert_eq!((facts[0].slot, facts[0].name.as_str()), (1, "cb"));
        assert_eq!((facts[1].slot, facts[1].name.as_str()), (2, "direct"));
        assert_eq!(facts[0].signature, facts[1].signature);
        assert_eq!(facts[0].signature, indices.free_functions[safe]);
    }

    #[test]
    fn grouped_callback_parameters_preserve_slot_and_name() {
        let files = parsed_files(&[(
            "p/grouped.go",
            "package p\nfunc invoke(a, b func(string)) {}\n",
        )]);
        let paths = BTreeMap::from([(
            "p/grouped.go".to_string(),
            "example.com/project/p".to_string(),
        )]);

        let indices = extract_go_callback_indices(&files, &paths);
        let invoke = function(&indices.parameters, "p/grouped.go", "invoke");

        assert_eq!(
            indices.parameters[invoke]
                .iter()
                .map(|fact| (fact.slot, fact.name.as_str()))
                .collect::<Vec<_>>(),
            vec![(0, "a"), (1, "b")]
        );
    }

    #[test]
    fn in_repo_default_import_uses_proven_package_clause_not_path_basename() {
        let files = parsed_files(&[
            ("ext/value.go", "package safe\ntype Value struct{}\n"),
            (
                "caller/callback.go",
                r#"package caller
import "example.com/extpkg"
type Handler func(safe.Value)
func invoke(cb Handler) {}
func target(safe.Value) {}
"#,
            ),
        ]);
        let paths = BTreeMap::from([
            ("ext/value.go".to_string(), "example.com/extpkg".to_string()),
            (
                "caller/callback.go".to_string(),
                "example.com/project/caller".to_string(),
            ),
        ]);

        let indices = extract_go_callback_indices(&files, &paths);
        let invoke = function(&indices.parameters, "caller/callback.go", "invoke");
        let target = function(&indices.free_functions, "caller/callback.go", "target");

        assert_eq!(
            indices.parameters[invoke][0].signature,
            indices.free_functions[target]
        );
        assert!(indices.parameters[invoke][0]
            .signature
            .contains("@example.com/extpkg::Value"));
    }

    #[test]
    fn unknown_external_default_import_cannot_authorize_a_signature() {
        let files = parsed_files(&[(
            "p/callback.go",
            r#"package p
import "example.com/extpkg"
type Handler func(extpkg.Value)
func invoke(cb Handler) {}
func target(extpkg.Value) {}
"#,
        )]);
        let paths = BTreeMap::from([(
            "p/callback.go".to_string(),
            "example.com/project/p".to_string(),
        )]);

        let indices = extract_go_callback_indices(&files, &paths);

        assert!(indices.parameters.is_empty());
        assert!(!indices
            .free_functions
            .keys()
            .any(|function| function.name == "target"));
    }

    #[test]
    fn named_callable_alias_chain_resolves_cycle_safely() {
        let files = parsed_files(&[(
            "p/callback.go",
            r#"package p
type Handler func(int)
type HandlerAlias = Handler
type CycleA = CycleB
type CycleB = CycleA
func invoke(ok HandlerAlias, bad CycleA) {}
"#,
        )]);
        let paths = BTreeMap::from([(
            "p/callback.go".to_string(),
            "example.com/project/p".to_string(),
        )]);

        let indices = extract_go_callback_indices(&files, &paths);
        let invoke = function(&indices.parameters, "p/callback.go", "invoke");

        assert_eq!(indices.parameters[invoke].len(), 1);
        assert_eq!(indices.parameters[invoke][0].name, "ok");
    }

    #[test]
    fn strict_match_rejects_unproven_bare_names() {
        assert!(signatures_match_strict("(int)(error)", "(int)(error)"));
        assert!(signatures_match_strict(
            "(~example.com/p::Value)()",
            "(@example.com/p::Value)()"
        ));
        assert!(!signatures_match_strict("(Value)()", "(Value)()"));
        assert!(!signatures_match_strict(
            "(@example.com/a::Value)()",
            "(@example.com/b::Value)()"
        ));
    }

    #[test]
    fn bare_function_occurrence_and_direct_parameter_invocation_are_exact() {
        let files = parsed_files(&[(
            "p/callback.go",
            "package p\nfunc invoke(cb func()) { cb() }\nfunc safe() {}\nfunc caller() { invoke(safe) }\n",
        )]);
        let graph = CallGraph::build(&files);
        let caller = graph_function(&graph, "p/callback.go", "caller");
        let invoke = graph_function(&graph, "p/callback.go", "invoke");
        let inbound = source_site(&graph, caller, "invoke");
        let argument = files["p/callback.go"]
            .call_argument_texts_and_spans_at(inbound.start_byte, "invoke")
            .pop()
            .expect("one argument");
        let paths = BTreeMap::new();
        let proof = GoCallbackProofContext::new(
            &files,
            &graph.functions,
            &graph.method_owners,
            &graph.go_file_profiles,
            &graph.go_dot_import_files,
            &paths,
        );

        let target = proof
            .bare_free_function_at(caller, argument.1, &argument.0)
            .expect("exact bare function");
        let parameter = &graph.go_callback_parameters[invoke][0];
        let invocations = proof
            .direct_parameter_invocations(&graph.calls, invoke, parameter)
            .expect("direct parameter invocation");

        assert_eq!(target.name, "safe");
        assert_eq!(invocations.len(), 1);
        assert_eq!(invocations[0].callee_name, "cb");
    }

    #[test]
    fn whole_function_binding_census_covers_conservative_families() {
        let parsed = ParsedFile::parse(
            "p/census.go",
            r#"package p
func caller(param int) (result int) {
    var local int
    const fixed = 1
    type named int
    short := 1
    assigned := 0
    assigned = 1
    for ranged := range []int{} { _ = ranged }
    switch switched := any(param).(type) { default: _ = switched }
    ch := make(chan int)
    select { case received := <-ch: _ = received; default: }
    _ = func(nested int) { var inner int; _ = inner }
    _, _, _, _ = local, fixed, named(short), result
}
"#,
            Language::Go,
        )
        .expect("parse binding census");
        let function_id = FunctionId {
            file: "p/census.go".to_string(),
            name: "caller".to_string(),
            start_line: 2,
            end_line: 15,
        };
        let function = named_function_node(&parsed, &function_id).expect("caller node");

        for name in [
            "param", "result", "local", "fixed", "named", "short", "assigned", "ranged",
            "switched", "received", "nested", "inner",
        ] {
            assert!(
                function_has_binding_or_mutation(&parsed, function, name, None),
                "missing binding family for {name}"
            );
        }
        assert!(!function_has_binding_or_mutation(
            &parsed, function, "free", None
        ));
    }

    #[test]
    fn clean_package_poison_removes_callable_facts() {
        let files = parsed_files(&[
            (
                "p/callback.go",
                "package p\nfunc invoke(cb func()) { cb() }\n",
            ),
            ("p/other.go", "package other\nfunc other() {}\n"),
        ]);

        let indices = extract_go_callback_indices(&files, &BTreeMap::new());

        assert!(indices.parameters.is_empty());
        assert!(indices.free_functions.is_empty());
    }

    #[test]
    fn bare_function_proof_rejects_unknown_default_import() {
        let files = parsed_files(&[(
            "p/callback.go",
            r#"package p
import "example.com/external"
func safe() {}
func invoke(cb func()) {}
func caller() { invoke(safe) }
"#,
        )]);
        let graph = CallGraph::build(&files);
        let caller = graph_function(&graph, "p/callback.go", "caller");
        let inbound = source_site(&graph, caller, "invoke");
        let argument = files["p/callback.go"]
            .call_argument_texts_and_spans_at(inbound.start_byte, "invoke")
            .pop()
            .expect("one argument");
        let paths = BTreeMap::new();
        let proof = GoCallbackProofContext::new(
            &files,
            &graph.functions,
            &graph.method_owners,
            &graph.go_file_profiles,
            &graph.go_dot_import_files,
            &paths,
        );

        assert_eq!(
            proof.bare_free_function_at(caller, argument.1, &argument.0),
            Err(GoLevel3DropReason::StrictImportNameUnavailable)
        );
    }

    #[test]
    fn bare_function_proof_rejects_package_non_function_collision() {
        let files = parsed_files(&[
            (
                "p/callback.go",
                "package p\nfunc safe() {}\nfunc invoke(cb func()) {}\nfunc caller() { invoke(safe) }\n",
            ),
            ("p/collision.go", "package p\nvar safe = 1\n"),
        ]);
        let graph = CallGraph::build(&files);
        let caller = graph_function(&graph, "p/callback.go", "caller");
        let inbound = source_site(&graph, caller, "invoke");
        let argument = files["p/callback.go"]
            .call_argument_texts_and_spans_at(inbound.start_byte, "invoke")
            .pop()
            .expect("one argument");
        let paths = BTreeMap::new();
        let proof = GoCallbackProofContext::new(
            &files,
            &graph.functions,
            &graph.method_owners,
            &graph.go_file_profiles,
            &graph.go_dot_import_files,
            &paths,
        );

        assert_eq!(
            proof.bare_free_function_at(caller, argument.1, &argument.0),
            Err(GoLevel3DropReason::FileOrPackageNamespaceCollision)
        );
    }

    #[test]
    fn parameter_use_proof_rejects_mutation_address_escape_and_nested_only_call() {
        for (source, expected) in [
            (
                "package p\nfunc safe() {}\nfunc invoke(cb func()) { cb = safe; cb() }\n",
                GoLevel3DropReason::CallbackParameterShadowOrMutation,
            ),
            (
                "package p\nfunc mutate(*func()) {}\nfunc invoke(cb func()) { mutate(&cb); cb() }\n",
                GoLevel3DropReason::CallbackParameterAddressEscape,
            ),
            (
                "package p\nfunc invoke(cb func()) { func() { cb() }() }\n",
                GoLevel3DropReason::NestedCallableInvocation,
            ),
        ] {
            let files = parsed_files(&[("p/callback.go", source)]);
            let graph = CallGraph::build(&files);
            let invoke = graph_function(&graph, "p/callback.go", "invoke");
            let paths = BTreeMap::new();
            let proof = GoCallbackProofContext::new(
                &files,
                &graph.functions,
                &graph.method_owners,
                &graph.go_file_profiles,
                &graph.go_dot_import_files,
                &paths,
            );

            assert_eq!(
                proof.direct_parameter_invocations(
                    &graph.calls,
                    invoke,
                    &graph.go_callback_parameters[invoke][0],
                ),
                Err(expected)
            );
        }
    }
}
