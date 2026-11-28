//! Indirect call resolution (function pointers, dispatch tables, struct callbacks and Go
//! callback arguments): the producer that adds `CallSiteOrigin::IndirectResolution` records.

use super::{
    go, level4, CallGraph, CallKind, CallSite, CallSiteOrigin, FunctionId, JsLocalBinding,
};
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};

impl CallGraph {
    pub(crate) fn recompute_indirect_calls(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.executable_owner = None;
        self.clear_indirect_calls();
        let (sites, level3_telemetry) = self.compute_indirect_call_sites(files, true);
        self.apply_indirect_call_sites(sites);
        self.level3_indirect_resolved = level3_telemetry.edges;
        self.go_level3_b1_telemetry = level3_telemetry;
    }

    pub(super) fn clear_indirect_calls(&mut self) {
        self.level3_indirect_resolved = 0;
        self.go_level3_b1_telemetry = Default::default();
        for sites in self.calls.values_mut() {
            sites.retain(|site| site.origin != CallSiteOrigin::IndirectResolution);
        }
        self.calls.retain(|_, sites| !sites.is_empty());

        for sites in self.callers.values_mut() {
            sites.retain(|site| site.origin != CallSiteOrigin::IndirectResolution);
        }
        self.callers.retain(|_, sites| !sites.is_empty());
    }

    pub(super) fn compute_indirect_call_sites(
        &self,
        files: &BTreeMap<String, ParsedFile>,
        stand_down: bool,
    ) -> (Vec<(FunctionId, CallSite)>, go::callback::GoLevel3Telemetry) {
        // Resolve indirect call sites (function pointer variables and dispatch
        // tables). Preserve the historical level ordering:
        // 1/2 local function pointer and array dispatch, 4 struct callbacks,
        // then 3 parameter-passed function pointers.
        let known_fn_names: BTreeSet<String> = self.functions.keys().cloned().collect();
        let mut extra_sites: Vec<(FunctionId, CallSite)> = Vec::new();

        for (caller_id, sites) in self
            .calls
            .iter()
            .filter(|(id, _)| crate::anon_owner::permits_legacy_indirect(id))
        {
            for site in sites {
                if self.functions.contains_key(&site.callee_name) {
                    continue;
                }

                let parsed = match files.get(&caller_id.file) {
                    Some(p) => p,
                    None => continue,
                };

                // Level 2: array dispatch table — callee_name like "handlers[0]".
                if site.callee_name.contains('[') {
                    let array_name = site.callee_name.split('[').next().unwrap_or("");
                    if array_name.is_empty() {
                        continue;
                    }

                    let func_source = Self::extract_func_source(parsed, caller_id);
                    let targets = crate::ast::resolve_array_dispatch(
                        &func_source,
                        array_name,
                        &known_fn_names,
                    );
                    let file_targets = if targets.is_empty() {
                        crate::ast::resolve_array_dispatch(
                            &parsed.source,
                            array_name,
                            &known_fn_names,
                        )
                    } else {
                        Vec::new()
                    };
                    for target in targets.iter().chain(file_targets.iter()) {
                        extra_sites.push((
                            caller_id.clone(),
                            Self::indirect_call_site(caller_id, target.clone(), site),
                        ));
                    }
                    continue;
                }

                // Level 1: local variable function pointer — callee_name is a plain identifier.
                if site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    let Some(func_source) =
                        crate::anon_owner::source_before(parsed, caller_id, site.start_byte)
                    else {
                        continue;
                    };
                    if let Some(resolved) = crate::ast::resolve_fptr_assignment(
                        func_source,
                        &site.callee_name,
                        &known_fn_names,
                    ) {
                        extra_sites.push((
                            caller_id.clone(),
                            Self::indirect_call_site(caller_id, resolved, site),
                        ));
                    }
                }
            }
        }

        // Level 4: struct field callback resolution (interprocedural).
        let level4_index = level4::build(files, &known_fn_names);

