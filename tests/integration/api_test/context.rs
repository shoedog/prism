//! External-crate public API contract (A01-A10). No crate-private carrier access.
use prism::finding_confidence::{MinConfidence, ResolutionMode};
use prism::slice::{SliceConfig, SlicingAlgorithm};
use prism_review::{api::*, output};
use serde_json::Value;
use std::{fs, path::Path};
use tempfile::TempDir;
#[path = "context/adapters.rs"]
mod adapters;
#[path = "context/fixtures.rs"]
mod fixtures;
use fixtures::*;

#[test]
fn a01_options_default_is_off_with_documented_defaults() {
    let options = TaintContextOptions::default();
    assert_eq!(options.mode, TaintContextMode::Off);
    assert_eq!(options.edges, TaintContextEdges::NameOnly);
    assert_eq!(options.depth, DEFAULT_TAINT_CONTEXT_DEPTH);
    assert_eq!(options.max_files, DEFAULT_TAINT_CONTEXT_MAX_FILES);
    assert_eq!((options.depth, options.max_files), (2, 64));
    let f = Fixture::new(B);
    for depth in [1, 4] {
        let mut params = AlgorithmParams::default();
        params.taint_context.depth = depth;
        params.taint_context.max_files = 1;
        assert!(review(&f.options, &f.diff, &[], &SliceConfig::default(), &params).is_ok());
    }
}

#[test]
fn a02_review_expands_through_the_public_api() {
    use prism::finding_confidence::{parse_quality_for_selected_evidence, ParseQuality};

    let f = Fixture::new(B);
    let result = f.review(&imports());
    let taint = &result.run.results[0];
    let index = sink(taint, "b.js");
    let finding = &taint.findings[index];
    let evidence = taint.evidence[index]
        .as_ref()
        .expect("behavior: selected b.js sink has evidence");
    let context: &ReviewContext = result
        .run
        .context()
        .expect("behavior: public context present");
    assert_eq!(context.sources()["b.js"], B);
    assert!(context.files().contains_key("b.js"));
    assert!(!context.parse_quality().contains_key(&finding.file));
    assert_eq!(
        parse_quality_for_selected_evidence(
            finding,
            Some(evidence),
            context.parse_quality(),
            context.files(),
        ),
        ParseQuality::Clean
    );
    assert_eq!(
        context
            .context_files()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a.js", "b.js"]
    );
    assert!(!result.inputs.files.contains_key("b.js"));
    assert!(!result.inputs.sources.contains_key("b.js"));
    assert!(!result.inputs.parse_quality.contains_key("b.js"));
}

#[test]
fn a03_authorized_preloaded_entries() {
    let f = Fixture::new(B);
    let inputs = f.inputs();
    let built = build_context(&inputs, &f.options).unwrap();
    let high = f.review(&imports());
    let multi = run_review_authorized(
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &[SlicingAlgorithm::Taint],
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
    );
    pair(&multi.results[0], &high.run.results[0]);
    sink(&multi.results[0], "b.js");
    let single: ReviewAlgorithmRun = run_algorithm_authorized(
        SlicingAlgorithm::Taint,
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
    )
    .unwrap();
    assert_eq!(single.view(&inputs).sources()["b.js"], B);
    assert!(single.context().is_some());
    let (outcome, context) = single.into_parts();
    pair(&outcome.result, &high.run.results[0]);
    assert!(context.is_some());
    assert_eq!(inputs.files.len(), 1);
}

#[test]
fn a04_legacy_preloaded_returns_baseline_and_names_the_entry() {
    let f = Fixture::new(B);
    let inputs = f.inputs();
    let built = build_context(&inputs, &f.options).unwrap();
    let run = |params: &AlgorithmParams| {
        run_review(
            &built.ctx,
            &inputs,
            &[SlicingAlgorithm::Taint],
            &SliceConfig::default(),
            params,
            f.dir.path(),
        )
    };
    let off = run(&AlgorithmParams::default());
    assert!(
        !off.findings.is_empty(),
        "behavior: nonempty baseline control"
    );
    let mut requested = run(&imports());
    pair(&off.results[0], &requested.results[0]);
    assert!(requested.context().is_none());
    assert!(requested.take_context().is_none());
    assert!(requested.take_context().is_none());
    let note = &notes(&requested)[0];
    assert_eq!(notes(&requested).len(), 1);
    assert_eq!(note.kind(), ContextRunNoteKind::Unauthorized);
    for name in ["run_review_authorized", "run_algorithm_authorized"] {
        assert!(note.message().contains(name));
    }
    assert_eq!(
        requested
            .warnings
            .iter()
            .filter(|w| *w == note.message())
            .count(),
        1
    );
    let single = run_algorithm(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &inputs,
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
    )
    .unwrap();
    assert!(!single.findings.iter().any(|f| f.file == "b.js"));
    assert!(single
        .warnings
        .iter()
        .any(|w| w.contains("run_algorithm_authorized")));
    assert!(notes(&off).is_empty());
}

