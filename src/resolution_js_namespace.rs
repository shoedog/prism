//! S1b-4 namespace R3 and its scoped E7 module fallback.
use super::{
    CallGraph, CallSite, ResolutionConfidence, ResolutionKind, ResolutionOutcome, ResolvedCallee,
};
use crate::call_graph::{js_ts_relative_module_candidates, resolve_js_ts_relative_module};
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
            let export = self.js_ts_namespace_exports.get(&file)?.get(member)?;
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
            return Some(ResolutionOutcome::hit(
                ids.into_iter()
                    .map(|target| ResolvedCallee {
                        target,
                        confidence: ResolutionConfidence::Exact,
                        kind: ResolutionKind::ImportQualified,
                    })
                    .collect(),
            ));
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
        if let Some(file) = js_ts_relative_module_candidates(&replacement, caller, indexed)?
            .into_iter()
            .find(|f| f.rsplit('/').next() == Some(basename))
        {
            return Some(file);
        }
    }
    None
}
