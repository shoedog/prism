//! M1 owns common support; M2 adds routing rows. REAL captured reviews unless explicitly CONTROLLED.
//! Library formatter composition only: this does not exercise the future public CLI ON path.
use super::context_options::{TaintContextEdges, TaintContextMode};
use super::context_route::{run_algorithm_authorized, run_review_authorized, ReviewAuthority};
use super::context_view::{view_for_result, ReviewAlgorithmRun};
use super::run::run_review_prepared;
use super::*;
use crate::algorithms::taint::{
    self,
    context_budget::{ContextLimits, TimeGuard},
    rounds::{expand, ExpansionOutcome, ExpansionRequest},
};
use crate::output::{
    self, review::to_review_output_in, review_compact::to_compact_review_output_in,
};
use crate::targets::{project_with_context, TargetsMeta};
use prism::{
    api::ReviewContextLoader,
    cpg::{CpgContext, CrossingCeiling},
    finding_confidence::{MinConfidence, ResolutionMode},
    slice::{MultiSliceResult, SliceConfig, SliceResult, SlicingAlgorithm},
};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

const MAIN: &str = "import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
const B: &str = "export function g(x) { eval(x); }\n";
const DEGRADED: &str =
    "export function g(x) { eval(x); }\nfunction unrelated() { const broken = ; }\n";
