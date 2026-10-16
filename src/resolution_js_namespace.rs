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
            let outcome = self
                .js_ts_namespace_proof(&file, member, site, base)
                .map(|out| {
                    let export = &self.js_ts_namespace_exports[&file][member];
                    self.certify_anonymous_namespace(site, module, export, out)
                });
            // A module found only by the normalized fallback does not get the legacy
            // Exact stem match when its export proof fails: `None` would authorize
            // it, so keep the NameOnly outcome below. A module the literal lookup
            // finds (the literal candidate list or the sibling probe) keeps
            // returning `None`.
            let found_by_literal_lookup =
                js_ts_literal_module_candidates(module, &site.caller.file, &self.indexed_files)
                    .is_some_and(|found| !found.is_empty())
                    || namespace_sibling(module, &site.caller.file, &self.indexed_files).is_some();
            if outcome.is_some() || found_by_literal_lookup {
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

// The sibling probe keeps the literal candidate list, so `found_by_literal_lookup`
// above separates literal-lookup modules from ones only the fallback finds.
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
