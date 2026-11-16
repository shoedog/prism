use super::*;
use crate::algorithms::taint::boundary::records::ContextRunNoteKind;
const IGNORED: &str = "taint-context ignored: taint not selected";
const UNAUTHORIZED:&str="taint-context: imports requested through an entry point without loading authority; baseline taint only (use run_review_authorized or run_algorithm_authorized)";
#[test]
fn o05_invalid_before_unauthorized() {
    let f = Fixture::new(B);
    let mut p = imports();
    p.taint_context.depth = 9;
    f.with(|ctx| {
        let e = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .err()
        .unwrap();
        assert!(e.to_string().starts_with("invalid taint-context options:"));
    });
}
#[test]
fn o06_invalid_nontaint_result() {
    let f = Fixture::new(B);
    for mode in [TaintContextMode::Off, TaintContextMode::Imports] {
        let mut p = imports();
        p.taint_context.mode = mode;
        p.taint_context.max_files = 0;
        f.with(|ctx| {
            let e = run_algorithm_authorized(
                SlicingAlgorithm::EchoSlice,
                ctx,
                ReviewAuthority::capture(&f.inputs),
                &SliceConfig::default(),
                &p,
                f.dir.path(),
            )
            .err()
            .unwrap()
            .to_string();
            assert!(e.starts_with("invalid taint-context options:"));
            assert!(e.contains("max_files"));
        });
    }
}

#[test]
fn o07_invalid_per_taint_errors() {
    let f = Fixture::new(B);
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::EchoSlice,
        SlicingAlgorithm::Taint,
    ];
    for mode in [TaintContextMode::Off, TaintContextMode::Imports] {
        let mut p = imports();
        p.taint_context.mode = mode;
        p.taint_context.depth = 0;
        f.with(|ctx| {
            let baseline = run_review_prepared(
                ctx,
                &f.inputs,
                &[SlicingAlgorithm::EchoSlice],
                &SliceConfig::default(),
                &p,
                f.dir.path(),
                None,
            );
            for authorized in [false, true] {
                let out = if authorized {
                    run_review_authorized(
                        ctx,
                        ReviewAuthority::capture(&f.inputs),
                        &algorithms,
                        &SliceConfig::default(),
                        &p,
                        f.dir.path(),
                    )
                } else {
                    run_review(
                        ctx,
                        &f.inputs,
                        &algorithms,
                        &SliceConfig::default(),
                        &p,
                        f.dir.path(),
                    )
                };
                assert_eq!(out.errors.len(), 2);
                assert!(out
                    .errors
                    .iter()
                    .all(|e| e.algorithm == SlicingAlgorithm::Taint.name()
                        && e.error.starts_with("invalid taint-context options:")));
                assert_eq!(out.results.len(), 1);
                same_result(&baseline.results[0], &out.results[0]);
                assert_eq!(
                    out.algorithms_run,
                    algorithms
                        .iter()
                        .map(|a| a.name().to_string())
                        .collect::<Vec<_>>()
                );
                assert!(out.diagnostics.as_ref().is_none_or(|d| d
                    .run_notes()
                    .iter()
                    .all(|n| n.kind() != ContextRunNoteKind::Ignored
                        && n.kind() != ContextRunNoteKind::Unauthorized)));
            }
        });
    }
}

#[test]
fn o08_no_taint_ignored_once() {
    let f = Fixture::new(B);
    let p = imports();
    let algorithms = [SlicingAlgorithm::AbsenceSlice, SlicingAlgorithm::EchoSlice];
    f.with(|ctx| {
        let baseline = run_review_prepared(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        );
        for authorized in [false, true] {
            let actual = if authorized {
                run_review_authorized(
                    ctx,
                    ReviewAuthority::capture(&f.inputs),
                    &algorithms,
                    &SliceConfig::default(),
                    &p,
                    f.dir.path(),
                )
            } else {
                run_review(
                    ctx,
                    &f.inputs,
                    &algorithms,
                    &SliceConfig::default(),
                    &p,
                    f.dir.path(),
                )
            };
            assert_eq!(actual.results.len(), baseline.results.len());
            for (a, b) in actual.results.iter().zip(&baseline.results) {
                same_result(a, b);
            }
            assert_eq!(
                format!("{:?}", actual.evidence),
                format!("{:?}", baseline.evidence)
            );
            assert_eq!(actual.algorithms_run, baseline.algorithms_run);
            assert_eq!(messages(&actual), [IGNORED]);
            let d = actual.diagnostics.as_ref().unwrap();
            assert!(d.located().is_empty());
            assert_eq!(d.run_notes().len(), 1);
            assert_eq!(d.run_notes()[0].kind(), ContextRunNoteKind::Ignored);
            assert_eq!(d.run_notes()[0].location(), None);
            assert_eq!(
                &actual.warnings[..baseline.warnings.len()],
                &baseline.warnings
            );
            assert_eq!(actual.warnings.len(), baseline.warnings.len() + 1);
            assert_eq!(actual.warnings.last().map(String::as_str), Some(IGNORED));
        }
    });
}

