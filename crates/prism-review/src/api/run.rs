use super::context_run::{ContextRequest, ContextState, RunExpansion};
use super::diagnostics::RunDiagnostics;
use crate::algorithms;
use anyhow::{Context, Result};
use prism::api::{build_context, load_review_inputs, ReviewInputs, ReviewOptions};
use prism::ast::ParsedFile;
use prism::cpg::CpgContext;
use prism::finding_confidence::{
    admit_finding_for_resolution, parse_quality_for_selected_evidence, EvidencePath, MinConfidence,
    ResolutionMode,
};
use prism::slice::{AlgorithmError, SliceConfig, SliceFinding, SliceResult, SlicingAlgorithm};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_BARRIER_DEPTH: usize = 2;
pub const DEFAULT_SPIRAL_MAX_RING: usize = 4;
pub const DEFAULT_TEMPORAL_DAYS: usize = 90;

#[non_exhaustive]
pub struct AlgorithmParams {
    /// Import-directed taint context. Default Off; legacy preloaded runners never load it.
    pub taint_context: super::context_options::TaintContextOptions,
    pub barrier_depth: usize,
    pub barrier_symbols: Vec<String>,
    pub chop_source: Option<String>,
    pub chop_sink: Option<String>,
    pub taint_sources: Vec<String>,
    pub taint_return_flow: bool,
    pub condition: Option<String>,
    pub old_repo: Option<PathBuf>,
    pub spiral_max_ring: usize,
    pub quantum_var: Option<String>,
    pub peer_pattern: Option<String>,
    pub layers: Option<String>,
    pub concern: Option<String>,
    pub temporal_days: usize,
}

impl Default for AlgorithmParams {
    fn default() -> Self {
        Self {
            taint_context: Default::default(),
            barrier_depth: DEFAULT_BARRIER_DEPTH,
            barrier_symbols: Vec::new(),
            chop_source: None,
            chop_sink: None,
            taint_sources: Vec::new(),
            taint_return_flow: false,
            condition: None,
            old_repo: None,
            spiral_max_ring: DEFAULT_SPIRAL_MAX_RING,
            quantum_var: None,
            peer_pattern: None,
            layers: None,
            concern: None,
            temporal_days: DEFAULT_TEMPORAL_DAYS,
        }
    }
}

