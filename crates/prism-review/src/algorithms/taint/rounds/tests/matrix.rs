//! Part M matrix: real captured review except explicitly labeled controlled rows.
use super::super::super::boundary::collect::collect_round;
use super::super::super::frontier::plan_round;
use super::super::super::source_scope::loading::project_loading_trace;
use super::*;
use prism::api::ReviewBoundaryHints;

mod ceilings;
mod controls;
mod routes;

fn named(files: &[(&str, &str)], original: &str, lines: &[usize]) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    for (name, source) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let options = ReviewOptions::new(dir.path());
    let diff = serde_json::json!({"files":[{"file_path":original,"modify_type":"Modified","diff_lines":lines}]});
    let inputs = load_review_inputs(&options, &diff.to_string()).unwrap();
    assert!(
        inputs.files.contains_key(original),
        "inadmissible precondition: original parse"
    );
    Fixture {
        _dir: dir,
        inputs,
        options,
    }
}
fn variant<F, R>(
    f: &Fixture,
    config: &TaintConfig,
    limits: ContextLimits,
    ceiling: CrossingCeiling,
    guard: &dyn RunGuard,
    cb: F,
) -> R
where
    F: FnOnce(&ExpansionRequest<'_>) -> R,
{
    let built = build_context(&f.inputs, &f.options).unwrap();
    let loader =
        ReviewContextLoader::new(&f.inputs).expect("inadmissible precondition: captured loader");
    let (_, trace) =
        super::super::super::slice_with_trace(&built.ctx, &f.inputs.diff, config).unwrap();
    let r = ExpansionRequest {
        loader: &loader,
        inputs: &f.inputs,
        baseline_ctx: &built.ctx,
        baseline_trace: &trace,
        config,
        limits,
        ceiling,
        guard,
    };
    cb(&r)
}
fn first_plan(r: &ExpansionRequest<'_>) -> FrontierState {
    let original = r.inputs.files.keys().cloned().collect();
    let trace = project_loading_trace(r.baseline_trace, &original, &r.inputs.files);
    let mut hints = ReviewBoundaryHints::new(r.inputs).expect("inadmissible precondition: hints");
    let collected = collect_round(&mut hints, r.baseline_ctx, &trace);
    plan_round(
        1,
        r.limits.depth,
        &collected,
        r.baseline_ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    )
}
fn sinks(out: &ExpansionOutcome, path: &str) -> Vec<usize> {
    out.bundle
        .analysis
        .as_ref()
        .into_iter()
        .flat_map(|a| a.result.findings.iter())
        .filter(|f| f.file == path && f.category.as_deref() == Some("taint_sink"))
        .map(|f| f.line)
        .collect()
}
