//! Slice R (owner decision 2026-10-13): a bound name whose value is unknown gets no name
//! search. A JS/TS bare-identifier callee whose nearest binding is an in-file declaration with
//! an unproven value (`JsLocalBinding::MayCall`) is not resolved by spelling: the repo-wide
//! rung is not consulted (rule 1), and a same-file function of that spelling is kept only when
//! it lies inside a value written to the binding (rule 2). Both drop with
//! `LocalBindingUnproven`; neither adds an edge or a NameOnly candidate.
use super::{exact, DropReason, FunctionId, ResolutionKind, ResolutionOutcome};
use crate::call_graph::JsLocalBinding;

/// Rule 2: the same-file free functions `local` of the site's spelling that belong to its
/// `MayCall` binding. `None` when the binding is not `MayCall` (the caller keeps its own
/// outcome); otherwise the kept functions as an Exact `LocalDef` hit, or the drop when none is
/// part of a written value.
pub(super) fn js_ts_may_call_local<'a>(
    binding: &JsLocalBinding,
    local: &[&'a FunctionId],
) -> Option<ResolutionOutcome<'a>> {
    let JsLocalBinding::MayCall { values, .. } = binding else {
        return None;
    };
    let kept: Vec<&'a FunctionId> = local
        .iter()
        .copied()
        .filter(|f| {
            values
                .iter()
                .any(|&(start, end)| f.start_line >= start && f.end_line <= end)
        })
        .collect();
    Some(if kept.is_empty() {
        ResolutionOutcome::dropped(DropReason::LocalBindingUnproven)
    } else {
        ResolutionOutcome::hit(exact(kept, ResolutionKind::LocalDef))
    })
}

/// Rule 1: a `MayCall` binding never reaches the repo-wide rung.
pub(super) fn js_ts_may_call_refuses_repo_wide(binding: &JsLocalBinding) -> bool {
    matches!(binding, JsLocalBinding::MayCall { .. })
}
