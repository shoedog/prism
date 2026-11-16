//! CONTROLLED carriers over REAL parsing. No runtime expansion/recall claim.
use super::*;
use crate::api::context_run::UnionState;
use prism::resolution::ResolutionConfidence;
use prism::slice::SlicingAlgorithm;
#[path = "context_view_tests/fixture.rs"]
mod fixture;
use fixture::*;

#[test]
fn v01_absent_context_borrows_original_maps() {
    let f = Fixture::new();
    original(view_for_result(None, &result(), &f.inputs), &f.inputs);
    original(
        view_for_finding(None, &finding("taint"), &f.inputs),
        &f.inputs,
    );
}

#[test]
fn v02_applied_context_moves_outcome_and_borrows_maps() {
    let f = Fixture::new();
    let expected = snapshot(&result());
    let run = single(result(), ContextState::Expanded(f.context()));
    assert_eq!(snapshot(&run.outcome.result), expected);
    assert_eq!(
        run.outcome.diagnostics,
        Some(crate::api::ReviewBoundaryDiagnostics::default())
    );
    let context = run.context().expect("Applied nonempty single carrier");
    let view = run.view(&f.inputs);
    assert!(std::ptr::eq(view.maps().files, &context.files));
    assert!(std::ptr::eq(view.sources(), &context.sources));
    assert!(std::ptr::eq(
        view.maps().parse_quality,
        &context.parse_quality
    ));
    assert!(std::ptr::eq(
        view.context_files.unwrap(),
        &context.context_files
    ));
    assert_eq!(view.sources()["context.py"], context.sources["context.py"]);
    let run = multi(ContextState::Expanded(f.context()));
    assert!(run.context().is_some(), "Applied nonempty multi carrier");
}

#[test]
fn v03_original_entries_and_owners_unchanged() {
    let f = Fixture::new();
    let before_sources = f.inputs.sources.clone();
    let before_quality = serde_json::to_vec(&f.inputs.parse_quality).unwrap();
    let context = f.context();
    for key in f.inputs.files.keys() {
        assert_eq!(context.sources[key], f.inputs.sources[key]);
        assert_eq!(
            serde_json::to_vec(&context.parse_quality.get(key)).unwrap(),
            serde_json::to_vec(&f.inputs.parse_quality.get(key)).unwrap()
        );
        assert_eq!(
            context.files[key].error_rate(),
            f.inputs.files[key].error_rate()
        );
        assert_eq!(
            context.files[key].tree.root_node().to_sexp(),
            f.inputs.files[key].tree.root_node().to_sexp()
        );
    }
    let _ = view_for_result(Some(&context), &result(), &f.inputs).context_parse_quality();
    assert_eq!(before_sources, f.inputs.sources);
    assert_eq!(
        before_quality,
        serde_json::to_vec(&f.inputs.parse_quality).unwrap()
    );
}

#[test]
fn v04_non_taint_uses_original_maps() {
    let f = Fixture::new();
    let context = f.context();
    let mut r = result();
    for algorithm in SlicingAlgorithm::all()
        .into_iter()
        .filter(|a| *a != SlicingAlgorithm::Taint)
    {
        r.algorithm = algorithm;
        original(view_for_result(Some(&context), &r, &f.inputs), &f.inputs);
    }
    for name in ["echo", "full", "Taint", "taint "] {
        original(
            view_for_finding(Some(&context), &finding(name), &f.inputs),
            &f.inputs,
        );
    }
}

#[test]
fn v05_finding_algorithm_exactly_taint_not_category() {
    let f = Fixture::new();
    let context = f.context();
    let mut finding = finding("taint");
    finding.category = None;
    assert!(std::ptr::eq(
        view_for_finding(Some(&context), &finding, &f.inputs)
            .maps()
            .files,
        &context.files
    ));
    finding.algorithm = "echo".into();
    finding.category = Some("taint_sink".into());
    original(
        view_for_finding(Some(&context), &finding, &f.inputs),
        &f.inputs,
    );
}

#[test]
fn v06_sparse_context_quality_is_owned_and_excludes_originals() {
    let f = Fixture::new();
    let context = f.context();
    let view = view_for_result(Some(&context), &result(), &f.inputs);
    let mut projected = view.context_parse_quality();
    assert_eq!(
        projected.keys().cloned().collect::<Vec<_>>(),
        vec!["dirty_context.py"]
    );
    assert_eq!(
        serde_json::to_vec(&projected["dirty_context.py"]).unwrap(),
        serde_json::to_vec(&context.parse_quality["dirty_context.py"]).unwrap()
    );
    projected.clear();
    assert!(context.parse_quality.contains_key("dirty_context.py"));
    assert!(f.inputs.parse_quality.contains_key("dirty.py"));
    assert!(!f.inputs.parse_quality.contains_key("dirty_context.py"));
}

#[test]
fn v07_ineligible_states_rechecked_even_when_directly_supplied() {
    let f = Fixture::new();
    for case in 0..3 {
        let mut context = f.context();
        match case {
            0 => context.context_files.clear(),
            1 => refused(&mut context),
            _ => context.union = UnionState::NotNeeded,
        }
        original(
            view_for_result(Some(&context), &result(), &f.inputs),
            &f.inputs,
        );
        original(
            view_for_finding(Some(&context), &finding("taint"), &f.inputs),
            &f.inputs,
        );
        let run = single(result(), ContextState::Expanded(context));
        assert!(run.context().is_none());
        if let ContextState::Expanded(context) = &run.context {
            assert_eq!(context.published.files, 2);
            assert_eq!(context.round, 2);
        } else {
            panic!("constructor lost expanded state");
        }
        let mut context = f.context();
        match case {
            0 => context.context_files.clear(),
            1 => refused(&mut context),
            _ => context.union = UnionState::NotNeeded,
        }
        assert!(multi(ContextState::Expanded(context)).context().is_none());
    }
}

