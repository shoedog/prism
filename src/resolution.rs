//! S3 call-resolution: confidence types, owner-key normalization, and the
//! R1-R7 resolution ladder (impl on CallGraph lives here to keep
//! call_graph.rs under the size cap).

use crate::call_graph::{CallGraph, CallSite, FunctionId};
use std::collections::BTreeSet;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ResolutionConfidence {
    Exact,
    NameOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    StaticLinkage,
    QualifiedOwner,
    SelfReceiver,
    ImportQualified,
    QualifierOwner,
    LocalDef,
    ImplicitThis,
    FreeSingle,
    FreeMulti,
    TypedParam,
    ConstructorLocal,
    TraitCha,
    R6SingleOwner,
    StemSingle,
    StemMulti,
    EmbeddedPromotion,
}

impl ResolutionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResolutionKind::StaticLinkage => "static_linkage",
            ResolutionKind::QualifiedOwner => "qualified_owner",
            ResolutionKind::SelfReceiver => "self_receiver",
            ResolutionKind::ImportQualified => "import_qualified",
            ResolutionKind::QualifierOwner => "qualifier_owner",
            ResolutionKind::LocalDef => "local_def",
            ResolutionKind::ImplicitThis => "implicit_this",
            ResolutionKind::FreeSingle => "free_single",
            ResolutionKind::FreeMulti => "free_multi",
            ResolutionKind::TypedParam => "typed_param",
            ResolutionKind::ConstructorLocal => "constructor_local",
            ResolutionKind::TraitCha => "trait_cha",
            ResolutionKind::R6SingleOwner => "r6_single_owner",
            ResolutionKind::StemSingle => "stem_single",
            ResolutionKind::StemMulti => "stem_multi",
            ResolutionKind::EmbeddedPromotion => "embedded_promotion",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallee<'a> {
    pub target: &'a FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

/// A resolved caller edge: who calls the seed, with what confidence, at which line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallEdge {
    pub caller: FunctionId,
    pub call_site_line: usize,
    pub confidence: ResolutionConfidence,
}

/// Normalize an owner type's source text to its bare index key:
/// strip refs/pointers, smart-pointer wrappers are NOT peeled here (that is
/// receiver peeling, `peel_type`), strip generic args, strip `dyn `/`impl `.
pub fn owner_key(text: &str) -> String {
    let t = text.trim();
    let t = t.trim_start_matches("&mut ").trim_start_matches('&');
    let t = t.trim_start_matches('*');
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    let t = t.split('<').next().unwrap_or(t);
    // C++ out-of-line `ns::Foo` declarator prefix -> last segment.
    let t = t.rsplit("::").next().unwrap_or(t);
    t.trim().to_string()
}

/// Closed-list syntactic peel (spec section 2.3): refs/pointers and std wrappers,
/// recursively; then generic args; then dyn/impl. NEVER Deref-semantic.
pub fn peel_type(text: &str) -> String {
    let mut t = text.trim();
    loop {
        let before = t;
        t = t
            .trim_start_matches("&mut ")
            .trim_start_matches('&')
            .trim_start_matches("*const ")
            .trim_start_matches("*mut ")
            .trim_start_matches('*')
            .trim();
        // Rust lifetime after a reference: `&'a Sender` / `&'a mut Sender`.
        // Drop a leading `'lifetime` token (and an optional following `mut`).
        if let Some(rest) = t.strip_prefix('\'') {
            let rest = rest.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
            t = rest.trim().trim_start_matches("mut ").trim();
        }
        for w in ["Box", "Arc", "Rc", "Pin"] {
            if let Some(inner) = t.strip_prefix(w) {
                if let Some(inner) = inner.trim().strip_prefix('<') {
                    if let Some(inner) = inner.strip_suffix('>') {
                        t = inner.trim();
                    }
                }
            }
        }
        if t == before {
            break;
        }
    }
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    t.split('<').next().unwrap_or(t).trim().to_string()
}

/// Which syntactic fact recovered a receiver type (stored on CallSite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReceiverRecovery {
    TypedParam,
    ConstructorLocal,
}

