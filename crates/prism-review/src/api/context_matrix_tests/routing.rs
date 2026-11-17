//! G01–G09 and G21–G23: REAL captured reviews unless a subrow says CONTROLLED.
use super::*;
use crate::algorithms::taint::boundary::records::ContextRunNoteKind;
use crate::api::context_route::{run_algorithm_authorized, run_review_authorized, ReviewAuthority};
use prism::slice::SlicingAlgorithm;

mod bounds;
mod cache;

fn taint() -> [SlicingAlgorithm; 1] {
    [SlicingAlgorithm::Taint]
}
fn doc(f: &Fixture, s: &Sample) -> Documents {
    Documents::multi(s, &f.inputs, RenderSettings::default())
}
fn single_doc(f: &Fixture, s: &SingleSample) -> Documents {
    Documents::single(s, &f.inputs, RenderSettings::default())
}
fn context(sample: &Sample) -> &super::super::context_run::ReviewContext {
    match &sample.run.context {
        super::super::context_run::ContextState::Expanded(c) => c,
        _ => panic!("authorized request must retain expanded state"),
    }
}
fn note_messages(run: &ReviewRun, kind: ContextRunNoteKind) -> Vec<String> {
    run.diagnostics
        .as_ref()
        .map(|d| {
            d.run_notes()
                .iter()
                .filter(|n| n.kind() == kind)
                .map(|n| n.message().to_owned())
                .collect()
        })
        .unwrap_or_default()
}
fn assert_note_once(run: &ReviewRun, d: &Documents, kind: ContextRunNoteKind) -> String {
    let notes = note_messages(run, kind);
    assert_eq!(notes.len(), 1, "one typed {kind:?} note");
    let text = &notes[0];
    assert_eq!(run.warnings.iter().filter(|s| *s == text).count(), 1);
    assert_eq!(
        d.json.value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v.as_str() == Some(text))
            .count(),
        1
    );
    assert_eq!(
        notices(&d.sarif.value)
            .iter()
            .filter(|v| v["message"]["text"] == text.as_str())
            .count(),
        1
    );
    assert_eq!(
        d.targets.value["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v.as_str() == Some(text))
            .count(),
        1
    );
    text.clone()
}
fn native_single(f: &Fixture) -> SingleSample {
    f.single(&imports())
}