#[test]
fn a05_invalid_options_err_and_error_shapes() {
    let f = Fixture::new(B);
    let inputs = f.inputs();
    let built = build_context(&inputs, &f.options).unwrap();
    let mut invalid = AlgorithmParams::default();
    invalid.taint_context.depth = 0;
    let valid_json_error = review(
        &f.options,
        "{",
        &[],
        &SliceConfig::default(),
        &AlgorithmParams::default(),
    )
    .err()
    .unwrap();
    assert!(!valid_json_error
        .to_string()
        .contains("invalid taint-context options"));
    let invalid_first = review(&f.options, "{", &[], &SliceConfig::default(), &invalid)
        .err()
        .unwrap();
    assert!(
        invalid_first
            .to_string()
            .contains("invalid taint-context options"),
        "error precedence, not absolute no-I/O proof"
    );
    for (depth, max_files) in [(0, 64), (5, 64), (2, 0)] {
        let mut p = AlgorithmParams::default();
        p.taint_context.depth = depth;
        p.taint_context.max_files = max_files;
        assert!(review(
            &f.options,
            &f.diff,
            &[SlicingAlgorithm::AbsenceSlice],
            &SliceConfig::default(),
            &p
        )
        .is_err());
        assert!(run_algorithm_authorized(
            SlicingAlgorithm::AbsenceSlice,
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &SliceConfig::default(),
            &p,
            f.dir.path()
        )
        .is_err());
        let run = run_review(
            &built.ctx,
            &inputs,
            &[
                SlicingAlgorithm::Taint,
                SlicingAlgorithm::Taint,
                SlicingAlgorithm::AbsenceSlice,
            ],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        assert_eq!(run.errors.len(), 2);
        assert!(run.errors.iter().all(|error| {
            error.algorithm == SlicingAlgorithm::Taint.name()
                && error.error.starts_with("invalid taint-context options:")
        }));
        assert_eq!(run.results.len(), 1);
        assert_eq!(run.results[0].algorithm, SlicingAlgorithm::AbsenceSlice);
        let off = run_review(
            &built.ctx,
            &inputs,
            &[SlicingAlgorithm::AbsenceSlice],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        assert!(notes(&off).is_empty());
        p.taint_context.mode = TaintContextMode::Imports;
        let ignored = run_review_authorized(
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &[SlicingAlgorithm::AbsenceSlice],
            &SliceConfig::default(),
            &p,
            f.dir.path(),
        );
        assert_eq!(notes(&ignored).len(), 1);
        assert_eq!(notes(&ignored)[0].kind(), ContextRunNoteKind::Ignored);
    }
}

#[test]
fn a06_no_taint_selected_is_ignored() {
    let f = Fixture::new(B);
    let inputs = f.inputs();
    let built = build_context(&inputs, &f.options).unwrap();
    let algorithms = [
        SlicingAlgorithm::AbsenceSlice,
        SlicingAlgorithm::AbsenceSlice,
    ];
    let run = run_review_authorized(
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &algorithms,
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
    );
    let off = run_review(
        &built.ctx,
        &inputs,
        &algorithms,
        &SliceConfig::default(),
        &AlgorithmParams::default(),
        f.dir.path(),
    );
    assert_eq!(run.results.len(), 2);
    for (a, b) in run.results.iter().zip(&off.results) {
        pair(a, b);
    }
    let note: &ContextRunNote = &notes(&run)[0];
    assert_eq!(notes(&run).len(), 1);
    assert_eq!(note.kind(), ContextRunNoteKind::Ignored);
    assert_eq!(note.message(), "taint-context ignored: taint not selected");
    assert_eq!(note.location(), None);
    assert_eq!(
        run.warnings.iter().filter(|w| *w == note.message()).count(),
        1
    );
    assert!(run.context().is_none());
    let single = run_algorithm_authorized(
        SlicingAlgorithm::AbsenceSlice,
        &built.ctx,
        ReviewAuthority::capture(&inputs),
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
    )
    .unwrap();
    assert!(single.context().is_none());
    let (outcome, context) = single.into_parts();
    assert!(context.is_none());
    assert_eq!(outcome.diagnostics.as_ref().unwrap().run_notes().len(), 1);
}

#[test]
fn a10_non_exhaustive_gate_still_counts() {
    super::api_types_are_non_exhaustive();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let paths = ["src/api", "crates/prism-review/src/api"];
    let count = paths
        .iter()
        .flat_map(|d| fs::read_dir(root.join(d)).unwrap())
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .map(|p| {
            fs::read_to_string(p)
                .unwrap()
                .lines()
                .filter(|l| {
                    l.trim_start().starts_with("pub struct ")
                        || l.trim_start().starts_with("pub enum ")
                })
                .count()
        })
        .sum::<usize>();
    // Measured base API-only count is 25; seven formerly crate-private declarations become public.
    assert!(
        count >= 32,
        "new public carrier declarations must join the nearest-attribute gate"
    );
}
