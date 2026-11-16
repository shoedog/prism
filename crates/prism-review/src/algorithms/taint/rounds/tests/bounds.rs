use super::*;

#[test]
fn files_cap_promotes_the_completed_prefix() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY), ("c.js", BODY)]);
    f.run(limits(2, 2), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g"), ("c.js", "h")]);
        let mut attempts = AttemptCounters::default();
        let (b, stop) = settle(
            old,
            run_stage(
                r,
                &{
                    let mut b = initial(r);
                    pending(&mut b, &[("a.js", "f"), ("b.js", "g"), ("c.js", "h")]);
                    b
                },
                &mut attempts,
            ),
        );
        assert_eq!(
            b.context_files,
            ["a.js".into(), "b.js".into()].into_iter().collect()
        );
        assert!(b.analysis.is_some());
        assert_eq!(
            stop,
            Some(Stop::Bound(AdmissionBound::Files {
                first_skipped: 2,
                skipped: 1
            }))
        );
        assert_eq!(attempts.attempted_files, 2);
    });
}
#[test]
fn byte_cap_promotes_the_completed_prefix() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY)]);
    let mut l = limits(2, 5);
    l.source_bytes = BODY.len() as u64;
    f.run(l, &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let stage = run_stage(r, &old, &mut attempts);
        let (b, stop) = settle(old, stage);
        assert_eq!(b.context_files, ["a.js".into()].into_iter().collect());
        assert_eq!(b.published.bytes, BODY.len() as u64);
        assert_eq!(
            stop,
            Some(Stop::Bound(AdmissionBound::SourceBytes {
                first_skipped: 1,
                skipped: 1
            }))
        );
    });
}
#[test]
fn cap_with_nothing_accepted_keeps_previous_bundle() {
    let f = Fixture::new(&[("a.js", BODY)]);
    for (files, bytes, expected) in [
        (
            0,
            100,
            AdmissionBound::Files {
                first_skipped: 0,
                skipped: 1,
            },
        ),
        (
            2,
            0,
            AdmissionBound::SourceBytes {
                first_skipped: 0,
                skipped: 1,
            },
        ),
    ] {
        let mut l = limits(2, files);
        l.source_bytes = bytes;
        f.run(l, &Guard::never(), |r| {
            let mut old = initial(r);
            pending(&mut old, &[("a.js", "f")]);
            let before = snapshot(&old);
            let stage = run_stage(r, &old, &mut AttemptCounters::default());
            let (b, stop) = settle(old, stage);
            assert_eq!(snapshot(&b), before);
            assert_eq!(stop, Some(Stop::Bound(expected)));
        });
    }
}
#[test]
fn all_skips_keep_prior_completed_bundle() {
    let f = Fixture::new(&[("a.js", BODY)]);
    std::fs::write(f._dir.path().join("a.js"), [0xff]).unwrap();
    f.run(limits(2, 3), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f")]);
        let before = snapshot(&old);
        let mut attempts = AttemptCounters::default();
        let stage = run_stage(r, &old, &mut attempts);
        let (b, stop) = settle(old, stage);
        assert_eq!(snapshot(&b), before);
        assert_eq!(stop, Some(Stop::FrontierExhausted));
        assert!(matches!(attempts.skips[0].1, SkipReason::NotUtf8));
    });
}
#[test]
fn per_file_skips_do_not_fail_the_round() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY)]);
    std::fs::write(f._dir.path().join("a.js"), [0xff]).unwrap();
    f.run(limits(2, 3), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let stage = run_stage(r, &old, &mut attempts);
        let (b, _) = settle(old, stage);
        assert_eq!(b.context_files, ["b.js".into()].into_iter().collect());
        assert!(matches!(attempts.skips[0].1, SkipReason::NotUtf8));
        assert_eq!(b.published.files, 1);
        assert_eq!(attempts.attempted_files, 2);
    });
}

