//! Authoritative Rust call routing and the concrete `Self::` scope proof.
use super::*;

impl CallGraph {
    /// `Self` has no lexical module binding. Let its dedicated rung run.
    pub(super) fn rust_authoritative_resolution(
        &self,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'_>> {
        let name = site.callee_name.as_str();
        if crate::languages::Language::from_path(&site.caller.file)
            != Some(crate::languages::Language::Rust)
            || name
                .strip_prefix("Self::")
                .is_some_and(|tail| !tail.contains("::"))
            || !(name.contains("::") || site.qualifier.is_none())
        {
            return None;
        }
        let graph = self.scope_graph.as_ref()?;
        let (file, from) = rust_authoritative_scope(graph, site)?;
        Some(self.rust_authoritative_nonself(graph, site, file, from))
    }

    fn rust_authoritative_nonself(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        file: FileId,
        from: ScopeId,
    ) -> ResolutionOutcome<'_> {
        let name = site.callee_name.as_str();
        let owner_method = if name.contains("::") {
            owner_method_key(name)
        } else {
            None
        };
        let has_bare_pool = owner_method.as_ref().is_some_and(|(owner, method)| {
            self.methods.contains_key(&(owner.clone(), method.clone()))
        });

        if has_bare_pool
            && name.contains("::")
            && self.rust_graph_qualified_target_is_free_fn(graph, site, file, from)
        {
            return match self.rust_scope_graph_resolution(graph, site, file, from) {
                Some(resolved) => ResolutionOutcome::hit(resolved),
                None => ResolutionOutcome::dropped(DropReason::UnknownName),
            };
        }

        if has_bare_pool {
            if let Some(resolved) = self.rust_scope_prune_owner(graph, site, file, from, name) {
                return ResolutionOutcome::hit(resolved);
            }

            let (owner, method) = owner_method.as_ref().expect("has_bare_pool");
            let segs: Vec<&str> = name.split("::").collect();
            let mut prefix: Vec<&str> = segs[..segs.len() - 1].to_vec();
            while matches!(prefix.first(), Some(&"crate") | Some(&"super")) {
                prefix.remove(0);
            }
            let module_segs = &prefix[..prefix.len().saturating_sub(1)];
            if let Some(resolved) = self.owner_lookup_in_modules(owner, method, module_segs) {
                // Authoritative scope site: a distinct-owner (trait-CHA)
                // pool must DECLINE so the CHA fan-out emits Exact — the
                // legacy TraitCha NameOnly edge is intentionally disabled
                // at authoritative scope sites (matches the pre-existing
                // `Render::draw(x)` behavior). Only the same-owner
                // collision floor (#120 QualifiedOwner) is a valid
                // fail-open here.
                if !resolved.iter().any(|c| c.kind == ResolutionKind::TraitCha) {
                    return ResolutionOutcome::hit(resolved);
                }
            }
            return ResolutionOutcome::dropped(DropReason::UnknownName);
        }

        if let Some(resolved) = self.rust_scope_graph_resolution(graph, site, file, from) {
            return ResolutionOutcome::hit(resolved);
        }
        ResolutionOutcome::dropped(DropReason::UnknownName)
    }

    /// Only newly reachable authoritative sites use this proof. Legacy/orphan
    /// sites keep their existing owner-lookup outcome.
    pub(super) fn rust_self_scope_resolution(
        &self,
        site: &CallSite,
        name: &str,
    ) -> Option<ResolutionOutcome<'_>> {
        if crate::languages::Language::from_path(&site.caller.file)
            != Some(crate::languages::Language::Rust)
        {
            return None;
        }
        let graph = self.scope_graph.as_ref()?;
        rust_authoritative_scope(graph, site)?;
        let resolve = || {
            let owner = self.method_owners.get(&site.caller)?;
            let _facts = self.method_facts.get(&site.caller)?;
            let caller = self.rust_self_proofs.methods.get(&site.caller)?;
            if !caller.module_level {
                return None;
            }
            if !caller
                .self_spans
                .contains(&(site.start_byte, site.end_byte))
            {
                return None;
            }
            // Empty impls do not put inherited defaults in the concrete pool.
            if self.rust_self_proofs.default_names.contains(name) {
                return None;
            }
            // Caller membership supplies the impl's declared type identity;
            // spelling alone could select a different module's same-named type.
            let scopes: BTreeSet<_> = self
                .methods_by_scope
                .iter()
                .filter(|((_, method), ids)| {
                    method == &site.caller.name && ids.contains(&site.caller)
                })
                .map(|((scope, _), _)| *scope)
                .filter(|scope| {
                    graph_owner_name_for_scope(graph, *scope).as_deref() == Some(owner.as_str())
                })
                .collect();
            if scopes.len() != 1 {
                return None;
            }
            let scope = *scopes.first()?;
            let ids = self.methods_by_scope.get(&(scope, name.to_string()))?;
            if ids.len() == 1 {
                let target = self.rust_self_proofs.methods.get(&ids[0])?;
                if !caller.admits(target) {
                    return None;
                }
                Some(exact(ids, ResolutionKind::SelfReceiver))
            } else {
                // Type ownership alone cannot distinguish trait overloads.
                None
            }
        };
        Some(match resolve() {
            Some(resolved) => ResolutionOutcome::hit(resolved),
            None => ResolutionOutcome::dropped(DropReason::UnknownName),
        })
    }
}

/// Persisted syntax evidence, replaced from the full parsed snapshot on refresh.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct ProofIndex {
    methods: BTreeMap<FunctionId, ImplProof>,
    default_names: BTreeSet<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ImplProof {
    file: String,
    span: (usize, usize),
    module_level: bool,
    inherent: bool,
    module: Vec<usize>,
    plain_self_path: bool,
    self_type: Vec<String>,
    generics: Vec<String>,
    where_clause: Vec<String>,
    self_spans: BTreeSet<(usize, usize)>,
}

