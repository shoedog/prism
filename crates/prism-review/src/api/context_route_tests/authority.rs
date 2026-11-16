use super::*;
use crate::algorithms::taint::boundary::records::ContextRunNoteKind;
use prism::api::ContextUnavailable;
#[test]
fn o13_authorized_single_and_multi_expand() {
    let f = Fixture::new(B);
    let p = imports();
    f.with(|ctx| {
        f.native(ctx);
        let req = ContextRequest {
            limits: ContextLimits::new(2, 64).unwrap(),
            ceiling: CrossingCeiling::NameOnly,
        };
        let direct = run_review_prepared(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            Some(&req),
        );
        let multi = run_review_authorized(
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        assert!(has_sink(&multi));
        assert!(multi.context().is_some());
        same_result(&direct.results[0], &multi.results[0]);
        assert_eq!(
            format!("{:?}", direct.diagnostics),
            format!("{:?}", multi.diagnostics)
        );
        let (expected, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            Some(&req),
        )
        .unwrap();
        let one = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        assert!(one.context().is_some());
        same_result(&expected.result, &one.outcome.result);
        same_result(&one.outcome.result, &multi.results[0]);
        assert_eq!(
            format!("{:?}", expected.diagnostics),
            format!("{:?}", one.outcome.diagnostics)
        );
        assert_eq!(
            format!("{:?}", one.outcome.diagnostics),
            format!("{:?}", multi.diagnostics)
        );
    });
}

#[test]
fn o14_high_level_review_authorized() {
    let f = Fixture::new(B);
    let p = imports();
    f.with(|ctx| {
        f.native(ctx);
        let req = ContextRequest {
            limits: ContextLimits::new(2, 64).unwrap(),
            ceiling: CrossingCeiling::NameOnly,
        };
        let expected = run_review_prepared(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            Some(&req),
        );
        let out = review(
            &f.options,
            DIFF,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
        )
        .unwrap();
        assert!(has_sink(&out.run));
        same_result(&expected.results[0], &out.run.results[0]);
        assert_eq!(
            format!("{:?}", expected.evidence),
            format!("{:?}", out.run.evidence)
        );
        assert!(!out.inputs.files.contains_key("b.js"));
        assert_eq!(out.inputs.diff_text_sha256, f.inputs.diff_text_sha256);
        assert_eq!(out.inputs.sources, f.inputs.sources);
        assert_eq!(
            serde_json::to_vec(&out.inputs.diff).unwrap(),
            serde_json::to_vec(&f.inputs.diff).unwrap()
        );
        assert_eq!(
            serde_json::to_vec(&out.inputs.parse_quality).unwrap(),
            serde_json::to_vec(&f.inputs.parse_quality).unwrap()
        );
        assert_eq!(
            out.inputs.files.keys().collect::<Vec<_>>(),
            f.inputs.files.keys().collect::<Vec<_>>()
        );
    });
}

#[test]
fn o15_missing_root_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let options = ReviewOptions::new(dir.path().join("missing"));
    let inputs = load_review_inputs(&options, r#"{"files":[]}"#).unwrap();
    assert!(
        matches!(
            ReviewContextLoader::new(&inputs),
            Err(ContextUnavailable::RootUnavailable { .. })
        ),
        "inadmissible precondition: captured root unavailable"
    );
    let built = build_context(&inputs, &options).unwrap();
    let p = imports();
    let before = run_review_prepared(
        &built.ctx,
        &inputs,
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &p,
        dir.path(),
        None,
    );
    let after = run_review_authorized(
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &p,
        dir.path(),
    );
    same_result(&before.results[0], &after.results[0]);
    let notes = after.diagnostics.as_ref().unwrap().run_notes();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].kind(), ContextRunNoteKind::Unavailable);
    assert!(notes[0].message().contains("context loading unavailable"));
    let (direct, _) = run_algorithm_prepared(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &p,
        dir.path(),
        None,
    )
    .unwrap();
    let single = run_algorithm_authorized(
        SlicingAlgorithm::Taint,
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &SliceConfig::default(),
        &p,
        dir.path(),
    )
    .unwrap();
    same_result(&direct.result, &single.outcome.result);
    let notes = single.outcome.diagnostics.as_ref().unwrap().run_notes();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].kind(), ContextRunNoteKind::Unavailable);
}

