//! Compose the public documents from the authorized library run on the same captured inputs.
use super::fixture::{Fixture, Policy};
use prism::finding_confidence::EvidencePath;
use prism::slice::{MultiSliceResult, SliceConfig, SliceResult};
use prism_review::api::{
    build_context, load_review_inputs, parse_algorithms, project_in_context,
    run_algorithm_authorized, run_review_authorized, ResultView, ReviewAuthority,
    ReviewBoundaryDiagnostics, ReviewContext, ReviewInputs, TargetsMeta,
};
use prism_review::output;
use serde::Serialize;
use std::path::PathBuf;

pub(super) fn pretty(value: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

pub(super) struct Oracle {
    pub inputs: ReviewInputs,
    pub context: Option<ReviewContext>,
    pub diagnostics: Option<ReviewBoundaryDiagnostics>,
    pub run: MultiSliceResult,
    evidence: Vec<Option<EvidencePath>>,
    build_warnings: Vec<String>,
    repo: PathBuf,
    single: bool,
}

impl Oracle {
    // Targets always use the multi-result facade, including a single algorithm request.
    pub fn capture(f: &Fixture, selection: &str, p: &Policy, targets: bool) -> Self {
        let options = p.options(f);
        let inputs = load_review_inputs(&options, &f.diff).unwrap();
        assert_eq!(
            inputs.files.keys().map(String::as_str).collect::<Vec<_>>(),
            [f.original],
            "inadmissible precondition: original diff capture"
        );
        let algorithms = parse_algorithms(selection).unwrap();
        let config = SliceConfig::default().with_algorithm(algorithms[0]);
        let built = build_context(&inputs, &options).unwrap();
        let authority = ReviewAuthority::capture(&inputs);
        let single = algorithms.len() == 1 && !targets;
        let (run, evidence, diagnostics, context) = if single {
            let mut owner = run_algorithm_authorized(
                algorithms[0],
                &built.ctx,
                authority,
                &config,
                &p.params,
                &f.repo,
            )
            .unwrap();
            owner.filter_findings(&inputs, p.floor, p.resolution);
            let (outcome, context) = owner.into_parts();
            let result = outcome.result;
            let evidence = result.evidence.clone();
            let mut warnings = inputs.parse_warnings.clone();
            warnings.extend(result.warnings.iter().cloned());
            if let Some(d) = &outcome.diagnostics {
                warnings.extend(d.messages());
            }
            let run = MultiSliceResult {
                version: "1.0".into(),
                algorithms_run: vec![result.algorithm.name().into()],
                findings: result.findings.clone(),
                errors: vec![],
                warnings,
                parse_quality: inputs.parse_quality.clone(),
                diagram_warnings: result.diagram_warnings.clone(),
                results: vec![result],
            };
            (run, evidence, outcome.diagnostics, context)
        } else {
            let mut owner = run_review_authorized(
                &built.ctx,
                authority,
                &algorithms,
                &config,
                &p.params,
                &f.repo,
            );
            let context = owner.take_context();
            let run = MultiSliceResult {
                version: "1.0".into(),
                algorithms_run: owner.algorithms_run,
                findings: owner.findings,
                errors: owner.errors,
                warnings: owner.warnings,
                parse_quality: inputs.parse_quality.clone(),
                diagram_warnings: owner
                    .results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.clone())
                    .collect(),
                results: owner.results,
            };
            (run, owner.evidence, owner.diagnostics, context)
        };
        let build_warnings = built.warnings;
        Self {
            inputs,
            context,
            diagnostics,
            run,
            evidence,
            build_warnings,
            repo: f.repo.clone(),
            single,
        }
    }

    pub fn messages(&self) -> Vec<String> {
        self.diagnostics.iter().flat_map(|d| d.messages()).collect()
    }

    fn view(&self, r: &SliceResult) -> ResultView<'_> {
        ResultView::for_result(self.context.as_ref(), r, &self.inputs)
    }

    pub fn doc(&self, format: &str) -> Vec<u8> {
        let run = &self.run;
        let rank = output::severity_rank("warning");
        match format {
            "targets" => {
                let mut meta = TargetsMeta::default();
                meta.algorithms_run = run.algorithms_run.clone();
                meta.errors = run.errors.clone();
                meta.repo_root = self.repo.clone();
                meta.min_confidence = self.inputs.min_confidence;
                meta.resolution = self.inputs.resolution;
                meta.run_warnings = self.inputs.parse_warnings.clone();
                meta.run_warnings.extend(self.inputs.load_warnings.clone());
                meta.run_warnings.extend(self.build_warnings.clone());
                meta.run_warnings.extend(self.messages());
                pretty(&project_in_context(
                    &run.findings,
                    &self.evidence,
                    &self.inputs,
                    self.context.as_ref(),
                    &meta,
                ))
            }
            "sarif" => {
                let mut data = output::SarifInputs::new(&run.findings)
                    .evidence(&self.evidence)
                    .errors(&run.errors)
                    .parse_warnings(&self.inputs.parse_warnings)
                    .load_warnings(&self.inputs.load_warnings)
                    .build_warnings(&self.build_warnings)
                    .algorithms_run(&run.algorithms_run)
                    .parse_quality(&self.inputs.parse_quality)
                    .files(&self.inputs.files)
                    .sources(&self.inputs.sources)
                    .min_confidence(self.inputs.min_confidence)
                    .resolution(self.inputs.resolution);
                if let Some(d) = &self.diagnostics {
                    data = data.boundary_diagnostics(d);
                }
                if let Some(c) = &self.context {
                    data = data.context(c);
                }
                pretty(&output::to_sarif(&data))
            }
            "paper" if self.single => pretty(&output::to_paper_format(&run.results[0].blocks)),
            "text" | "paper" => {
                let mut text = String::new();
                for result in &run.results {
                    if !self.single {
                        text.push_str(&format!("=== {} ===\n", result.algorithm.name()));
                    }
                    text.push_str(&output::format_slice_result(
                        &result.blocks,
                        self.view(result).sources(),
                    ));
                }
                text.into_bytes()
            }
            "mermaid" => {
                let mut multi = run.clone();
                multi.diagram_warnings.clear();
                multi.warnings = self.inputs.parse_warnings.clone();
                if self.single {
                    multi.warnings.clear();
                    multi.parse_quality.clear();
                    multi.findings.clear();
                    multi.results[0].warnings = self.inputs.parse_warnings.clone();
                }
                format!("{}\n", output::format_mermaid_report(&multi)).into_bytes()
            }
            "json" | "review" if self.single => {
                let mut result = run.results[0].clone();
                result.warnings = run.warnings.clone();
                let view = self.view(&result);
                if format == "review" {
                    pretty(&output::to_compact_review_output_for(
                        &result, view, rank, false, false,
                    ))
                } else {
                    pretty(&output::to_review_output_for(&result, view))
                }
            }
            "json" => pretty(&output::MultiReviewOutput {
                version: run.version.clone(),
                algorithms_run: run.algorithms_run.clone(),
                results: run
                    .results
                    .iter()
                    .map(|r| output::to_review_output_for(r, self.view(r)))
                    .collect(),
                all_findings: run.findings.clone(),
                errors: run.errors.clone(),
                warnings: run.warnings.clone(),
                parse_quality: run.parse_quality.clone(),
                diagram_warnings: run.diagram_warnings.clone(),
            }),
            "review" => pretty(&output::CompactMultiReviewOutput {
                version: run.version.clone(),
                algorithms_run: run.algorithms_run.clone(),
                results: run
                    .results
                    .iter()
                    .map(|r| {
                        output::to_compact_review_output_for(r, self.view(r), rank, false, false)
                    })
                    .collect(),
                all_findings: run
                    .findings
                    .iter()
                    .filter(|f| output::severity_rank(&f.severity) >= rank)
                    .cloned()
                    .collect(),
                findings_below_floor: output::count_below_floor(&run.findings, rank),
                errors: run.errors.clone(),
                warnings: run.warnings.clone(),
                parse_quality: run.parse_quality.clone(),
                diagram_warnings: run.diagram_warnings.clone(),
            }),
            _ => panic!("unexpected format {format}"),
        }
    }
}