#[test]
fn g01_single_authorized() {
    let f = Fixture::js(B);
    let on = native_single(&f);
    let c = on.run.context().expect("eligible context");
    assert!(c.context_files.contains("b.js"));
    assert!(on
        .run
        .outcome
        .result
        .findings
        .iter()
        .any(|x| x.file == "b.js" && x.category.as_deref() == Some("taint_sink")));
    let d = single_doc(&f, &on);
    assert!(d.text.contains(B.trim()));
    assert!(on.run.stats().is_some());
    assert!(!f.inputs.files.contains_key("b.js"));
}
#[test]
fn g02_multi_authorized() {
    let f = Fixture::js(B);
    let on = f.on(&taint());
    assert_expanded(&on, "b.js");
    let d = doc(&f, &on);
    assert!(d.text.contains(B.trim()));
    assert!(on.run.stats().is_some());
    let single = native_single(&f);
    same_result(&on.run.results[0], &single.run.outcome.result);
}
#[test]
fn g03_high_level_review() {
    let f = Fixture::js(B);
    let diff = r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#;
    let out = crate::api::review(
        &f.options,
        diff,
        &taint(),
        &SliceConfig::default(),
        &imports(),
    )
    .unwrap();
    assert!(out.run.context().is_some());
    assert!(!out.inputs.files.contains_key("b.js"));
    assert!(out
        .run
        .findings
        .iter()
        .any(|x| x.file == "b.js" && x.category.as_deref() == Some("taint_sink")));
    let on = f.on(&taint());
    same_result(&out.run.results[0], &on.run.results[0]);
}
#[test]
fn g04_legacy_single_is_baseline_and_warning_once() {
    let f = Fixture::js(B);
    let built = build_context(&f.inputs, &f.options).unwrap();
    let p = imports();
    let (base, _, _) = super::super::run::run_algorithm_prepared(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &f.inputs,
        &SliceConfig::default(),
        &p,
        f.dir.path(),
        None,
    )
    .unwrap();
    let legacy = crate::api::run_algorithm_with_diagnostics(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &f.inputs,
        &SliceConfig::default(),
        &p,
        f.dir.path(),
    )
    .unwrap();
    same_result(&base.result, &legacy.result);
    let notes = legacy.diagnostics.as_ref().unwrap().run_notes();
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].kind(), ContextRunNoteKind::Unauthorized);
    let msg = notes[0].message().to_owned();
    assert!(msg.contains("run_algorithm_authorized"));
    let result = legacy.into_legacy_result();
    assert_eq!(result.warnings.iter().filter(|w| *w == &msg).count(), 1);
}
#[test]
fn g05_legacy_multi_is_baseline_and_unauthorized_once() {
    let f = Fixture::js(B);
    let built = build_context(&f.inputs, &f.options).unwrap();
    let p = imports();
    let baseline = f.off(&taint());
    let run = crate::api::run_review(
        &built.ctx,
        &f.inputs,
        &taint(),
        &SliceConfig::default(),
        &p,
        f.dir.path(),
    );
    same_result(&baseline.run.results[0], &run.results[0]);
    assert!(run.context().is_none());
    let notes = note_messages(&run, ContextRunNoteKind::Unauthorized);
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("run_review_authorized"));
    assert_eq!(run.warnings.iter().filter(|s| *s == &notes[0]).count(), 1);
    let sample = Sample {
        run,
        build_warnings: built.warnings.clone(),
        repo_root: f.dir.path().into(),
    };
    assert_eq!(
        assert_note_once(
            &sample.run,
            &doc(&f, &sample),
            ContextRunNoteKind::Unauthorized
        ),
        notes[0]
    );
}
#[test]
fn g06_targets_from_high_level_review() {
    let f = Fixture::js(B);
    let diff = r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#;
    let out = crate::api::review(
        &f.options,
        diff,
        &taint(),
        &SliceConfig::default(),
        &imports(),
    )
    .unwrap();
    let on = Sample {
        run: out.run,
        build_warnings: out.build_warnings,
        repo_root: f.dir.path().into(),
    };
    assert_expanded(&on, "b.js");
    let d = Documents::multi(&on, &out.inputs, RenderSettings::default());
    assert!(d.targets.value["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["site"]["file"] == "b.js" && t["category"] == "taint_sink"));
    assert_eq!(
        d.targets.value["diff"]["sha256"],
        out.inputs.diff_text_sha256
    );
}
#[test]
fn g07_invalid_result_and_multi_errors_preserve_others() {
    let f = Fixture::js(B);
    let built = build_context(&f.inputs, &f.options).unwrap();
    for (depth, max) in [(0, 64), (5, 64), (2, 0)] {
        for mode in [TaintContextMode::Off, TaintContextMode::Imports] {
            let mut p = imports();
            p.taint_context.mode = mode;
            p.taint_context.depth = depth;
            p.taint_context.max_files = max;
            assert!(run_algorithm_authorized(
                SlicingAlgorithm::OriginalDiff,
                &built.ctx,
                ReviewAuthority::capture(&f.inputs),
                &SliceConfig::default(),
                &p,
                f.dir.path()
            )
            .is_err());
            assert!(crate::api::review(
                &f.options,
                r#"{"files":[]}"#,
                &[],
                &SliceConfig::default(),
                &p
            )
            .is_err());
            let alg = [
                SlicingAlgorithm::Taint,
                SlicingAlgorithm::OriginalDiff,
                SlicingAlgorithm::Taint,
            ];
            let run = run_review_authorized(
                &built.ctx,
                ReviewAuthority::capture(&f.inputs),
                &alg,
                &SliceConfig::default(),
                &p,
                f.dir.path(),
            );
            assert_eq!(run.errors.len(), 2);
            assert_eq!(run.results.len(), 1);
            assert_eq!(run.results[0].algorithm, SlicingAlgorithm::OriginalDiff);
            assert_eq!(
                run.algorithms_run,
                alg.iter().map(|a| a.name().to_string()).collect::<Vec<_>>()
            );
            let base = f.off(&[SlicingAlgorithm::OriginalDiff]);
            same_result(&run.results[0], &base.run.results[0]);
            let sample = Sample {
                run,
                build_warnings: built.warnings.clone(),
                repo_root: f.dir.path().into(),
            };
            let documents = doc(&f, &sample);
            assert_eq!(documents.json.value["errors"].as_array().unwrap().len(), 2);
        }
    }
}
#[test]
fn g08_imports_no_taint_has_one_ignored_note_everywhere() {
    let f = Fixture::js(B);
    let alg = [SlicingAlgorithm::OriginalDiff];
    let off = f.off(&alg);
    let on = f.on(&alg);
    same_result(&off.run.results[0], &on.run.results[0]);
    let d = doc(&f, &on);
    let msg = assert_note_once(&on.run, &d, ContextRunNoteKind::Ignored);
    assert_eq!(msg, "taint-context ignored: taint not selected");
    assert!(on.run.context().is_none());
    assert!(on.run.stats().is_none());
}
#[test]
fn g09_off_and_invalid_off_without_taint_are_note_free() {
    let f = Fixture::js(B);
    let alg = [SlicingAlgorithm::OriginalDiff];
    let off = f.off(&alg);
    for depth in [4, 0] {
        let mut p = AlgorithmParams::default();
        p.taint_context.depth = depth;
        p.taint_context.max_files = 1;
        let run = f.run(&alg, &p);
        same_result(&off.run.results[0], &run.run.results[0]);
        assert!(note_messages(&run.run, ContextRunNoteKind::Ignored).is_empty());
        assert!(note_messages(&run.run, ContextRunNoteKind::Unauthorized).is_empty());
        assert!(run.run.stats().is_none());
    }
}
#[test]
fn g21_mixed_suite_preserves_non_taint_results_and_order() {
    let f = Fixture::js(B);
    let alg = [
        SlicingAlgorithm::OriginalDiff,
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::EchoSlice,
    ];
    let off = f.off(&alg);
    let on = f.on(&alg);
    assert_expanded(&on, "b.js");
    assert_eq!(off.run.algorithms_run, on.run.algorithms_run);
    assert_eq!(bytes(&off.run.errors), bytes(&on.run.errors));
    for (a, b) in off.run.results.iter().zip(&on.run.results) {
        if a.algorithm != SlicingAlgorithm::Taint {
            same_result(a, b);
        }
    }
    assert!(doc(&f, &on).text.contains(B.trim()));
}
#[test]
fn g22_context_only_roots_do_not_seed_findings() {
    let main = "import { f } from './a';\nexport function main(x) { f(x); }\n";
    let source="const express = require('express');\nconst app = express();\napp.get('/', (req,res) => { const q = req.query.q; eval(q); });\nexport function f(x) {}\n";
    let f = Fixture::new(&[("main.js", main), ("a.js", source)], "main.js", &[2]);
    let native = f.native();
    assert!(native.bundle.context_files.contains("a.js"));
    let built = build_context(&f.inputs, &f.options).unwrap();
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let round = prism::api::ReviewContextRound::new(&loader, &native.bundle.files);
    let full = round.build_context(f.inputs.type_db.as_ref(), &built.ctx.types);
    let (_, unrestricted) =
        taint::slice_with_trace(&full, &f.inputs.diff, &taint::TaintConfig::default()).unwrap();
    assert!(
        unrestricted.roots.iter().any(
            |root| root.seed.file == "a.js" && root.origin == taint::TaintRootOrigin::Framework
        ),
        "native context framework root is present"
    );
    assert!(native
        .bundle
        .analysis
        .as_ref()
        .unwrap()
        .trace
        .roots
        .iter()
        .all(|root| root.seed.file != "a.js"));
    let on = f.on(&taint());
    let c = context(&on);
    assert!(c.context_files.contains("a.js"));
    assert!(on.run.results[0].findings.iter().all(|v| v.file != "a.js"));
    let d = doc(&f, &on);
    assert!(!d.targets.value["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["site"]["file"] == "a.js"));
}
#[test]
fn g23_explicit_source_suppresses_diff_roots() {
    let main="import { f } from './a';\nimport { g } from './b';\nexport function main(x,y) {\n f(x);\n g(y);\n}\n";
    let f = Fixture::new(
        &[
            ("main.js", main),
            ("a.js", "export function f(x) { eval(x); }\n"),
            ("b.js", "export const other = 1;\n"),
        ],
        "main.js",
        &[5],
    );
    let mut p = imports();
    p.taint_sources = vec!["main.js:4".into(), "a.js:1".into()];
    let built = build_context(&f.inputs, &f.options).unwrap();
    let config = taint::TaintConfig {
        sources: vec![("main.js".into(), 4), ("a.js".into(), 1)],
        taint_from_diff: false,
        ..Default::default()
    };
    let (_, trace) = taint::slice_with_trace(&built.ctx, &f.inputs.diff, &config).unwrap();
    assert!(
        trace
            .roots
            .iter()
            .any(|root| root.seed.file == "main.js"
                && root.origin == taint::TaintRootOrigin::Explicit)
    );
    assert!(trace
        .roots
        .iter()
        .all(|root| root.origin != taint::TaintRootOrigin::Diff));
    let on = f.run(&taint(), &p);
    let c = context(&on);
    assert!(c.context_files.contains("a.js"));
    assert!(!c.context_files.contains("b.js"));
    assert!(on.run.stats().is_some());
    assert!(!doc(&f, &on).targets.value["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["site"]["file"] == "b.js"));
}