#[test]
fn controlled_depth_one_preserves_all_depth_bound_origins() {
    let f = Fixture::new(&[
        ("a.js", "export { f } from './c';\n"),
        ("b.js", "export { g } from './c';\n"),
        ("c.js", BODY),
    ]);
    f.run(limits(1, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let b = run_stage(r, &old, &mut AttemptCounters::default())
            .unwrap()
            .bundle;
        assert!(b
            .frontier
            .dispositions
            .iter()
            .any(|(o, p, d)| *o == 0 && p == "c.js" && *d == Disposition::DepthBound));
        assert!(b
            .frontier
            .dispositions
            .iter()
            .any(|(o, p, d)| *o == 1 && p == "c.js" && *d == Disposition::DepthBound));
        assert_eq!(super::driver::terminal(&b, 1), Stop::DepthReached);
    });
}
#[test]
fn real_expansion_has_nonempty_context_and_preserves_originals() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY), ("c.js", BODY)]);
    f.run(limits(2, 8), &Guard::never(), |r| {
        let mut hints = prism::api::ReviewBoundaryHints::new(r.inputs).unwrap();
        let originals = r.inputs.files.keys().cloned().collect();
        let trace = super::super::super::source_scope::loading::project_loading_trace(
            r.baseline_trace,
            &originals,
            &r.inputs.files,
        );
        let c = super::super::super::boundary::collect::collect_round(
            &mut hints,
            r.baseline_ctx,
            &trace,
        );
        assert!(
            !c.records.is_empty(),
            "inadmissible precondition: real eligible call"
        );
        let outcome = run_owned(r);
        assert!(!outcome.bundle.context_files.is_empty());
        for (path, source) in &r.inputs.sources {
            assert_eq!(outcome.bundle.sources.get(path), Some(source));
        }
        assert_eq!(outcome.stats.rounds_started, outcome.stats.rounds_completed);
    });
}

#[test]
fn quality_exactly_matches_sparse_reader_results() {
    let mut bad = String::new();
    for i in 0..70 {
        bad.push_str(&format!("export const v{i} = {i};\n"));
    }
    for _ in 0..8 {
        bad.push_str("const = ;\n");
    }
    let f = Fixture::new(&[("a.js", &bad), ("b.js", BODY)]);
    f.run(limits(2, 8), &Guard::never(), |r| {
        let expected = match r.loader.read("a.js", 100_000).outcome {
            prism::api::ContextReadOutcome::Loaded(file) => file
                .parse_quality
                .expect("inadmissible precondition: degraded but admitted fixture"),
            _ => panic!("inadmissible precondition: degraded fixture admitted"),
        };
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let b = run_stage(r, &old, &mut AttemptCounters::default())
            .unwrap()
            .bundle;
        assert_eq!(
            serde_json::to_value(&b.parse_quality["a.js"]).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert!(!b.parse_quality.contains_key("b.js"));
        for (p, q) in &r.inputs.parse_quality {
            assert_eq!(
                serde_json::to_value(&b.parse_quality[p]).unwrap(),
                serde_json::to_value(q).unwrap()
            );
        }
        assert!(b.parse_quality.keys().all(|p| b.files.contains_key(p)));
    });
}
#[test]
fn round_zero_capture_is_not_mutated_by_completed_or_discarded_stage() {
    let f = Fixture::new(&[("a.js", BODY)]);
    for guard in [Guard::never(), Guard::at(1, StopCause::Cancelled)] {
        f.run(limits(2, 4), &guard, |r| {
            let scope = serde_json::to_string(&r.inputs.scope_graph_inputs).unwrap();
            let original: Vec<_> = r
                .inputs
                .files
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        v.source.clone(),
                        v.parse_error_count,
                        v.parse_node_count,
                    )
                })
                .collect();
            let sources = r.inputs.sources.clone();
            let mut old = initial(r);
            pending(&mut old, &[("a.js", "f")]);
            let before = snapshot(&old);
            let staged = run_stage(r, &old, &mut AttemptCounters::default());
            assert_eq!(snapshot(&old), before);
            let _ = settle(old, staged);
            assert_eq!(
                serde_json::to_string(&r.inputs.scope_graph_inputs).unwrap(),
                scope
            );
            assert_eq!(r.inputs.sources, sources);
            assert_eq!(
                r.inputs
                    .files
                    .iter()
                    .map(|(k, v)| (
                        k.clone(),
                        v.source.clone(),
                        v.parse_error_count,
                        v.parse_node_count
                    ))
                    .collect::<Vec<_>>(),
                original
            );
        });
    }
}