/// Why a call site resolved to nothing - the classification API that
/// collision warnings and call-stats telemetry consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// R6: method name defined on multiple owner types, receiver unknown.
    MultiOwnerCollision,
    /// P6-lite: receiver type recovered but (T, m) has no entry - provably
    /// external (Vec::truncate class) or wrong-name.
    ExternalReceiver,
    /// R3: qualifier is an import that narrows to no in-repo candidate.
    ImportExternal,
    /// Name not defined in-repo at all (ordinary unresolved call).
    UnknownName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionOutcome<'a> {
    pub resolved: Vec<ResolvedCallee<'a>>,
    /// Some(..) iff `resolved` is empty for a classified reason.
    pub drop: Option<DropReason>,
}

impl<'a> ResolutionOutcome<'a> {
    pub fn hit(resolved: Vec<ResolvedCallee<'a>>) -> Self {
        Self {
            resolved,
            drop: None,
        }
    }

    pub fn dropped(reason: DropReason) -> Self {
        Self {
            resolved: Vec::new(),
            drop: Some(reason),
        }
    }
}

fn exact<'a>(
    ids: impl IntoIterator<Item = &'a FunctionId>,
    kind: ResolutionKind,
) -> Vec<ResolvedCallee<'a>> {
    ids.into_iter()
        .map(|target| ResolvedCallee {
            target,
            confidence: ResolutionConfidence::Exact,
            kind,
        })
        .collect()
}

fn demoted<'a>(
    ids: impl IntoIterator<Item = &'a FunctionId>,
    kind: ResolutionKind,
) -> Vec<ResolvedCallee<'a>> {
    ids.into_iter()
        .map(|target| ResolvedCallee {
            target,
            confidence: ResolutionConfidence::NameOnly,
            kind,
        })
        .collect()
}

fn is_simple_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

