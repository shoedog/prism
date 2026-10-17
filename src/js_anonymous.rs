//! S2a-prep: the single fail-closed Exact admission boundary for anonymous JS owners.
//! Source-backed proof facts persist with the content-validated graph; certificates are issued per resolution.
use crate::ast::{JsBindingCache, ParsedFile};
use crate::call_graph::{CallGraph, CallSite, FunctionId, ImportBinding, JsLocalBinding};
use crate::resolution::{ResolutionConfidence, ResolutionOutcome};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

type Span = (usize, usize);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Environment {
    pub escaped: bool,
    pub with: bool,
    pub eval: bool,
    pub nested_import: bool,
    pub uncertain: bool,
}
impl Environment {
    fn supported(&self) -> bool {
        !self.escaped && !self.with && !self.eval && !self.nested_import && !self.uncertain
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ImportEvidence {
    pub declaration: Span,
    pub binding: ImportBinding,
}
impl ImportEvidence {
    fn matches(&self, binding: &ImportBinding) -> bool {
        self.declaration.0 < self.declaration.1
            && self.binding.local == binding.local
            && self.binding.module_path == binding.module_path
            && self.binding.member == binding.member
            && self.binding.kind == binding.kind
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BindingEvidence {
    binding: JsLocalBinding,
    dependencies_supported: bool,
    imports: Vec<ImportEvidence>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct FileProofs {
    program_supported: bool,
    // Negative syntax evidence, independent of callable/import proof support.
    property_methods: BTreeSet<FunctionId>,
    owners: BTreeMap<FunctionId, bool>,
    sites: BTreeMap<Span, BindingEvidence>,
    returns: BTreeMap<FunctionId, (bool, Vec<ImportEvidence>)>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ProofIndex(BTreeMap<String, FileProofs>);

impl ProofIndex {
    pub(crate) fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        if !crate::anon_owner::EMIT_ANONYMOUS_OWNERS && !cfg!(test) {
            return Self::default();
        }
        let mut result = Self::default();
        for (file, parsed) in files {
            if !matches!(
                parsed.language,
                crate::languages::Language::JavaScript
                    | crate::languages::Language::TypeScript
                    | crate::languages::Language::Tsx
            ) {
                continue;
            }
            let file_supported = parsed.anonymous_file_environment().supported();
            let mut proof = FileProofs {
                program_supported: file_supported,
                ..Default::default()
            };
            let mut cache = JsBindingCache::for_sites();
            for owner in crate::anon_owner::candidate_nodes(parsed, true) {
                let Some(id) = crate::anon_owner::identity(parsed, &owner, file) else {
                    continue;
                };
                // Inferred property names identify owners, never lexical declarations.
                // A named function expression still binds its own explicit name.
                if owner.kind() == "method_definition"
                    || (owner.child_by_field_name("name").is_none()
                        && owner.parent().is_some_and(|p| p.kind() == "pair"))
                {
                    proof.property_methods.insert(id.clone());
                }
                let supported = file_supported && parsed.anonymous_evaluation_supported(owner);
                proof.owners.insert(id.clone(), supported);
                cache.proof_trace = Some(Vec::new());
                parsed.js_ts_return_facts(owner, &mut cache);
                let (safe, imports) = dependencies(parsed, &mut cache);
                proof.returns.insert(id, (supported && safe, imports));
            }
            let mut stack = vec![parsed.tree.root_node()];
            while let Some(node) = stack.pop() {
                if matches!(
                    node.kind(),
                    "call_expression" | "jsx_opening_element" | "jsx_self_closing_element"
                ) {
                    let callee = node.child_by_field_name(if node.kind() == "call_expression" {
                        "function"
                    } else {
                        "name"
                    });
                    if let Some(callee) = callee {
                        let name = if callee.kind() == "member_expression" {
                            callee
                                .child_by_field_name("property")
                                .map(|n| parsed.node_text(&n))
                        } else {
                            Some(parsed.node_text(&callee))
                        };
                        if let Some(name) = name {
                            cache.proof_trace = Some(Vec::new());
                            let binding = CallGraph::js_local_binding_at(
                                parsed,
                                node.start_byte(),
                                node.end_byte(),
                                name,
                                &mut cache,
                            );
                            let (safe, imports) = dependencies(parsed, &mut cache);
                            proof.sites.insert(
                                (node.start_byte(), node.end_byte()),
                                BindingEvidence {
                                    binding,
                                    dependencies_supported: file_supported && safe,
                                    imports,
                                },
                            );
                        }
                    }
                }
                let mut cursor = node.walk();
                stack.extend(node.named_children(&mut cursor));
            }
            result.0.insert(file.clone(), proof);
        }
        result
    }
    pub(crate) fn remove_files(&mut self, files: &BTreeSet<String>) {
        // Dependencies may cross file boundaries; invalidate all authority on any removal.
        if !files.is_empty() {
            self.0.retain(|file, _| !files.contains(file));
            for proof in self.0.values_mut() {
                let property_methods = std::mem::take(&mut proof.property_methods);
                *proof = FileProofs {
                    property_methods,
                    ..Default::default()
                };
            }
        }
    }
    pub(crate) fn merge(&mut self, other: Self) {
        self.0.extend(other.0);
    }
    pub(crate) fn is_property_method(&self, id: &FunctionId) -> bool {
        self.0
            .get(&id.file)
            .is_some_and(|p| p.property_methods.contains(id))
    }
    fn site(&self, site: &CallSite) -> Option<&BindingEvidence> {
        let file = self.0.get(&site.caller.file)?;
        if !file.owners.get(&site.caller).copied().unwrap_or(false) {
            return None;
        }
        file.sites
            .get(&(site.start_byte, site.end_byte))
            .filter(|p| p.dependencies_supported && p.binding == site.local_binding)
    }
}

fn dependencies<'a>(
    parsed: &'a ParsedFile,
    cache: &mut JsBindingCache<'a>,
) -> (bool, Vec<ImportEvidence>) {
    let trace = cache.proof_trace.take().unwrap_or_default();
    let supported = trace
        .iter()
        .all(|n| parsed.anonymous_evaluation_supported(*n));
    let imports = trace
        .into_iter()
        .filter_map(|n| parsed.anonymous_program_import(n, cache))
        .collect();
    (supported, imports)
}

/// Constructed by an approved producer after its target proof, never by the boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Certificate {
    site: CallSite,
    target: FunctionId,
    evidence: BindingEvidence,
    export: Option<(String, crate::js_exports::ResolvedJsExport)>,
}
impl Certificate {
    fn proves(&self, site: &CallSite, target: &FunctionId) -> bool {
        self.site == *site
            && self.target == *target
            && self.evidence.dependencies_supported
            && self
                .export
                .as_ref()
                .is_none_or(|(_, e)| export_matches(e, target))
    }
}
fn anonymous(site: &CallSite) -> bool {
    // Only the shared JS/TS minting helper can create this reserved owner identity.
    crate::anon_owner::is_anonymous_owner_name(&site.caller.name)
}
impl CallGraph {
    /// R4/R5 spelling fallback cannot prove a property name's lexical identity.
    /// Keep existing NameOnly pools intact; proven binding routes bypass this helper.
    pub(crate) fn bare_name_exact<'a>(
        &self,
        candidates: Vec<&'a FunctionId>,
        kind: crate::resolution::ResolutionKind,
    ) -> ResolutionOutcome<'a> {
        let resolved = candidates
            .into_iter()
            // `super()` is a constructor keyword, never a lexical identifier lookup.
            .filter(|id| id.name == "super" || !self.js_anonymous.is_property_method(id))
            .map(|target| crate::resolution::ResolvedCallee {
                target,
                confidence: ResolutionConfidence::Exact,
                kind,
            })
            .collect::<Vec<_>>();
        if resolved.is_empty() {
            ResolutionOutcome::dropped(crate::resolution::DropReason::LocalBindingUnproven)
        } else {
            ResolutionOutcome::hit(resolved)
        }
    }
    /// The only public full-resolution entry. Future candidate returns also pass this tail.
    pub fn resolve_call_site_full(&self, site: &CallSite) -> ResolutionOutcome<'_> {
        let outcome = self.resolve_call_site_candidates(site);
        self.admit_anonymous_exact(site, outcome)
    }
    pub(crate) fn admit_anonymous_exact<'a>(
        &self,
        site: &CallSite,
        mut outcome: ResolutionOutcome<'a>,
    ) -> ResolutionOutcome<'a> {
        if !anonymous(site) {
            return outcome;
        }
        let admitted = self.js_anonymous.site(site).is_some()
            && matches!(outcome.resolved.as_slice(), [one] if outcome.anonymous_certificate.as_ref()
                .is_some_and(|c| c.proves(site, one.target)));
        if !admitted {
            if outcome.resolved.len() > 1 {
                outcome.resolved.clear();
                outcome.drop = Some(crate::resolution::DropReason::LocalBindingUnproven);
            }
            for candidate in &mut outcome.resolved {
                candidate.confidence = ResolutionConfidence::NameOnly;
            }
            outcome.via_alias_route = false;
        }
        outcome
    }
    pub(crate) fn certify_anonymous_site<'a>(
        &self,
        site: &CallSite,
        mut outcome: ResolutionOutcome<'a>,
    ) -> ResolutionOutcome<'a> {
        if !anonymous(site) {
            return outcome;
        }
        if let (Some(proof), [one]) = (self.js_anonymous.site(site), outcome.resolved.as_slice()) {
            if matches!(
                proof.binding,
                JsLocalBinding::Callable(_)
                    | JsLocalBinding::AliasCallable(_)
                    | JsLocalBinding::AliasImport { .. }
                    | JsLocalBinding::AliasNamespaceMember { .. }
                    | JsLocalBinding::Returned { .. }
            ) && one.confidence == ResolutionConfidence::Exact
                && self.anonymous_target_supported(one.target)
                && binding_imports_proven(&proof.binding, &proof.imports)
                && self.proof_imports_match(&site.caller.file, &proof.imports)
            {
                outcome.anonymous_certificate = Some(Certificate {
                    site: site.clone(),
                    target: one.target.clone(),
                    evidence: proof.clone(),
                    export: None,
                });
            }
        }
        outcome
    }
    fn proof_imports_match(&self, file: &str, imports: &[ImportEvidence]) -> bool {
        imports.iter().all(|proof| {
            self.resolve_js_ts_member_module(&proof.binding.module_path, file)
                .is_some_and(|module| {
                    self.anonymous_module_supported(&module, &mut BTreeSet::new())
                })
                && self
                    .import_bindings
                    .get(file)
                    .is_some_and(|bindings| bindings.iter().any(|b| b.eligible && proof.matches(b)))
        })
    }
    pub(crate) fn anonymous_return_supported(
        &self,
        site: &CallSite,
        hook: &FunctionId,
        value: &crate::ast::JsFollow,
    ) -> bool {
        !anonymous(site)
            || self
                .js_anonymous
                .0
                .get(&hook.file)
                .and_then(|f| f.returns.get(hook))
                .is_some_and(|(safe, imports)| {
                    *safe
                        && terminal_imports_proven(value, imports)
                        && self.proof_imports_match(&hook.file, imports)
                })
    }
    pub(crate) fn certify_anonymous_import<'a>(
        &self,
        site: &CallSite,
        binding: &ImportBinding,
        mut outcome: ResolutionOutcome<'a>,
    ) -> ResolutionOutcome<'a> {
        if !anonymous(site) {
            return outcome;
        }
        if let (Some(proof), [one]) = (self.js_anonymous.site(site), outcome.resolved.as_slice()) {
            if proof.imports.iter().any(|p| p.matches(binding))
                && one.confidence == ResolutionConfidence::Exact
                && self.anonymous_target_supported(one.target)
                && self.proof_imports_match(&site.caller.file, &proof.imports)
            {
                let Some(export) = self.anonymous_import_export(site, binding, one.target) else {
                    return outcome;
                };
                outcome.anonymous_certificate = Some(Certificate {
                    site: site.clone(),
                    target: one.target.clone(),
                    evidence: proof.clone(),
                    export: Some(export),
                });
            }
        }
        outcome
    }
    /// Only called after the namespace export helper proved the actual member identity.
    pub(crate) fn certify_anonymous_namespace<'a>(
        &self,
        site: &CallSite,
        module: &str,
        export: &crate::js_exports::ResolvedJsExport,
        mut outcome: ResolutionOutcome<'a>,
    ) -> ResolutionOutcome<'a> {
        if !anonymous(site) {
            return outcome;
        }
        if let (Some(proof), [one]) = (self.js_anonymous.site(site), outcome.resolved.as_slice()) {
            if matches!(&proof.binding, JsLocalBinding::NamespaceImport { module_path } if module_path == module)
                && proof
                    .imports
                    .iter()
                    .any(|p| p.binding.member.is_none() && p.binding.module_path == module)
                && one.confidence == ResolutionConfidence::Exact
                && export_matches(export, one.target)
                && self.anonymous_target_supported(one.target)
                && self.proof_imports_match(&site.caller.file, &proof.imports)
            {
                outcome.anonymous_certificate = Some(Certificate {
                    site: site.clone(),
                    target: one.target.clone(),
                    evidence: proof.clone(),
                    export: Some((site.callee_name.clone(), export.clone())),
                });
            }
        }
        outcome
    }
}

