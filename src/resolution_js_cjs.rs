//! #598: resolution of a call whose callee the binder named an export of an in-repo module,
//! reached by a top-level relative `require` (`m(..)`, `m.f(..)`, `require('./x').f(..)`,
//! `f(..)` with `f = require('./x').f`).
//!
//! One proof decides: the required file is the one relative module the call graph resolves, the
//! producer's CommonJS export facts (`JsExportFacts::cjs_callables` and `cjs_module_callable`,
//! recorded only behind the export-object barrier and the terminal proof) hold a callable for
//! the member (`default`-less: `cjs_module_callable` for the module value), the name is present
//! and unconflicted in the file's raw export claims, and exactly one non-method function in that
//! file has the recorded name and line span. Anything short of that proves nothing (`None`) and
//! the site continues as it did before this route existed. A caller that is itself an anonymous
//! or module-level owner is not admitted: its Exact needs the anonymous certificate, which this
//! route does not issue.
use super::{exact, CallGraph, CallSite, FunctionId, ResolutionKind, ResolutionOutcome};
use crate::call_graph::JsLocalBinding;

impl CallGraph {
    /// The route for a site bound `RequireModule`, as a followed-binding hit (`via_alias_route`).
    /// `None` for every other site and wherever the proof fails.
    pub(super) fn js_ts_cjs_route(&self, site: &CallSite) -> Option<ResolutionOutcome<'_>> {
        if !matches!(site.local_binding, JsLocalBinding::RequireModule { .. }) {
            return None;
        }
        let mut outcome = self.js_ts_alias_route(&site.caller, &site.callee_name, site)?;
        outcome.via_alias_route = true;
        Some(outcome)
    }

    /// The proof itself; see the module comment. Read by the alias route's `RequireModule` arm.
    pub(super) fn js_ts_cjs_module_outcome(
        &self,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'_>> {
        let JsLocalBinding::RequireModule {
            module_path,
            member,
        } = &site.local_binding
        else {
            return None;
        };
        if crate::anon_owner::is_anonymous_owner_name(&site.caller.name) {
            return None;
        }
        let file = self.resolve_js_ts_member_module(module_path, &site.caller.file)?;
        let facts = self.js_ts_exports.get(&file)?;
        let (claimed, callable) = match member {
            Some(member) => (member.as_str(), facts.cjs_callables.get(member)?),
            None => ("default", facts.cjs_module_callable.as_ref()?),
        };
        if facts.conflicted.contains(claimed) || !facts.named.contains_key(claimed) {
            return None;
        }
        // The producer recorded the span only for the file's one function of that name
        // (`js_ts_cjs_callable_span`), so the span and count checks below hold by construction;
        // they stay as defence in depth. A mutant of either is equivalent (it survives
        // `same_name_functions_in_the_producer_file`, which pins the producer's refusal).
        let ids: Vec<&FunctionId> = self
            .functions
            .get(&callable.local)?
            .iter()
            .filter(|id| {
                id.file == file
                    && !self.method_owners.contains_key(*id)
                    && id.start_line == callable.start_line
                    && id.end_line == callable.end_line
            })
            .collect();
        (ids.len() == 1).then(|| ResolutionOutcome::hit(exact(ids, ResolutionKind::ImportMember)))
    }
}
