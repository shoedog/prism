//! Slice L: resolution of a JS/TS call or JSX site whose callee is a `const` alias chain the
//! binder followed to a callable, an import local or a member of a namespace import.
//!
//! One predicate governs the slice: "the alias route produced a proven hit for this site",
//! exactly one Exact target through the terminal's own route (`js_ts_alias_route_hit`). Only
//! three things depend on it: the resolver returns that hit for the site's `Source` record, the
//! textual function-pointer path stands down for the site, and the callers index counts the site
//! once for that exact target. Where the predicate is false the site behaves exactly as a site
//! bound `MayCall("alias")` does on main: the same rungs, records, grades and drop reasons.
use super::{
    demoted, exact, CallGraph, CallSite, DropReason, FunctionId, ResolutionKind, ResolutionOutcome,
};
use crate::call_graph::ImportBinding;

impl CallGraph {
    /// The JS/TS import-member route for one eligible member-import binding used at `site`:
    /// the typed export facts decide; zero candidates drop (an unshadowed import names that
    /// module's export, never a same-named global function), several demote.
    pub(super) fn js_ts_import_member_outcome<'a>(
        &'a self,
        caller: &FunctionId,
        binding: &ImportBinding,
        site: &CallSite,
    ) -> ResolutionOutcome<'a> {
        // S1b-2b: a JS/TS member import whose name is not matchable (no member) binds nothing.
        let Some(member) = binding.member.as_deref() else {
            return ResolutionOutcome::dropped(DropReason::UnknownName);
        };
        match self.js_ts_import_member_candidates(caller, binding, member, site, false) {
            Err(drop) => ResolutionOutcome::dropped(drop),
            Ok(matched) => match matched.len() {
                0 => ResolutionOutcome::dropped(DropReason::UnknownName),
                1 => ResolutionOutcome::hit(exact(matched, ResolutionKind::ImportMember)),
                _ => ResolutionOutcome::hit(demoted(matched, ResolutionKind::ImportMember)),
            },
        }
    }

    /// The alias route for a site whose binder-proven binding is a followed alias: `Some` only
    /// for a proven hit (exactly one Exact target through the terminal's own route); `None`
    /// means the route proved nothing and the caller continues as main does for an unfollowed
    /// alias. Also read by the stand-down, `nav call-stats` and the callers consumers.
    pub(crate) fn js_ts_alias_route<'a>(
        &'a self,
        caller: &FunctionId,
        name: &str,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'a>> {
        use crate::call_graph::JsLocalBinding;
        let outcome = match &site.local_binding {
            JsLocalBinding::AliasCallable(t) => {
                Some(self.js_ts_local_callable(caller, name, t, site))
            }
            JsLocalBinding::AliasImport { local } => {
                self.js_ts_alias_import_outcome(caller, local, site)
            }
            JsLocalBinding::AliasNamespaceMember {
                module_path,
                member,
            } => self.js_ts_alias_member_outcome(module_path, member, site),
            _ => None,
        }?;
        (outcome.drop.is_none()
            && matches!(
                outcome.resolved.as_slice(),
                [one] if one.confidence == crate::resolution::ResolutionConfidence::Exact
            ))
        .then_some(outcome)
    }

    /// The exact targets of a site the alias route proved (one `FunctionId` with its file, name
    /// and span), or `None` when the predicate is false. `#[doc(hidden)] pub` for the callers
    /// output, which must credit only the exact callee, never a same-named function.
    #[doc(hidden)]
    pub fn js_ts_alias_route_outcome(&self, site: &CallSite) -> Option<Vec<FunctionId>> {
        self.js_ts_alias_route(&site.caller, &site.callee_name, site)
            .map(|outcome| outcome.resolved.iter().map(|r| r.target.clone()).collect())
    }

    /// The one predicate of the slice: the site's binding is a followed alias and the alias
    /// route produced a proven hit (exactly one Exact target). `#[doc(hidden)] pub` so the
    /// structural rule test can read it.
    #[doc(hidden)]
    pub fn js_ts_alias_route_hit(&self, site: &CallSite) -> bool {
        self.js_ts_alias_route(&site.caller, &site.callee_name, site)
            .is_some()
    }

    /// A chain proven to end at the program-scope import local `local`: a direct call to
    /// `local` through the member-import route, without the caller-locals shadow guard on the
    /// site's own spelling (the binder already proved what the alias names). Proven only when
    /// the route yields exactly one source-backed target; otherwise `None`.
    fn js_ts_alias_import_outcome<'a>(
        &'a self,
        caller: &FunctionId,
        local: &str,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'a>> {
        let binding = self.import_bindings.get(&caller.file)?.iter().find(|b| {
            b.local == local
                && b.eligible
                && matches!(b.kind, crate::call_graph::ImportBindingKind::MemberImport)
        })?;
        let member = binding.member.as_deref()?;
        match self.js_ts_import_member_candidates(caller, binding, member, site, true) {
            Ok(ids) if ids.len() == 1 => Some(ResolutionOutcome::hit(exact(
                ids,
                ResolutionKind::ImportMember,
            ))),
            _ => None,
        }
    }

    /// A chain proven to end at member `member` of the namespace import of `module_path`:
    /// "member `m` of `P`'s namespace" is the named export `m` of `P`, so it resolves the way
    /// `import { m } from P` does: the relative module file, the per-export resolved facts, then
    /// the one function with that file, local name and span. The proof is per export and follows
    /// re-export chains as named imports do. It never uses the qualified-call stem match (R3's
    /// legacy fallback). A package or paths module, an unresolvable module, no entry, an entry
    /// without a span, a wrapped export at a non-JSX site, or anything short of one identity is
    /// not a proof: `None`.
    fn js_ts_alias_member_outcome<'a>(
        &'a self,
        module_path: &str,
        member: &str,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'a>> {
        let module_file = module_path
            .trim()
            .starts_with('.')
            .then(|| self.resolve_js_ts_member_module(module_path, &site.caller.file))
            .flatten()?;
        let resolved = self
            .js_ts_resolved_exports
            .get(&module_file)
            .and_then(|exports| exports.get(member))?;
        let (start, end) = resolved.span?;
        if resolved.wrapped && !site.jsx_element {
            return None;
        }
        let ids: Vec<&FunctionId> =
            self.functions
                .get(&resolved.local_name)
                .map_or_else(Vec::new, |ids| {
                    ids.iter()
                        .filter(|fid| {
                            fid.file == resolved.file
                                && !self.method_owners.contains_key(*fid)
                                && fid.start_line == start
                                && fid.end_line == end
                        })
                        .collect()
                });
        (ids.len() == 1).then(|| ResolutionOutcome::hit(exact(ids, ResolutionKind::ImportMember)))
    }
}