fn terminal_imports_proven(value: &crate::ast::JsFollow, imports: &[ImportEvidence]) -> bool {
    match value {
        crate::ast::JsFollow::Callable(_) => true,
        crate::ast::JsFollow::ImportLocal(local) => imports
            .iter()
            .any(|p| p.binding.local == *local && p.binding.member.is_some()),
        crate::ast::JsFollow::NamespaceMember { module_path, .. } => imports
            .iter()
            .any(|p| p.binding.module_path == *module_path && p.binding.member.is_none()),
    }
}
fn binding_imports_proven(binding: &JsLocalBinding, imports: &[ImportEvidence]) -> bool {
    match binding {
        JsLocalBinding::AliasImport { local } => imports
            .iter()
            .any(|p| p.binding.local == *local && p.binding.member.is_some()),
        JsLocalBinding::AliasNamespaceMember { module_path, .. } => imports
            .iter()
            .any(|p| p.binding.module_path == *module_path && p.binding.member.is_none()),
        JsLocalBinding::Returned { hook, .. } => terminal_imports_proven(hook, imports),
        _ => true,
    }
}

fn export_matches(export: &crate::js_exports::ResolvedJsExport, target: &FunctionId) -> bool {
    export.file == target.file
        && export.local_name == target.name
        && export.span == Some((target.start_line, target.end_line))
}
impl CallGraph {
    fn anonymous_target_supported(&self, target: &FunctionId) -> bool {
        self.js_anonymous
            .0
            .get(&target.file)
            .and_then(|f| f.owners.get(target))
            .copied()
            .unwrap_or(false)
    }
    fn anonymous_import_export(
        &self,
        site: &CallSite,
        binding: &ImportBinding,
        target: &FunctionId,
    ) -> Option<(String, crate::js_exports::ResolvedJsExport)> {
        let file = self.resolve_js_ts_member_module(&binding.module_path, &site.caller.file)?;
        let exports = if binding.module_path.trim().starts_with('.') {
            &self.js_ts_resolved_exports
        } else {
            let (_, project) = self
                .js_ts_path_modules
                .get(&(site.caller.file.clone(), binding.module_path.clone()))?;
            self.js_ts_path_exports.get(project)?
        };
        let member = binding.member.as_ref()?;
        let export = exports.get(&file)?.get(member)?;
        export_matches(export, target).then(|| (member.clone(), export.clone()))
    }
    /// Re-export barrels contribute environments to the proof, including uniqueness branches.
    fn anonymous_module_supported(&self, file: &str, seen: &mut BTreeSet<String>) -> bool {
        if !seen.insert(file.to_string()) {
            return true;
        }
        if !self
            .js_anonymous
            .0
            .get(file)
            .is_some_and(|f| f.program_supported)
        {
            return false;
        }
        let Some(facts) = self.js_ts_exports.get(file) else {
            return false;
        };
        let reexports = facts.named.values().filter_map(|v| match v {
            crate::js_exports::JsExportTarget::ReExport { module_path, .. }
            | crate::js_exports::JsExportTarget::ImportForward { module_path, .. } => {
                Some(module_path)
            }
            _ => None,
        });
        facts.star_reexports.iter().chain(reexports).all(|module| {
            self.resolve_js_ts_member_module(module, file)
                .is_some_and(|next| self.anonymous_module_supported(&next, seen))
        })
    }
}

impl CallGraph {
    pub(crate) fn anonymous_local_outcome(
        &self,
        caller: &FunctionId,
        name: &str,
        terminal: &crate::ast::JsTerminal,
        site: &CallSite,
    ) -> ResolutionOutcome<'_> {
        self.certify_anonymous_site(
            site,
            self.js_ts_local_callable(caller, name, terminal, site),
        )
    }
    pub(crate) fn anonymous_import_outcome(
        &self,
        caller: &FunctionId,
        binding: &ImportBinding,
        site: &CallSite,
    ) -> ResolutionOutcome<'_> {
        self.certify_anonymous_import(
            site,
            binding,
            self.js_ts_import_member_outcome(caller, binding, site),
        )
    }
}
