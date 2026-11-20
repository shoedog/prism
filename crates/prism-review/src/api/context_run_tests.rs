//! REAL parser/loader/engine recipes unless a row explicitly controls coordinator order or evidence.
use super::*;
use crate::algorithms::taint::{
    self,
    context_budget::{Published, TimeGuard},
    result_union::{union_results, UnionInput},
    rounds::{expand, ExpansionRequest},
};
use crate::api::run::{run_algorithm_prepared, run_review_prepared};
use crate::api::{
    build_context, load_review_inputs, run_algorithm_with_diagnostics, run_review, AlgorithmParams,
    ReviewOptions, ReviewRun,
};
use prism::api::{ContextReadOutcome, ReviewContextLoader, ReviewContextRound};
use prism::finding_confidence::{EvidenceHop, EvidencePath, MinConfidence, ResolutionMode};
use prism::slice::{SliceConfig, SliceFinding};
use std::path::Path;
#[path = "context_run_tests/edges.rs"]
mod edges;
#[path = "context_run_tests/expansion.rs"]
mod expansion;
#[path = "context_run_tests/identity.rs"]
mod identity;

const MAIN: &str = "import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
const B: &str = "export function g(x) { eval(x); }\n";
struct Fixture {
    dir: tempfile::TempDir,
    inputs: ReviewInputs,
    options: ReviewOptions,
}
impl Fixture {
    fn new(body: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (name, source) in [("main.js", MAIN), ("a.js", A), ("b.js", body)] {
            std::fs::write(dir.path().join(name), source).unwrap();
        }
        let mut options = ReviewOptions::new(dir.path());
        options.no_cache = true;
        options.resolution = ResolutionMode::Scoped;
        let inputs = load_review_inputs(
            &options,
            r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#,
        )
        .unwrap();
        assert_eq!(
            inputs.files.keys().map(String::as_str).collect::<Vec<_>>(),
            vec!["main.js"],
            "inadmissible precondition: original-only input"
        );
        Self {
            dir,
            inputs,
            options,
        }
    }
    fn with<R>(&self, f: impl FnOnce(&CpgContext<'_>) -> R) -> R {
        let built = build_context(&self.inputs, &self.options).unwrap();
        f(&built.ctx)
    }
    fn review(
        &self,
        algorithms: &[SlicingAlgorithm],
        request: Option<&ContextRequest>,
    ) -> ReviewRun {
        self.with(|ctx| {
            run_review_prepared(
                ctx,
                &self.inputs,
                algorithms,
                &SliceConfig::default(),
                &AlgorithmParams::default(),
                self.dir.path(),
                request,
            )
        })
    }
    fn single(
        &self,
        request: Option<&ContextRequest>,
        repo: &Path,
    ) -> (crate::api::AlgorithmOutcome, ContextState) {
        self.with(|ctx| {
            let (outcome, state, _) = run_algorithm_prepared(
                SlicingAlgorithm::Taint,
                ctx,
                &self.inputs,
                &SliceConfig::default(),
                &AlgorithmParams::default(),
                repo,
                request,
            )
            .unwrap();
            (outcome, state)
        })
    }
    fn control(&self) -> (SliceResult, ExpansionOutcome) {
        self.with(|ctx| {
            let config = TaintConfig::default();
            let (baseline, trace) =
                taint::slice_with_trace(ctx, &self.inputs.diff, &config).unwrap();
            assert!(
                !baseline.findings.is_empty(),
                "inadmissible precondition: nonempty original baseline"
            );
            let loader = ReviewContextLoader::new(&self.inputs).unwrap();
            let request = request(64);
            let guard = TimeGuard::new(request.limits.time);
            let outcome = expand(ExpansionRequest {
                loader: &loader,
                inputs: &self.inputs,
                baseline_ctx: ctx,
                baseline_trace: &trace,
                config: &config,
                limits: request.limits,
                ceiling: request.ceiling,
                guard: &guard,
            });
            assert!(
                outcome.bundle.context_files.contains("b.js"),
                "inadmissible precondition: real driver reaches b"
            );
            let analysis = outcome.bundle.analysis.as_ref().unwrap();
            assert!(
                analysis
                    .result
                    .findings
                    .iter()
                    .any(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink")),
                "inadmissible precondition: real context sink"
            );
            assert!(
                analysis
                    .result
                    .evidence
                    .iter()
                    .flatten()
                    .flat_map(|e| &e.hops)
                    .any(|hop| match hop {
                        EvidenceHop::DataFlow { from, to, .. } =>
                            from.file == "b.js" || to.file == "b.js",
                        EvidenceHop::Call { edge, .. } => edge.caller.file == "b.js",
                        _ => false,
                    }),
                "inadmissible precondition: real b witness"
            );
            (baseline, outcome)
        })
    }
}
fn request(max_files: usize) -> ContextRequest {
    ContextRequest {
        limits: ContextLimits::new(2, max_files).unwrap(),
        ceiling: CrossingCeiling::NameOnly,
    }
}
fn context(state: &ContextState) -> &ReviewContext {
    match state {
        ContextState::Expanded(context) => context,
        _ => panic!("expected Expanded context state"),
    }
}
fn bytes<T: serde::Serialize + ?Sized>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn evidence(result: &SliceResult) -> String {
    format!("{:?}", result.evidence)
}
fn same_result(a: &SliceResult, b: &SliceResult) {
    assert_eq!(bytes(a), bytes(b));
    assert_eq!(evidence(a), evidence(b));
}
fn same_run(a: &ReviewRun, b: &ReviewRun) {
    assert_eq!(bytes(&a.results), bytes(&b.results));
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
    for (a, b) in a.results.iter().zip(&b.results) {
        assert_eq!(evidence(a), evidence(b));
    }
    assert_eq!(bytes(&a.findings), bytes(&b.findings));
    assert_eq!(bytes(&a.errors), bytes(&b.errors));
    assert_eq!(a.warnings, b.warnings);
    assert_eq!(a.algorithms_run, b.algorithms_run);
    assert_eq!(
        a.diagnostics.as_ref().map(|d| d.messages()),
        b.diagnostics.as_ref().map(|d| d.messages())
    );
}
fn cache_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}