pub(super) struct Fixture {
    pub dir: tempfile::TempDir,
    pub options: ReviewOptions,
    pub inputs: ReviewInputs,
}
pub(super) struct Sample {
    pub run: ReviewRun,
    pub build_warnings: Vec<String>,
    repo_root: std::path::PathBuf,
}
pub(super) struct SingleSample {
    pub run: ReviewAlgorithmRun,
    pub build_warnings: Vec<String>,
    repo_root: std::path::PathBuf,
}
impl Fixture {
    pub fn new(files: &[(&str, &str)], original: &str, lines: &[usize]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let mut options = ReviewOptions::new(dir.path());
        options.no_cache = true;
        options.resolution = ResolutionMode::Scoped;
        let diff = serde_json::json!({"files":[{"file_path":original,"modify_type":"Modified","diff_lines":lines}]}).to_string();
        let inputs = load_review_inputs(&options, &diff).unwrap();
        assert_eq!(
            inputs.files.keys().map(String::as_str).collect::<Vec<_>>(),
            [original],
            "inadmissible precondition: original-only parsed inputs"
        );
        Self {
            dir,
            options,
            inputs,
        }
    }
    pub fn js(body: &str) -> Self {
        Self::new(
            &[("main.js", MAIN), ("a.js", A), ("b.js", body)],
            "main.js",
            &[2, 3],
        )
    }
    pub fn policy(&mut self, resolution: ResolutionMode, floor: MinConfidence) {
        self.options.resolution = resolution;
        self.options.min_confidence = floor;
        self.inputs.resolution = resolution;
        self.inputs.min_confidence = floor;
    }
    pub fn run(&self, algorithms: &[SlicingAlgorithm], params: &AlgorithmParams) -> Sample {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let run = run_review_authorized(
            &built.ctx,
            ReviewAuthority::capture(&self.inputs),
            algorithms,
            &SliceConfig::default(),
            params,
            self.dir.path(),
        );
        Sample {
            run,
            build_warnings: built.warnings,
            repo_root: self.dir.path().into(),
        }
    }
    pub fn on(&self, algorithms: &[SlicingAlgorithm]) -> Sample {
        self.run(algorithms, &imports())
    }
    pub fn off(&self, algorithms: &[SlicingAlgorithm]) -> Sample {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let run = run_review_prepared(
            &built.ctx,
            &self.inputs,
            algorithms,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            self.dir.path(),
            None,
        );
        Sample {
            run,
            build_warnings: built.warnings,
            repo_root: self.dir.path().into(),
        }
    }
    pub fn single(&self, params: &AlgorithmParams) -> SingleSample {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let run = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            &built.ctx,
            ReviewAuthority::capture(&self.inputs),
            &SliceConfig::default(),
            params,
            self.dir.path(),
        )
        .unwrap();
        SingleSample {
            run,
            build_warnings: built.warnings,
            repo_root: self.dir.path().into(),
        }
    }
    pub fn native(&self) -> ExpansionOutcome {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let config = taint::TaintConfig::default();
        let (_, trace) = taint::slice_with_trace(&built.ctx, &self.inputs.diff, &config).unwrap();
        let loader = ReviewContextLoader::new(&self.inputs).unwrap();
        let limits = ContextLimits::default();
        let guard = TimeGuard::new(limits.time);
        expand(ExpansionRequest {
            loader: &loader,
            inputs: &self.inputs,
            baseline_ctx: &built.ctx,
            baseline_trace: &trace,
            config: &config,
            limits,
            ceiling: CrossingCeiling::NameOnly,
            guard: &guard,
        })
    }
}
pub(super) fn imports() -> AlgorithmParams {
    let mut params = AlgorithmParams::default();
    params.taint_context.mode = TaintContextMode::Imports;
    params
}
pub(super) fn bytes(value: &(impl Serialize + ?Sized)) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
pub(super) fn same_result(a: &SliceResult, b: &SliceResult) {
    assert_eq!(bytes(a), bytes(b));
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
}
pub(super) fn assert_expanded(sample: &Sample, target: &str) {
    let super::context_run::ContextState::Expanded(context) = &sample.run.context else {
        panic!("ON must retain actual expanded state");
    };
    assert!(
        matches!(
            context.union,
            super::context_run::UnionState::Applied { .. }
        ),
        "ON union must be applied"
    );
    assert!(
        context.context_files.contains(target),
        "ON admits required target"
    );
    assert!(
        sample
            .run
            .findings
            .iter()
            .any(|f| f.file == target && f.category.as_deref() == Some("taint_sink")),
        "ON must produce native context sink"
    );
}
pub(super) fn notices(doc: &Value) -> &[Value] {
    doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
#[derive(Clone, Copy, Default)]
pub(super) struct RenderSettings {
    pub min_rank: u8,
    pub full_slices: bool,
    pub no_diagrams: bool,
}
#[derive(Debug, PartialEq)]
pub(super) struct Document {
    pub value: Value,
    pub bytes: Vec<u8>,
}
impl Document {
    fn new(value: &impl Serialize) -> Self {
        Self {
            value: serde_json::to_value(value).unwrap(),
            bytes: serde_json::to_vec_pretty(value).unwrap(),
        }
    }
}
pub(super) struct Documents {
    pub json: Document,
    pub review: Document,
    pub sarif: Document,
    pub text: String,
    pub paper: Document,
    pub mermaid: String,
    pub targets: Document,
}
impl Documents {
    pub fn multi(sample: &Sample, inputs: &ReviewInputs, settings: RenderSettings) -> Self {
        let run = &sample.run;
        let mut json_results = Vec::new();
        let mut compact_results = Vec::new();
        let mut text = String::new();
        let mut blocks = Vec::new();
        for result in &run.results {
            let view = view_for_result(run.context(), result, inputs);
            let context_files = run
                .context()
                .filter(|_| result.algorithm == SlicingAlgorithm::Taint)
                .map(|c| &c.context_files);
            json_results.push(to_review_output_in(result, view.maps(), context_files));
            compact_results.push(to_compact_review_output_in(
                result,
                view.maps(),
                context_files,
                settings.min_rank,
                settings.full_slices,
                settings.no_diagrams,
            ));
            for block in &result.blocks {
                if !text.is_empty() {
                    text.push_str("\n---\n\n");
                }
                text.push_str(&output::format_slice_result(
                    std::slice::from_ref(block),
                    view.sources(),
                ));
            }
            blocks.extend(result.blocks.clone());
        }
        let warnings = run
            .results
            .iter()
            .flat_map(|r| r.diagram_warnings.clone())
            .collect::<Vec<_>>();
        let json = output::MultiReviewOutput {
            version: "1.0".into(),
            algorithms_run: run.algorithms_run.clone(),
            results: json_results,
            all_findings: run.findings.clone(),
            errors: run.errors.clone(),
            warnings: run.warnings.clone(),
            parse_quality: inputs.parse_quality.clone(),
            diagram_warnings: warnings.clone(),
        };
        let mut findings: Vec<_> = run
            .findings
            .iter()
            .filter(|f| output::severity_rank(&f.severity) >= settings.min_rank)
            .cloned()
            .collect();
        if settings.no_diagrams {
            output::strip_finding_diagrams(&mut findings);
        }
        let review = output::CompactMultiReviewOutput {
            version: "1.0".into(),
            algorithms_run: run.algorithms_run.clone(),
            results: compact_results,
            all_findings: findings,
            findings_below_floor: output::count_below_floor(&run.findings, settings.min_rank),
            errors: run.errors.clone(),
            warnings: run.warnings.clone(),
            parse_quality: inputs.parse_quality.clone(),
            diagram_warnings: warnings.clone(),
        };
        let multi = MultiSliceResult {
            version: "1.0".into(),
            algorithms_run: run.algorithms_run.clone(),
            results: run.results.clone(),
            findings: run.findings.clone(),
            errors: run.errors.clone(),
            warnings: run.warnings.clone(),
            parse_quality: inputs.parse_quality.clone(),
            diagram_warnings: vec![],
        };
        let sarif = sarif(
            &run.findings,
            &run.evidence,
            &run.errors,
            run.diagnostics.as_ref(),
            &run.algorithms_run,
            inputs,
            run.context().and_then(|c| c.view()),
            &sample.build_warnings,
        );
        let mut meta = TargetsMeta::default();
        meta.repo_root = sample.repo_root.clone();
        meta.algorithms_run = run.algorithms_run.clone();
        meta.errors = run.errors.clone();
        meta.run_warnings = run.warnings.clone();
        meta.min_confidence = inputs.min_confidence;
        meta.resolution = inputs.resolution;
        let targets = project_with_context(
            &run.findings,
            &run.evidence,
            inputs,
            run.context().and_then(|c| c.view()),
            &meta,
        );
        Self {
            json: Document::new(&json),
            review: Document::new(&review),
            sarif: Document::new(&sarif),
            text,
            paper: Document::new(&output::to_paper_format(&blocks)),
            mermaid: output::format_mermaid_report(&multi),
            targets: Document::new(&targets),
        }
    }
    pub fn single(sample: &SingleSample, inputs: &ReviewInputs, settings: RenderSettings) -> Self {
        let run = &sample.run;
        let view = run.view(inputs);
        let context_files = run.context().map(|c| &c.context_files);
        // Existing conversion owns the warning append; rendering does not finalize or collect.
        let legacy = AlgorithmOutcome {
            result: run.outcome.result.clone(),
            diagnostics: run.outcome.diagnostics.clone(),
        }
        .into_legacy_result();
        let json = to_review_output_in(&legacy, view.maps(), context_files);
        let review = to_compact_review_output_in(
            &legacy,
            view.maps(),
            context_files,
            settings.min_rank,
            settings.full_slices,
            settings.no_diagrams,
        );
        let names = vec![SlicingAlgorithm::Taint.name().to_string()];
        let sarif = sarif(
            &legacy.findings,
            &legacy.evidence,
            &[],
            run.outcome.diagnostics.as_ref(),
            &names,
            inputs,
            Some(view.maps()),
            &sample.build_warnings,
        );
        let multi = MultiSliceResult {
            version: "1.0".into(),
            algorithms_run: names.clone(),
            results: vec![legacy.clone()],
            findings: legacy.findings.clone(),
            errors: vec![],
            warnings: legacy.warnings.clone(),
            parse_quality: inputs.parse_quality.clone(),
            diagram_warnings: vec![],
        };
        let mut meta = TargetsMeta::default();
        meta.repo_root = sample.repo_root.clone();
        meta.algorithms_run = names;
        meta.run_warnings = legacy.warnings.clone();
        meta.min_confidence = inputs.min_confidence;
        meta.resolution = inputs.resolution;
        let targets = project_with_context(
            &legacy.findings,
            &legacy.evidence,
            inputs,
            Some(view.maps()),
            &meta,
        );
        Self {
            json: Document::new(&json),
            review: Document::new(&review),
            sarif: Document::new(&sarif),
            text: output::format_slice_result(&legacy.blocks, view.sources()),
            paper: Document::new(&output::to_paper_format(&legacy.blocks)),
            mermaid: output::format_mermaid_report(&multi),
            targets: Document::new(&targets),
        }
    }
    pub fn bytes(&self) -> BTreeMap<&str, Vec<u8>> {
        BTreeMap::from([
            ("json", self.json.bytes.clone()),
            ("review", self.review.bytes.clone()),
            ("sarif", self.sarif.bytes.clone()),
            ("text", self.text.as_bytes().to_vec()),
            ("paper", self.paper.bytes.clone()),
            ("mermaid", self.mermaid.as_bytes().to_vec()),
            ("targets", self.targets.bytes.clone()),
        ])
    }
}
fn sarif(
    findings: &[prism::slice::SliceFinding],
    evidence: &[Option<prism::finding_confidence::EvidencePath>],
    errors: &[prism::slice::AlgorithmError],
    diagnostics: Option<&ReviewBoundaryDiagnostics>,
    names: &[String],
    inputs: &ReviewInputs,
    context: Option<super::map_view::MapView<'_>>,
    build_warnings: &[String],
) -> Value {
    let mut data = output::SarifInputs::new(findings)
        .evidence(evidence)
        .errors(errors)
        .parse_warnings(&inputs.parse_warnings)
        .load_warnings(&inputs.load_warnings)
        .build_warnings(build_warnings)
        .algorithms_run(names)
        .parse_quality(&inputs.parse_quality)
        .files(&inputs.files)
        .sources(&inputs.sources)
        .min_confidence(inputs.min_confidence)
        .resolution(inputs.resolution);
    if let Some(diagnostics) = diagnostics {
        data = data.boundary_diagnostics(diagnostics);
    }
    if let Some(context) = context {
        data = data.context_maps(context);
    }
    output::to_sarif(&data)
}

mod formats;
