//! Descent and ascent gates of the taint trace walker: which cross-function hops are crossed.
//!
//! Moved verbatim out of `trace.rs` (lane 8 slice I-1, commit 1, move only).

use std::collections::BTreeMap;

use crate::call_graph::CallSite;
use crate::resolution::ResolutionConfidence;
use petgraph::graph::NodeIndex;

use super::trace::Relation;
use super::{CodePropertyGraph, CpgNode, FlowConfidence, FlowDoubt, VarAccess};

pub(super) type DescentGateKey = (
    String,    // caller file
    String,    // caller function
    usize,     // caller function start line
    usize,     // source occurrence line
    usize,     // source occurrence start byte
    usize,     // source occurrence end byte
    String,    // callee name
    String,    // target function file
    usize,     // target function start line
    NodeIndex, // target binding
);

impl CodePropertyGraph {
    /// Is `to` a parameter binding reached by an arg→param call edge from `from`?
    ///
    /// Parameter defs and one-line body locals can share the function start line. Statement nodes are
    /// also line-deduped, so byte/statement heuristics misclassify locals in minified multi-function
    /// lines. The boundary we need to preserve is narrower and edge-sensitive: a recursive/self call
    /// whose DataFlow edge lands on the callee parameter. If call resolution misses that self-call,
    /// this stays intra-function and fails open rather than manufacturing a false `NotReached`.
    pub(super) fn is_parameter_binding_from(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        rel: Relation,
    ) -> bool {
        if rel != Relation::DataFlow {
            return false;
        }
        let CpgNode::Variable {
            access: VarAccess::Def,
            function: callee_name,
            ..
        } = &self.graph[to]
        else {
            return false;
        };
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start_line,
            line: source_line,
            start_byte: source_start_byte,
            end_byte: source_end_byte,
            ..
        } = &self.graph[from]
        else {
            return false;
        };
        self.call_graph
            .callers
            .get(callee_name)
            .is_some_and(|sites| {
                sites.iter().any(|site| {
                    Self::call_site_is_candidate(
                        site,
                        caller_file,
                        caller_name,
                        *caller_start_line,
                        *source_line,
                        *source_start_byte,
                        *source_end_byte,
                    )
                })
            })
    }

    /// Under Exact, a binding edge's own `call_nameonly` or `builtin_callback`
    /// doubt refuses as `not_exact` before consulting any site proof. Only
    /// existing read-side doubts can proceed to the site check, so the
    /// lane-9 `builtin_callback` doubt is refused when it becomes available.
    ///
    /// Count the legacy candidates first (`ambiguous_site` for several).
    /// The unique site must contain the occurrence (`no_matching_site` for
    /// zero candidates or failed containment) and resolve Exactly to the target owner.
    pub(super) fn descend_target(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        binding_label: Option<FlowConfidence>,
        descent_gate_cache: &mut BTreeMap<DescentGateKey, bool>,
    ) -> bool {
        match binding_label {
            Some(FlowConfidence::Exact)
            | Some(FlowConfidence::NameOnly(
                FlowDoubt::Killed { .. }
                | FlowDoubt::SameLine
                | FlowDoubt::CfgIncomplete
                | FlowDoubt::AliasUnstable,
            )) => {}
            _ => return false,
        }

        let CpgNode::Variable {
            file: target_file,
            function: target_fn,
            function_start_line: target_fn_start_line,
            access: VarAccess::Def,
            ..
        } = &self.graph[to]
        else {
            return false;
        };

        let CpgNode::Variable {
            file: caller_file,
            function: caller_fn,
            function_start_line: caller_fn_start_line,
            line: source_line,
            start_byte: source_start_byte,
            end_byte: source_end_byte,
            ..
        } = &self.graph[from]
        else {
            return false;
        };

        let cache_key = (
            caller_file.clone(),
            caller_fn.clone(),
            *caller_fn_start_line,
            *source_line,
            *source_start_byte,
            *source_end_byte,
            target_fn.clone(),
            target_file.clone(),
            *target_fn_start_line,
            to,
        );
        if let Some(can_descend) = descent_gate_cache.get(&cache_key) {
            return *can_descend;
        }

        let can_descend = self
            .call_graph
            .callers
            .get(target_fn)
            .and_then(|sites| {
                let matching_sites: Vec<_> = sites
                    .iter()
                    .filter(|site| {
                        Self::call_site_is_candidate(
                            site,
                            caller_file,
                            caller_fn,
                            *caller_fn_start_line,
                            *source_line,
                            *source_start_byte,
                            *source_end_byte,
                        )
                    })
                    .collect();

                match matching_sites.len() {
                    1 => Some(matching_sites[0]),
                    _ => None,
                }
            })
            .is_some_and(|site| {
                site_contains_occurrence(site, *source_start_byte, *source_end_byte)
                    && self
                        .call_graph
                        .resolve_call_site(site)
                        .into_iter()
                        .any(|resolved| {
                            resolved.confidence == ResolutionConfidence::Exact
                                && resolved.target.file == *target_file
                                && resolved.target.name == *target_fn
                                && resolved.target.start_line == *target_fn_start_line
                        })
            });

        descent_gate_cache.insert(cache_key, can_descend);
        can_descend
    }

    /// Nominate sites from the executing caller by the legacy line or byte
    /// rule. A same-line collision remains ambiguous before containment.
    fn call_site_is_candidate(
        site: &CallSite,
        caller_file: &str,
        caller_name: &str,
        caller_start_line: usize,
        source_line: usize,
        source_start_byte: usize,
        source_end_byte: usize,
    ) -> bool {
        site.caller.file == caller_file
            && site.caller.name == caller_name
            && site.caller.start_line == caller_start_line
            && (site.line == source_line
                || site_contains_occurrence(site, source_start_byte, source_end_byte))
    }

    pub(super) fn return_flow_ascent_allowed(&self, from: NodeIndex, to: NodeIndex) -> bool {
        let Some((callee_file, callee_name, callee_start)) = self.node_file_fn(from) else {
            return false;
        };
        let CpgNode::Variable {
            file: caller_file,
            function: caller_name,
            function_start_line: caller_start,
            ..
        } = &self.graph[to]
        else {
            return false;
        };
        let caller = crate::call_graph::FunctionId {
            file: caller_file.clone(),
            name: caller_name.clone(),
            start_line: *caller_start,
            end_line: usize::MAX,
        };
        self.call_graph
            .calls
            .iter()
            .filter(|(id, _)| {
                id.file == caller.file
                    && id.name == caller.name
                    && id.start_line == caller.start_line
            })
            .flat_map(|(_, sites)| sites)
            .any(|site| {
                super::build::singleton_exact(&self.call_graph.resolve_call_site_full(site))
                    .is_some_and(|resolved| {
                        resolved.target.file == callee_file
                            && resolved.target.name == callee_name
                            && resolved.target.start_line == callee_start
                    })
            })
    }
}

fn site_contains_occurrence(site: &CallSite, start_byte: usize, end_byte: usize) -> bool {
    site.start_byte <= start_byte && end_byte <= site.end_byte
}