#[test]
fn analysis_findings_and_evidence_reference_the_promoted_map() {
    let f = Fixture::new(&[("a.js", "export function f(x) { eval(x); }\n")]);
    f.run(limits(2, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f")]);
        let bundle = run_stage(r, &old, &mut AttemptCounters::default())
            .unwrap()
            .bundle;
        let analysis = bundle.analysis.as_ref().unwrap();
        assert_eq!(
            analysis.result.findings.len(),
            analysis.result.evidence.len()
        );
        for finding in &analysis.result.findings {
            assert!(bundle.files.contains_key(&finding.file));
        }
        for evidence in analysis.result.evidence.iter().flatten() {
            for hop in &evidence.hops {
                match hop {
                    prism::finding_confidence::EvidenceHop::DataFlow { from, to, .. } => {
                        assert!(bundle.files.contains_key(&from.file));
                        assert!(bundle.files.contains_key(&to.file));
                    }
                    prism::finding_confidence::EvidenceHop::Call { edge, .. } => {
                        assert!(bundle.files.contains_key(&edge.caller.file))
                    }
                    _ => {}
                }
            }
        }
    });
}

#[test]
fn real_diff_file_order_does_not_change_expansion() {
    fn run(reverse: bool) -> String {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("main.js"),
            "import { f } from './a';\nexport function m(x) { f(x); }\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("other.js"),
            "import { g } from './b';\nexport function n(x) { g(x); }\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("a.js"), BODY).unwrap();
        std::fs::write(dir.path().join("b.js"), BODY).unwrap();
        let mut names = vec!["main.js", "other.js"];
        if reverse {
            names.reverse();
        }
        let diff = serde_json::json!({"files": names.iter().map(|p| serde_json::json!({"file_path":p,"modify_type":"Modified","diff_lines":[2]})).collect::<Vec<_>>()});
        let options = ReviewOptions::new(dir.path());
        let inputs = load_review_inputs(&options, &diff.to_string()).unwrap();
        let built = build_context(&inputs, &options).unwrap();
        let loader = ReviewContextLoader::new(&inputs).unwrap();
        let config = TaintConfig::default();
        let (_, trace) =
            super::super::super::slice_with_trace(&built.ctx, &inputs.diff, &config).unwrap();
        let guard = Guard::never();
        let result = expand(ExpansionRequest {
            loader: &loader,
            inputs: &inputs,
            baseline_ctx: &built.ctx,
            baseline_trace: &trace,
            config: &config,
            limits: limits(2, 8),
            ceiling: CrossingCeiling::NameOnly,
            guard: &guard,
        });
        assert!(
            !result.bundle.context_files.is_empty(),
            "at least one real context request must be admitted"
        );
        outcome_snapshot(&result)
    }
    assert_eq!(run(false), run(true));
}