impl CallGraph {
    /// Owner-index lookup that knows whether the key is a multi-impl trait key.
    fn owner_lookup(&self, owner: &str, name: &str) -> Option<Vec<ResolvedCallee<'_>>> {
        let mut resolved = self.owner_lookup_in_modules(owner, name, &[])?;
        if self
            .promoted_aliases
            .contains_key(&(owner.to_string(), name.to_string()))
        {
            for c in &mut resolved {
                c.kind = ResolutionKind::EmbeddedPromotion;
            }
        }
        Some(resolved)
    }

    /// Like `owner_lookup`, but for a qualified `mod::T::m` call the preceding
    /// module segments narrow candidates to files under that module — so
    /// `foo::Engine::start()` does NOT also resolve `bar::Engine::start()` (same
    /// bare owner key, different module) as Exact. Narrowing only applies when
    /// module segments are present AND there is >1 candidate; if it eliminates
    /// everything it is ignored (a wrong module hint must not drop a real edge).
    fn owner_lookup_in_modules(
        &self,
        owner: &str,
        name: &str,
        module_segs: &[&str],
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
        let pool: Vec<&FunctionId> = if !module_segs.is_empty() && ids.len() > 1 {
            let narrowed: Vec<&FunctionId> = ids
                .iter()
                .filter(|fid| {
                    module_segs
                        .iter()
                        .any(|seg| file_has_path_segment(&fid.file, seg))
                })
                .collect();
            if narrowed.is_empty() {
                ids.iter().collect()
            } else {
                narrowed
            }
        } else {
            ids.iter().collect()
        };
        let primary_owners: BTreeSet<&str> = pool
            .iter()
            .filter_map(|fid| self.method_owners.get(*fid).map(|s| s.as_str()))
            .collect();
        Some(if pool.len() > 1 && primary_owners.len() > 1 {
            demoted(pool, ResolutionKind::TraitCha)
        } else {
            exact(pool, ResolutionKind::QualifiedOwner)
        })
    }

    /// S3 R1-R7 ladder. This is the single full-resolution entry point for the
    /// new precision ladder. Legacy callers continue to use the old resolver
    /// until Tasks 9-11 migrate them.
    pub fn resolve_call_site_full(&self, site: &CallSite) -> ResolutionOutcome<'_> {
        let name = site.callee_name.as_str();
        let caller = &site.caller;

        // R1/R2/R7: Rust/C++ `::` path shapes where the raw name carries the path.
        if name.contains("::") {
            let mut segs: Vec<&str> = name.split("::").collect();
            let fn_name = segs.pop().unwrap_or(name);
            while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
                segs.remove(0);
            }

            if segs.as_slice() == ["self"] {
                if let Some(ids) = self.functions.get(fn_name) {
                    let local: Vec<&FunctionId> =
                        ids.iter().filter(|fid| fid.file == caller.file).collect();
                    if !local.is_empty() {
                        return ResolutionOutcome::hit(exact(local, ResolutionKind::LocalDef));
                    }
                }
                return ResolutionOutcome::dropped(DropReason::UnknownName);
            }

            if let Some(&head) = segs.last() {
                if head == "Self" {
                    if let Some(owner) = self.method_owners.get(caller) {
                        if let Some(mut resolved) = self.owner_lookup(owner, fn_name) {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = ResolutionKind::SelfReceiver;
                                }
                            }
                            return ResolutionOutcome::hit(resolved);
                        }
                    }
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }

                // R1: `T::m` / `mod::T::m` — the segments before the type narrow
                // candidates by module (so same-named types in different modules
                // don't all resolve Exact).
                let module_segs = &segs[..segs.len() - 1];
                if let Some(resolved) = self.owner_lookup_in_modules(head, fn_name, module_segs) {
                    return ResolutionOutcome::hit(resolved);
                }

                if let Some(ids) = self.functions.get(fn_name) {
                    let matched: Vec<&FunctionId> = ids
                        .iter()
                        .filter(|fid| file_stem(&fid.file) == head)
                        // Static-linkage preservation (matches the legacy
                        // resolver's final filter): a `static` C/C++ function
                        // in another file is not callable from here.
                        .filter(|fid| {
                            fid.file == caller.file
                                || !self
                                    .static_functions
                                    .contains(&(fid.file.clone(), fn_name.to_string()))
                        })
                        .collect();
                    return match matched.len() {
                        0 => ResolutionOutcome::dropped(DropReason::UnknownName),
                        1 => ResolutionOutcome::hit(exact(matched, ResolutionKind::StemSingle)),
                        _ => ResolutionOutcome::hit(demoted(matched, ResolutionKind::StemMulti)),
                    };
                }
            }

            return ResolutionOutcome::dropped(DropReason::UnknownName);
        }

        // Static linkage preservation from the legacy resolver.
        if site.qualifier.is_none()
            && self
                .static_functions
                .contains(&(caller.file.clone(), name.to_string()))
        {
            if let Some(ids) = self.functions.get(name) {
                let local: Vec<&FunctionId> =
                    ids.iter().filter(|fid| fid.file == caller.file).collect();
                if !local.is_empty() {
                    return ResolutionOutcome::hit(exact(local, ResolutionKind::StaticLinkage));
                }
            }
            return ResolutionOutcome::dropped(DropReason::UnknownName);
        }

        match site.qualifier.as_deref() {
            Some(q)
                if q == "self"
                    || q == "this"
                    || q == "cls"
                    || self.receiver_vars.get(caller).map(String::as_str) == Some(q) =>
            {
                if let Some(owner) = self.method_owners.get(caller) {
                    if let Some(mut resolved) = self.owner_lookup(owner, name) {
                        for callee in &mut resolved {
                            if callee.kind == ResolutionKind::QualifiedOwner {
                                callee.kind = ResolutionKind::SelfReceiver;
                            }
                        }
                        return ResolutionOutcome::hit(resolved);
                    }
                }
                ResolutionOutcome::dropped(DropReason::UnknownName)
            }
            Some(q) => {
                // R3: imported-module qualifier. If an import matches, the
                // narrowed set is final; empty means the call is external.
                if let Some(file_imports) = self.imports.get(&caller.file) {
                    if let Some(module_path) = file_imports.get(q) {
                        let ids = match self.functions.get(name) {
                            Some(v) => v.as_slice(),
                            None => return ResolutionOutcome::dropped(DropReason::UnknownName),
                        };
                        let module_last = module_path.rsplit('/').next().unwrap_or(module_path);
                        let module_stem = module_last.rsplit('.').last().unwrap_or(module_last);
                        let matched: Vec<&FunctionId> = ids
                            .iter()
                            // `pkg.f()` names a module-level function, never a method
                            // on a class defined in that module — exclude methods so
                            // an imported module with a same-named method can't forge
                            // a false package-function edge.
                            .filter(|fid| !self.method_owners.contains_key(*fid))
                            .filter(|fid| {
                                let stem_hit = file_stem(&fid.file) == module_stem;
                                let dir_hit = fid
                                    .file
                                    .rsplit('/')
                                    .nth(1)
                                    .map(|d| d == module_last)
                                    .unwrap_or(false);
                                stem_hit || dir_hit
                            })
                            .collect();
                        if matched.is_empty() {
                            return ResolutionOutcome::dropped(DropReason::ImportExternal);
                        }
                        return ResolutionOutcome::hit(exact(
                            matched,
                            ResolutionKind::ImportQualified,
                        ));
                    }
                }

                // R3b: qualifier text is itself an owner key.
                if is_simple_ident(q) {
                    if let Some(mut resolved) = self.owner_lookup(q, name) {
                        for callee in &mut resolved {
                            if callee.kind == ResolutionKind::QualifiedOwner {
                                callee.kind = ResolutionKind::QualifierOwner;
                            }
                        }
                        return ResolutionOutcome::hit(resolved);
                    }
                }

                // R6 step 1: P6-lite recovered receiver.
                if let Some(recv_ty) = site.receiver_type.as_deref() {
                    let recovered_kind = match site.receiver_recovery {
                        Some(ReceiverRecovery::ConstructorLocal) => {
                            ResolutionKind::ConstructorLocal
                        }
                        _ => ResolutionKind::TypedParam,
                    };
                    return match self.owner_lookup(recv_ty, name) {
                        Some(mut resolved) => {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = recovered_kind;
                                }
                                // Trait-CHA hits keep TraitCha (dyn Trait receivers).
                            }
                            ResolutionOutcome::hit(resolved)
                        }
                        None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                    };
                }

                // R6 residue (P2): method candidates only, never free fns.
                let method_ids: Vec<&FunctionId> = self
                    .functions
                    .get(name)
                    .map(|v| {
                        v.iter()
                            .filter(|fid| self.method_owners.contains_key(*fid))
                            .collect()
                    })
                    .unwrap_or_default();
                if method_ids.is_empty() {
                    // C has no methods: a receiver-syntax call `ptr->field()` /
                    // `s.field()` is a FUNCTION-POINTER field access, not a method
                    // call, so R6's "never bind to free functions" rule does not
                    // apply. Bind to a single same-named free function, demoted
                    // (the struct-callback heuristic membrane relies on; spec
                    // tiering: a single candidate is not provably-wrong → demote,
                    // not drop). Gated to C only — method-languages keep the
                    // method-only rule, since there `x.m()` is syntactically a
                    // method call.
                    if matches!(
                        crate::languages::Language::from_path(&caller.file),
                        Some(crate::languages::Language::C)
                    ) {
                        if let Some(ids) = self.functions.get(name) {
                            let free: Vec<&FunctionId> = ids
                                .iter()
                                .filter(|fid| {
                                    fid.file == caller.file
                                        || !self
                                            .static_functions
                                            .contains(&(fid.file.clone(), name.to_string()))
                                })
                                .collect();
                            if free.len() == 1 {
                                return ResolutionOutcome::hit(demoted(
                                    free,
                                    ResolutionKind::R6SingleOwner,
                                ));
                            }
                        }
                    }
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }

                // Caller's-own-file preference: exactly one owner defining it there.
                let local: Vec<&FunctionId> = method_ids
                    .iter()
                    .copied()
                    .filter(|f| f.file == caller.file)
                    .collect();
                let local_owners: BTreeSet<&str> = local
                    .iter()
                    .filter_map(|f| self.method_owners.get(*f).map(String::as_str))
                    .collect();
                if local_owners.len() == 1 {
                    return ResolutionOutcome::hit(demoted(local, ResolutionKind::R6SingleOwner));
                }

                let owners: BTreeSet<&str> = method_ids
                    .iter()
                    .filter_map(|f| self.method_owners.get(*f).map(String::as_str))
                    .collect();
                if owners.len() == 1 {
                    return ResolutionOutcome::hit(demoted(
                        method_ids,
                        ResolutionKind::R6SingleOwner,
                    ));
                }
                ResolutionOutcome::dropped(DropReason::MultiOwnerCollision)
            }
            None => {
                let ids = match self.functions.get(name) {
                    Some(v) => v,
                    None => return ResolutionOutcome::dropped(DropReason::UnknownName),
                };

                let free: Vec<&FunctionId> = ids
                    .iter()
                    .filter(|fid| !self.method_owners.contains_key(*fid))
                    .collect();

                // R4: local free definition wins alone.
                let local: Vec<&FunctionId> = free
                    .iter()
                    .copied()
                    .filter(|f| f.file == caller.file)
                    .collect();
                if !local.is_empty() {
                    return ResolutionOutcome::hit(exact(local, ResolutionKind::LocalDef));
                }

                // R4b: Java/C++ unqualified calls inside methods are implicit-this.
                if let Some(owner) = self.method_owners.get(caller) {
                    if caller.file.ends_with(".java")
                        || caller.file.ends_with(".cpp")
                        || caller.file.ends_with(".cc")
                        || caller.file.ends_with(".cxx")
                        || caller.file.ends_with(".hpp")
                        || caller.file.ends_with(".h")
                    {
                        if let Some(mut resolved) = self.owner_lookup(owner, name) {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = ResolutionKind::ImplicitThis;
                                }
                            }
                            return ResolutionOutcome::hit(resolved);
                        }
                    }
                }

                // R5: cross-file free functions only, preserving legacy static exclusion.
                let nonstatic: Vec<&FunctionId> = free
                    .into_iter()
                    .filter(|fid| {
                        !self
                            .static_functions
                            .contains(&(fid.file.clone(), name.to_string()))
                    })
                    .collect();
                match nonstatic.len() {
                    0 => ResolutionOutcome::dropped(DropReason::UnknownName),
                    1 => ResolutionOutcome::hit(exact(nonstatic, ResolutionKind::FreeSingle)),
                    _ => ResolutionOutcome::hit(demoted(nonstatic, ResolutionKind::FreeMulti)),
                }
            }
        }
    }

    pub fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>> {
        self.resolve_call_site_full(site).resolved
    }

    /// All call sites that resolve to `callee`, with caller, line, and confidence.
    /// The site-line source for slice witnesses; CPG edges carry no line.
    pub fn resolved_caller_edges(&self, callee: &FunctionId) -> Vec<ResolvedCallEdge> {
        let mut out = Vec::new();
        for sites in self.calls.values() {
            for site in sites {
                for r in self.resolve_call_site(site) {
                    if r.target == callee {
                        out.push(ResolvedCallEdge {
                            caller: site.caller.clone(),
                            call_site_line: site.line,
                            confidence: r.confidence,
                        });
                    }
                }
            }
        }
        out
    }
}

/// File stem matching `resolve_callees_qualified`'s idiom (`a.b.rs` -> `a`).
pub fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .last()
        .unwrap_or(path)
}

/// Whether `seg` (a module path segment) names a directory component of `path`
/// or its file stem — e.g. `worker` matches both `src/worker.rs` and
/// `src/worker/mod.rs`. Used to narrow `mod::T::m` candidates by module.
fn file_has_path_segment(path: &str, seg: &str) -> bool {
    file_stem(path) == seg || path.split('/').any(|c| c == seg)
}

#[cfg(test)]
mod embedding_kind_tests {
    use super::ResolutionKind;
    #[test]
    fn embedded_promotion_as_str() {
        assert_eq!(
            ResolutionKind::EmbeddedPromotion.as_str(),
            "embedded_promotion"
        );
    }
}
