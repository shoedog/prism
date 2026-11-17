//! G10–G17: real loader/driver except the explicitly controlled private limit/guard.
use super::*;
use crate::algorithms::taint::{
    boundary::collect,
    context_budget::{ContextLimits, RunGuard, StopCause},
    rounds::{expand, ExpansionRequest, Stop},
};
use crate::api::{
    context_run::{ContextRequest, ContextState, ReviewContext, UnionState},
    context_view::ReviewAlgorithmRun,
    diagnostics::RunDiagnostics,
};
use prism::{
    api::{ContextReadOutcome, ContextSkip, ReviewContextLoader},
    cpg::CrossingCeiling,
};
use std::cell::Cell;

#[test]
fn g10_depth_one_retains_completed_prefix() {
    let f = Fixture::js(B);
    let mut p = imports();
    p.taint_context.depth = 1;
    let on = f.run(&taint(), &p);
    let c = context(&on);
    assert_eq!(c.round, 1);
    assert!(c.context_files.contains("a.js"));
    assert!(!c.context_files.contains("b.js"));
    let baseline = f.off(&taint());
    let rendered = doc(&f, &on);
    assert!(rendered.text.starts_with(&doc(&f, &baseline).text));
    let message = assert_note_once(&on.run, &rendered, ContextRunNoteKind::DepthBound);
    assert_eq!(
        message,
        "taint-context: depth bound 1 reached; 1 boundary calls not followed"
    );
}
#[test]
fn g11_files_bound_keeps_completed_prefix() {
    let f = Fixture::js(B);
    let mut p = imports();
    p.taint_context.max_files = 1;
    let on = f.run(&taint(), &p);
    let c = context(&on);
    assert_eq!(c.published.files, 1);
    assert!(c.context_files.contains("a.js"));
    assert!(!c.context_files.contains("b.js"));
    let msg = assert_note_once(&on.run, &doc(&f, &on), ContextRunNoteKind::FilesBound);
    assert_eq!(
        msg,
        "taint-context: max-files bound 1 reached; 1 context files not loaded"
    );
}
#[test]
fn g12_controlled_byte_limit_prepared_and_serialized() {
    let f = Fixture::js(B);
    let built = build_context(&f.inputs, &f.options).unwrap();
    let mut limits = ContextLimits::new(2, 64).unwrap();
    limits.source_bytes = A.len() as u64;
    let req = ContextRequest {
        limits,
        ceiling: CrossingCeiling::NameOnly,
    };
    let (outcome, state, stats) = super::super::super::run::run_algorithm_prepared(
        SlicingAlgorithm::Taint,
        &built.ctx,
        &f.inputs,
        &SliceConfig::default(),
        &imports(),
        f.dir.path(),
        Some(&req),
    )
    .unwrap();
    let run = ReviewAlgorithmRun::new(outcome, state).with_stats(stats);
    let c = run.context().expect("completed admitted prefix");
    assert_eq!(c.round, 1);
    assert_eq!(c.published.files, 1);
    assert_eq!(c.published.bytes, A.len() as u64);
    assert!(c.context_files.contains("a.js"));
    assert!(!c.context_files.contains("b.js"));
    assert!(c.attempts.attempted_files >= 2);
    assert!(run.stats().is_some());
    let messages = run.outcome.diagnostics.as_ref().unwrap().run_notes();
    let note = messages
        .iter()
        .find(|n| n.kind() == ContextRunNoteKind::BytesBound)
        .expect("actual bytes bound note")
        .message()
        .to_owned();
    assert_eq!(
        note,
        format!(
            "taint-context: context source bound {} bytes reached; 1 context files not loaded",
            A.len()
        )
    );
    let sample = SingleSample {
        run,
        build_warnings: built.warnings,
        repo_root: f.dir.path().into(),
    };
    let d = single_doc(&f, &sample);
    assert_eq!(
        notices(&d.sarif.value)
            .iter()
            .filter(|n| n["message"]["text"] == note)
            .count(),
        1
    );
}
struct StopAfterRead(Cell<usize>);
impl RunGuard for StopAfterRead {
    fn stop(&self) -> Option<StopCause> {
        let n = self.0.get();
        self.0.set(n + 1);
        if n == 0 {
            None
        } else {
            Some(StopCause::Time)
        }
    }
}
#[test]
fn g13_controlled_guard_after_one_read_keeps_round_zero() {
    let main = "import { f } from './a';\nexport function main(x) { f(x); }\n";
    let a = "export function f(x) { eval(x); }\n";
    let f = Fixture::new(&[("main.js", main), ("a.js", a)], "main.js", &[2]);
    let built = build_context(&f.inputs, &f.options).unwrap();
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    assert!(
        matches!(
            loader.read("a.js", 1024).outcome,
            ContextReadOutcome::Loaded(_)
        ),
        "inadmissible precondition: valid listed target"
    );
    let config = taint::TaintConfig::default();
    let (mut baseline, trace) =
        taint::slice_with_trace(&built.ctx, &f.inputs.diff, &config).unwrap();
    let limits = ContextLimits::new(2, 64).unwrap();
    let guard = StopAfterRead(Cell::new(0));
    let out = expand(ExpansionRequest {
        loader: &loader,
        inputs: &f.inputs,
        baseline_ctx: &built.ctx,
        baseline_trace: &trace,
        config: &config,
        limits,
        ceiling: CrossingCeiling::NameOnly,
        guard: &guard,
    });
    assert_eq!(out.bundle.frontier.load_order, ["a.js"]);
    assert_eq!(out.attempts.attempted_files, 1);
    assert!(out.attempts.attempted_read_bytes > 0);
    assert_eq!(out.stop, Stop::Guard(StopCause::Time));
    assert_eq!(
        (out.stats.rounds_started, out.stats.rounds_completed),
        (1, 0)
    );
    assert_eq!(out.bundle.round, 0);
    assert!(out.bundle.context_files.is_empty());
    assert_eq!(out.bundle.published.files, 0);
    assert_eq!(out.bundle.published.bytes, 0);
    assert!(!out.bundle.sources.contains_key("a.js"));
    assert_eq!(out.bundle.sources, f.inputs.sources);
    let b = out.bundle;
    let context = ReviewContext {
        files: b.files,
        sources: b.sources,
        parse_quality: b.parse_quality,
        parse_warnings: b.parse_warnings,
        context_files: b.context_files,
        round: b.round,
        published: b.published,
        stats: out.stats,
        stop: out.stop,
        attempts: out.attempts,
        frontier: b.frontier,
        union: UnionState::NotNeeded,
    };
    let state = ContextState::Expanded(context);
    let req = ContextRequest {
        limits,
        ceiling: CrossingCeiling::NameOnly,
    };
    let collection = collect::collect(&f.inputs, &built.ctx, &trace);
    let mut diag = RunDiagnostics::default();
    diag.capture_once(|| collection);
    diag.reconcile_context(&state, Some(&req), &f.inputs.parse_warnings);
    baseline.align_evidence();
    crate::algorithms::finalize_diagrams(&mut baseline, SliceConfig::default().diagram_node_cap);
    let mut outcome = crate::api::AlgorithmOutcome::new(baseline);
    outcome.diagnostics = diag.project();
    let run = ReviewAlgorithmRun::new(outcome, state);
    assert!(run.context().is_none());
    assert!(run.stats().is_none());
    assert!(run.outcome.result.findings.iter().all(|f| f.file != "a.js"));
    let messages = run.outcome.diagnostics.as_ref().unwrap().run_notes();
    let note = messages
        .iter()
        .find(|n| n.kind() == ContextRunNoteKind::TimeGuard)
        .unwrap()
        .message()
        .to_owned();
    assert_eq!(note,"taint-context: time guard (20s) stopped expansion; results are from the last completed round 0");
    let sample = SingleSample {
        run,
        build_warnings: built.warnings,
        repo_root: f.dir.path().into(),
    };
    let d = single_doc(&f, &sample);
    for (format, bytes) in d.bytes() {
        assert!(
            !String::from_utf8(bytes).unwrap().contains(a.trim()),
            "staged source in {format}"
        );
    }
    assert_eq!(
        notices(&d.sarif.value)
            .iter()
            .filter(|v| v["message"]["text"] == note)
            .count(),
        1
    );
}
#[test]
fn g14_actual_skipped_reads_keep_baseline() {
    for (body, expected) in [
        (vec![0xff], "not UTF-8"),
        (vec![b'x'; 2 * 1024 * 1024 + 1], "too large"),
    ] {
        let f = Fixture::js(B);
        std::fs::write(f.dir.path().join("a.js"), body).unwrap();
        let loader = ReviewContextLoader::new(&f.inputs).unwrap();
        let observed = loader.read("a.js", 8 * 1024 * 1024).outcome;
        assert!(
            matches!(
                &observed,
                ContextReadOutcome::Skipped(ContextSkip::NotUtf8)
                    | ContextReadOutcome::Skipped(ContextSkip::TooLarge { .. })
            ),
            "inadmissible precondition: actual rejected reader outcome"
        );
        let off = f.off(&taint());
        let on = f.on(&taint());
        let c = context(&on);
        assert_eq!(c.published.files, 0);
        assert!(c.context_files.is_empty());
        same_result(&off.run.results[0], &on.run.results[0]);
        let notes = note_messages(&on.run, ContextRunNoteKind::FileSkipped);
        assert_eq!(notes.len(), 1);
        assert!(notes[0].contains(expected));
        let d = doc(&f, &on);
        assert_eq!(
            notices(&d.sarif.value)
                .iter()
                .filter(|v| v["message"]["text"] == notes[0])
                .count(),
            1
        );
    }
    // CONTROLLED reason records: no claim that this executor's reader produced them.
    let f = Fixture::js(B);
    let mut on = f.on(&taint());
    let ContextState::Expanded(c) = &mut on.run.context else {
        panic!("controlled expanded state")
    };
    c.attempts.skips = vec![
        (
            "unreadable.js".into(),
            crate::algorithms::taint::context_budget::SkipReason::Unreadable {
                kind: std::io::ErrorKind::PermissionDenied,
                message: "denied\nline".into(),
            },
        ),
        (
            "parse.js".into(),
            crate::algorithms::taint::context_budget::SkipReason::ParseFailed {
                message: "bad\nparse".into(),
            },
        ),
        (
            "severe.js".into(),
            crate::algorithms::taint::context_budget::SkipReason::SevereParse {
                error_count: 4,
                node_count: 9,
            },
        ),
    ];
    let req = ContextRequest {
        limits: ContextLimits::default(),
        ceiling: CrossingCeiling::NameOnly,
    };
    let notes = crate::algorithms::taint::boundary::context_notes::notes_only(
        &on.run.context,
        &req,
        &f.inputs.parse_warnings,
    );
    let messages: Vec<_> = notes
        .iter()
        .filter(|n| n.kind() == ContextRunNoteKind::FileSkipped)
        .map(|n| n.message())
        .collect();
    assert_eq!(messages.len(), 3);
    assert!(messages
        .iter()
        .any(|s| s.contains("unreadable PermissionDenied: denied\\nline")));
    assert!(messages
        .iter()
        .any(|s| s.contains("parse failed: bad\\nparse")));
    assert!(messages
        .iter()
        .any(|s| s.contains("severe parse (4/9 nodes)")));
}
#[test]
fn g15_loaded_unresolved_preserves_boundary_without_invented_finding() {
    let main = "import { h } from './a';\nexport function main(x) { h(x); }\n";
    let f = Fixture::new(
        &[("main.js", main), ("a.js", "export const other = 1;\n")],
        "main.js",
        &[2],
    );
    let on = f.on(&taint());
    let c = context(&on);
    assert!(c.context_files.contains("a.js"));
    assert!(on.run.results[0].findings.iter().all(|x| x.file != "a.js"));
    let d = on.run.diagnostics.as_ref().unwrap();
    let note = d
        .located()
        .iter()
        .find(|n| n.kind() == crate::api::ReviewBoundaryKind::LoadedUnresolved)
        .expect("actual loaded unresolved boundary");
    let rendered = doc(&f, &on);
    assert_eq!(
        notices(&rendered.sarif.value)
            .iter()
            .filter(|v| v["message"]["text"] == note.message())
            .count(),
        1
    );
    assert!(!rendered.targets.value["targets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["site"]["file"] == "a.js"));
}
#[test]
fn g16_twelve_eligible_sites_are_not_capped() {
    let mut main = String::new();
    let mut files = Vec::<(String, String)>::new();
    for i in 0..12 {
        main.push_str(&format!("import {{ f{i} }} from './m{i}';\n"));
        files.push((
            format!("m{i}.js"),
            format!("export const other{i} = {i};\n"),
        ));
    }
    main.push_str("export function main(x) { ");
    for i in 0..12 {
        main.push_str(&format!("f{i}(x); "));
    }
    main.push_str("}\n");
    files.push(("main.js".into(), main));
    let refs = files
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect::<Vec<_>>();
    let f = Fixture::new(&refs, "main.js", &[13]);
    let mut p = imports();
    p.taint_context.depth = 1;
    let on = f.run(&taint(), &p);
    let c = context(&on);
    assert_eq!(c.frontier.provenance.len(), 12);
    assert_eq!(c.context_files.len(), 12);
    assert_eq!(c.attempts.attempted_files, 12);
    let d = on.run.diagnostics.as_ref().unwrap();
    assert_eq!(d.located().len(), 10);
    assert_eq!(d.omitted_count(), 2);
    assert!(d
        .located()
        .iter()
        .all(|note| note.kind() == crate::api::ReviewBoundaryKind::LoadedUnresolved));
}
#[test]
fn g17_bound_notes_repeat_for_input_order() {
    let f = Fixture::js(B);
    let mut p = imports();
    p.taint_context.depth = 1;
    let one = f.run(&taint(), &p);
    let two = f.run(&taint(), &p);
    let first = note_messages(&one.run, ContextRunNoteKind::DepthBound);
    assert_eq!(
        first,
        note_messages(&two.run, ContextRunNoteKind::DepthBound)
    );
    assert_eq!(first.len(), 1);
    std::fs::write(f.dir.path().join("other.js"), "export const v = 1;\n").unwrap();
    let diff1 = r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]},{"file_path":"other.js","modify_type":"Modified","diff_lines":[1]}]}"#;
    let diff2 = r#"{"files":[{"file_path":"other.js","modify_type":"Modified","diff_lines":[1]},{"file_path":"main.js","modify_type":"Modified","diff_lines":[2,3]}]}"#;
    let a = crate::api::review(&f.options, diff1, &taint(), &SliceConfig::default(), &p).unwrap();
    let b = crate::api::review(&f.options, diff2, &taint(), &SliceConfig::default(), &p).unwrap();
    assert_eq!(
        note_messages(&a.run, ContextRunNoteKind::DepthBound),
        note_messages(&b.run, ContextRunNoteKind::DepthBound)
    );
    assert_eq!(
        note_messages(&a.run, ContextRunNoteKind::DepthBound).len(),
        1
    );
}
