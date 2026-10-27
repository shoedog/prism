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
            // Rust extraction retains a nested item's call under its outer
            // method too. That copy cannot borrow the outer method's Self.
            if self.rust_self_captured_from_nested_item(site) {
                return None;
            }
            let owner = self.method_owners.get(&site.caller)?;
            let facts = self.method_facts.get(&site.caller)?;
            // A trait default's Self ranges over implementors, even when only
            // one implementation happens to be indexed in this repository.
            if matches!(&facts.kind, MethodKind::Trait(tr) if tr == owner) {
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

    fn rust_self_captured_from_nested_item(&self, site: &CallSite) -> bool {
        self.calls.iter().any(|(other, calls)| {
            other != &site.caller
                && other.file == site.caller.file
                && other.start_line >= site.caller.start_line
                && other.end_line <= site.caller.end_line
                && calls.iter().any(|inner| {
                    inner.start_byte == site.start_byte
                        && inner.end_byte == site.end_byte
                        && inner.callee_name == site.callee_name
                })
        })
    }
}
