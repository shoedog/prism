use super::*;
use crate::algorithms::taint::{
    self,
    boundary::records::{ContextRunNote, ContextRunNoteKind},
    context_budget::{ContextLimits, TimeGuard},
    frontier::Disposition,
    result_union::{union_results, UnionInput},
    rounds::{expand, ExpansionRequest},
};
use crate::api::{
    context_run::{ContextRequest, ContextState, UnionState},
    run::{run_algorithm_prepared, run_review_prepared},
    AlgorithmParams,
};
use prism::{
    api::{build_context, load_review_inputs, ReviewContextLoader, ReviewOptions},
    cpg::CrossingCeiling,
    finding_confidence::ResolutionMode,
    slice::{SliceConfig, SlicingAlgorithm},
};
fn note(text: &str) -> ContextRunNote {
    ContextRunNote {
        kind: ContextRunNoteKind::Ignored,
        message: text.into(),
        location: None,
    }
}
fn request(depth: u8) -> ContextRequest {
    ContextRequest {
        limits: ContextLimits::new(depth, 64).unwrap(),
        ceiling: CrossingCeiling::NameOnly,
    }
}
#[test]
fn d16_context_setter_is_once() {
    let mut d = RunDiagnostics::default();
    d.apply_context(None, vec![note("first")]);
    d.apply_context(None, vec![note("second")]);
    assert_eq!(d.project().unwrap().messages(), ["first"]);
}
#[test]
fn d17_notes_only_without_collection() {
    let mut d = RunDiagnostics::default();
    assert!(d.project().is_none());
    d.apply_context(None, vec![note("only")]);
    let result = d.project().unwrap();
    assert!(result.located().is_empty());
    assert_eq!(result.messages(), ["only"]);
}
#[test]
fn not_requested_some_does_not_invent_note() {
    let mut d = RunDiagnostics::default();
    d.reconcile_context(&ContextState::NotRequested, Some(&request(1)), &[]);
    assert!(d.project().is_none());
}
#[test]
fn d13_real_missing_root_notes_only() {
    let dir = tempfile::tempdir().unwrap();
    let options = ReviewOptions::new(dir.path().join("missing"));
    let inputs = load_review_inputs(&options, r#"{"files":[]}"#).unwrap();
    assert!(
        ReviewContextLoader::new(&inputs).is_err(),
        "inadmissible precondition: captured root unavailable"
    );
    let built = build_context(&inputs, &options).unwrap();
    let (out, state, _) = run_algorithm_prepared(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        Some(&request(1)),
    )
    .unwrap();
    assert!(matches!(state, ContextState::Unavailable(_)));
    assert!(out
        .diagnostics
        .unwrap()
        .messages()
        .iter()
        .any(|m| m.contains("context loading unavailable")));
}
#[test]
fn d18_native_depth_boundary_and_once() {
    let dir = tempfile::tempdir().unwrap();
    let files=[("main.js","import { h } from './a';\nexport function baseline(z) { eval(z); /* baseline diagram label deliberately long enough to exceed the renderer label limit, without changing flow */ }\nexport function main(x) { h(x); }\n"),("a.js","import { g } from './b';\nexport function h(x) { g(x); }\n"),("b.js","export function g(x) { eval(x); }\n")];
    for (p, s) in files {
        std::fs::write(dir.path().join(p), s).unwrap();
    }
    let mut options = ReviewOptions::new(dir.path());
    options.no_cache = true;
    options.resolution = ResolutionMode::Scoped;
    let inputs = load_review_inputs(
        &options,
        r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#,
    )
    .unwrap();
    let built = build_context(&inputs, &options).unwrap();
    let cfg = taint::TaintConfig::default();
    let (baseline, trace) = taint::slice_with_trace(&built.ctx, &inputs.diff, &cfg).unwrap();
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let direct = |depth| {
        let lim = ContextLimits::new(depth, 64).unwrap();
        let guard = TimeGuard::new(lim.time);
        expand(ExpansionRequest {
            loader: &loader,
            inputs: &inputs,
            baseline_ctx: &built.ctx,
            baseline_trace: &trace,
            config: &cfg,
            limits: lim,
            ceiling: CrossingCeiling::NameOnly,
            guard: &guard,
        })
    };
    let two = direct(2);
    let proof = two
        .bundle
        .analysis
        .as_ref()
        .expect("inadmissible precondition: native depth2 analysis");
    assert!(
        two.bundle.context_files.contains("b.js")
            && proof
                .result
                .findings
                .iter()
                .enumerate()
                .any(|(i, f)| f.file == "b.js"
                    && f.category.as_deref() == Some("taint_sink")
                    && proof
                        .result
                        .evidence
                        .get(i)
                        .and_then(Option::as_ref)
                        .is_some_and(|e| !e.hops.is_empty())),
        "inadmissible precondition: native depth2 sink/evidence"
    );
    let one = direct(1);
    assert!(
        one.bundle.context_files.contains("a.js") && !one.bundle.context_files.contains("b.js"),
        "inadmissible precondition: actual depth1 stage"
    );
    assert!(
        one.bundle
            .frontier
            .dispositions
            .iter()
            .any(|(i, _, d)| *d == Disposition::Resolved
                && one.bundle.frontier.provenance[*i].site.caller.file == "main.js"),
        "inadmissible precondition: main call resolved"
    );
    assert!(
        one.bundle
            .frontier
            .dispositions
            .iter()
            .any(|(i, target, d)| *d == Disposition::DepthBound
                && target == "b.js"
                && one.bundle.frontier.provenance[*i].site.caller.file == "a.js"),
        "inadmissible precondition: context call depth bound"
    );
    let analysis = one.bundle.analysis.as_ref().unwrap();
    let originals = inputs.files.keys().cloned().collect();
    let mut expected = union_results(UnionInput {
        baseline: &baseline,
        expanded: &analysis.result,
        attribution: &analysis.emission,
        original_files: &originals,
        context_files: &one.bundle.context_files,
    })
    .unwrap()
    .result;
    expected.align_evidence();
    crate::algorithms::finalize_diagrams(&mut expected, SliceConfig::default().diagram_node_cap);
    let req = request(1);
    let multi = run_review_prepared(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::Taint, SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        Some(&req),
    );
    assert_eq!(
        serde_json::to_vec(&multi.results[0]).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    assert_eq!(
        format!("{:?}", multi.results[0].evidence),
        format!("{:?}", expected.evidence)
    );
    let d = multi.diagnostics.unwrap();
    assert!(!d.located().iter().any(|n| n.file() == "main.js"));
    assert!(d
        .located()
        .iter()
        .any(|n| n.file() == "a.js" && n.line() == 2));
    assert_eq!(
        d.run_notes()
            .iter()
            .filter(|n| n.kind() == ContextRunNoteKind::DepthBound)
            .count(),
        1
    );
    let (single, _, _) = run_algorithm_prepared(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        Some(&req),
    )
    .unwrap();
    assert!(single
        .diagnostics
        .unwrap()
        .run_notes()
        .iter()
        .any(|n| n.kind() == ContextRunNoteKind::DepthBound));
    let off = run_review_prepared(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        None,
    );
    let no_taint = run_review_prepared(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::EchoSlice],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        Some(&req),
    );
    assert!(matches!(no_taint.context, ContextState::NotRequested));
    assert!(no_taint
        .diagnostics
        .as_ref()
        .is_none_or(|d| d.run_notes().is_empty()));
    assert!(off
        .diagnostics
        .as_ref()
        .is_none_or(|d| d.run_notes().is_empty()));
    // REAL coherent zero-admission state: same captured review, no invented frontier.
    let mut zero = request(1);
    zero.limits.max_files = 0;
    let no_admission = run_review_prepared(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        Some(&zero),
    );
    match &no_admission.context {
        ContextState::Expanded(c) => {
            assert!(matches!(c.union, UnionState::NotNeeded));
            assert!(c.context_files.is_empty());
        }
        _ => panic!("expected completed zero-admission state"),
    }
    assert_eq!(
        serde_json::to_vec(&no_admission.results[0]).unwrap(),
        serde_json::to_vec(&off.results[0]).unwrap()
    );
    assert_eq!(
        format!("{:?}", no_admission.results[0].evidence),
        format!("{:?}", off.results[0].evidence)
    );
    let original = off
        .diagnostics
        .as_ref()
        .expect("baseline boundary collection");
    let actual = no_admission
        .diagnostics
        .as_ref()
        .expect("zero-bound diagnostics");
    assert_eq!(actual.located(), original.located());
    assert_eq!(actual.run_notes().len(), 1);
    assert_eq!(actual.run_notes()[0].kind(), ContextRunNoteKind::FilesBound);
    assert!(actual.run_notes()[0]
        .message()
        .contains("max-files bound 0"));
}
