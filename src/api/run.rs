use super::{build_context, load_review_inputs, ReviewInputs, ReviewOptions};
use crate::algorithms;
use crate::ast::ParsedFile;
use crate::cpg::CpgContext;
use crate::slice::{AlgorithmError, SliceConfig, SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const DEFAULT_BARRIER_DEPTH: usize = 2;
pub const DEFAULT_SPIRAL_MAX_RING: usize = 4;
pub const DEFAULT_TEMPORAL_DAYS: usize = 90;

#[non_exhaustive]
pub struct AlgorithmParams {
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

pub fn run_algorithm(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> Result<SliceResult> {
    crate::build_pool::install(|| {
        let mut result = run_algorithm_raw(algorithm, ctx, inputs, config, params, repo)?;
        annotate_finding_parse_quality(&mut result.findings, &inputs.files);
        Ok(result)
    })
}

fn run_algorithm_raw(
    algorithm: SlicingAlgorithm,
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
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
            crate::algorithms::taint::slice(ctx, &inputs.diff, &taint_config)
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

/// Multi-algorithm output in the legacy-compatible facade shape.
///
/// `findings` is the annotated set; `results[*].findings` are the algorithms' raw findings
/// (legacy shape). Classify via `finding_confidence::parse_quality_for` with the authoritative
/// map, never via `finding.parse_quality`.
#[non_exhaustive]
pub struct ReviewRun {
    pub results: Vec<SliceResult>,
    pub findings: Vec<SliceFinding>,
    pub errors: Vec<AlgorithmError>,
    pub warnings: Vec<String>,
    pub algorithms_run: Vec<String>,
}

pub fn run_review(
    ctx: &CpgContext,
    inputs: &ReviewInputs,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
    repo: &Path,
) -> ReviewRun {
    crate::build_pool::install(|| {
        let mut results = Vec::new();
        let mut errors = Vec::new();

        for &algorithm in algorithms {
            let algorithm_config = SliceConfig {
                algorithm,
                ..config.clone()
            };
            match run_algorithm_raw(algorithm, ctx, inputs, &algorithm_config, params, repo) {
                Ok(result) => results.push(result),
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
        annotate_finding_parse_quality(&mut findings, &inputs.files);

        ReviewRun {
            results,
            findings,
            errors,
            warnings: inputs.parse_warnings.clone(),
            algorithms_run,
        }
    })
}

#[non_exhaustive]
pub struct ReviewOutcome {
    pub inputs: ReviewInputs,
    pub run: ReviewRun,
    pub build_warnings: Vec<String>,
}

/// Run a complete review through the stable facade.
///
/// ```
/// use prism::api::{review, AlgorithmParams, ReviewOptions};
/// use prism::slice::{SliceConfig, SlicingAlgorithm};
/// use std::fs;
///
/// let root = std::env::temp_dir().join(format!("prism-api-doc-{}", std::process::id()));
/// let _ = fs::remove_dir_all(&root);
/// fs::create_dir_all(&root)?;
/// fs::write(root.join("a.py"), "def f():\n    return 1\n")?;
/// let diff = r#"{"files":[{"file_path":"a.py","modify_type":"Modified","diff_lines":[2]}]}"#;
/// let outcome = review(
///     &ReviewOptions::new(&root),
///     diff,
///     &[SlicingAlgorithm::AbsenceSlice],
///     &SliceConfig::default(),
///     &AlgorithmParams::default(),
/// )?;
/// assert_eq!(outcome.inputs.diff.files.len(), 1);
/// fs::remove_dir_all(root)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn review(
    opts: &ReviewOptions,
    diff_text: &str,
    algorithms: &[SlicingAlgorithm],
    config: &SliceConfig,
    params: &AlgorithmParams,
) -> Result<ReviewOutcome> {
    crate::build_pool::install(|| {
        let inputs = load_review_inputs(opts, diff_text)?;
        let (run, build_warnings) = {
            let built = build_context(&inputs, opts)?;
            let run = run_review(&built.ctx, &inputs, algorithms, config, params, &opts.repo);
            (run, built.warnings)
        };
        Ok(ReviewOutcome {
            inputs,
            run,
            build_warnings,
        })
    })
}
