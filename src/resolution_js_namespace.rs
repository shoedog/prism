//! S1b-4 namespace R3 and its scoped E7 module fallback.
use super::{
    CallGraph, CallSite, ResolutionConfidence, ResolutionKind, ResolutionOutcome, ResolvedCallee,
};
use crate::call_graph::{js_ts_literal_module_candidates, resolve_js_ts_relative_module};
use std::collections::BTreeSet;

impl CallGraph {
    pub(super) fn js_ts_namespace_outcome<'a>(
        &'a self,
        module: &str,
        member: &str,
        site: &CallSite,
        base: &[&'a crate::call_graph::FunctionId],
    ) -> Option<ResolutionOutcome<'a>> {
        let module_file =
            resolve_js_ts_relative_module(module, &site.caller.file, &self.indexed_files)
                .or_else(|| namespace_sibling(module, &site.caller.file, &self.indexed_files));
        if let Some(file) = module_file {
            let outcome = self.js_ts_namespace_proof(&file, member, site, base);
            // I1: a module found only by the normalized fallback was not found on
            // main, where a failed proof reached the NameOnly E7 return below. Keep
            // that outcome instead of letting `None` authorize the legacy Exact stem
            // match. Modules main already found keep returning `None`.
            let found_on_main =
                js_ts_literal_module_candidates(module, &site.caller.file, &self.indexed_files)
                    .is_some_and(|found| !found.is_empty())
                    || namespace_sibling(module, &site.caller.file, &self.indexed_files).is_some();
            if outcome.is_some() || found_on_main {
                return outcome;
            }
        }
        // E7 is grading only: preserve every base candidate, including siblings
        // and opaque terminals. No export lookup grants authority on this route.
        Some(ResolutionOutcome::hit(
            base.iter()
                .map(|target| ResolvedCallee {
                    target,
                    confidence: ResolutionConfidence::NameOnly,
                    kind: ResolutionKind::ImportQualified,
                })
                .collect(),
        ))
    }
}

impl CallGraph {
    /// The positive namespace proof: `None` means "no proof", never "wrong".
    fn js_ts_namespace_proof<'a>(
        &'a self,
        file: &str,
        member: &str,
        site: &CallSite,
        base: &[&'a crate::call_graph::FunctionId],
    ) -> Option<ResolutionOutcome<'a>> {
        let export = self.js_ts_namespace_exports.get(file)?.get(member)?;
        if export.wrapped && !site.jsx_element {
            return None;
        }
        let ids = base
            .iter()
            .copied()
            .filter(|target| {
                target.file == export.file
                    && target.name == export.local_name
                    && export.span == Some((target.start_line, target.end_line))
            })
            .collect::<Vec<_>>();
        // A proof cannot add an identity base did not select, including renamed
        // exports, parameter defaults, escaped specifiers and stem misses.
        if ids.len() != 1 {
            return None;
        }
        Some(ResolutionOutcome::hit(
            ids.into_iter()
                .map(|target| ResolvedCallee {
                    target,
                    confidence: ResolutionConfidence::Exact,
                    kind: ResolutionKind::ImportQualified,
                })
                .collect(),
        ))
    }
}

// The sibling probe keeps the literal candidate list: it is the pre-I1 module
// lookup, which the "found on main" test above relies on.
fn namespace_sibling(module: &str, caller: &str, indexed: &BTreeSet<String>) -> Option<String> {
    let (prefix, extensions): (&str, &[&str]) = if let Some(p) = module.strip_suffix(".js") {
        (p, &[".ts", ".tsx"])
    } else if let Some(p) = module.strip_suffix(".jsx") {
        (p, &[".tsx"])
    } else if let Some(p) = module.strip_suffix(".mjs") {
        (p, &[".mts"])
    } else if let Some(p) = module.strip_suffix(".cjs") {
        (p, &[".cts"])
    } else {
        return None;
    };
    for extension in extensions {
        let replacement = format!("{prefix}{extension}");
        let basename = replacement.rsplit('/').next()?;
        if let Some(file) = js_ts_literal_module_candidates(&replacement, caller, indexed)?
            .into_iter()
            .find(|f| f.rsplit('/').next() == Some(basename))
        {
            return Some(file);
        }
    }
    None
}