#[test]
fn o09_invalid_no_taint_distinguishes_mode() {
    let f = Fixture::new(B);
    let algorithms = [SlicingAlgorithm::AbsenceSlice, SlicingAlgorithm::EchoSlice];
    for mode in [TaintContextMode::Off, TaintContextMode::Imports] {
        let mut p = imports();
        p.taint_context.mode = mode;
        p.taint_context.depth = 0;
        f.with(|ctx| {
            let baseline = run_review_prepared(
                ctx,
                &f.inputs,
                &algorithms,
                &SliceConfig::default(),
                &p,
                f.dir.path(),
                None,
            );
            for authorized in [false, true] {
                let out = if authorized {
                    run_review_authorized(
                        ctx,
                        ReviewAuthority::capture(&f.inputs),
                        &algorithms,
                        &SliceConfig::default(),
                        &p,
                        f.dir.path(),
                    )
                } else {
                    run_review(
                        ctx,
                        &f.inputs,
                        &algorithms,
                        &SliceConfig::default(),
                        &p,
                        f.dir.path(),
                    )
                };
                assert!(out.errors.is_empty());
                assert_eq!(out.algorithms_run, baseline.algorithms_run);
                assert_eq!(out.results.len(), baseline.results.len());
                for (a, b) in out.results.iter().zip(&baseline.results) {
                    same_result(a, b);
                }
                assert_eq!(
                    format!("{:?}", out.evidence),
                    format!("{:?}", baseline.evidence)
                );
                let mut expected = baseline.warnings.clone();
                if mode == TaintContextMode::Imports {
                    expected.push(IGNORED.into());
                    assert_eq!(messages(&out), [IGNORED]);
                    let notes = out.diagnostics.as_ref().unwrap().run_notes();
                    assert_eq!(notes.len(), 1);
                    assert_eq!(notes[0].kind(), ContextRunNoteKind::Ignored);
                    assert_eq!(notes[0].location(), None);
                } else {
                    assert!(messages(&out).is_empty());
                }
                assert_eq!(out.warnings, expected);
            }
        });
    }
}

#[test]
fn o10_legacy_unauthorized_is_baseline() {
    let f = Fixture::new(B);
    let p = imports();
    f.with(|ctx| {
        f.native(ctx);
        let base = run_review_prepared(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        );
        let out = run_review(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        same_result(&base.results[0], &out.results[0]);
        let mut expected = messages(&base);
        expected.push(UNAUTHORIZED.into());
        assert_eq!(messages(&out), expected);
        let old = base.diagnostics.as_ref().unwrap();
        let new = out.diagnostics.as_ref().unwrap();
        assert_eq!(new.located(), old.located());
        assert_eq!(new.summary(), old.summary());
        assert_eq!(new.availability(), old.availability());
        assert_eq!(new.run_notes().len(), 1);
        assert_eq!(new.run_notes()[0].kind(), ContextRunNoteKind::Unauthorized);
        assert_eq!(new.run_notes()[0].location(), None);
        let mut warnings = base.warnings.clone();
        warnings.push(UNAUTHORIZED.into());
        assert_eq!(out.warnings, warnings);
        let (direct, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        )
        .unwrap();
        let typed = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        same_result(&direct.result, &typed.result);
        let old = direct.diagnostics.as_ref().unwrap();
        let new = typed.diagnostics.as_ref().unwrap();
        let mut expected = old.messages();
        expected.push(UNAUTHORIZED.into());
        assert_eq!(new.messages(), expected);
        assert_eq!(new.located(), old.located());
        assert_eq!(new.summary(), old.summary());
        assert_eq!(new.availability(), old.availability());
        assert_eq!(new.run_notes().len(), 1);
        assert_eq!(new.run_notes()[0].kind(), ContextRunNoteKind::Unauthorized);
    });
}

#[test]
fn o11_legacy_does_not_read_context() {
    // Output isolation under a hostile non-UTF8 target; not a syscall-count assertion.
    let f = Fixture::new(B);
    let p = imports();
    f.with(|ctx| {
        f.native(ctx);
        let a = run_review(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        let single_before = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        std::fs::write(f.dir.path().join("b.js"), [0xff]).unwrap();
        let b = run_review(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        same_run(&a, &b);
        assert!(messages(&b)
            .iter()
            .all(|m| !m.contains("context file skipped")));
        let single_after = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        same_result(&single_before.result, &single_after.result);
        assert_eq!(
            format!("{:?}", single_before.diagnostics),
            format!("{:?}", single_after.diagnostics)
        );
    });
}

#[test]
fn o12_legacy_result_appends_note_once() {
    let f = Fixture::new(B);
    let p = imports();
    f.with(|ctx| {
        let (direct, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        )
        .unwrap();
        let baseline = direct.into_legacy_result();
        let out = run_algorithm(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        assert_eq!(
            out.warnings
                .iter()
                .filter(|m| m.as_str() == UNAUTHORIZED)
                .count(),
            1
        );
        assert_eq!(out.warnings.last().map(String::as_str), Some(UNAUTHORIZED));
        let mut stripped = out.clone();
        stripped.warnings.pop();
        same_result(&baseline, &stripped);
    });
}

#[test]
fn o20_decision_total() {
    // CONTROLLED total decision table using the same nondefault borrowed request.
    let req = ContextRequest {
        limits: ContextLimits::new(3, 7).unwrap(),
        ceiling: CrossingCeiling::Exact,
    };
    for selected in [false, true] {
        for authorized in [false, true] {
            assert!(matches!(decide(None, selected, authorized), Route::Plain));
            match (
                selected,
                authorized,
                decide(Some(&req), selected, authorized),
            ) {
                (false, _, Route::Ignored) | (true, false, Route::Unauthorized) => {}
                (true, true, Route::Authorized(actual)) => {
                    assert!(std::ptr::eq(actual, &req));
                    assert_eq!(actual.limits.depth, 3);
                    assert_eq!(actual.limits.max_files, 7);
                    assert_eq!(actual.ceiling, CrossingCeiling::Exact);
                }
                _ => panic!("incorrect route"),
            }
        }
    }
}
