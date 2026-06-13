//! S3 call-resolution: confidence types, owner-key normalization, and the
//! R1-R7 resolution ladder (impl on CallGraph lives here to keep
//! call_graph.rs under the size cap).

use crate::call_graph::{CallGraph, CallSite, FunctionId};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallee<'a> {
    pub target: &'a FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
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

impl CallGraph {
    /// Owner-index lookup that knows whether the key is a multi-impl trait key.
    fn owner_lookup(&self, owner: &str, name: &str) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
        let primary_owners: BTreeSet<&str> = ids
            .iter()
            .filter_map(|fid| self.method_owners.get(fid).map(|s| s.as_str()))
            .collect();
        Some(if ids.len() > 1 && primary_owners.len() > 1 {
            demoted(ids.iter(), ResolutionKind::TraitCha)
        } else {
            exact(ids.iter(), ResolutionKind::QualifiedOwner)
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

                if let Some(resolved) = self.owner_lookup(head, fn_name) {
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
        if self
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
            Some(_) | None => ResolutionOutcome::dropped(DropReason::UnknownName),
        }
    }

    pub fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>> {
        self.resolve_call_site_full(site).resolved
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