#[test]
fn completed_stage_plans_from_loading_not_framework_trace() {
    let dir = tempfile::tempdir().unwrap();
    let main = "import { f } from './a';\nimport { g } from './b';\nconst express = require('express');\nconst app = express();\napp.get('/', (req, res) => { const x = req.query.q; g(x); });\n";
    for (p, s) in [("main.js", main), ("a.js", BODY), ("b.js", BODY)] {
        std::fs::write(dir.path().join(p), s).unwrap();
    }
    let options = ReviewOptions::new(dir.path());
    let inputs = load_review_inputs(
        &options,
        r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[]}]}"#,
    )
    .unwrap();
    let built = build_context(&inputs, &options).unwrap();
    let loader = ReviewContextLoader::new(&inputs).unwrap();
    let config = TaintConfig {
        taint_from_diff: false,
        ..TaintConfig::default()
    };
    let (_, trace) =
        super::super::super::slice_with_trace(&built.ctx, &inputs.diff, &config).unwrap();
    let guard = Guard::never();
    let r = ExpansionRequest {
        loader: &loader,
        inputs: &inputs,
        baseline_ctx: &built.ctx,
        baseline_trace: &trace,
        config: &config,
        limits: limits(2, 8),
        ceiling: CrossingCeiling::NameOnly,
        guard: &guard,
    };
    let mut old = initial(&r);
    pending(&mut old, &[("a.js", "f")]);
    let mut bundle = run_stage(&r, &old, &mut AttemptCounters::default())
        .unwrap()
        .bundle;
    let mut analysis = bundle.analysis.take().unwrap();
    let framework = super::super::super::TaintRootOrigin::Framework;
    assert!(
        analysis
            .trace
            .roots
            .iter()
            .any(|root| root.origin == framework && root.location.path.base == "x"),
        "inadmissible precondition: real producer framework x root"
    );
    assert!(!bundle.frontier.load_order.contains(&"b.js".to_string()));
    let round = prism::api::ReviewContextRound::new(&loader, &bundle.files);
    let ctx = round.build_context(inputs.type_db.as_ref(), &built.ctx.types);
    let mut hints = round.hints().unwrap();
    let call = ctx
        .cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|call| call.caller.file == "main.js" && call.callee_name == "g")
        .expect("inadmissible precondition: real g call")
        .clone();
    let spans = hints.argument_spans(&call);
    assert_eq!(
        spans.len(),
        1,
        "inadmissible precondition: one real g argument"
    );
    assert_eq!(
        &main[spans[0].clone()],
        "x",
        "inadmissible precondition: g argument occurrence"
    );
    let hint = hints.hint_for_call(&call);
    assert!(
        matches!(hint, Some(ReviewImportHint::InRepo { ref files, .. }) if files.iter().any(|p| p == "b.js")),
        "inadmissible precondition: real b hint"
    );
    let site = PhysicalSite {
        caller: call.caller.clone(),
        start_byte: call.start_byte,
        end_byte: call.end_byte,
    };
    assert!(
        !super::super::super::boundary::collect::site_has_loaded_callable(&ctx, &site),
        "inadmissible precondition: g unresolved"
    );

    // CONTROLLED: the real framework producer emitted an empty synthetic occurrence;
    // supply the observed physical g argument as a reached location, without changing the graph.
    let root = analysis
        .trace
        .roots
        .iter_mut()
        .find(|root| root.origin == framework && root.location.path.base == "x")
        .unwrap();
    let mut location = root.location.clone();
    location.start_byte = spans[0].start;
    location.end_byte = spans[0].end;
    location.line = call.line;
    location.function = call.caller.name.clone();
    location.function_start_line = call.caller.start_line;
    location.kind = prism::data_flow::VarAccessKind::Use;
    root.reached.push(super::super::super::TaintReached {
        location,
        evidence: prism::finding_confidence::EvidencePath::default(),
    });
    let originals = inputs.files.keys().cloned().collect();
    analysis.loading = super::super::super::source_scope::loading::project_loading_trace(
        &analysis.trace,
        &originals,
        &bundle.files,
    );
    let full =
        super::super::super::boundary::collect::collect_round(&mut hints, &ctx, &analysis.trace);
    let loading =
        super::super::super::boundary::collect::collect_round(&mut hints, &ctx, &analysis.loading);
    assert!(full.records.iter().any(|record| matches!(&record.hint, ReviewImportHint::InRepo { files, .. } if files.iter().any(|p| p == "b.js"))), "inadmissible precondition: controlled full trace reaches b");
    assert!(
        loading.records.is_empty(),
        "inadmissible precondition: no framework loading roots"
    );
    let frontier =
        super::super::stage::plan_completed(&r, &old, &round, &ctx, &analysis, &BTreeSet::new())
            .unwrap();
    assert!(!frontier.load_order.contains(&"b.js".to_string()));
}
