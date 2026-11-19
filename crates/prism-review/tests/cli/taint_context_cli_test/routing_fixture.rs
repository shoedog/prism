use prism::{
    finding_confidence::ResolutionMode,
    slice::{MultiSliceResult, SliceConfig, SlicingAlgorithm},
};
use prism_review::{
    api::{
        self, AlgorithmOutcome, AlgorithmParams, ResultView, ReviewAuthority, ReviewContext,
        ReviewInputs, ReviewOptions, TaintContextMode,
    },
    output,
    targets::{project_in_context, TargetsMeta},
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::{Command, Output},
};

pub(super) const MAIN: &str = "import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
pub(super) const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
pub(super) const B: &str = "export function g(x) { eval(x); }\n";

pub(super) struct Fixture {
    dir: tempfile::TempDir,
    pub options: ReviewOptions,
    pub diff: String,
}

impl Fixture {
    pub fn new(files: &[(&str, &str)], changed: &[(&str, &[usize])]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let mut options = ReviewOptions::new(dir.path().canonicalize().unwrap());
        options.no_cache = true;
        options.resolution = ResolutionMode::Scoped;
        let files_changed: Vec<_> = changed
            .iter()
            .map(|(file, lines)| {
                json!({"file_path":file, "modify_type":"Modified", "diff_lines":lines})
            })
            .collect();
        let diff = json!({"files": files_changed}).to_string();
        let fixture = Self { dir, options, diff };
        for (name, source) in files {
            fixture.write(name, source);
        }
        fixture.write("diff.json", &fixture.diff);
        fixture
    }

    pub fn js(changed: &[(&str, &[usize])]) -> Self {
        Self::new(&[("main.js", MAIN), ("a.js", A), ("b.js", B)], changed)
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn write(&self, file: &str, source: &str) {
        let path = self.path().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, source).unwrap();
    }

    pub fn inputs(&self) -> ReviewInputs {
        api::load_review_inputs(&self.options, &self.diff).unwrap()
    }

    pub fn single(&self, name: &str, params: &AlgorithmParams, strict: bool) -> Sample {
        let inputs = self.inputs();
        let algorithm = algorithm(name);
        let config = SliceConfig {
            strict_diagrams: strict,
            ..self.config(algorithm)
        };
        let built = api::build_context(&inputs, &self.options).unwrap();
        let mut run = api::run_algorithm_authorized(
            algorithm,
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &config,
            params,
            &self.options.repo,
        )
        .unwrap();
        run.filter_findings(&inputs, inputs.min_confidence, inputs.resolution);
        let (outcome, context) = run.into_parts();
        let build_warnings = built.warnings;
        Sample {
            inputs,
            outcome,
            context,
            build_warnings,
        }
    }

    pub fn targets(&self, name: &str, params: &AlgorithmParams) -> Vec<u8> {
        let algorithm = algorithm(name);
        let review = api::review(
            &self.options,
            &self.diff,
            &[algorithm],
            &self.config(algorithm),
            params,
        )
        .unwrap();
        let mut meta = TargetsMeta::default();
        meta.algorithms_run = review.run.algorithms_run.clone();
        meta.repo_root = self.options.repo.clone();
        meta.errors = review.run.errors.clone();
        meta.run_warnings = review.inputs.parse_warnings.clone();
        meta.run_warnings
            .extend(review.inputs.load_warnings.clone());
        meta.run_warnings.extend(review.build_warnings);
        if let Some(diagnostics) = &review.run.diagnostics {
            meta.run_warnings.extend(diagnostics.messages());
        }
        meta.min_severity_rank = output::severity_rank("info");
        meta.min_confidence = review.min_confidence;
        meta.resolution = review.resolution;
        pretty(&project_in_context(
            &review.run.findings,
            &review.run.evidence,
            &review.inputs,
            review.run.context(),
            &meta,
        ))
    }

    fn config(&self, algorithm: SlicingAlgorithm) -> SliceConfig {
        SliceConfig {
            algorithm,
            scoped_cpg: self.options.scoped_cpg,
            ..SliceConfig::default()
        }
    }

