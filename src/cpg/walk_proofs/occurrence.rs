//! Rule (i), occurrence: the bytes the walk reached must be an ordinary actual of the call that
//! the walk's descent names. A receiver, callee or keyword token, bytes inside a function
//! literal that is not the owner the graph attributes them to, and an argument of a nested
//! call (which supplies the nested call's result, not the wider call's actual) are refused.

use petgraph::graph::NodeIndex;

use super::syntax::{actual_sites, span, unwrap_wrappers, ActualFailure, OwnerId};
use super::{SiteCert, Verdict};
use crate::cpg::{CodePropertyGraph, CpgNode, LossReason, SameLineOrderView, VarAccess};

/// What the reached bytes and the call graph say about the descent's site.
pub(super) enum Outcome {
    /// Exactly one call consumes the bytes as an actual, and the call graph lists that very
    /// call as a site of the target.
    Site(SiteCert),
    /// Provably no call of the target consumes the bytes as an actual.
    Refused(&'static str),
    Unknown(&'static str),
    /// Several arguments of one call cover the bytes.
    Multiple,
}

pub(super) fn inspect(
    cpg: &CodePropertyGraph,
    view: &dyn SameLineOrderView,
    from: NodeIndex,
    to: NodeIndex,
) -> Outcome {
    let (
        CpgNode::Variable {
            file,
            function,
            function_start_line,
            start_byte,
            end_byte,
            access: VarAccess::Use,
            ..
        },
        CpgNode::Variable {
            file: target_file,
            function: target_function,
            function_start_line: target_start,
            access: VarAccess::Def,
            ..
        },
    ) = (cpg.node(from), cpg.node(to))
    else {
        return Outcome::Unknown("not_use_to_def");
    };
    let Some(parsed) = view.ledger_file(file) else {
        return Outcome::Unknown("no_source");
    };
    let caller: OwnerId = (file.clone(), function.clone(), *function_start_line);
    let actual = (*start_byte, *end_byte);
    let sites = match actual_sites(parsed, actual, &caller) {
        Ok(sites) => sites,
        Err(ActualFailure::Refused(family)) => return Outcome::Refused(family),
        Err(ActualFailure::Unbound(family)) => return Outcome::Unknown(family),
    };
    let [(call, slot, arg)] = sites.as_slice() else {
        return Outcome::Multiple;
    };
    let call_span = span(*call);
    let listed: Vec<(usize, usize)> = cpg
        .call_graph
        .callers
        .get(target_function)
        .into_iter()
        .flatten()
        .filter(|site| {
            site.caller.file == *file
                && site.caller.name == *function
                && site.caller.start_line == *function_start_line
        })
        .map(|site| (site.start_byte, site.end_byte))
        .collect();
    if !listed.contains(&call_span) {
        // The listed site of the target is a call that wraps this one: the bytes supply the
        // inner call's actual only.
        let wraps = listed.iter().any(|&(start, end)| {
            start <= call_span.0 && call_span.1 <= end && (start, end) != call_span
        });
        return if !wraps {
            Outcome::Unknown("target_or_site_unbound")
        } else if let Some(family) = inner_site_undecided(
            cpg,
            &caller,
            call_span,
            &(target_file.clone(), target_function.clone(), *target_start),
        ) {
            Outcome::Unknown(family)
        } else {
            Outcome::Refused("outer_site_only")
        };
    }
    let exact = span(*arg) == actual || unwrap_wrappers(Some(*arg)).map(span) == Some(actual);
    Outcome::Site(SiteCert {
        file: file.clone(),
        caller,
        site: call_span,
        actual,
        slot: *slot,
        target: (target_file.clone(), target_function.clone(), *target_start),
        family: if exact {
            "ordinary_actual"
        } else {
            "nested_value_operand"
        },
    })
}

/// The index lists a target's sites under the spelling each call was written with, so a wider
/// call listed under the target's own name does not show what the inner call is. The inner site
/// is proven not to be a site of the target only when the call graph holds it at exactly these
/// bytes in the same caller, spelled with the target's own name or resolving to other callees;
/// an inner call spelled otherwise that resolves to the target (an import alias) or to nothing
/// (a builtin, a conversion, an unfollowed alias) is undecided.
fn inner_site_undecided(
    cpg: &CodePropertyGraph,
    caller: &OwnerId,
    call_span: (usize, usize),
    target: &OwnerId,
) -> Option<&'static str> {
    let inner = cpg
        .call_graph
        .calls
        .iter()
        .filter(|(id, _)| (&id.file, &id.name, id.start_line) == (&caller.0, &caller.1, caller.2))
        .flat_map(|(_, sites)| sites)
        .find(|site| (site.start_byte, site.end_byte) == call_span);
    let Some(inner) = inner else {
        return Some("inner_site_unbound");
    };
    if inner.callee_name == target.1 {
        return None;
    }
    let resolved = cpg.call_graph.resolve_call_site_full(inner).resolved;
    if resolved.is_empty() {
        Some("inner_site_unresolved")
    } else if resolved.iter().any(|entry| {
        (
            &entry.target.file,
            &entry.target.name,
            entry.target.start_line,
        ) == (&target.0, &target.1, target.2)
    }) {
        Some("inner_site_other_spelling")
    } else {
        None
    }
}

pub(super) fn prove_occurrence(
    cpg: &CodePropertyGraph,
    view: &dyn SameLineOrderView,
    from: NodeIndex,
    to: NodeIndex,
) -> Verdict {
    match inspect(cpg, view, from, to) {
        Outcome::Site(cert) => Verdict::Proved(cert),
        Outcome::Refused(family) => Verdict::Refused(LossReason::NoMatchingSite, family),
        Outcome::Unknown(family) => Verdict::Unknown(family),
        Outcome::Multiple => Verdict::Unknown("multiple_actuals"),
    }
}
