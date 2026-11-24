use super::*;
pub(super) const MAIN:&str="import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n";
pub(super) const A: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
pub(super) const B: &str = "export function g(x) { eval(x); }\n";
pub(super) const DIFF: &str =
    r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#;
pub(super) struct Fixture {
    pub dir: tempfile::TempDir,
    pub options: ReviewOptions,
    pub inputs: ReviewInputs,
}
impl Fixture {
    pub fn new(b: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (p, s) in [("main.js", MAIN), ("a.js", A), ("b.js", b)] {
            std::fs::write(dir.path().join(p), s).unwrap();
        }
        let mut options = ReviewOptions::new(dir.path());
        options.no_cache = true;
        options.resolution = ResolutionMode::Scoped;
        let inputs = load_review_inputs(&options, DIFF).unwrap();
        assert!(
            inputs.files.contains_key("main.js"),
            "inadmissible precondition: original parsed"
        );
        Self {
            dir,
            options,
            inputs,
        }
    }
    pub fn with<R>(&self, f: impl FnOnce(&CpgContext<'_>) -> R) -> R {
        let built = build_context(&self.inputs, &self.options).unwrap();
        f(&built.ctx)
    }
    pub fn native_unchecked(&self, ctx: &CpgContext<'_>) -> ExpansionOutcome {
        let loader = ReviewContextLoader::new(&self.inputs).unwrap();
        let config = taint::TaintConfig::default();
        let (_, trace) = taint::slice_with_trace(ctx, &self.inputs.diff, &config).unwrap();
        let limits = ContextLimits::new(2, 64).unwrap();
        let guard = TimeGuard::new(limits.time);
        expand(ExpansionRequest {
            loader: &loader,
            inputs: &self.inputs,
            baseline_ctx: ctx,
            baseline_trace: &trace,
            config: &config,
            limits,
            ceiling: CrossingCeiling::NameOnly,
            guard: &guard,
        })
    }
    pub fn native(&self, ctx: &CpgContext<'_>) -> ExpansionOutcome {
        let out = self.native_unchecked(ctx);
        assert!(
            out.bundle.context_files.contains("b.js")
                && out.bundle.analysis.as_ref().is_some_and(|a| a
                    .result
                    .findings
                    .iter()
                    .enumerate()
                    .any(|(i, f)| f.file == "b.js"
                        && f.category.as_deref() == Some("taint_sink")
                        && a.result
                            .evidence
                            .get(i)
                            .and_then(Option::as_ref)
                            .is_some_and(|e| !e.hops.is_empty()))),
            "inadmissible precondition: independent native b sink/evidence"
        );
        out
    }
}
pub(super) fn imports() -> AlgorithmParams {
    let mut p = AlgorithmParams::default();
    p.taint_context.mode = TaintContextMode::Imports;
    p
}
pub(super) fn same_result(a: &SliceResult, b: &SliceResult) {
    assert_eq!(
        serde_json::to_vec(a).unwrap(),
        serde_json::to_vec(b).unwrap()
    );
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
}
pub(super) fn same_run(a: &ReviewRun, b: &ReviewRun) {
    assert_eq!(
        serde_json::to_vec(&a.results).unwrap(),
        serde_json::to_vec(&b.results).unwrap()
    );
    assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
    assert_eq!(a.results.len(), b.results.len());
    for (left, right) in a.results.iter().zip(&b.results) {
        assert_eq!(
            format!("{:?}", left.evidence),
            format!("{:?}", right.evidence)
        );
    }
    assert_eq!(
        serde_json::to_vec(&a.findings).unwrap(),
        serde_json::to_vec(&b.findings).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&a.errors).unwrap(),
        serde_json::to_vec(&b.errors).unwrap()
    );
    assert_eq!(a.warnings, b.warnings);
    assert_eq!(a.algorithms_run, b.algorithms_run);
    assert_eq!(
        format!("{:?}", a.diagnostics),
        format!("{:?}", b.diagnostics)
    );
    assert_eq!(a.min_confidence, b.min_confidence);
    assert_eq!(a.resolution, b.resolution);
}
pub(super) fn messages(r: &ReviewRun) -> Vec<String> {
    r.diagnostics
        .as_ref()
        .map(|d| d.messages())
        .unwrap_or_default()
}
pub(super) fn has_sink(r: &ReviewRun) -> bool {
    r.results
        .iter()
        .flat_map(|x| &x.findings)
        .any(|f| f.file == "b.js" && f.category.as_deref() == Some("taint_sink"))
}