    pub fn command(&self, algorithm: &str, format: &str, args: &[&str]) -> Command {
        let mut command = crate::prism_std();
        if format == "targets" {
            command.arg("targets");
        } else {
            command.args(["--format", format]);
        }
        command
            .env_remove("PRISM_PROFILE")
            .arg("--repo")
            .arg(&self.options.repo)
            .arg("--diff")
            .arg(self.path().join("diff.json"))
            .args(["--algorithm", algorithm, "--resolution", "scoped"])
            .args(args);
        command
    }
}

pub(super) struct Sample {
    pub inputs: ReviewInputs,
    pub outcome: AlgorithmOutcome,
    pub context: Option<ReviewContext>,
    build_warnings: Vec<String>,
}

impl Sample {
    pub fn messages(&self) -> Vec<String> {
        self.outcome
            .diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.messages())
            .unwrap_or_default()
    }

    pub fn document(&self, format: &str) -> (Vec<u8>, Vec<u8>) {
        let mut result = self.outcome.result.clone();
        let view = ResultView::for_result(self.context.as_ref(), &result, &self.inputs);
        let messages = self.messages();
        let mut warnings = self.inputs.parse_warnings.clone();
        let diagram_warnings = result.diagram_warnings.clone();
        let routed_to_stderr = match format {
            "json" | "review" | "text" => {
                warnings.append(&mut result.warnings);
                warnings.extend(messages);
                result.warnings = warnings;
                if format == "text" {
                    result.warnings.clone()
                } else {
                    vec![]
                }
            }
            _ => {
                result.warnings = warnings;
                if matches!(format, "paper" | "mermaid") {
                    messages
                } else {
                    vec![]
                }
            }
        };
        let stdout = match format {
            "json" => pretty(&output::to_review_output_for(&result, view)),
            "review" => pretty(&output::to_compact_review_output_for(
                &result,
                view,
                output::severity_rank("warning"),
                false,
                false,
            )),
            "text" => output::format_slice_result(&result.blocks, view.sources()).into_bytes(),
            "paper" => pretty(&output::to_paper_format(&result.blocks)),
            "mermaid" => {
                let report = MultiSliceResult {
                    version: "1.0".into(),
                    algorithms_run: vec![result.algorithm.name().to_owned()],
                    results: vec![result],
                    findings: vec![],
                    errors: vec![],
                    warnings: vec![],
                    parse_quality: Default::default(),
                    diagram_warnings: vec![],
                };
                format!("{}\n", output::format_mermaid_report(&report)).into_bytes()
            }
            "sarif" => {
                let names = [result.algorithm.name().to_owned()];
                let mut sarif = output::SarifInputs::new(&result.findings)
                    .evidence(&result.evidence)
                    .parse_warnings(&self.inputs.parse_warnings)
                    .load_warnings(&self.inputs.load_warnings)
                    .build_warnings(&self.build_warnings)
                    .algorithms_run(&names)
                    .parse_quality(&self.inputs.parse_quality)
                    .files(&self.inputs.files)
                    .sources(&self.inputs.sources)
                    .min_confidence(self.inputs.min_confidence)
                    .resolution(self.inputs.resolution);
                if let Some(diagnostics) = &self.outcome.diagnostics {
                    sarif = sarif.boundary_diagnostics(diagnostics);
                }
                if let Some(context) = &self.context {
                    sarif = sarif.context(context);
                }
                pretty(&output::to_sarif(&sarif))
            }
            _ => unreachable!("unsupported routing test format {format}"),
        };
        let mut stderr = String::new();
        for warning in routed_to_stderr {
            stderr.push_str(&format!("WARNING: {warning}\n"));
        }
        for warning in &diagram_warnings {
            let title = warning.graph_title.as_deref().unwrap_or("(no title)");
            stderr.push_str(&format!(
                "prism: diagram warning: {}/{} - {:?}: {}\n",
                warning.algorithm, title, warning.kind, warning.detail
            ));
        }
        (stdout, stderr.into_bytes())
    }
}

fn algorithm(name: &str) -> SlicingAlgorithm {
    api::parse_algorithms(name).unwrap()[0]
}

pub(super) fn pretty(doc: &impl Serialize) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(doc).unwrap();
    bytes.push(b'\n');
    bytes
}

pub(super) fn imports() -> AlgorithmParams {
    let mut params = AlgorithmParams::default();
    params.taint_context.mode = TaintContextMode::Imports;
    params
}

