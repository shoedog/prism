//! Slice L: resolution of a JS/TS call or JSX site whose callee is a `const` alias chain the
//! binder proved ends at an import local or at a member of a namespace import. The chain was
//! proven in one file, so the site resolves through the route a direct use of the terminal would
//! take and never falls to the same-file or repo-wide name rungs: no candidate means dropped.
use super::{
    demoted, exact, file_stem, CallGraph, CallSite, DropReason, FunctionId, ResolutionKind,
    ResolutionOutcome,
};
use crate::call_graph::ImportBinding;

impl CallGraph {
    /// `pkg.f()` / `N.f()`: the module-level functions named `name` that the module string
    /// `module_path` can denote by file stem or directory (methods excluded: `pkg.f()` names a
    /// module-level function, never a method on a class defined in that module, so an imported
    /// module with a same-named method cannot forge a false package-function edge). `None` when
    /// no function has that name at all.
    pub(super) fn module_qualified_matches(
        &self,
        module_path: &str,
        name: &str,
    ) -> Option<Vec<&FunctionId>> {
        let ids = self.functions.get(name)?;
        let module_last = module_path.rsplit('/').next().unwrap_or(module_path);
        let module_stem = module_last.rsplit('.').next_back().unwrap_or(module_last);
        Some(
            ids.iter()
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
                .collect(),
        )
    }

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
    /// exactly a qualified call `N.member()` (R3). The module string must name an in-repo
    /// function of that name by file stem or directory (an external package or a missing
    /// member has none and drops); the namespace export proof then decides, and without a proof
    /// the stem match grades exactly as it does for the qualified call.
    pub(super) fn js_ts_alias_member_outcome<'a>(
        &'a self,
        module_path: &str,
        member: &str,
        site: &CallSite,
    ) -> ResolutionOutcome<'a> {
        let Some(matched) = self.module_qualified_matches(module_path, member) else {
            return ResolutionOutcome::dropped(DropReason::UnknownName);
        };
        if matched.is_empty() {
            return ResolutionOutcome::dropped(DropReason::ImportExternal);
        }
        match self.js_ts_namespace_outcome(module_path, member, site, &matched) {
            Some(outcome) => outcome,
            None => ResolutionOutcome::hit(exact(matched, ResolutionKind::ImportQualified)),
        }
    }
}
