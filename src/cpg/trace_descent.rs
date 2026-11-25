//! Descent and ascent gates of the taint trace walker: which cross-function hops are crossed.

use std::collections::BTreeMap;

use crate::call_graph::CallSite;
use crate::resolution::ResolutionConfidence;
use petgraph::graph::NodeIndex;

use super::trace::Relation;
use super::{CodePropertyGraph, CpgNode, FlowConfidence, FlowDoubt, VarAccess};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LossReason {
    SelfParam,
    SelfReturn,
    ReturnNotEnteredSite,
    DepthCap,
    TargetNotBinding,
    NoMatchingSite,
    AmbiguousSite,
    NotExact,
    UnprovenSlot,
    PolicyCeiling,
    UnprovenReturnInput,
    ShortcutSuppressed,
    CfgPruned,
    Unmodelled,
    NoGraphEdge,
    ReviewNoReturnFlow,
    ReviewNoRecoveredDefUse,
    ReviewCfgFilter,
    ReviewContextPolicy,
    SanitizerModel,
    OperandGranularity,
    UncertifiedFlow,
    MatcherAdmission,
}

impl std::fmt::Display for LossReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::SelfParam => "self_param",
            Self::SelfReturn => "self_return",
            Self::ReturnNotEnteredSite => "return_not_entered_site",
            Self::DepthCap => "depth_cap",
            Self::TargetNotBinding => "target_not_binding",
            Self::NoMatchingSite => "no_matching_site",
            Self::AmbiguousSite => "ambiguous_site",
            Self::NotExact => "not_exact",
            Self::UnprovenSlot => "unproven_slot",
            Self::PolicyCeiling => "policy_ceiling",
            Self::UnprovenReturnInput => "unproven_return_input",
            Self::ShortcutSuppressed => "shortcut_suppressed",
            Self::CfgPruned => "cfg_pruned",
            Self::Unmodelled => "unmodelled",
            Self::NoGraphEdge => "no_graph_edge",
            Self::ReviewNoReturnFlow => "review_no_return_flow",
            Self::ReviewNoRecoveredDefUse => "review_no_recovered_def_use",
            Self::ReviewCfgFilter => "review_cfg_filter",
            Self::ReviewContextPolicy => "review_context_policy",
            Self::SanitizerModel => "sanitizer_model",
            Self::OperandGranularity => "operand_granularity",
            Self::UncertifiedFlow => "uncertified_flow",
            Self::MatcherAdmission => "matcher_admission",
        })
    }
}

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
        descent_gate_cache: &mut BTreeMap<DescentGateKey, Result<(), LossReason>>,
    ) -> Result<(), LossReason> {
        match binding_label {
            Some(FlowConfidence::Exact)
            | Some(FlowConfidence::NameOnly(
                FlowDoubt::Killed { .. }
                | FlowDoubt::SameLine
                | FlowDoubt::CfgIncomplete
                | FlowDoubt::AliasUnstable,
            )) => {}
            // `not_exact`: the call-binding doubt, and (lane 9) `builtin_callback`, are refused here.
            // Naming the variant keeps a new doubt kind a compile-time decision.
            Some(FlowConfidence::NameOnly(FlowDoubt::CallNameOnly)) | None => {
                return Err(LossReason::NotExact);
            }
        }

        let CpgNode::Variable {
            file: target_file,
            function: target_fn,
            function_start_line: target_fn_start_line,
            access: VarAccess::Def,
            ..
        } = &self.graph[to]
        else {
            return Err(LossReason::TargetNotBinding);
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
            return Err(LossReason::TargetNotBinding);
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

        let result = self.binding_site(from, to).and_then(|site| {
            if !site_contains_occurrence(site, *source_start_byte, *source_end_byte) {
                return Err(LossReason::NoMatchingSite);
            }
            self.call_graph
                .resolve_call_site(site)
                .into_iter()
                .any(|resolved| {
                    resolved.confidence == ResolutionConfidence::Exact
                        && resolved.target.file == *target_file
                        && resolved.target.name == *target_fn
                        && resolved.target.start_line == *target_fn_start_line
                })
                .then_some(())
                .ok_or(LossReason::NotExact)
        });
        descent_gate_cache.insert(cache_key, result);
        result
    }

    pub(super) fn binding_site(
        &self,
        from: NodeIndex,
        to: NodeIndex,
    ) -> Result<&CallSite, LossReason> {
        let CpgNode::Variable {
            function: target, ..
        } = &self.graph[to]
        else {
            return Err(LossReason::TargetNotBinding);
        };
        let CpgNode::Variable {
            file,
            function,
            function_start_line,
            line,
            start_byte,
            end_byte,
            ..
        } = &self.graph[from]
        else {
            return Err(LossReason::NoMatchingSite);
        };
        let mut sites = self
            .call_graph
            .callers
            .get(target)
            .into_iter()
            .flatten()
            .filter(|site| {
                Self::call_site_is_candidate(
                    site,
                    file,
                    function,
                    *function_start_line,
                    *line,
                    *start_byte,
                    *end_byte,
                )
            });
        let site = sites.next().ok_or(LossReason::NoMatchingSite)?;
        if sites.next().is_some() {
            return Err(LossReason::AmbiguousSite);
        }
        Ok(site)
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
                || (site.start_byte <= source_start_byte && source_end_byte <= site.end_byte))
    }

    pub(super) fn return_flow_ascent(
        &self,
        from: NodeIndex,
        to: NodeIndex,
    ) -> Result<(), LossReason> {
        let owner = self.node_file_fn(from).ok_or(LossReason::NoMatchingSite)?;
        let mut candidate = false;
        for site in self.return_sites(to) {
            let resolved = self.call_graph.resolve_call_site_full(site);
            let matches_owner = |target: &crate::call_graph::FunctionId| {
                (&target.file, &target.name, target.start_line) == (&owner.0, &owner.1, owner.2)
            };
            if super::build::singleton_exact(&resolved)
                .is_some_and(|entry| matches_owner(entry.target))
            {
                return Ok(());
            }
            candidate |= resolved
                .resolved
                .iter()
                .any(|entry| matches_owner(entry.target));
        }
        Err(if candidate {
            LossReason::NotExact
        } else {
            LossReason::NoMatchingSite
        })
    }

    pub(super) fn return_sites(&self, to: NodeIndex) -> impl Iterator<Item = &CallSite> {
        let owner = self.node_file_fn(to);
        self.call_graph
            .calls
            .iter()
            .filter(move |(id, _)| {
                owner.as_ref().is_some_and(|(file, name, start)| {
                    (&id.file, &id.name, id.start_line) == (file, name, *start)
                })
            })
            .flat_map(|(_, sites)| sites)
    }

    pub(super) fn crossing_kind(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        rel: Relation,
        label: Option<FlowConfidence>,
    ) -> Option<super::BoundaryKind> {
        use super::BoundaryKind::*;
        let from_owner = self.node_file_fn(from)?;
        let to_owner = self.node_file_fn(to)?;
        if from_owner != to_owner {
            Some(CrossFunction)
        } else if rel == Relation::ReturnFlow {
            Some(SelfFunctionReturn)
        } else if label == Some(FlowConfidence::NameOnly(FlowDoubt::CallNameOnly))
            || self.is_parameter_binding_from(from, to, rel)
        {
            Some(SelfFunctionParam)
        } else {
            None
        }
    }

    pub(super) fn crossing_gate(
        &self,
        kind: super::BoundaryKind,
        relation: Relation,
        from: NodeIndex,
        to: NodeIndex,
        label: Option<FlowConfidence>,
        depth: usize,
        cache: &mut BTreeMap<DescentGateKey, Result<(), LossReason>>,
        policy: Option<&super::TaintCrossingPolicy<'_>>,
    ) -> Result<(), LossReason> {
        use super::BoundaryKind::*;
        match kind {
            SelfFunctionParam => return Err(LossReason::SelfParam),
            SelfFunctionReturn => return Err(LossReason::SelfReturn),
            CrossFunction => {}
        }
        if depth >= super::trace::MAX_TAINT_DESCENT_DEPTH {
            return Err(LossReason::DepthCap);
        }
        if relation == Relation::ReturnFlow {
            return self.return_flow_ascent(from, to);
        }
        self.descend_target(from, to, label, cache)?;
        if policy.is_some() && self.admitted_data_flow_label(policy, from, to).is_none() {
            return Err(LossReason::PolicyCeiling);
        }
        Ok(())
    }
}

