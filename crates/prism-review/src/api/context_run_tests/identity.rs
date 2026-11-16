use super::*;

#[test]
fn n01_no_request_is_todays_run() {
    let f = Fixture::new(B);
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    f.with(|ctx| {
        let old = run_review(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            f.dir.path(),
        );
        let new = run_review_prepared(
            ctx,
            &f.inputs,
            &algorithms,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            f.dir.path(),
            None,
        );
        same_run(&old, &new);
        assert!(matches!(new.context, ContextState::NotRequested));
        let old = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            f.dir.path(),
        )
        .unwrap();
        let (new, state) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &f.inputs,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            f.dir.path(),
            None,
        )
        .unwrap();
        same_result(&old.result, &new.result);
        assert_eq!(
            old.diagnostics.as_ref().map(|d| d.messages()),
            new.diagnostics.as_ref().map(|d| d.messages())
        );
        assert!(matches!(state, ContextState::NotRequested));
    });
    let no_taint = f.review(&[SlicingAlgorithm::EchoSlice], Some(&request(64)));
    assert!(matches!(no_taint.context, ContextState::NotRequested));
}

#[test]
fn n03_mixed_algorithms_do_not_see_context() {
    let body = "import { main } from './main';\nexport function g(x) { eval(x); main(x); }\n";
    let f = Fixture::new(body);
    let (_, outcome) = f.control();
    f.with(|ctx| {
        let loader = ReviewContextLoader::new(&f.inputs).unwrap();
        let round = ReviewContextRound::new(&loader, &outcome.bundle.files);
        let full = round.build_context(f.inputs.type_db.as_ref(), &ctx.types);
        let original = crate::algorithms::membrane_slice::slice(ctx, &f.inputs.diff).unwrap();
        let leaked = crate::algorithms::membrane_slice::slice(&full, &f.inputs.diff).unwrap();
        assert_ne!(
            bytes(&original),
            bytes(&leaked),
            "inadmissible precondition: context caller would perturb Membrane"
        );
    });
    let algorithms = [
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    let off = f.review(&algorithms, None);
    let on = f.review(&algorithms, Some(&request(64)));
    assert!(!context(&on.context).context_files.is_empty());
    for i in [1, 2] {
        same_result(&off.results[i], &on.results[i]);
    }
    assert!(context(&on.context)
        .view()
        .unwrap()
        .files
        .contains_key("b.js"));
    assert!(!on
        .context
        .view(&f.inputs, SlicingAlgorithm::MembraneSlice)
        .files
        .contains_key("b.js"));
}

#[test]
fn n04_single_and_multi_agree() {
    let f = Fixture::new(B);
    f.control();
    let (single, state) = f.single(Some(&request(64)), f.dir.path());
    let multi = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    assert!(!context(&state).context_files.is_empty());
    assert!(!context(&multi.context).context_files.is_empty());
    same_result(&single.result, &multi.results[0]);
    assert_eq!(
        single.diagnostics.as_ref().map(|d| d.messages()),
        multi.diagnostics.as_ref().map(|d| d.messages())
    );
    // Single facade keeps its existing unfiltered behavior even with Exact requested in inputs.
    let mut exact = Fixture::new(B);
    exact.inputs.min_confidence = MinConfidence::Exact;
    exact.with(|ctx| {
        let old = run_algorithm_with_diagnostics(
            SlicingAlgorithm::Taint,
            ctx,
            &exact.inputs,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            exact.dir.path(),
        )
        .unwrap();
        let (new, _) = run_algorithm_prepared(
            SlicingAlgorithm::Taint,
            ctx,
            &exact.inputs,
            &SliceConfig::default(),
            &AlgorithmParams::default(),
            exact.dir.path(),
            None,
        )
        .unwrap();
        same_result(&old.result, &new.result);
    });
}

#[test]
fn n07_duplicate_taint_selections_share_one_expansion() {
    let f = Fixture::new(B);
    f.control();
    let single = f.review(&[SlicingAlgorithm::Taint], Some(&request(64)));
    let duplicate = f.review(
        &[SlicingAlgorithm::Taint, SlicingAlgorithm::Taint],
        Some(&request(64)),
    );
    assert!(!context(&single.context).context_files.is_empty());
    assert_eq!(duplicate.results.len(), 2);
    same_result(&single.results[0], &duplicate.results[0]);
    same_result(&duplicate.results[0], &duplicate.results[1]);
    assert_eq!(duplicate.findings.len(), single.findings.len() * 2);
    assert_eq!(
        format!("{:?}", context(&single.context).attempts),
        format!("{:?}", context(&duplicate.context).attempts)
    );
    assert_eq!(
        context(&single.context).stats.rounds_completed,
        context(&duplicate.context).stats.rounds_completed
    );
    // CONTROLLED coordinator sequencing, real engine: changing disk between calls reveals reruns.
    f.with(|ctx| {
        let config = TaintConfig::default();
        let (baseline, trace) = taint::slice_with_trace(ctx, &f.inputs.diff, &config).unwrap();
        let request = request(64);
        let mut expansion = RunExpansion::new(Some(&request));
        let first = expansion.apply(&f.inputs, ctx, &config, baseline.clone(), &trace);
        let first_sources = context(&expansion.state).sources.clone();
        assert_eq!(first_sources["b.js"], B);
        std::fs::write(
            f.dir.path().join("b.js"),
            "export function g(x) { eval(x); eval(x); }\n",
        )
        .unwrap();
        let second = expansion.apply(&f.inputs, ctx, &config, baseline, &trace);
        same_result(&first, &second);
        assert_eq!(context(&expansion.state).sources, first_sources);
    });
}
