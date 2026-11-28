//! The E2 route: credit an indexed parameter call with the callables its admitted
//! callers supplied. The index is computed once with this step inert (it is empty
//! while `compute` runs), so a caller value never rides on another site's route.

use super::ArgOutcome;
use crate::call_graph::{CallGraph, CallSite};
use crate::resolution::{
    DropReason, ResolutionConfidence, ResolutionKind, ResolutionOutcome, ResolvedCallee,
};

impl CallGraph {
    /// Only an empty `UnknownName` or `LocalBindingUnproven` outcome changes, and only
    /// for a site the index holds (JS-family Source call or JSX occurrence whose callee
    /// is a holder parameter). `Targets` becomes NameOnly `ArgumentFollow`, `Fanout`
    /// drops as `ArgFanout`, `Empty` keeps the original drop.
    pub(crate) fn js_ts_argument_follow<'a>(
        &'a self,
        site: &CallSite,
        outcome: ResolutionOutcome<'a>,
    ) -> ResolutionOutcome<'a> {
        if !outcome.resolved.is_empty()
            || !matches!(
                outcome.drop,
                Some(DropReason::UnknownName | DropReason::LocalBindingUnproven)
            )
        {
            return outcome;
        }
        match self.js_anonymous.arg_follow.lookup(site) {
            Some(ArgOutcome::Targets(targets)) => {
                let resolved: Vec<_> = targets
                    .iter()
                    .filter_map(|target| {
                        self.functions
                            .get(&target.name)?
                            .iter()
                            .find(|candidate| *candidate == target)
                    })
                    .map(|target| ResolvedCallee {
                        target,
                        confidence: ResolutionConfidence::NameOnly,
                        kind: ResolutionKind::ArgumentFollow,
                    })
                    .collect();
                // A target the graph no longer holds is not credited.
                if resolved.len() == targets.len() {
                    ResolutionOutcome::hit(resolved)
                } else {
                    outcome
                }
            }
            Some(ArgOutcome::Fanout(_)) => ResolutionOutcome::dropped(DropReason::ArgFanout),
            Some(ArgOutcome::Empty(_)) | None => outcome,
        }
    }
}