pub(super) fn execute(command: &mut Command) -> Output {
    let output = command.output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{command:?} stderr={}",
        stderr(&output),
    );
    output
}

pub(super) fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).unwrap()
}

pub(super) fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

pub(super) fn assert_document(output: &Output, sample: &Sample, format: &str) {
    let (expected_stdout, expected_stderr) = sample.document(format);
    assert_bytes(&output.stdout, &expected_stdout, format);
    assert_bytes(&output.stderr, &expected_stderr, format);
}

pub(super) fn assert_bytes(actual: &[u8], expected: &[u8], label: &str) {
    assert_eq!(
        std::str::from_utf8(actual).unwrap(),
        std::str::from_utf8(expected).unwrap(),
        "{label} bytes",
    );
}

pub(super) fn note_line(note: &str) -> String {
    format!("WARNING: {note}\n")
}

fn note_channel(
    format: &str,
    note: &str,
    level: &str,
) -> Option<(&'static str, &'static str, Value)> {
    match format {
        "json" | "review" | "targets" => Some(("", "warnings", Value::String(note.to_owned()))),
        "sarif" => Some((
            "/runs/0/invocations/0",
            "toolExecutionNotifications",
            serde_json::json!({"level": level, "message": {"text": note}}),
        )),
        _ => None,
    }
}

pub(super) fn strip_note(output: &Output, format: &str, note: &str, level: &str) -> Option<Value> {
    if let Some((parent, key, expected)) = note_channel(format, note, level) {
        let mut document = json(output);
        let object = document
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap();
        let notes = object
            .get_mut(key)
            .and_then(Value::as_array_mut)
            .unwrap_or_else(|| panic!("{format} must have a structured channel for {note}"));
        assert_eq!(
            notes.iter().filter(|value| **value == expected).count(),
            1,
            "{format}: {note}"
        );
        notes.retain(|value| value != &expected);
        if notes.is_empty() {
            object.remove(key);
        }
        assert!(
            !stderr(output).contains(note),
            "{format} must route the note only to its document"
        );
        Some(document)
    } else {
        assert_eq!(
            stderr(output).matches(&note_line(note)).count(),
            1,
            "{format}: {note}"
        );
        assert!(!std::str::from_utf8(&output.stdout).unwrap().contains(note));
        None
    }
}

pub(super) fn profile_counters(path: &Path) -> BTreeMap<String, u64> {
    let rows: Vec<Value> = fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.first().unwrap()["record"], "header");
    assert!(rows.iter().any(|record| record["record"] == "command"));
    let mut counters = BTreeMap::new();
    for record in rows.iter().filter(|record| record["record"] == "counter") {
        let name = record["name"].as_str().unwrap();
        if let Some(name) = name.strip_prefix("review.taint_context.") {
            assert!(
                counters
                    .insert(name.to_owned(), record["value"].as_u64().unwrap())
                    .is_none(),
                "duplicate profile counter {name}",
            );
        }
    }
    counters
}

pub(super) fn assert_context_counter_names(counters: &BTreeMap<String, u64>) {
    let expected = [
        "additional_sinks",
        "additional_sources",
        "admitted_bytes",
        "admitted_depth",
        "admitted_files",
        "attempted_files",
        "attempted_read_bytes",
        "bound_bytes",
        "bound_depth",
        "bound_files",
        "bound_time",
        "cancelled",
        "context_unavailable",
        "driver_stats_available",
        "elapsed_analysis_ns",
        "elapsed_build_ns",
        "elapsed_discovery_ns",
        "elapsed_read_parse_ns",
        "elapsed_total_ns",
        "eligible_boundary_calls",
        "loaded_unresolved_calls",
        "perturbation_dropped_sinks",
        "perturbation_dropped_sources",
        "rounds_completed",
        "rounds_started",
        "skips.invalid_path",
        "skips.not_listed",
        "skips.not_regular",
        "skips.not_utf8",
        "skips.original_file",
        "skips.parse_failed",
        "skips.severe_parse",
        "skips.too_large",
        "skips.unknown",
        "skips.unreadable",
        "skips.unsafe_path",
        "skips.unsupported",
        "stage_failed",
        "union_stats_available",
    ];
    assert_eq!(
        counters.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        BTreeSet::from(expected),
        "accepted context counter population",
    );
}
