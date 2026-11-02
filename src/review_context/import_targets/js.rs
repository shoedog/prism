//! JS/TS attribution: a call's root name (the callee, or the leftmost name of
//! its member chain) must still be the program-scope import at the site; a
//! relative specifier takes the precedence winner of the shared candidate
//! list; a bare specifier goes to the tsconfig alias lookup (`js_alias`) or is
//! a package. The specifier comes from the program-scope import index
//! (`js_ts_program_imports`), never from a nested `require` of the same name.
use super::js_alias::Alias;
use super::{
    bound, is_same_file_function, root_identifier, unresolvable, Attribution, Basis, ImportTargets,
    InRepo, Why, MAX_AMBIGUOUS,
};
use crate::ast::{JsRootScope, ParsedFile};
use crate::call_graph::{
    is_js_ts_relative_specifier, js_ts_relative_module_candidates, CallSite, ImportBindingKind,
};

impl ImportTargets<'_> {
    pub(super) fn js_attribution(
        &mut self,
        file: &str,
        parsed: &ParsedFile,
        call: &CallSite,
    ) -> Attribution {
        let key = match call.qualifier.as_deref() {
            Some(qualifier) => match root_identifier(qualifier) {
                Some(root) => root,
                None => return Attribution::Unknown,
            },
            None => call.callee_name.as_str(),
        };
        match parsed.js_ts_call_root_scope(call.start_byte, call.end_byte, key) {
            JsRootScope::Program => {}
            JsRootScope::Nearer { callable: true } if call.qualifier.is_none() => {
                return Attribution::SameFile;
            }
            JsRootScope::Nearer { .. } | JsRootScope::Unproven => return Attribution::Unknown,
        }
        let imports = self.imports_of(file, parsed);
        match imports.map.get(key) {
            Some(spec) => {
                let binding = imports.bindings.iter().rev().find(|b| b.local == key);
                let member = match (call.qualifier.as_deref(), binding) {
                    (None, Some(b)) => b.member.clone(),
                    // `import d from './m'; d.f()` calls a property of the default
                    // export, not an export named `f`.
                    (Some(_), Some(b)) if b.kind == ImportBindingKind::MemberImport => None,
                    (Some(q), _) if q == key => Some(call.callee_name.clone()),
                    _ => None,
                };
                self.js_spec(file, spec, member)
            }
            None if is_same_file_function(parsed, call) => Attribution::SameFile,
            None => Attribution::Unknown,
        }
    }

    /// A JS/TS specifier from `file`: relative via the precedence winner of the
    /// shared candidate list, non-relative via the tsconfig alias lookup, else a
    /// package.
    pub(super) fn js_spec(
        &mut self,
        file: &str,
        spec: &str,
        member: Option<String>,
    ) -> Attribution {
        let spec = spec.trim();
        if is_js_ts_relative_specifier(spec) {
            // The first candidate is the file prism's own resolver binds
            // (`resolve_js_ts_relative_module`); lower-precedence files that
            // also exist are shadowed by it.
            let winner = js_ts_relative_module_candidates(spec, file, &self.listing)
                .and_then(|found| found.into_iter().next());
            return match winner {
                Some(file) => bound(vec![file], spec, member),
                None => unresolvable(spec, Why::NoFile),
            };
        }
        match self
            .js_alias
            .lookup(&self.inputs.js_paths_snapshot, &self.listing, file, spec)
        {
            Alias::NotAlias => Attribution::External,
            Alias::Declared => unresolvable(spec, Why::NoFile),
            Alias::Unavailable => Attribution::Unknown,
            Alias::Files(files) if files.len() > MAX_AMBIGUOUS => unresolvable(spec, Why::TooMany),
            Alias::Files(files) => {
                let basis = if files.len() == 1 {
                    Basis::Bound
                } else {
                    Basis::Candidates
                };
                Attribution::InRepo(InRepo {
                    files,
                    spec: spec.to_string(),
                    member,
                    declaring: Vec::new(),
                    basis,
                })
            }
        }
    }
}