#[test]
fn v08_unavailable_and_notrequested_expose_no_maps() {
    let f = Fixture::new();
    for state in [
        ContextState::NotRequested,
        ContextState::Unavailable(crate::api::ContextUnavailable::RootChangedDuringCapture),
    ] {
        let run = single(result(), state);
        assert!(run.context().is_none());
        original(run.view(&f.inputs), &f.inputs);
    }
    assert!(multi(ContextState::NotRequested).context().is_none());
    assert!(multi(ContextState::Unavailable(
        crate::api::ContextUnavailable::RootChangedDuringCapture
    ))
    .context()
    .is_none());
}

#[test]
fn v09_selected_maps_change_scoped_tier_not_exact_admission() {
    use prism::finding_confidence::{
        admit_finding_for_resolution, classify_with_evidence, parse_quality_for_selected_evidence,
        FindingConfidence, FindingTier, ParseQuality,
    };
    let f = Fixture::new();
    let mut run = single(result(), ContextState::Expanded(f.context()));
    let evidence = run.outcome.result.evidence[0].as_ref().unwrap();
    let finding = &run.outcome.result.findings[0];
    let original = MapView::original(&f.inputs);
    let selected = run.view(&f.inputs).maps();
    let old_quality = parse_quality_for_selected_evidence(
        finding,
        Some(evidence),
        original.parse_quality,
        original.files,
    );
    let new_quality = parse_quality_for_selected_evidence(
        finding,
        Some(evidence),
        selected.parse_quality,
        selected.files,
    );
    assert_eq!(old_quality, ParseQuality::Unknown);
    assert_eq!(new_quality, ParseQuality::Clean);
    assert_eq!(
        classify_with_evidence("taint", old_quality, evidence),
        (FindingConfidence::Exact, FindingTier::Candidate)
    );
    assert_eq!(
        classify_with_evidence("taint", new_quality, evidence),
        (FindingConfidence::Exact, FindingTier::Asserted)
    );
    assert_eq!(
        admit_finding_for_resolution(
            "taint",
            new_quality,
            Some(evidence),
            MinConfidence::Exact,
            ResolutionMode::Nominal
        ),
        Some((FindingConfidence::Unlabeled, FindingTier::Candidate))
    );
    let before = snapshot(&run.outcome.result);
    let diagnostics = run.outcome.diagnostics.clone();
    let mut old = run.outcome.result.clone();
    crate::api::filter_result_findings(
        &mut old,
        &f.inputs,
        MinConfidence::Exact,
        ResolutionMode::Scoped,
    );
    run.filter_findings(&f.inputs, MinConfidence::Exact, ResolutionMode::Scoped);
    assert_eq!(snapshot(&run.outcome.result), before);
    assert_eq!(snapshot(&old), before);
    assert_eq!(run.outcome.diagnostics, diagnostics);
}

#[test]
fn v10_filter_parity_default_noop_and_negative_floors() {
    let f = Fixture::new();
    for evidence in [Some(witness(ResolutionConfidence::NameOnly)), None] {
        let mut r = result();
        r.evidence = vec![evidence];
        for mode in [ResolutionMode::Scoped, ResolutionMode::Nominal] {
            let mut old = r.clone();
            crate::api::filter_result_findings(&mut old, &f.inputs, MinConfidence::Exact, mode);
            for state in [
                ContextState::NotRequested,
                ContextState::Expanded(f.context()),
            ] {
                let mut run = single(r.clone(), state);
                run.filter_findings(&f.inputs, MinConfidence::Exact, mode);
                assert!(run.outcome.result.findings.is_empty());
                assert_eq!(snapshot(&old), snapshot(&run.outcome.result));
            }
        }
    }
    let mut malformed = result();
    malformed.evidence.clear();
    let before = snapshot(&malformed);
    let mut run = single(malformed, ContextState::Expanded(f.context()));
    run.filter_findings(&f.inputs, MinConfidence::NameOnly, ResolutionMode::Scoped);
    assert_eq!(snapshot(&run.outcome.result), before);
    let mut r = result();
    r.algorithm = SlicingAlgorithm::OriginalDiff;
    let mut old = r.clone();
    crate::api::filter_result_findings(
        &mut old,
        &f.inputs,
        MinConfidence::Exact,
        ResolutionMode::Scoped,
    );
    let mut run = single(r, ContextState::Expanded(f.context()));
    original(run.view(&f.inputs), &f.inputs);
    run.filter_findings(&f.inputs, MinConfidence::Exact, ResolutionMode::Scoped);
    assert_eq!(snapshot(&old), snapshot(&run.outcome.result));
}

#[test]
fn v11_duplicate_pairs_keep_order_and_multiplicity() {
    let f = Fixture::new();
    for confidence in [ResolutionConfidence::Exact, ResolutionConfidence::NameOnly] {
        let mut r = result();
        r.findings.push(r.findings[0].clone());
        r.evidence = vec![Some(witness(confidence)); 2];
        let before = snapshot(&r);
        let mut run = single(r, ContextState::Expanded(f.context()));
        run.filter_findings(&f.inputs, MinConfidence::Exact, ResolutionMode::Scoped);
        if confidence == ResolutionConfidence::Exact {
            assert_eq!(run.outcome.result.findings.len(), 2);
            assert_eq!(snapshot(&run.outcome.result), before);
        } else {
            assert!(run.outcome.result.findings.is_empty());
            assert!(run.outcome.result.evidence.is_empty());
        }
    }
}
