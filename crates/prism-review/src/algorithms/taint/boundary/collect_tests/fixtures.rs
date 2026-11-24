use super::*;
use prism::api::{build_context, load_review_inputs, ReviewOptions};

pub struct Fixture {
    pub dir: tempfile::TempDir,
    pub inputs: ReviewInputs,
    options: ReviewOptions,
}
impl Fixture {
    pub fn new(files: &[(&str, &str)], loaded: &[&str], seeds: &[(&str, usize)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let diff = serde_json::json!({"files": loaded.iter().map(|path| serde_json::json!({
            "file_path": path, "modify_type": "Modified", "diff_lines": seeds.iter()
                .filter(|(file, _)| file == path).map(|(_, line)| line).collect::<Vec<_>>()
        })).collect::<Vec<_>>()});
        let options = ReviewOptions::new(dir.path());
        let inputs = load_review_inputs(&options, &diff.to_string()).unwrap();
        assert!(inputs.files.values().all(|p| p.parse_error_count == 0));
        Self {
            dir,
            inputs,
            options,
        }
    }
    pub fn context(&self) -> CpgContext<'_> {
        build_context(&self.inputs, &self.options).unwrap().ctx
    }
    pub fn trace(
        &self,
        ctx: &CpgContext<'_>,
        config: &TaintConfig,
    ) -> (prism::slice::SliceResult, TaintTrace) {
        let legacy = taint::slice(ctx, &self.inputs.diff, config).unwrap();
        let (result, trace) = taint::slice_with_trace(ctx, &self.inputs.diff, config).unwrap();
        assert_eq!(
            serde_json::to_value(&legacy).unwrap(),
            serde_json::to_value(&result).unwrap()
        );
        assert_eq!(
            format!("{:?}", legacy.evidence),
            format!("{:?}", result.evidence)
        );
        (result, trace)
    }
}
pub fn js(body: &str) -> Fixture {
    let source = format!("import {{ send }} from './transport.js';\nfunction h(req) {{\n{body}\n}}\nfunction loaded(x) {{}}\n");
    let seeds: Vec<_> = (3..3 + body.lines().count())
        .map(|line| ("a.js", line))
        .collect();
    Fixture::new(
        &[
            ("a.js", &source),
            ("transport.js", "export function send(x) {}"),
        ],
        &["a.js"],
        &seeds,
    )
}
pub fn site(ctx: &CpgContext<'_>, name: &str) -> CallSite {
    ctx.cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == name)
        .unwrap()
        .clone()
}
pub fn root_match(ctx: &CpgContext<'_>, trace: &TaintTrace, site: &CallSite) -> bool {
    let recovery = RecoveryContext::new(&ctx.files[&site.caller.file]);
    trace
        .roots
        .iter()
        .any(|r| recovery.matches_argument(site, &r.location))
}
pub fn checked(f: &Fixture, ctx: &CpgContext<'_>, trace: &TaintTrace) -> BoundaryCollection {
    let before = format!("{trace:?}");
    let result = collect(&f.inputs, ctx, trace);
    assert_eq!(before, format!("{trace:?}"));
    result
}
pub fn hint() -> ReviewImportHint {
    ReviewImportHint::InRepo {
        files: vec!["z.js".into(), "a.js".into()],
        spec: "./choice".into(),
        member: Some("send".into()),
        declaring: vec![],
        basis: ReviewHintBasis::Candidates,
    }
}