fn site_contains_occurrence(site: &CallSite, start_byte: usize, end_byte: usize) -> bool {
    if start_byte == end_byte {
        // An empty occurrence has no actual bytes: it is inside the call only strictly before its
        // exclusive end (`g(t)` ending at byte 168 does not contain `[168, 168)`).
        return site.start_byte <= start_byte && start_byte < site.end_byte;
    }
    site.start_byte <= start_byte && end_byte <= site.end_byte
}

impl CodePropertyGraph {
    pub(super) fn crossing_site(
        &self,
        from: NodeIndex,
        to: NodeIndex,
        relation: Relation,
        order: Option<&dyn super::SameLineOrderView>,
    ) -> Option<super::CrossingSite> {
        let (site, slot) = if relation == Relation::ReturnFlow {
            let order = order?;
            let mut matching = self.return_sites(to).filter_map(|site| {
                order
                    .crossing_slot(to, site, relation)
                    .map(|slot| (site, Some(slot)))
            });
            let pair = matching.next()?;
            if matching.next().is_some() {
                return None;
            }
            pair
        } else {
            let site = self.binding_site(from, to).ok()?;
            let (start, end) = super::trace_cuts::byte_span(self.node(from));
            if !site_contains_occurrence(site, start, end) {
                return None;
            }
            (
                site,
                order.and_then(|order| order.crossing_slot(from, site, relation)),
            )
        };
        Some(super::CrossingSite {
            start_byte: site.start_byte,
            end_byte: site.end_byte,
            slot,
        })
    }
}
