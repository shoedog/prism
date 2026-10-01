//! S1b-4 namespace R3 and its scoped E7 module fallback.
use super::{
    CallGraph, CallSite, DropReason, ResolutionConfidence, ResolutionKind, ResolutionOutcome,
    ResolvedCallee,
};
use crate::call_graph::{js_ts_relative_module_candidates, resolve_js_ts_relative_module};
use std::collections::BTreeSet;

impl CallGraph {
    pub(super) fn js_ts_namespace_outcome(
        &self,
        module: &str,
        member: &str,
        site: &CallSite,
    ) -> Option<ResolutionOutcome<'_>> {
        let missing = || {
            ResolutionOutcome::dropped(if self.functions.contains_key(member) {
                DropReason::ImportExternal
            } else {
                DropReason::UnknownName
            })
        };
        let module_file =
            resolve_js_ts_relative_module(module, &site.caller.file, &self.indexed_files)
                .or_else(|| namespace_sibling(module, &site.caller.file, &self.indexed_files));
        if let Some(file) = module_file {
            let Some(export) = self
                .js_ts_namespace_exports
                .get(&file)
                .and_then(|e| e.get(member))
            else {
                return self.js_ts_namespace_complete(&file).then(missing);
            };
            if self.js_ts_namespace_keeps_base(export) {
                return None;
            }
            if export.span.is_none() {
                if export.file == file || !self.js_ts_namespace_private_barrel(&file) {
                    return None;
                }
                // An opaque terminal identifies a value binding, not a callable
                // origin. Only this inert forwarding barrel's private functions
                // are disproved; all other base R3 candidates remain unproven.
                let last = module.rsplit('/').next().unwrap_or(module);
                let stem = last.rsplit('.').next_back().unwrap_or(last);
                let ids = self
                    .functions
                    .get(member)
                    .into_iter()
                    .flatten()
                    .filter(|f| {
                        f.file != file
                            && !self.method_owners.contains_key(*f)
                            && (super::file_stem(&f.file) == stem
                                || f.file.rsplit('/').nth(1) == Some(last))
                    })
                    .map(|target| ResolvedCallee {
                        target,
                        confidence: ResolutionConfidence::Exact,
                        kind: ResolutionKind::ImportQualified,
                    })
                    .collect::<Vec<_>>();
                return Some(if ids.is_empty() {
                    missing()
                } else {
                    ResolutionOutcome::hit(ids)
                });
            }
            return Some(match self.js_ts_export_target_candidates(export, site) {
                Err(reason) => ResolutionOutcome::dropped(reason),
                Ok(ids) if ids.is_empty() => missing(),
                Ok(ids) => ResolutionOutcome::hit(
                    ids.into_iter()
                        .map(|target| ResolvedCallee {
                            target,
                            confidence: ResolutionConfidence::Exact,
                            kind: ResolutionKind::ImportQualified,
                        })
                        .collect(),
                ),
            });
        }
        // Only unresolved modules use stem/directory fallback. Each candidate's
        // own export identity still filters it; even a single survivor is NameOnly.
        let last = module.rsplit('/').next().unwrap_or(module);
        let stem = last.rsplit('.').next_back().unwrap_or(last);
        let mut ids = BTreeSet::new();
        let mut wrapped = false;
        for file in &self.indexed_files {
            if super::file_stem(file) != stem && file.rsplit('/').nth(1) != Some(last) {
                continue;
            }
            let Some(export) = self
                .js_ts_namespace_exports
                .get(file)
                .and_then(|e| e.get(member))
            else {
                if !self.js_ts_namespace_complete(file) {
                    return None;
                }
                continue;
            };
            if self.js_ts_namespace_keeps_base(export) {
                return None;
            }
            if export.span.is_none() {
                // E7 grades the original base candidates, without equating the
                // opaque binding's local name with its function's identity.
                ids.extend(
                    self.functions
                        .get(member)
                        .into_iter()
                        .flatten()
                        .filter(|f| {
                            f.file == *file
                                && !self.method_owners.contains_key(*f)
                                && !self.js_ts_namespace_private_barrel(file)
                        }),
                );
                continue;
            }
            match self.js_ts_export_target_candidates(export, site) {
                Ok(candidates) => ids.extend(candidates),
                Err(DropReason::WrappedExportNonJsx) => wrapped = true,
                Err(reason) => return Some(ResolutionOutcome::dropped(reason)),
            }
        }
        Some(if ids.is_empty() {
            if wrapped {
                ResolutionOutcome::dropped(DropReason::WrappedExportNonJsx)
            } else {
                missing()
            }
        } else {
            ResolutionOutcome::hit(
                ids.into_iter()
                    .map(|target| ResolvedCallee {
                        target,
                        confidence: ResolutionConfidence::NameOnly,
                        kind: ResolutionKind::ImportQualified,
                    })
                    .collect(),
            )
        })
    }

    fn js_ts_namespace_keeps_base(&self, export: &crate::js_exports::ResolvedJsExport) -> bool {
        // The bounded projection carries the terminal binding's file/name;
        // use that origin only for E5, never to infer callable identity.
        export.span.is_none()
            && self
                .js_ts_exports
                .get(&export.file)
                .is_some_and(|facts| facts.namespace_may_call_locals.contains(&export.local_name))
    }

    fn js_ts_namespace_private_barrel(&self, file: &str) -> bool {
        self.js_ts_exports
            .get(file)
            .is_some_and(|f| f.namespace_private_barrel)
    }

    fn js_ts_namespace_complete(&self, file: &str) -> bool {
        crate::js_exports::namespace_exports_complete(
            &self.js_ts_exports,
            &|from, module| resolve_js_ts_relative_module(module, from, &self.indexed_files),
            file,
            0,
            &BTreeSet::new(),
        )
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