impl ImplProof {
    fn same_module(&self, target: &Self) -> bool {
        self.file == target.file && self.module == target.module
    }

    fn admits(&self, target: &Self) -> bool {
        if self.file == target.file && self.span == target.span {
            return true;
        }
        if !target.module_level || !target.inherent {
            return false;
        }
        if self.self_type != target.self_type {
            return false;
        }
        let constrained = !self.generics.is_empty()
            || !target.generics.is_empty()
            || !self.where_clause.is_empty()
            || !target.where_clause.is_empty();
        // Generic arguments and bounds can bind different names in other modules.
        let needs_context = constrained || !self.plain_self_path || !target.plain_self_path;
        if needs_context && !self.same_module(target) {
            return false;
        }
        !constrained
            || (self.generics == target.generics && self.where_clause == target.where_clause)
    }
}

impl ProofIndex {
    pub(crate) fn build(files: &BTreeMap<String, crate::ast::ParsedFile>) -> Self {
        let mut result = Self::default();
        for (file, parsed) in files {
            if parsed.language != crate::languages::Language::Rust {
                continue;
            }
            for method in parsed.all_functions() {
                let Some(name) = parsed.language.function_name(&method) else {
                    continue;
                };
                let Some(item) = parsed.language.rust_enclosing_method_item(&method) else {
                    continue;
                };
                if item.kind() == "trait_item" {
                    if method.child_by_field_name("body").is_some() {
                        result
                            .default_names
                            .insert(parsed.node_text(&name).to_string());
                    }
                    continue;
                }
                // Only direct impl members supply an impl identity.
                if method.parent().and_then(|p| p.parent()) != Some(item) {
                    continue;
                }
                let Some(ty) = item.child_by_field_name("type") else {
                    continue;
                };
                let mut ancestor = item.parent();
                let mut module_level = true;
                let mut module = Vec::new();
                while let Some(node) = ancestor {
                    if !matches!(node.kind(), "source_file" | "declaration_list" | "mod_item") {
                        module_level = false;
                    }
                    if node.kind() == "mod_item" {
                        module.push(node.start_byte());
                    }
                    ancestor = node.parent();
                }
                let normalize = |node: tree_sitter::Node<'_>| syntax_tokens(parsed, node);
                let mut cursor = item.walk();
                let where_clause = item
                    .named_children(&mut cursor)
                    .find(|n| n.kind() == "where_clause")
                    .map(normalize)
                    .unwrap_or_default();
                let (start_line, end_line) = parsed.node_line_range(&method);
                let fid = FunctionId {
                    file: file.clone(),
                    name: parsed.node_text(&name).to_string(),
                    start_line,
                    end_line,
                };
                let proof = ImplProof {
                    file: file.clone(),
                    span: (item.start_byte(), item.end_byte()),
                    module_level,
                    inherent: item.child_by_field_name("trait").is_none(),
                    module,
                    plain_self_path: plain_nominal_path(ty),
                    self_type: normalize(ty),
                    generics: item
                        .child_by_field_name("type_parameters")
                        .map(normalize)
                        .unwrap_or_default(),
                    where_clause,
                    self_spans: self_call_spans(method),
                };
                // A line-based identity shared by two syntax nodes proves neither.
                if result.methods.insert(fid.clone(), proof).is_some() {
                    result.methods.get_mut(&fid).unwrap().module_level = false;
                }
            }
        }
        result
    }
}

// Preserve token boundaries and literal contents; formatting whitespace is absent.
fn syntax_tokens(parsed: &crate::ast::ParsedFile, node: tree_sitter::Node<'_>) -> Vec<String> {
    fn visit(
        parsed: &crate::ast::ParsedFile,
        node: tree_sitter::Node<'_>,
        tokens: &mut Vec<String>,
    ) {
        if node.child_count() == 0 {
            tokens.push(parsed.node_text(&node).to_string());
        } else {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                visit(parsed, child, tokens);
            }
        }
    }
    let mut tokens = Vec::new();
    visit(parsed, node, &mut tokens);
    tokens
}

fn plain_nominal_path(node: tree_sitter::Node<'_>) -> bool {
    match node.kind() {
        "type_identifier" | "identifier" | "crate" | "self" | "super" | "::" => {
            node.child_count() == 0
        }
        "scoped_type_identifier" | "scoped_identifier" => {
            let mut cursor = node.walk();
            let plain = node.children(&mut cursor).all(plain_nominal_path);
            plain
        }
        _ => false,
    }
}

fn self_call_spans(method: tree_sitter::Node<'_>) -> BTreeSet<(usize, usize)> {
    fn visit(node: tree_sitter::Node<'_>, spans: &mut BTreeSet<(usize, usize)>) {
        // Named items start a new Self binding (or have none). Expression
        // closures, async blocks and inline const blocks inherit this method's.
        if matches!(
            node.kind(),
            "function_item"
                | "const_item"
                | "static_item"
                | "struct_item"
                | "enum_item"
                | "union_item"
                | "impl_item"
                | "mod_item"
                | "trait_item"
                | "type_item"
        ) {
            return;
        }
        if node.kind() == "call_expression" {
            spans.insert((node.start_byte(), node.end_byte()));
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            visit(child, spans);
        }
    }
    let mut spans = BTreeSet::new();
    let mut cursor = method.walk();
    for child in method.named_children(&mut cursor) {
        visit(child, &mut spans);
    }
    spans
}
