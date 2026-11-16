use super::*;
#[test]
fn o01_default_matches_direct_prepared_none() {
    let f = Fixture::new(B);
    let params = AlgorithmParams::default();
    assert_eq!(params.taint_context.mode, TaintContextMode::Off);
    assert_eq!(params.taint_context.edges, TaintContextEdges::NameOnly);
    assert_eq!(params.taint_context.depth, 2);
    assert_eq!(params.taint_context.max_files, 64);
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    f.with(|ctx| {
        let baseline = run_review_prepared(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
            None,
        );
        assert!(!baseline.results.is_empty());
        assert!(
            !baseline.findings.is_empty(),
            "baseline finding nonvacuity is behavior"
        );
        let routed = run_review(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
        );
        same_run(&baseline, &routed);
        let authorized = run_review_authorized(
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &algorithms,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
        );
        same_run(&baseline, &authorized);
        let (direct, state, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
            None,
        )
        .unwrap();
        assert!(matches!(state, ContextState::NotRequested));
        let typed = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
        )
        .unwrap();
        same_result(&direct.result, &typed.result);
        assert_eq!(
            format!("{:?}", direct.diagnostics),
            format!("{:?}", typed.diagnostics)
        );
        let direct_legacy = direct.into_legacy_result();
        let legacy = run_algorithm(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
        )
        .unwrap();
        same_result(&direct_legacy, &legacy);
        let (direct, _, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &params,
            f.dir.path(),
            None,
        )
        .unwrap();
        let auth = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &SliceConfig::default(),
            &params,
            f.dir.path(),
        )
        .unwrap();
        same_result(&direct.result, &auth.outcome.result);
        assert_eq!(
            format!("{:?}", direct.diagnostics),
            format!("{:?}", auth.outcome.diagnostics)
        );
        assert!(auth.context().is_none());
    });
    let built = build_context(&f.inputs, &f.options).unwrap();
    let manual = run_review_prepared(
        &built.ctx,
        &f.inputs,
        &algorithms,
        &SliceConfig::default(),
        &params,
        f.dir.path(),
        None,
    );
    let actual = review(
        &f.options,
        DIFF,
        &algorithms,
        &SliceConfig::default(),
        &params,
    )
    .unwrap();
    same_run(&manual, &actual.run);
    assert_eq!(built.warnings, actual.build_warnings);
    assert_eq!(f.inputs.diff_text_sha256, actual.inputs.diff_text_sha256);
    assert_eq!(f.inputs.sources, actual.inputs.sources);
}
// CONTROLLED option-value table; no engine fixture.
#[test]
fn o02_valid_ranges_and_ceiling() {
    for depth in [1, 4] {
        for max in [1, usize::MAX] {
            for edges in [TaintContextEdges::Exact, TaintContextEdges::NameOnly] {
                let opt = TaintContextOptions {
                    mode: TaintContextMode::Imports,
                    edges,
                    depth,
                    max_files: max,
                };
                let r = opt
                    .validate()
                    .ok()
                    .flatten()
                    .expect("valid Imports must make request");
                assert_eq!(r.limits.depth, depth);
                assert_eq!(r.limits.max_files, max);
                assert_eq!(
                    r.ceiling,
                    if edges == TaintContextEdges::Exact {
                        CrossingCeiling::Exact
                    } else {
                        CrossingCeiling::NameOnly
                    }
                );
                let mut off = opt;
                off.mode = TaintContextMode::Off;
                assert!(matches!(off.validate(), Ok(None)));
            }
        }
    }
}
// CONTROLLED option-value table; no engine fixture.
#[test]
fn o03_invalid_ranges() {
    for mode in [TaintContextMode::Off, TaintContextMode::Imports] {
        for depth in [0, 5] {
            let opt = TaintContextOptions {
                mode,
                depth,
                ..Default::default()
            };
            let text = opt.validate().err().unwrap().to_string();
            assert!(text.contains("depth") && text.contains("1..=4"));
        }
        let opt = TaintContextOptions {
            mode,
            max_files: 0,
            ..Default::default()
        };
        let text = opt.validate().err().unwrap().to_string();
        assert!(text.contains("max_files") && text.contains("at least 1"));
    }
}
#[test]
fn o04_invalid_off_result() {
    let f = Fixture::new(B);
    let mut p = AlgorithmParams::default();
    p.taint_context.depth = 9;
    f.with(|ctx| {
        assert!(run_algorithm(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path()
        )
        .err()
        .unwrap()
        .to_string()
        .starts_with("invalid taint-context options:"));
    });
    let malformed = "{";
    assert!(
        load_review_inputs(&f.options, malformed).is_err(),
        "inadmissible precondition: identical malformed input fails loading"
    );
    for mode in [TaintContextMode::Off, TaintContextMode::Imports] {
        p.taint_context.mode = mode;
        let err = review(&f.options, malformed, &[], &SliceConfig::default(), &p)
            .err()
            .unwrap();
        assert!(err
            .to_string()
            .starts_with("invalid taint-context options:"));
        f.with(|ctx| {
            let typed = run_algorithm_with_diagnostics(
                SlicingAlgorithm::EchoSlice,
                ctx,
                &f.inputs,
                &SliceConfig::default(),
                &p,
                f.dir.path(),
            )
            .err()
            .unwrap();
            assert!(typed
                .to_string()
                .starts_with("invalid taint-context options:"));
            let legacy = run_algorithm(
                SlicingAlgorithm::EchoSlice,
                ctx,
                &f.inputs,
                &SliceConfig::default(),
                &p,
                f.dir.path(),
            )
            .err()
            .unwrap();
            assert!(legacy
                .to_string()
                .starts_with("invalid taint-context options:"));
            let authorized = run_algorithm_authorized(
                SlicingAlgorithm::EchoSlice,
                ctx,
                ReviewAuthority::capture(&f.inputs),
                &SliceConfig::default(),
                &p,
                f.dir.path(),
            )
            .err()
            .unwrap();
            assert!(authorized
                .to_string()
                .starts_with("invalid taint-context options:"));
        });
    }
}
#[test]
fn o16_explicit_off_parity() {
    let f = Fixture::new(B);
    let mut p = AlgorithmParams::default();
    p.taint_context.depth = 4;
    p.taint_context.max_files = 1;
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    f.with(|ctx| {
        let direct = run_review_prepared(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        );
        let legacy = run_review(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        let authorized = run_review_authorized(
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &algorithms,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        same_run(&direct, &legacy);
        same_run(&direct, &authorized);
        let (expected, state, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
            None,
        )
        .unwrap();
        assert!(matches!(state, ContextState::NotRequested));
        let actual = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        same_result(&expected.result, &actual.result);
        assert_eq!(
            format!("{:?}", expected.diagnostics),
            format!("{:?}", actual.diagnostics)
        );
        let authorized = run_algorithm_authorized(
            SlicingAlgorithm::Taint,
            ctx,
            ReviewAuthority::capture(&f.inputs),
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        same_result(&expected.result, &authorized.outcome.result);
        assert!(authorized.context().is_none());
        assert_eq!(
            format!("{:?}", expected.diagnostics),
            format!("{:?}", authorized.outcome.diagnostics)
        );
        let expected_legacy = expected.into_legacy_result();
        let actual_legacy = run_algorithm(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        )
        .unwrap();
        same_result(&expected_legacy, &actual_legacy);
    });
    let built = build_context(&f.inputs, &f.options).unwrap();
    let expected = run_review_prepared(
        &built.ctx,
        &f.inputs,
        &algorithms,
        &SliceConfig::default(),
        &p,
        f.dir.path(),
        None,
    );
    let actual = review(&f.options, DIFF, &algorithms, &SliceConfig::default(), &p).unwrap();
    same_run(&expected, &actual.run);
    assert_eq!(built.warnings, actual.build_warnings);
    assert_eq!(f.inputs.diff_text_sha256, actual.inputs.diff_text_sha256);
    assert_eq!(f.inputs.sources, actual.inputs.sources);
}
