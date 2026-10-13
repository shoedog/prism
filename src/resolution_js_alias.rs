//! Slice L: resolution of a JS/TS call or JSX site whose callee is a `const` alias chain the
//! binder proved ends at an import local or at a member of a namespace import. The chain was
//! proven in one file, so the site resolves through the route a direct use of the terminal would
//! take and never falls to the same-file or repo-wide name rungs: no candidate means dropped.
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
        match self.js_ts_import_member_candidates(caller, binding, member, site) {
            Err(drop) => ResolutionOutcome::dropped(drop),
            Ok(matched) => match matched.len() {
                0 => ResolutionOutcome::dropped(DropReason::UnknownName),
                1 => ResolutionOutcome::hit(exact(matched, ResolutionKind::ImportMember)),
                _ => ResolutionOutcome::hit(demoted(matched, ResolutionKind::ImportMember)),
            },
        }
    }

    /// A chain proven to end at the program-scope import local `local`: exactly a direct call
    /// to `local` through the member-import route, without the caller-locals shadow guard on
    /// the site's own spelling (the binder already proved what the alias names). A local that
    /// is not an eligible member import (a namespace or type-only import) has no candidate.
    pub(super) fn js_ts_alias_import_outcome<'a>(
        &'a self,
        caller: &FunctionId,
        local: &str,
        site: &CallSite,
    ) -> ResolutionOutcome<'a> {
        let binding = self.import_bindings.get(&caller.file).and_then(|bindings| {
            bindings.iter().find(|b| {
                b.local == local
                    && b.eligible
                    && matches!(b.kind, crate::call_graph::ImportBindingKind::MemberImport)
            })
        });
        match binding {
            Some(binding) => self.js_ts_import_member_outcome(caller, binding, site),
            None => ResolutionOutcome::dropped(DropReason::UnknownName),
        }
    }

    /// A chain proven to end at member `member` of the namespace import of `module_path`:
    /// "member `m` of `P`'s namespace" is the named export `m` of `P`, so it resolves the way
    /// `import { m } from P` does: the module file, then the per-export resolved facts, then the
    /// one function with that file, local name and span. The proof is per export (a module with
    /// an unproven export still proves its others) and follows re-export chains as named imports
    /// do. It never uses the qualified-call stem match (R3's legacy fallback), so a same-stem file
    /// that lacks the export cannot bind. Not relative (package, tsconfig-paths alias) or not a
    /// resolvable module: `ImportExternal`; anything short of one identity: `UnknownName`.
    pub(super) fn js_ts_alias_member_outcome<'a>(
        &'a self,
        module_path: &str,
        member: &str,
        site: &CallSite,
    ) -> ResolutionOutcome<'a> {
        let file = &site.caller.file;
        let Some(module_file) = module_path
            .trim()
            .starts_with('.')
            .then(|| self.resolve_js_ts_member_module(module_path, file))
            .flatten()
        else {
            return ResolutionOutcome::dropped(DropReason::ImportExternal);
        };
        let Some(resolved) = self
            .js_ts_resolved_exports
            .get(&module_file)
            .and_then(|exports| exports.get(member))
        else {
            return ResolutionOutcome::dropped(DropReason::UnknownName);
        };
        if resolved.wrapped && !site.jsx_element {
            return ResolutionOutcome::dropped(DropReason::WrappedExportNonJsx);
        }
        let ids: Vec<&FunctionId> =
            self.functions
                .get(&resolved.local_name)
                .map_or_else(Vec::new, |ids| {
                    ids.iter()
                        .filter(|fid| {
                            fid.file == resolved.file && !self.method_owners.contains_key(*fid)
                        })
                        .filter(|fid| {
                            resolved
                                .span
                                .is_none_or(|(s, e)| fid.start_line == s && fid.end_line == e)
                        })
                        .collect()
                });
        if ids.len() == 1 {
            ResolutionOutcome::hit(exact(ids, ResolutionKind::ImportMember))
        } else {
            ResolutionOutcome::dropped(DropReason::UnknownName)
        }
    }
}
