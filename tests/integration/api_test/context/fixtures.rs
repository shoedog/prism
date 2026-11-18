//! REAL accepted separated-line JS fixture; output requirements are behavioral assertions.
use super::*;
pub(super) const MAIN: &str = "import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
pub(super) const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
pub(super) const B: &str = "export function g(x) { eval(x); }\n";
pub(super) const DEGRADED: &str =
    "export function g(x) { eval(x); }\nfunction unrelated() { const broken = ; }\n";
pub(super) struct Fixture {
    pub dir: TempDir,
    pub options: ReviewOptions,
    pub diff: String,
}
impl Fixture {
    pub fn new(body: &str) -> Self {
        let dir = TempDir::new().unwrap();
        for (file, text) in [("main.js", MAIN), ("a.js", A), ("b.js", body)] {
            fs::write(dir.path().join(file), text).unwrap();
        }
        let mut options = ReviewOptions::new(dir.path());
        options.no_cache = true;
        options.resolution = ResolutionMode::Scoped;
        Self {
            dir,
            options,
            diff:
                r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#
                    .into(),
        }
    }
    pub fn inputs(&self) -> ReviewInputs {
        let inputs = load_review_inputs(&self.options, &self.diff).unwrap();
        assert_eq!(
            inputs.files.keys().map(String::as_str).collect::<Vec<_>>(),
            ["main.js"],
            "inadmissible precondition: only the original file was captured"
        );
        assert_eq!(
            inputs.sources["main.js"], MAIN,
            "inadmissible precondition: accepted source bytes"
        );
        inputs
    }
    pub fn review(&self, params: &AlgorithmParams) -> ReviewOutcome {
        // Capture independently first; do not disguise missing SUT output as a fixture guard.
        self.inputs();
        review(
            &self.options,
            &self.diff,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            params,
        )
        .unwrap()
    }
}
pub(super) fn imports() -> AlgorithmParams {
    let mut params = AlgorithmParams::default();
    params.taint_context.mode = TaintContextMode::Imports;
    params
}
pub(super) fn sink(result: &prism::slice::SliceResult, file: &str) -> usize {
    result
        .findings
        .iter()
        .position(|f| f.file == file && f.category.as_deref() == Some("taint_sink"))
        .expect("behavior: required native context sink")
}
pub(super) fn pair(a: &prism::slice::SliceResult, b: &prism::slice::SliceResult) {
    let normalize = |r: &prism::slice::SliceResult| {
        let mut r = r.clone();
        for f in &mut r.findings {
            f.parse_quality = None;
        }
        serde_json::to_value(r).unwrap()
    };
    assert_eq!(normalize(a), normalize(b));
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
}
pub(super) fn notes(run: &ReviewRun) -> &[ContextRunNote] {
    run.diagnostics
        .as_ref()
        .map(|d| d.run_notes())
        .unwrap_or(&[])
}
pub(super) fn value<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap()
}
pub(super) fn has_sink(v: &Value, field: &str, file: &str) -> bool {
    v[field].as_array().is_some_and(|fs| {
        fs.iter()
            .any(|f| f["file"] == file && f["category"] == "taint_sink")
    })
}

// REAL accepted N05 missing-root/empty-diff capture; no constructor for private refusal state.
pub(super) fn unavailable_inputs() -> (TempDir, ReviewOptions, ReviewInputs) {
    let dir = TempDir::new().unwrap();
    let options = ReviewOptions::new(dir.path().join("missing"));
    let inputs = load_review_inputs(&options, r#"{"files":[]}"#).unwrap();
    assert!(
        matches!(
            ReviewContextLoader::new(&inputs),
            Err(ContextUnavailable::RootUnavailable { .. })
        ),
        "inadmissible precondition: accepted missing-root capture"
    );
    (dir, options, inputs)
}
