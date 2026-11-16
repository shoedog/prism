use super::super::boundary::collect::collect_round;
use super::super::frontier::{plan_round, Disposition};
use super::super::source_scope::loading::project_loading_trace;
use super::stage::{run_stage, StageStop, Staged};
use super::*;
use prism::api::ReviewBoundaryHints;

pub(super) fn settle(
    previous: CompletedBundle,
    staged: Result<Staged, StageStop>,
) -> (CompletedBundle, Option<Stop>) {
    match staged {
        Ok(staged) => (staged.bundle, staged.bound.map(Stop::Bound)),
        Err(StageStop::NothingAccepted(Some(bound))) => (previous, Some(Stop::Bound(bound))),
        Err(StageStop::NothingAccepted(None)) => (previous, Some(Stop::FrontierExhausted)),
        Err(StageStop::Guard(cause)) => (previous, Some(Stop::Guard(cause))),
        Err(StageStop::Failed(message)) => (previous, Some(Stop::StageFailed(message))),
    }
}
pub(super) fn terminal(bundle: &CompletedBundle, depth: u8) -> Stop {
    if bundle.round >= depth
        && bundle
            .frontier
            .dispositions
            .iter()
            .any(|(_, _, disposition)| *disposition == Disposition::DepthBound)
    {
        Stop::DepthReached
    } else {
        Stop::FrontierExhausted
    }
}
pub(crate) fn expand(request: ExpansionRequest<'_>) -> ExpansionOutcome {
    let mut bundle = CompletedBundle {
        round: 0,
        files: request.inputs.files.clone(),
        sources: request.inputs.sources.clone(),
        parse_quality: request.inputs.parse_quality.clone(),
        parse_warnings: request.inputs.parse_warnings.clone(),
        context_files: BTreeSet::new(),
        published: Published::default(),
        analysis: None,
        frontier: FrontierState::default(),
    };
    let attempts = AttemptCounters::default();
    let stats = RoundStats::default();
    let originals: BTreeSet<_> = request.inputs.files.keys().cloned().collect();
    let mut hints = match ReviewBoundaryHints::new(request.inputs) {
        Ok(hints) => hints,
        Err(error) => {
            return ExpansionOutcome {
                bundle,
                stop: Stop::StageFailed(error.to_string()),
                attempts,
                stats,
            }
        }
    };
    let loading = project_loading_trace(request.baseline_trace, &originals, &request.inputs.files);
    let collection = collect_round(&mut hints, request.baseline_ctx, &loading);
    bundle.frontier = plan_round(
        1,
        request.limits.depth,
        &collection,
        request.baseline_ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    );
    drive(&request, bundle, attempts, stats)
}

pub(super) fn drive(
    request: &ExpansionRequest<'_>,
    mut bundle: CompletedBundle,
    mut attempts: AttemptCounters,
    mut stats: RoundStats,
) -> ExpansionOutcome {
    while bundle.round < request.limits.depth {
        let loaded_continuation = bundle
            .frontier
            .open
            .iter()
            .any(|r| bundle.files.contains_key(&r.target));
        if bundle.frontier.load_order.is_empty() && !loaded_continuation {
            break;
        }
        stats.rounds_started += 1;
        let staged = run_stage(request, &bundle, &mut attempts);
        let completed = staged.is_ok();
        let (next, stop) = settle(bundle, staged);
        bundle = next;
        if completed {
            stats.rounds_completed += 1;
        }
        if let Some(stop) = stop {
            return ExpansionOutcome {
                bundle,
                stop,
                attempts,
                stats,
            };
        }
    }
    let stop = terminal(&bundle, request.limits.depth);
    ExpansionOutcome {
        bundle,
        stop,
        attempts,
        stats,
    }
}