        let mut level4_sites: Vec<(FunctionId, CallSite)> = Vec::new();
        for (caller_id, sites) in self
            .calls
            .iter()
            .filter(|(id, _)| crate::anon_owner::permits_legacy_indirect(id))
        {
            for site in sites {
                if self.functions.contains_key(&site.callee_name) {
                    continue;
                }
                if site.qualifier.is_none() {
                    continue;
                }
                if !site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    continue;
                }

                let already_resolved = extra_sites.iter().any(|(cid, es)| {
                    cid == caller_id
                        && es.line == site.line
                        && known_fn_names.contains(&es.callee_name)
                });
                if already_resolved {
                    continue;
                }

                if let Some(by_file) = level4_index.get(&site.callee_name) {
                    for targets in by_file.values() {
                        for target in targets {
                            level4_sites.push((
                                caller_id.clone(),
                                Self::indirect_call_site(caller_id, target.clone(), site),
                            ));
                        }
                    }
                }
            }
        }
        extra_sites.extend(level4_sites);

        // Level 3: Go bare free-function values passed to proven free-function HOFs.
        // Enumerate only original source sites; synthetic sites never donate evidence.
        let proof = go::callback::GoCallbackProofContext::new(
            files,
            &self.functions,
            &self.method_owners,
            &self.go_file_profiles,
            &self.go_dot_import_files,
            &self.go_file_import_paths,
        );
        let mut callback_slots_by_name: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for (hof, parameters) in &self.go_callback_parameters {
            callback_slots_by_name
                .entry(hof.name.clone())
                .or_default()
                .extend(parameters.iter().map(|parameter| parameter.slot));
        }
        let mut telemetry = go::callback::GoLevel3Telemetry::default();
        let mut accepted_targets = BTreeSet::new();
        let mut level3_sites = BTreeSet::new();
        for (caller, sites) in &self.calls {
            let Some(parsed) = files.get(&caller.file) else {
                continue;
            };
            for site in sites {
                if site.origin != CallSiteOrigin::Source || site.kind != CallKind::Call {
                    continue;
                }
                let Some(candidate_slots) = callback_slots_by_name.get(&site.callee_name) else {
                    continue;
                };
                let arguments =
                    parsed.call_argument_texts_and_spans_at(site.start_byte, &site.callee_name);
                for slot in candidate_slots {
                    telemetry.candidates += 1;
                    let mut record = go::callback::GoLevel3SiteRecord {
                        inbound_caller: caller.clone(),
                        inbound_line: site.line,
                        inbound_start_byte: site.start_byte,
                        inbound_end_byte: site.end_byte,
                        hof_name: site.callee_name.clone(),
                        slot: *slot,
                        argument: arguments
                            .get(*slot)
                            .map(|(argument, _)| argument.trim().to_string()),
                        hof: None,
                        target: None,
                        callback_parameter: None,
                        invocation_spans: Vec::new(),
                        accepted: false,
                        drop_reason: None,
                    };
                    let result: Result<_, go::callback::GoLevel3DropReason> = (|| {
                        if parsed.language != crate::languages::Language::Go {
                            return Err(go::callback::GoLevel3DropReason::NonGoOrTest);
                        }
                        if site.arg_spread {
                            return Err(
                                go::callback::GoLevel3DropReason::MissingSlotOrArgumentIdentity,
                            );
                        }
                        let hof = proof.hof_at(caller, site)?;
                        record.hof = Some(hof.clone());
                        let resolved = self.resolve_call_site_full(site);
                        if resolved.resolved.len() != 1
                            || resolved.resolved[0].confidence
                                != crate::resolution::ResolutionConfidence::Exact
                            || resolved.resolved[0].target != &hof
                        {
                            return Err(go::callback::GoLevel3DropReason::HofNotSingletonExact);
                        }
                        telemetry.exact_inbound_sites += 1;
                        let parameter = self
                            .go_callback_parameters
                            .get(&hof)
                            .and_then(|parameters| {
                                parameters.iter().find(|parameter| parameter.slot == *slot)
                            })
                            .ok_or(go::callback::GoLevel3DropReason::MissingCallbackParameter)?;
                        record.callback_parameter = Some(parameter.name.clone());
                        let (argument, span) = arguments.get(*slot).ok_or(
                            go::callback::GoLevel3DropReason::MissingSlotOrArgumentIdentity,
                        )?;
                        let target =
                            proof.bare_free_function_at(caller, span.clone(), argument.trim())?;
                        record.target = Some(target.clone());
                        let target_signature = self
                            .go_free_function_signatures
                            .get(&target)
                            .ok_or(go::callback::GoLevel3DropReason::SignatureUnprovenOrMismatch)?;
                        if !go::callback::signatures_match_strict(
                            &parameter.signature,
                            target_signature,
                        ) {
                            return Err(
                                go::callback::GoLevel3DropReason::SignatureUnprovenOrMismatch,
                            );
                        }
                        let invocations =
                            proof.direct_parameter_invocations(&self.calls, &hof, parameter)?;
                        record.invocation_spans = invocations
                            .iter()
                            .map(|invocation| go::callback::GoLevel3InvocationSpan {
                                line: invocation.line,
                                start_byte: invocation.start_byte,
                                end_byte: invocation.end_byte,
                            })
                            .collect();
                        Ok((hof, target, parameter.clone(), invocations))
                    })(
                    );
                    match result {
                        Ok((hof, target, parameter, invocations)) => {
                            telemetry.accepted_inbound_sites += 1;
                            record.accepted = true;
                            accepted_targets.insert(target.clone());
                            for invocation in invocations {
                                let mut synthetic = Self::indirect_call_site(
                                    &hof,
                                    target.name.clone(),
                                    &invocation,
                                );
                                synthetic.source_callee_name = Some(parameter.name.clone());
                                synthetic.pre_resolved_target = Some(target.clone());
                                level3_sites.insert((hof.clone(), synthetic));
                            }
                        }
                        Err(reason) => {
                            record.drop_reason = Some(reason.as_str().to_string());
                            *telemetry
                                .drops
                                .entry(reason.as_str().to_string())
                                .or_default() += 1;
                        }
                    }
                    telemetry.sites.push(record);
                }
            }
        }
        telemetry.unique_targets = accepted_targets.len();
        telemetry.edges = level3_sites.len();
        extra_sites.extend(level3_sites);
        // Slice L stand-down: every level above ran unchanged, so each read of the Level-1/2
        // output (Level 4's `already_resolved`; Level 3 reads `self.calls`, not `extra_sites`)
        // saw main's population. Only now, on the finished output, are the records whose source
        // site is an alias site with a proven hit (the one predicate) removed, by exact site
        // identity (caller, start byte, end byte).
        if stand_down {
            let proven: BTreeSet<(&FunctionId, usize, usize)> = self
                .calls
                .iter()
                .flat_map(|(caller, sites)| {
                    sites
                        .iter()
                        .filter(|s| {
                            s.origin == CallSiteOrigin::Source && self.js_ts_alias_route_hit(s)
                        })
                        .map(move |s| (caller, s.start_byte, s.end_byte))
                })
                .collect();
            if !proven.is_empty() {
                extra_sites.retain(|(caller, record)| {
                    !proven.contains(&(caller, record.start_byte, record.end_byte))
                });
            }
        }
        debug_assert_eq!(
            telemetry.candidates,
            telemetry.accepted_inbound_sites + telemetry.drops.values().sum::<usize>()
        );
        (extra_sites, telemetry)
    }

    fn apply_indirect_call_sites(&mut self, sites: Vec<(FunctionId, CallSite)>) {
        for (caller_id, site) in sites {
            let callee_name = site.callee_name.clone();
            self.calls
                .entry(caller_id)
                .or_default()
                .insert(site.clone());
            self.callers.entry(callee_name).or_default().push(site);
        }
    }

    fn indirect_call_site(
        caller_id: &FunctionId,
        target: String,
        source_site: &CallSite,
    ) -> CallSite {
        CallSite {
            caller: caller_id.clone(),
            callee_name: target,
            source_callee_name: None,
            line: source_site.line,
            kind: CallKind::Call,
            // Carry the source call site span so same-line indirect dups do not collapse.
            start_byte: source_site.start_byte,
            end_byte: source_site.end_byte,
            qualifier: None,
            receiver_lexically_bound: false,
            receiver_type: None,
            receiver_owner_identity: None,
            receiver_local_type_shadowed: false,
            receiver_recovery: None,
            receiver_materialized: false,
            receiver_newly_recovered: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::IndirectResolution,
            pre_resolved_target: None,
            jsx_element: false,
            path_anchor: None,
            // E10 non-goal: indirect-resolution sites never carry a lexical binding.
            local_binding: JsLocalBinding::Unchecked,
        }
    }

    /// Extract the source text for a function from its parsed file.
    fn extract_func_source(parsed: &ParsedFile, func_id: &FunctionId) -> String {
        let lines: Vec<&str> = parsed.source.lines().collect();
        let start = func_id.start_line.saturating_sub(1); // 1-indexed to 0-indexed
        let end = func_id.end_line.min(lines.len());
        lines[start..end].join("\n")
    }
}

#[cfg(test)]
#[path = "indirect_tests.rs"]
mod tests;
