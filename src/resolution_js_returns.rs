//! Slice V: resolution of a JS/TS site whose callee is a `const` binding taking its value from
//! a call to one in-repo function `H` (`const { save } = useEditor()`). The chain is: `H`
//! resolves, by the alias route's terminals, to exactly one function; that function's
//! returned-value fact gives a callable value in its own file at the binding's path; the value
//! resolves the way slice L's terminals do, as seen from that file. Every step must prove
//! exactly one source-backed target, otherwise the route proves nothing (`None`) and the site
//! continues as it did before this route existed. The result keeps the kind of the route that
//! produced it (`LocalDef` or `ImportMember`).
use super::{CallGraph, CallSite, FunctionId, ResolutionConfidence, ResolutionOutcome};
use crate::ast::JsFollow;
use crate::call_graph::JsLocalBinding;
use crate::js_returns::JsReturnPath;

impl CallGraph {
    /// The returned-value route for `site`, whose binding is `Returned { hook, path, .. }`.
    pub(super) fn js_ts_returned_route<'a>(
        &'a self,
        hook: &JsFollow,
        path: &JsReturnPath,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'a>> {
        // `H` is a function, never a wrapped component, so it is looked up as a non-JSX use.
        let hook_fn = self.js_ts_terminal_function(&site.caller, hook, site, false)?;
        let value = self.js_ts_return_facts.get(hook_fn)?.get(path)?;
        let outcome = self.js_ts_terminal_outcome(hook_fn, value, site, site.jsx_element)?;
        Some(outcome)
    }

    /// The one function a followed terminal names, as seen from `from`'s file.
    fn js_ts_terminal_function<'a>(
        &'a self,
        from: &FunctionId,
        terminal: &JsFollow,
        site: &CallSite,
        jsx_element: bool,
    ) -> Option<&'a FunctionId> {
        let outcome = self.js_ts_terminal_outcome(from, terminal, site, jsx_element)?;
        Some(outcome.resolved[0].target)
    }

    /// Resolves `terminal` as seen from `from`'s file: slice L's route for a callable, an
    /// import local or a namespace member, run on a copy of `site` placed in that file.
    /// Proven only by exactly one Exact target.
    fn js_ts_terminal_outcome<'a>(
        &'a self,
        from: &FunctionId,
        terminal: &JsFollow,
        site: &CallSite,
        jsx_element: bool,
    ) -> Option<ResolutionOutcome<'a>> {
        let (name, local_binding) = match terminal {
            JsFollow::Callable(t) => (t.local.clone(), JsLocalBinding::AliasCallable(t.clone())),
            JsFollow::ImportLocal(local) => (
                local.clone(),
                JsLocalBinding::AliasImport {
                    local: local.clone(),
                },
            ),
            JsFollow::NamespaceMember {
                module_path,
                member,
            } => (
                member.clone(),
                JsLocalBinding::AliasNamespaceMember {
                    module_path: module_path.clone(),
                    member: member.clone(),
                },
            ),
        };
        let probe = CallSite {
            caller: from.clone(),
            callee_name: name.clone(),
            jsx_element,
            local_binding,
            ..site.clone()
        };
        let outcome = self.js_ts_alias_route(from, &name, &probe)?;
        (outcome.resolved.len() == 1
            && outcome.resolved[0].confidence == ResolutionConfidence::Exact)
            .then_some(outcome)
    }
}
