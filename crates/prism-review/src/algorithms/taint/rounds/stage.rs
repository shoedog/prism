use super::super::boundary::collect::collect_round;
use super::super::context_budget::{admit_prefix, report_from_attempt};
use super::super::frontier::plan_round;
use super::super::source_scope::admission::slice_in_context;
use super::*;
use prism::api::ReviewContextRound;
use prism::cpg::{CpgContext, TaintCrossingPolicy};

pub(super) struct Staged {
    pub(super) bundle: CompletedBundle,
    pub(super) bound: Option<AdmissionBound>,
}
#[derive(Debug)]
pub(super) enum StageStop {
    Guard(StopCause),
    Failed(String),
    NothingAccepted(Option<AdmissionBound>),
}
fn guard(request: &ExpansionRequest<'_>) -> Result<(), StageStop> {
    match request.guard.stop() {
        Some(cause) => Err(StageStop::Guard(cause)),
        None => Ok(()),
    }
}
pub(super) fn run_stage(
    request: &ExpansionRequest<'_>,
    previous: &CompletedBundle,
    attempts: &mut AttemptCounters,
) -> Result<Staged, StageStop> {
    let admission = admit_prefix(
        &previous.frontier.load_order,
        &request.limits,
        previous.published,
        attempts,
        request.guard,
        &mut |path, remaining| report_from_attempt(request.loader.read(path, remaining)),
    );
    if let Some(cause) = admission.stopped {
        return Err(StageStop::Guard(cause));
    }
    let loaded_continuation = previous
        .frontier
        .open
        .iter()
        .any(|r| previous.files.contains_key(&r.target));
    if admission.accepted.is_empty() && admission.bound.is_some() {
        return Err(StageStop::NothingAccepted(admission.bound));
    }
    if admission.accepted.is_empty() && !loaded_continuation {
        return Err(StageStop::NothingAccepted(None));
    }
    // This is the first stage guard when the admission loop performs no reads.
    guard(request)?;
    let accepted_count = admission.accepted.len();
    let mut files = previous.files.clone();
    let mut sources = previous.sources.clone();
    let mut parse_quality = previous.parse_quality.clone();
    let mut parse_warnings = previous.parse_warnings.clone();
    let mut context_files = previous.context_files.clone();
    for (target, file) in admission.accepted {
        if target != file.path || files.contains_key(&target) {
            return Err(StageStop::Failed(format!(
                "invalid admitted identity: {target}"
            )));
        }
        context_files.insert(target.clone());
        sources.insert(target.clone(), file.source);
        files.insert(target.clone(), file.parsed);
        if let Some(quality) = file.parse_quality {
            parse_quality.insert(target, quality);
        }
        parse_warnings.extend(file.parse_warnings);
    }
    let published = Published {
        files: previous.published.files + accepted_count,
        bytes: previous.published.bytes + admission.accepted_bytes,
    };
    let refused: BTreeSet<_> = attempts
        .skips
        .iter()
        .map(|(path, _)| path.clone())
        .collect();
    let original_files: BTreeSet<_> = request.inputs.files.keys().cloned().collect();
    guard(request)?;
    let (analysis, frontier) = {
        let round = ReviewContextRound::new(request.loader, &files);
        guard(request)?;
        let ctx = round.build_context(request.inputs.type_db.as_ref(), &request.baseline_ctx.types);
        guard(request)?;
        let policy = TaintCrossingPolicy::new(&context_files, request.ceiling);
        let analysis = slice_in_context(
            &ctx,
            &request.inputs.diff,
            request.config,
            &original_files,
            &policy,
        )
        .map_err(|error| StageStop::Failed(error.to_string()))?;
        guard(request)?;
        let frontier = plan_completed(request, previous, &round, &ctx, &analysis, &refused)?;
        guard(request)?;
        (analysis, frontier)
    };
    Ok(Staged {
        bundle: CompletedBundle {
            round: previous.round + 1,
            files,
            sources,
            parse_quality,
            parse_warnings,
            context_files,
            published,
            analysis: Some(analysis),
            frontier,
        },
        bound: admission.bound,
    })
}

/// Plan the next frontier from the loading projection of this completed analysis.
pub(super) fn plan_completed(
    request: &ExpansionRequest<'_>,
    previous: &CompletedBundle,
    round: &ReviewContextRound<'_>,
    ctx: &CpgContext<'_>,
    analysis: &ScopedTaintOutcome,
    refused: &BTreeSet<String>,
) -> Result<FrontierState, StageStop> {
    let mut hints = round
        .hints()
        .map_err(|error| StageStop::Failed(error.to_string()))?;
    let collection = collect_round(&mut hints, ctx, &analysis.loading);
    Ok(plan_round(
        previous.round + 2,
        request.limits.depth,
        &collection,
        ctx,
        &previous.frontier,
        refused,
        &mut |barrel, member| {
            hints
                .follow_barrel(barrel, member)
                .map(|hop| (hop.file, hop.member))
        },
    ))
}
