use super::*;

#[test]
fn n05_missing_authority_returns_the_baseline() {
    let dir = tempfile::tempdir().unwrap();
    let options = ReviewOptions::new(dir.path().join("missing"));
    let inputs = load_review_inputs(&options, r#"{"files":[]}"#).unwrap();
    assert!(
        matches!(
            ReviewContextLoader::new(&inputs),
            Err(ContextUnavailable::RootUnavailable { .. })
        ),
        "inadmissible precondition: public missing-root capture recipe"
    );
    let built = build_context(&inputs, &options).unwrap();
    let old = run_algorithm_with_diagnostics(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
    )
    .unwrap();
    let (new, state, _) = run_algorithm_prepared(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        dir.path(),
        Some(&request(64)),
    )
    .unwrap();
    assert!(matches!(
        state,
        ContextState::Unavailable(ContextUnavailable::RootUnavailable { .. })
    ));
    same_result(&old.result, &new.result);
    assert!(new.result.findings.is_empty());
}

#[test]
fn n11_nothing_admitted_and_union_refusal_keep_baseline() {
    let f = Fixture::new(B);
    let off = f.review(&[SlicingAlgorithm::Taint], None);
    let on = f.review(&[SlicingAlgorithm::Taint], Some(&request(0)));
    let c = context(&on.context);
    assert!(matches!(c.stop, Stop::Bound(_)));
    assert!(matches!(c.union, UnionState::NotNeeded));
    assert_eq!(c.attempts.attempted_files, 0);
    assert_eq!(c.published.files, 0);
    assert!(c.view().is_none() && c.union_stats().is_none() && c.added_block_ids().is_empty());
    same_result(&off.results[0], &on.results[0]);
    // CONTROLLED raw baseline ID overflow through the ordinary owned settlement seam.
    let (mut baseline, outcome) = f.control();
    baseline.blocks[0].block_id = usize::MAX;
    let before = bytes(&baseline);
    let witness = evidence(&baseline);
    let stop = format!("{:?}", outcome.stop);
    let (result, c) = complete(baseline, outcome, &f.inputs.files.keys().cloned().collect());
    assert_eq!(bytes(&result), before);
    assert_eq!(evidence(&result), witness);
    assert!(matches!(
        c.union,
        UnionState::Refused(UnionError::BlockIdOverflow)
    ));
    assert!(
        c.context_files.contains("b.js") && c.published.files > 0 && c.attempts.attempted_files > 0
    );
    assert!(c.stats.rounds_completed > 0 && c.round > 0);
    assert_eq!(format!("{:?}", c.stop), stop);
    assert!(c.union_stats().is_none() && c.added_block_ids().is_empty() && c.view().is_none());
    let state = ContextState::Expanded(c);
    assert!(!state
        .view(&f.inputs, SlicingAlgorithm::Taint)
        .files
        .contains_key("b.js"));
}

#[test]
fn n12_diagrams_are_finalized_once() {
    let f = Fixture::new(B);
    let (baseline, outcome) = f.control();
    let analysis = outcome.bundle.analysis.as_ref().unwrap();
    let mut once = union_results(UnionInput {
        baseline: &baseline,
        expanded: &analysis.result,
        attribution: &analysis.emission,
        original_files: &f.inputs.files.keys().cloned().collect(),
        context_files: &outcome.bundle.context_files,
    })
    .unwrap()
    .result;
    assert!(
        once.diagram_warnings.is_empty(),
        "inadmissible precondition: independent union is raw"
    );
    crate::algorithms::finalize_diagrams(&mut once, 40);
    assert!(
        !once.diagram_warnings.is_empty(),
        "inadmissible precondition: actual long-label warnings"
    );
    let mut twice = once.clone();
    crate::algorithms::finalize_diagrams(&mut twice, 40);
    assert!(
        twice.diagram_warnings.len() > once.diagram_warnings.len(),
        "inadmissible precondition: double-finalization control differs"
    );
    let on = f.review(
        &[SlicingAlgorithm::Taint, SlicingAlgorithm::Taint],
        Some(&request(64)),
    );
    assert!(context(&on.context).view().is_some());
    for result in &on.results {
        assert_eq!(result.diagram_warnings, once.diagram_warnings);
        same_result(result, &once);
    }
    f.with(|ctx| {
        let old = run_review(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            f.dir.path(),
        );
        let new = run_review_prepared(
            ctx,
            &f.inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            f.dir.path(),
            None,
        );
        same_run(&old, &new);
    });
}
