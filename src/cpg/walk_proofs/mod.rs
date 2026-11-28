//! Real proofs for the taint walk's crossings (lane 8, I-walk; DESIGN 5.2). A proof reads the
//! call site from the parsed files the walk's view hands out, and answers `Proved`, `Refused`
//! (with the walk's own reason) or `Unknown`. PR 1 holds rule (i), the occurrence proof, rule
//! (ii), the emission proof, and the per-walk cache; the other rules panic by name when asked
//! for (`trace_proofs.rs`).
//!
//! Nothing here runs unless a real rule or compare mode is switched on.

mod emission;
mod occurrence;
pub(super) mod syntax;

use std::collections::BTreeMap;

use petgraph::graph::NodeIndex;

use super::trace_proofs::Rule;
use super::{CodePropertyGraph, CpgNode, FlowConfidence, LossReason, Relation, SameLineOrderView};
use syntax::OwnerId;

/// The call site a proof certified: the call node's bytes, the reached actual's bytes and the
/// argument position it occupies, with the owners on both sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SiteCert {
    pub file: String,
    pub caller: OwnerId,
    pub site: (usize, usize),
    pub actual: (usize, usize),
    pub slot: usize,
    pub target: OwnerId,
    pub family: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Verdict {
    Proved(SiteCert),
    /// Provably not a hop the walk may take; `&str` names the shape.
    Refused(LossReason, &'static str),
    /// The evidence at hand does not decide.
    Unknown(&'static str),
}

impl Verdict {
    /// `(verdict, reason, family)` as the compare lines print them.
    pub(super) fn describe(&self) -> (&'static str, String, &'static str) {
        match self {
            Self::Proved(cert) => ("proved", "-".into(), cert.family),
            Self::Refused(reason, family) => ("refused", reason.to_string(), family),
            Self::Unknown(family) => ("unknown", "-".into(), family),
        }
    }
}

/// What a verdict depends on. Root, depth, budget and exclusions are never part of it: a proof
/// is a property of the hop, not of the walk that reached it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ProofKey {
    rule: Rule,
    relation: Relation,
    caller: OwnerId,
    site: (usize, usize),
    value: (usize, usize),
    from: NodeIndex,
    to: NodeIndex,
    target: OwnerId,
    slot: usize,
    label: Option<FlowConfidence>,
}

impl ProofKey {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        rule: Rule,
        relation: Relation,
        caller: OwnerId,
        site: (usize, usize),
        value: (usize, usize),
        (from, to): (NodeIndex, NodeIndex),
        target: OwnerId,
        slot: usize,
        label: Option<FlowConfidence>,
    ) -> Self {
        Self {
            rule,
            relation,
            caller,
            site,
            value,
            from,
            to,
            target,
            slot,
            label,
        }
    }

    /// The key of a hop between two variable occurrences. The site is the legacy gate's
    /// candidate when it has exactly one (the proof itself may certify another); the slot is
    /// only known after a proof, so it stays unset here.
    fn of_hop(
        cpg: &CodePropertyGraph,
        rule: Rule,
        relation: Relation,
        label: Option<FlowConfidence>,
        from: NodeIndex,
        to: NodeIndex,
    ) -> Option<Self> {
        let (
            CpgNode::Variable {
                file,
                function,
                function_start_line,
                start_byte,
                end_byte,
                ..
            },
            CpgNode::Variable {
                file: target_file,
                function: target_function,
                function_start_line: target_start,
                ..
            },
        ) = (cpg.node(from), cpg.node(to))
        else {
            return None;
        };
        let site = cpg
            .binding_site(from, to)
            .map_or((usize::MAX, usize::MAX), |site| {
                (site.start_byte, site.end_byte)
            });
        Some(Self::new(
            rule,
            relation,
            (file.clone(), function.clone(), *function_start_line),
            site,
            (*start_byte, *end_byte),
            (from, to),
            (target_file.clone(), target_function.clone(), *target_start),
            usize::MAX,
            label,
        ))
    }
}

/// Verdicts of one walk entry, shared by every root it walks.
#[derive(Debug, Default)]
pub(super) struct ProofCache {
    verdicts: BTreeMap<ProofKey, Verdict>,
}

/// What the walk hands a proof: the parsed-file view (absent for graph-only entries), the
/// hop's edge label and the cache.
pub(super) struct ProofCtx<'a> {
    pub view: Option<&'a dyn SameLineOrderView>,
    pub label: Option<FlowConfidence>,
    pub cache: &'a mut ProofCache,
}

impl ProofCtx<'_> {
    /// The view, when it holds the parsed file of both endpoints.
    pub(super) fn evidence(
        &self,
        cpg: &CodePropertyGraph,
        from: NodeIndex,
        to: NodeIndex,
    ) -> Option<&dyn SameLineOrderView> {
        let view = self.view?;
        [from, to]
            .into_iter()
            .all(|node| view.ledger_file(cpg.node(node).file()).is_some())
            .then_some(view)
    }
}

/// Rule `rule`'s verdict for the crossing `from -> to`, uncached.
pub(super) fn prove_rule(
    rule: Rule,
    cpg: &CodePropertyGraph,
    view: &dyn SameLineOrderView,
    from: NodeIndex,
    to: NodeIndex,
) -> Verdict {
    match rule {
        Rule::I => occurrence::prove_occurrence(cpg, view, from, to),
        Rule::Ii => match occurrence::inspect(cpg, view, from, to) {
            occurrence::Outcome::Site(cert) => emission::prove_emission(cpg, view, &cert),
            occurrence::Outcome::Multiple => {
                Verdict::Refused(LossReason::AmbiguousSite, "indistinguishable_emissions")
            }
            occurrence::Outcome::Refused(_) => {
                Verdict::Refused(LossReason::NoMatchingSite, "no_compatible_emission")
            }
            occurrence::Outcome::Unknown(family) => Verdict::Unknown(family),
        },
        _ => panic!("walk proof rule `{}` is not implemented", rule.name()),
    }
}

/// [`prove_rule`] through the per-walk cache; `Unknown` when no view holds both files.
pub(super) fn prove(
    rule: Rule,
    cpg: &CodePropertyGraph,
    ctx: &mut ProofCtx<'_>,
    relation: Relation,
    from: NodeIndex,
    to: NodeIndex,
) -> Verdict {
    let Some(view) = ctx.evidence(cpg, from, to) else {
        return Verdict::Unknown("no_view");
    };
    let Some(key) = ProofKey::of_hop(cpg, rule, relation, ctx.label, from, to) else {
        return Verdict::Unknown("not_variable");
    };
    if let Some(verdict) = ctx.cache.verdicts.get(&key) {
        return verdict.clone();
    }
    let verdict = prove_rule(rule, cpg, view, from, to);
    ctx.cache.verdicts.insert(key, verdict.clone());
    verdict
}
