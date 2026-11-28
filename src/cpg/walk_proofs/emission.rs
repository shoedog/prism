//! Rule (ii), emission: the call graph de-duplicates the edge a call emits, so the edge carries
//! no site of its own. A certified site is therefore the only candidate emitter only when no
//! wider call of the same spelling also takes the reached bytes as its actual in the same slot
//! (`g(g(t))`: the outer call can emit the same edge). Two disjoint calls on one line are two
//! separate byte ranges and cannot stand for each other.

use super::syntax::{
    ancestors, args, call_at, covers, is_call, owner, same_line_peers, spelling_checked,
};
use super::{SiteCert, Verdict};
use crate::cpg::{CodePropertyGraph, LossReason, SameLineOrderView};

pub(super) fn prove_emission(
    _cpg: &CodePropertyGraph,
    view: &dyn SameLineOrderView,
    cert: &SiteCert,
) -> Verdict {
    let Some(parsed) = view.ledger_file(&cert.file) else {
        return Verdict::Unknown("no_source");
    };
    let Some(call) = call_at(parsed, cert.site) else {
        return Verdict::Unknown("emission_unbound");
    };
    let function = owner(Some(call));
    let Ok(label) = spelling_checked(parsed, call, function) else {
        return Verdict::Unknown("alias_scope_unknown");
    };
    let mut competes = false;
    for outer in ancestors(call.parent()).take_while(|outer| Some(*outer) != function) {
        if !is_call(outer) {
            continue;
        }
        match spelling_checked(parsed, outer, function) {
            Ok(spelled) if spelled == label => {}
            Ok(_) => continue,
            Err(family) => return Verdict::Unknown(family),
        }
        let arguments = args(outer);
        competes |= arguments.iter().any(|arg| covers(*arg, cert.actual))
            && arguments
                .get(cert.slot)
                .is_some_and(|arg| covers(*arg, cert.actual));
    }
    if competes {
        return Verdict::Refused(LossReason::AmbiguousSite, "nested_same_spelling");
    }
    let peers = !same_line_peers(parsed, call, function).is_empty();
    Verdict::Proved(SiteCert {
        family: if peers {
            "disjoint_same_line_calls"
        } else {
            "one_exact_occurrence"
        },
        ..cert.clone()
    })
}