/// Run a preloaded algorithm without loading taint context.
///
/// For a valid `Imports` request, taint runs on the preloaded inputs. On success,
/// `SliceResult::warnings` includes the text of an `Unauthorized` note naming
/// `run_review_authorized` and `run_algorithm_authorized` for taint, or an `Ignored`
/// note for any other algorithm.
/// Invalid taint-context options return an error before the algorithm runs.
pub fn run_algorithm(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> Result<SliceResult> {
    Ok(
        run_algorithm_with_diagnostics(algorithm, ctx, inputs, config, params, repo)?
            .into_legacy_result(),
    )
}

/// Run one preloaded algorithm with typed boundary diagnostics and no taint-context loading.
///
/// For a valid `Imports` request, taint runs on the preloaded inputs and the successful
/// outcome carries an `Unauthorized` note naming `run_review_authorized` and
/// `run_algorithm_authorized`. With another algorithm, a successful outcome carries an
/// `Ignored` note. These notes are in `AlgorithmOutcome::diagnostics`, separate from the
/// result's warnings. Invalid taint-context options return an error before the algorithm runs.
pub fn run_algorithm_with_diagnostics(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> Result<super::AlgorithmOutcome> {
    super::context_route::run_algorithm_legacy(algorithm, ctx, inputs, config, params, repo)
}

pub(crate) fn run_algorithm_prepared(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    request: Option<&ContextRequest>,
) -> Result<(
    super::AlgorithmOutcome,
    ContextState,
    Option<super::context_stats::ContextStats>,
)> {
    prism::build_pool::install(|| {
        let mut diagnostics = RunDiagnostics::default();
        let mut expansion = RunExpansion::new(request, algorithm == SlicingAlgorithm::Taint);
        let result = run_algorithm_raw(
            algorithm,
            ctx,
            inputs,
            config,
            params,
            repo,
            &mut diagnostics,
            &mut expansion,
        );
        let mut result = match result {
            Ok(result) => result,
            Err(error) => {
                if let Some(stats) = expansion.take_stats() {
                    stats.export();
                }
                return Err(error);
            }
        };
        annotate_finding_parse_quality(
            &mut result.findings,
            expansion.view(inputs, algorithm).files,
        );
        let stats = expansion.take_stats();
        if let Some(stats) = &stats {
            stats.export();
        }
        let state = expansion.into_state();
        diagnostics.reconcile_context(&state, request, &inputs.parse_warnings);
        Ok((
            super::AlgorithmOutcome {
                result,
                diagnostics: diagnostics.project(),
            },
            state,
            stats,
        ))
    })
}

fn run_algorithm_raw(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    diagnostics: &mut RunDiagnostics,
    expansion: &mut RunExpansion<'_>,
) -> Result<SliceResult> {
    let mut result = match algorithm {
        SlicingAlgorithm::BarrierSlice => {
            let barrier_config = crate::algorithms::barrier_slice::BarrierConfig {
                max_depth: params.barrier_depth,
                barrier_symbols: params.barrier_symbols.iter().cloned().collect(),
                barrier_modules: Vec::new(),
            };
            crate::algorithms::barrier_slice::slice(ctx, &inputs.diff, config, &barrier_config)
        }
        SlicingAlgorithm::Chop => {
            let source = params
                .chop_source
                .as_ref()
                .context("--chop-source required for chop algorithm")?;
            let sink = params
                .chop_sink
                .as_ref()
                .context("--chop-sink required for chop algorithm")?;
            let (sf, sl) = parse_file_line(source)?;
            let (kf, kl) = parse_file_line(sink)?;
            crate::algorithms::chop::slice(
                ctx,
                &crate::algorithms::chop::ChopConfig {
                    source_file: sf,
                    source_line: sl,
                    sink_file: kf,
                    sink_line: kl,
                },
            )
        }
        SlicingAlgorithm::Taint => {
            if let Some(result) = expansion.cached() {
                Ok(result)
            } else {
                let taint_config = crate::algorithms::taint::TaintConfig {
                    sources: params
                        .taint_sources
                        .iter()
                        .filter_map(|s| parse_file_line(s).ok())
                        .collect(),
                    taint_from_diff: params.taint_sources.is_empty(),
                    extra_sinks: Vec::new(),
                    return_flow: params.taint_return_flow,
                };
                if diagnostics.needs_collection() {
                    let (result, trace) = crate::algorithms::taint::slice_with_trace(
                        ctx,
                        &inputs.diff,
                        &taint_config,
                    )?;
                    diagnostics.capture_once(|| {
                        crate::algorithms::taint::boundary::collect::collect(inputs, ctx, &trace)
                    });
                    Ok(expansion.apply(inputs, ctx, &taint_config, result, &trace))
                } else if expansion.requested() {
                    let (result, trace) = crate::algorithms::taint::slice_with_trace(
                        ctx,
                        &inputs.diff,
                        &taint_config,
                    )?;
                    Ok(expansion.apply(inputs, ctx, &taint_config, result, &trace))
                } else {
                    crate::algorithms::taint::slice(ctx, &inputs.diff, &taint_config)
                }
            }
        }
        SlicingAlgorithm::ConditionedSlice => {
            let cond_str = params
                .condition
                .as_ref()
                .context("--condition required for conditioned algorithm")?;
            let condition = crate::algorithms::conditioned_slice::Condition::parse(cond_str)
                .context(format!("Failed to parse condition: {}", cond_str))?;
            crate::algorithms::conditioned_slice::slice(&ctx, &inputs.diff, config, &condition)
        }
        SlicingAlgorithm::DeltaSlice => {
            let old_repo = params
                .old_repo
                .as_ref()
                .context("--old-repo required for delta algorithm")?;
            crate::algorithms::delta_slice::slice(ctx, &inputs.diff, old_repo)
        }
        SlicingAlgorithm::SpiralSlice => {
            let spiral_config = crate::algorithms::spiral_slice::SpiralConfig {
                max_ring: params.spiral_max_ring,
                auto_stop_threshold: 0.05,
            };
            crate::algorithms::spiral_slice::slice(ctx, &inputs.diff, config, &spiral_config)
        }
        SlicingAlgorithm::QuantumSlice => crate::algorithms::quantum_slice::slice(
            ctx.files,
            &inputs.diff,
            params.quantum_var.as_deref(),
        ),
        SlicingAlgorithm::HorizontalSlice => {
            let pattern = match params.peer_pattern.as_deref() {
                Some(p) if p.starts_with("decorator:") => {
                    crate::algorithms::horizontal_slice::PeerPattern::Decorator(
                        p.strip_prefix("decorator:").unwrap().to_string(),
                    )
                }
                Some(p) if p.starts_with("name:") => {
                    crate::algorithms::horizontal_slice::PeerPattern::NamePattern(
                        p.strip_prefix("name:").unwrap().to_string(),
                    )
                }
                Some(p) if p.starts_with("class:") => {
                    crate::algorithms::horizontal_slice::PeerPattern::ParentClass(
                        p.strip_prefix("class:").unwrap().to_string(),
                    )
                }
                _ => crate::algorithms::horizontal_slice::PeerPattern::Auto,
            };
            crate::algorithms::horizontal_slice::slice(ctx.files, &inputs.diff, &pattern)
        }
        SlicingAlgorithm::VerticalSlice => {
            let vertical_config = crate::algorithms::vertical_slice::VerticalConfig {
                layers: params
                    .layers
                    .as_deref()
                    .map(|l| l.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default(),
            };
            crate::algorithms::vertical_slice::slice(ctx, &inputs.diff, &vertical_config)
        }
        SlicingAlgorithm::AngleSlice => {
            let concern = params
                .concern
                .as_deref()
                .map(crate::algorithms::angle_slice::Concern::from_str)
                .unwrap_or(crate::algorithms::angle_slice::Concern::ErrorHandling);
            crate::algorithms::angle_slice::slice(ctx.files, &inputs.diff, &concern)
        }
        SlicingAlgorithm::ThreeDSlice => {
            let threed_config = crate::algorithms::threed_slice::ThreeDConfig {
                temporal_days: params.temporal_days,
                git_dir: repo.to_string_lossy().to_string(),
            };
            crate::algorithms::threed_slice::slice(ctx, &inputs.diff, &threed_config)
        }
        SlicingAlgorithm::ResonanceSlice => {
            let resonance_config = crate::algorithms::resonance_slice::ResonanceConfig {
                git_dir: repo.to_string_lossy().to_string(),
                days: params.temporal_days,
                ..Default::default()
            };
            crate::algorithms::resonance_slice::slice(ctx.files, &inputs.diff, &resonance_config)
        }
        SlicingAlgorithm::PhantomSlice => {
            let phantom_config = crate::algorithms::phantom_slice::PhantomConfig {
                git_dir: repo.to_string_lossy().to_string(),
                ..Default::default()
            };
            crate::algorithms::phantom_slice::slice(ctx.files, &inputs.diff, &phantom_config)
        }
        SlicingAlgorithm::ContractSlice => {
            if let Some(old_repo) = &params.old_repo {
                crate::algorithms::contract_slice::slice_delta(ctx.files, &inputs.diff, old_repo)
            } else {
                crate::algorithms::contract_slice::slice(ctx.files, &inputs.diff)
            }
        }
        // Fallback: use run_slicing_inner (not run_slicing) so that the
        // finalize_diagrams call below is the single owner.  run_slicing
        // would finalize and then the call below would finalize again,
        // duplicating all diagram warnings.
        _ => algorithms::run_slicing_inner(ctx, &inputs.diff, config),
    }?;
    result.align_evidence();
    crate::algorithms::finalize_diagrams(&mut result, config.diagram_node_cap);
    Ok(result)
}

pub fn annotate_finding_parse_quality(
    findings: &mut [SliceFinding],
    files: &BTreeMap<String, ParsedFile>,
) {
    for finding in findings.iter_mut() {
        if let Some(pf) = files.get(&finding.file) {
            let rate = pf.error_rate();
            if rate > 0.01 {
                let q = if rate > 0.3 {
                    "unparseable"
                } else if rate > 0.1 {
                    "poor"
                } else {
                    "degraded"
                };
                finding.parse_quality = Some(q.to_string());
            }
        }
    }
}

fn parse_file_line(s: &str) -> Result<(String, usize)> {
    let parts: Vec<&str> = s.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Expected file:line format, got: {}", s);
    }
    let line: usize = parts[0]
        .parse()
        .context(format!("Invalid line number: {}", parts[0]))?;
    Ok((parts[1].to_string(), line))
}

pub fn parse_algorithms(spec: &str) -> Result<Vec<SlicingAlgorithm>> {
    match spec.to_lowercase().as_str() {
        "review" => Ok(SlicingAlgorithm::review_suite()),
        "all" => Ok(SlicingAlgorithm::all()),
        multi if multi.contains(',') => {
            let mut algos = Vec::new();
            for part in multi.split(',') {
                let part = part.trim();
                let algo = SlicingAlgorithm::from_str(part).context(format!(
                    "Unknown algorithm: {}. Use --list-algorithms to see options.",
                    part
                ))?;
                algos.push(algo);
            }
            Ok(algos)
        }
        single => {
            let algo = SlicingAlgorithm::from_str(single).context(format!(
                "Unknown algorithm: {}. Use --list-algorithms to see options.",
                spec
            ))?;
            Ok(vec![algo])
        }
    }
}

/// Retain the index-aligned finding/evidence pairs admitted by the selected
/// emitter confidence floor. The default is an exact no-op, including for a
/// malformed evidence vector, so legacy output and mismatch diagnostics keep
/// their existing shape.
pub fn filter_result_findings(
    result: &mut SliceResult,
    inputs: &ReviewInputs,
    min_confidence: MinConfidence,
    resolution: ResolutionMode,
) {
    filter_result_findings_in(
        result,
        crate::api::map_view::MapView::original(inputs),
        min_confidence,
        resolution,
    );
}

pub(crate) fn filter_result_findings_in(
    result: &mut SliceResult,
    view: crate::api::map_view::MapView<'_>,
    min_confidence: MinConfidence,
    resolution: ResolutionMode,
) {
    if min_confidence == MinConfidence::NameOnly {
        return;
    }

    let findings = std::mem::take(&mut result.findings);
    let evidence = std::mem::take(&mut result.evidence);
    for (index, finding) in findings.into_iter().enumerate() {
        let selected_evidence = evidence.get(index).and_then(Option::as_ref);
        let parse_quality = parse_quality_for_selected_evidence(
            &finding,
            selected_evidence,
            view.parse_quality,
            view.files,
        );
        let admitted = admit_finding_for_resolution(
            &finding.algorithm,
            parse_quality,
            selected_evidence,
            min_confidence,
            resolution,
        );
        if admitted.is_some() {
            result.findings.push(finding);
            result
                .evidence
                .push(evidence.get(index).cloned().unwrap_or(None));
        }
    }
}

/// Multi-algorithm output in the legacy-compatible facade shape.
///
/// `findings` is the annotated set; `results[*].findings` are the algorithms' raw findings
/// (legacy shape). Classify via `finding_confidence::parse_quality_for` with the authoritative
/// map, never via `finding.parse_quality`.
#[non_exhaustive]
pub struct ReviewRun {
    pub(crate) context: ContextState,
    pub(crate) context_stats: Option<super::context_stats::ContextStats>,
    pub diagnostics: Option<super::ReviewBoundaryDiagnostics>,
    pub results: Vec<SliceResult>,
    pub findings: Vec<SliceFinding>,
    pub evidence: Vec<Option<EvidencePath>>,
    pub errors: Vec<AlgorithmError>,
    pub warnings: Vec<String>,
    pub algorithms_run: Vec<String>,
    pub min_confidence: MinConfidence,
    pub resolution: ResolutionMode,
}

impl ReviewRun {
    pub(crate) fn stats(&self) -> Option<&super::context_stats::ContextStats> {
        self.context_stats.as_ref()
    }
    /// Completed context is available only after an applied union with admitted files.
    pub fn context(&self) -> Option<&super::context_run::ReviewContext> {
        match &self.context {
            ContextState::Expanded(context) if context.view().is_some() => Some(context),
            _ => None,
        }
    }
    /// Move eligible context out before moving public result fields. A failed take preserves
    /// the state, and optional private execution statistics stay on this owner in every case.
    pub fn take_context(&mut self) -> Option<super::context_run::ReviewContext> {
        self.context()?;
        match std::mem::replace(&mut self.context, ContextState::NotRequested) {
            ContextState::Expanded(context) => Some(context),
            _ => None,
        }
    }
}

/// Run preloaded algorithms without loading taint context.
///
/// For a valid `Imports` request, selecting taint runs it on the preloaded inputs and adds
/// one `Unauthorized` note naming `run_review_authorized` and `run_algorithm_authorized`.
/// Without taint, the request adds one `Ignored` note. The run reports the note in
/// `ReviewRun::diagnostics` and appends its text to `ReviewRun::warnings`.
/// Invalid taint-context options produce an error for each selected taint entry while other
/// algorithms still run.
pub fn run_review(
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> ReviewRun {
    super::context_route::run_review_legacy(ctx, inputs, algorithms, config, params, repo)
}

pub(crate) fn run_review_routed(
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    request: Option<&ContextRequest>,
    taint_error: Option<&str>,
) -> ReviewRun {
    if taint_error.is_some() {
        run_review_prepared_policy(
            ctx,
            inputs,
            algorithms,
            config,
            params,
            repo,
            None,
            taint_error,
        )
    } else if let Some(request) = request {
        run_review_prepared(ctx, inputs, algorithms, config, params, repo, Some(request))
    } else {
        run_review_prepared(ctx, inputs, algorithms, config, params, repo, None)
    }
}

pub(crate) fn run_review_prepared(
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    request: Option<&ContextRequest>,
) -> ReviewRun {
    run_review_prepared_policy(ctx, inputs, algorithms, config, params, repo, request, None)
}

fn run_review_prepared_policy(
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
    request: Option<&ContextRequest>,
    taint_error: Option<&str>,
) -> ReviewRun {
    prism::build_pool::install(|| {
        let mut results = Vec::new();
        let mut errors = Vec::new();
        let mut diagnostics = RunDiagnostics::default();
        let mut expansion =
            RunExpansion::new(request, algorithms.contains(&SlicingAlgorithm::Taint));

        for &algorithm in algorithms {
            if algorithm == SlicingAlgorithm::Taint {
                if let Some(error) = taint_error {
                    errors.push(AlgorithmError {
                        algorithm: algorithm.name().to_string(),
                        error: error.to_owned(),
                    });
                    continue;
                }
            }
            let algorithm_config = SliceConfig {
                algorithm,
                ..config.clone()
            };
            match run_algorithm_raw(
                algorithm,
                ctx,
                inputs,
                &algorithm_config,
                params,
                repo,
                &mut diagnostics,
                &mut expansion,
            ) {
                Ok(mut result) => {
                    filter_result_findings_in(
                        &mut result,
                        expansion.view(inputs, algorithm),
                        inputs.min_confidence,
                        inputs.resolution,
                    );
                    results.push(result);
                }
                Err(error) => errors.push(AlgorithmError {
                    algorithm: algorithm.name().to_string(),
                    error: error.to_string(),
                }),
            }
        }

        let algorithms_run = algorithms
            .iter()
            .map(|algorithm| algorithm.name().to_string())
            .collect();
        let mut findings: Vec<SliceFinding> = results
            .iter()
            .flat_map(|result| result.findings.clone())
            .collect();
        let evidence = results
            .iter()
            .flat_map(|result| result.evidence.clone())
            .collect();
        for finding in &mut findings {
            let algorithm = if finding.algorithm == "taint" {
                SlicingAlgorithm::Taint
            } else {
                SlicingAlgorithm::OriginalDiff
            };
            annotate_finding_parse_quality(
                std::slice::from_mut(finding),
                expansion.view(inputs, algorithm).files,
            );
        }
        let context_stats = expansion.take_stats();
        if let Some(stats) = &context_stats {
            stats.export();
        }
        let mut run = ReviewRun {
            context: expansion.into_state(),
            context_stats,
            diagnostics: None,
            results,
            findings,
            evidence,
            errors,
            warnings: inputs.parse_warnings.clone(),
            algorithms_run,
            min_confidence: inputs.min_confidence,
            resolution: inputs.resolution,
        };
        diagnostics.reconcile_context(&run.context, request, &inputs.parse_warnings);
        let diagnostics = diagnostics.project();
        if let Some(notes) = &diagnostics {
            run.warnings.extend(notes.messages());
        }
        run.diagnostics = diagnostics;
        run
    })
}

#[non_exhaustive]
pub struct ReviewOutcome {
    pub inputs: ReviewInputs,
    pub run: ReviewRun,
    pub build_warnings: Vec<String>,
    pub min_confidence: MinConfidence,
    pub resolution: ResolutionMode,
}

/// Run a complete review through the stable facade.
///
/// The worked example lives on the `prism_review::api::review` re-export in
/// `api/mod.rs` — one canonical, `TempDir`-based doc-test, which is also the
/// README's "Library use" snippet. A second example here (the one this comment
/// replaced) only gave rustdoc two to render and `std::env::temp_dir` a
/// directory to leak on a failed assertion.
pub fn review(
    opts: &ReviewOptions,
    diff_text: &str,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
) -> Result<ReviewOutcome> {
    prism::build_pool::install(|| {
        let request = super::context_route::validate_for_review(params)?;
        let inputs = load_review_inputs(opts, diff_text)?;
        let (run, build_warnings) = {
            let built = build_context(&inputs, opts)?;
            let run = super::context_route::run_review_validated(
                &built.ctx,
                &inputs,
                algorithms,
                config,
                params,
                &opts.repo,
                request.as_ref(),
                true,
            );
            (run, built.warnings)
        };
        let min_confidence = run.min_confidence;
        let resolution = run.resolution;
        Ok(ReviewOutcome {
            inputs,
            run,
            build_warnings,
            min_confidence,
            resolution,
        })
    })
}
