//! Private routing and authority borrowed from one captured review.
use super::{
    context_options::TaintContextMode,
    context_run::{ContextRequest, ContextState},
    context_view::ReviewAlgorithmRun,
    run::{run_algorithm_prepared, run_review_routed, AlgorithmParams, ReviewRun},
    AlgorithmOutcome, ReviewBoundaryDiagnostics, ReviewInputs,
};
use crate::algorithms::taint::boundary::records::{ContextRunNote, ContextRunNoteKind};
use prism::{
    cpg::CpgContext,
    slice::{SliceConfig, SlicingAlgorithm},
};
use std::path::Path;

#[non_exhaustive]
pub(crate) struct ReviewAuthority<'a> {
    inputs: &'a ReviewInputs,
}
impl<'a> ReviewAuthority<'a> {
    pub(crate) fn capture(inputs: &'a ReviewInputs) -> Self {
        Self { inputs }
    }
    pub(crate) fn inputs(&self) -> &'a ReviewInputs {
        self.inputs
    }
}
pub(crate) enum Route<'a> {
    Plain,
    Ignored,
    Unauthorized,
    Authorized(&'a ContextRequest),
}
pub(crate) fn decide<'a>(
    request: Option<&'a ContextRequest>,
    taint: bool,
    authorized: bool,
) -> Route<'a> {
    match request {
        None => Route::Plain,
        Some(_) if !taint => Route::Ignored,
        Some(_) if !authorized => Route::Unauthorized,
        Some(request) => Route::Authorized(request),
    }
}
const IGNORED: &str = "taint-context ignored: taint not selected";
const UNAUTHORIZED: &str = "taint-context: imports requested through an entry point without loading authority; baseline taint only (use run_review_authorized or run_algorithm_authorized)";
fn append_note(
    diagnostics: &mut Option<ReviewBoundaryDiagnostics>,
    kind: ContextRunNoteKind,
    text: &str,
) {
    diagnostics
        .get_or_insert_with(ReviewBoundaryDiagnostics::default)
        .run_notes
        .push(ContextRunNote {
            kind,
            message: text.into(),
            location: None,
        });
}
fn append_multi(run: &mut ReviewRun, kind: ContextRunNoteKind, text: &str) {
    append_note(&mut run.diagnostics, kind, text);
    run.warnings.push(text.into());
}
fn invalid(error: super::context_options::ContextOptionError) -> anyhow::Error {
    anyhow::anyhow!("invalid taint-context options: {error}")
}
fn single(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext<'_>,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    authorized: bool,
) -> anyhow::Result<(AlgorithmOutcome, ContextState)> {
    let request = params.taint_context.validate().map_err(invalid)?;
    let route = decide(
        request.as_ref(),
        algorithm == SlicingAlgorithm::Taint,
        authorized,
    );
    let request = match &route {
        Route::Authorized(request) => Some(*request),
        _ => None,
    };
    let (mut outcome, state) =
        run_algorithm_prepared(algorithm, ctx, inputs, config, params, repo, request)?;
    match route {
        Route::Ignored => append_note(
            &mut outcome.diagnostics,
            ContextRunNoteKind::Ignored,
            IGNORED,
        ),
        Route::Unauthorized => append_note(
            &mut outcome.diagnostics,
            ContextRunNoteKind::Unauthorized,
            UNAUTHORIZED,
        ),
        _ => {}
    }
    Ok((outcome, state))
}
pub(crate) fn run_algorithm_legacy(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext<'_>,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> anyhow::Result<AlgorithmOutcome> {
    single(algorithm, ctx, inputs, config, params, repo, false).map(|(outcome, _)| outcome)
}
pub(crate) fn run_algorithm_authorized(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext<'_>,
    authority: ReviewAuthority<'_>,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> anyhow::Result<ReviewAlgorithmRun> {
    let (outcome, state) = single(
        algorithm,
        ctx,
        authority.inputs(),
        config,
        params,
        repo,
        true,
    )?;
    Ok(ReviewAlgorithmRun::new(outcome, state))
}
pub(crate) fn run_review_validated(
    ctx: &CpgContext<'_>,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    request: Option<&ContextRequest>,
    authorized: bool,
) -> ReviewRun {
    let selected = algorithms.contains(&SlicingAlgorithm::Taint);
    let route = decide(request, selected, authorized);
    let effective = match &route {
        Route::Authorized(request) => Some(*request),
        _ => None,
    };
    let mut run = run_review_routed(
        ctx, inputs, algorithms, config, params, repo, effective, None,
    );
    match route {
        Route::Ignored => append_multi(&mut run, ContextRunNoteKind::Ignored, IGNORED),
        Route::Unauthorized => {
            append_multi(&mut run, ContextRunNoteKind::Unauthorized, UNAUTHORIZED)
        }
        _ => {}
    }
    run
}
fn multi(
    ctx: &CpgContext<'_>,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    authorized: bool,
) -> ReviewRun {
    match params.taint_context.validate() {
        Ok(request) => run_review_validated(
            ctx,
            inputs,
            algorithms,
            config,
            params,
            repo,
            request.as_ref(),
            authorized,
        ),
        Err(error) if algorithms.contains(&SlicingAlgorithm::Taint) => {
            let error = invalid(error).to_string();
            run_review_routed(
                ctx,
                inputs,
                algorithms,
                config,
                params,
                repo,
                None,
                Some(&error),
            )
        }
        Err(_) => {
            let mut run =
                run_review_routed(ctx, inputs, algorithms, config, params, repo, None, None);
            if params.taint_context.mode == TaintContextMode::Imports {
                append_multi(&mut run, ContextRunNoteKind::Ignored, IGNORED);
            }
            run
        }
    }
}
pub(crate) fn run_review_legacy(
    ctx: &CpgContext<'_>,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> ReviewRun {
    multi(ctx, inputs, algorithms, config, params, repo, false)
}
pub(crate) fn run_review_authorized(
    ctx: &CpgContext<'_>,
    authority: ReviewAuthority<'_>,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> ReviewRun {
    multi(
        ctx,
        authority.inputs(),
        algorithms,
        config,
        params,
        repo,
        true,
    )
}
pub(crate) fn validate_for_review(
    params: &AlgorithmParams,
) -> anyhow::Result<Option<ContextRequest>> {
    params.taint_context.validate().map_err(invalid)
}
#[cfg(test)]
#[path = "context_route_tests.rs"]
mod tests;