#[test]
fn o17_captured_root_wins() {
    let f = Fixture::new(B);
    let other = tempfile::tempdir().unwrap();
    std::fs::write(
        other.path().join("b.js"),
        "export function g(x) { eval(x); eval(x); }\n",
    )
    .unwrap();
    let p = imports();
    f.with(|ctx| {
        f.native(ctx);
        let out = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &SliceConfig::default(),
            &p,
            other.path(),
        )
        .unwrap();
        let own = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        assert_eq!(out.context().unwrap().sources["b.js"], B);
        same_result(&out.outcome.result, &own.outcome.result);
        assert_eq!(
            format!("{:?}", out.outcome.diagnostics),
            format!("{:?}", own.outcome.diagnostics)
        );
    });
}

#[test]
fn o18_duplicate_taint_one_expansion() {
    let f = Fixture::new(B);
    let p = imports();
    f.with(|ctx| {
        f.native(ctx);
        let one = run_review_authorized(
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        let two = run_review_authorized(
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &[SlicingAlgorithm::Taint, SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        assert!(one.context().is_some());
        assert!(has_sink(&one));
        let a = one.context().unwrap().attempts.attempted_files;
        assert!(a > 0);
        assert_eq!(two.context().unwrap().attempts.attempted_files, a);
        assert_eq!(
            two.context().unwrap().attempts.attempted_read_bytes,
            one.context().unwrap().attempts.attempted_read_bytes
        );
        assert_eq!(
            two.context().unwrap().attempts.skips,
            one.context().unwrap().attempts.skips
        );
        same_result(&one.results[0], &two.results[0]);
        same_result(&two.results[0], &two.results[1]);
    });
}
#[test]
fn o19_mixed_membrane_counterfactual() {
    let f = Fixture::new(
        "import { main } from './main';\nexport function g(x) { eval(x); main(x); }\n",
    );
    let p = imports();
    f.with(|ctx| {
        let native = f.native_unchecked(ctx);
        assert!(
            native.bundle.context_files.contains("b.js"),
            "behavioral native context admission"
        );
        assert!(
            native
                .bundle
                .analysis
                .as_ref()
                .is_some_and(|analysis| analysis.result.findings.iter().enumerate().any(
                    |(i, finding)| finding.file == "b.js"
                        && finding.category.as_deref() == Some("taint_sink")
                        && analysis
                            .result
                            .evidence
                            .get(i)
                            .and_then(Option::as_ref)
                            .is_some_and(|evidence| !evidence.hops.is_empty())
                )),
            "behavioral native sink and evidence"
        );
        let loader = ReviewContextLoader::new(&f.inputs).unwrap();
        let round = ReviewContextRound::new(&loader, &native.bundle.files);
        let union = round.build_context(f.inputs.type_db.as_ref(), &ctx.types);
        let original = crate::algorithms::membrane_slice::slice(ctx, &f.inputs.diff).unwrap();
        let full = crate::algorithms::membrane_slice::slice(&union, &f.inputs.diff).unwrap();
        assert_ne!(
            serde_json::to_vec(&original).unwrap(),
            serde_json::to_vec(&full).unwrap(),
            "behavioral native Membrane counterfactual differs with context"
        );
        let algorithms = [
            SlicingAlgorithm::Taint,
            SlicingAlgorithm::MembraneSlice,
            SlicingAlgorithm::EchoSlice,
        ];
        let off = run_review_prepared(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        );
        let on = run_review_authorized(
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &algorithms,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        assert!(on.context().is_some());
        assert!(on.context().unwrap().context_files.contains("b.js"));
        assert!(has_sink(&on));
        same_result(&off.results[1], &on.results[1]);
        same_result(&off.results[2], &on.results[2]);
    });
}
