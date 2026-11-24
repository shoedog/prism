use super::*;

#[test]
fn settle_drops_failed_and_empty_stages() {
    let f = Fixture::new(&[]);
    f.run(limits(2, 4), &Guard::never(), |r| {
        for (stage, expected) in [
            (
                Err(StageStop::Failed("broken".into())),
                Stop::StageFailed("broken".into()),
            ),
            (
                Err(StageStop::NothingAccepted(None)),
                Stop::FrontierExhausted,
            ),
            (
                Err(StageStop::NothingAccepted(Some(AdmissionBound::Files {
                    first_skipped: 0,
                    skipped: 1,
                }))),
                Stop::Bound(AdmissionBound::Files {
                    first_skipped: 0,
                    skipped: 1,
                }),
            ),
            (
                Err(StageStop::Guard(StopCause::Cancelled)),
                Stop::Guard(StopCause::Cancelled),
            ),
        ] {
            let old = initial(r);
            let before = snapshot(&old);
            let (b, stop) = settle(old, stage);
            assert_eq!(snapshot(&b), before);
            assert_eq!(stop, Some(expected));
        }
        let mut new = initial(r);
        new.round = 1;
        let (b, stop) = settle(
            initial(r),
            Ok(Staged {
                bundle: new,
                bound: Some(AdmissionBound::Files {
                    first_skipped: 1,
                    skipped: 1,
                }),
            }),
        );
        assert_eq!(b.round, 1);
        assert_eq!(
            stop,
            Some(Stop::Bound(AdmissionBound::Files {
                first_skipped: 1,
                skipped: 1
            }))
        );
    });
}
#[test]
fn guard_after_reads_promotes_nothing_and_attempts_are_not_refunded() {
    let f = Fixture::new(&[("a.js", BODY)]);
    let guard = Guard::at(1, StopCause::Time);
    f.run(limits(2, 4), &guard, |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f")]);
        let before = snapshot(&old);
        let mut attempts = AttemptCounters::default();
        let stage = run_stage(r, &old, &mut attempts, &mut RoundStats::default());
        let (b, stop) = settle(old, stage);
        assert_eq!(snapshot(&b), before);
        assert_eq!(stop, Some(Stop::Guard(StopCause::Time)));
        assert_eq!(attempts.attempted_files, 1);
        assert_eq!(attempts.attempted_read_bytes, BODY.len() as u64);
    });
}
#[test]
fn a_stop_at_each_consultation_never_leaks_a_stage() {
    let f = Fixture::new(&[("a.js", BODY)]);
    for cause in [StopCause::Time, StopCause::Cancelled] {
        for at in 0..10 {
            let guard = Guard::at(at, cause);
            f.run(limits(2, 4), &guard, |r| {
                let mut old = initial(r);
                pending(&mut old, &[("a.js", "f")]);
                let before = snapshot(&old);
                let mut attempts = AttemptCounters::default();
                let stage = run_stage(r, &old, &mut attempts, &mut RoundStats::default());
                let (b, stop) = settle(old, stage);
                if stop == Some(Stop::Guard(cause)) {
                    assert_eq!(snapshot(&b), before);
                } else {
                    assert_eq!(b.round, 1);
                    assert!(b.analysis.is_some());
                    assert_eq!(b.context_files.len(), 1);
                }
            });
        }
    }
}
#[test]
fn bundle_maps_quality_and_originals_are_consistent() {
    let f = Fixture::new(&[("a.js", BODY)]);
    f.run(limits(2, 4), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f")]);
        let original_sources = r.inputs.sources.clone();
        let paths = r.inputs.scope_graph_inputs.all_file_paths.clone();
        let stage = run_stage(
            r,
            &old,
            &mut AttemptCounters::default(),
            &mut RoundStats::default(),
        );
        let (b, _) = settle(old, stage);
        assert!(b.analysis.is_some());
        assert_eq!(
            b.files.keys().collect::<Vec<_>>(),
            b.sources.keys().collect::<Vec<_>>()
        );
        assert!(b.parse_quality.keys().all(|k| b.files.contains_key(k)));
        assert!(
            !b.parse_quality.contains_key("a.js"),
            "inadmissible precondition: clean file quality is sparse"
        );
        for (p, s) in original_sources {
            assert_eq!(b.sources.get(&p), Some(&s));
        }
        assert_eq!(r.inputs.scope_graph_inputs.all_file_paths, paths);
        assert_eq!(b.published.files, b.context_files.len());
        assert_eq!(b.published.bytes, BODY.len() as u64);
    });
}
#[test]
fn loaded_continuation_advances_without_read_and_preserves_origins() {
    let f = Fixture::with_main(
        MAIN_NO_C_CALL,
        &[
            ("a.js", "export { f } from './b';\n"),
            ("b.js", "export { f } from './c';\nexport const g = 1;\n"),
            ("c.js", BODY),
        ],
    );
    f.run(limits(3, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let first = run_stage(r, &old, &mut attempts, &mut RoundStats::default())
            .expect("first stage")
            .bundle;
        assert_eq!(attempts.attempted_files, 2);
        assert_eq!(first.round, 1);
        assert!(first.frontier.load_order.is_empty());
        assert!(first
            .frontier
            .open
            .iter()
            .any(|p| p.target == "b.js" && p.member.as_deref() == Some("f") && p.origin == 0));
        let before_bytes = first.published.bytes;
        let second = run_stage(r, &first, &mut attempts, &mut RoundStats::default())
            .expect("logical no-read stage")
            .bundle;
        assert_eq!(second.round, 2);
        assert_eq!(attempts.attempted_files, 2);
        assert_eq!(second.published.bytes, before_bytes);
        assert_eq!(second.frontier.load_order, ["c.js"]);
        assert!(second
            .frontier
            .open
            .iter()
            .any(|p| p.target == "c.js" && p.origin == 0));
        let third = run_stage(r, &second, &mut attempts, &mut RoundStats::default())
            .expect("third stage")
            .bundle;
        assert_eq!(third.round, 3);
        assert!(third.context_files.contains("c.js"));
        assert_eq!(attempts.attempted_files, 3);
    });
}
#[test]
fn no_read_stage_guard_keeps_prior_bundle() {
    let f = Fixture::new(&[("a.js", BODY)]);
    f.run(limits(3, 8), &Guard::at(0, StopCause::Cancelled), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("main.js", "f")]);
        let before = snapshot(&old);
        let mut attempts = AttemptCounters::default();
        let staged = run_stage(r, &old, &mut attempts, &mut RoundStats::default());
        let (b, stop) = settle(old, staged);
        assert_eq!(snapshot(&b), before);
        assert_eq!(stop, Some(Stop::Guard(StopCause::Cancelled)));
        assert_eq!(attempts.attempted_files, 0);
    });
}

#[test]
fn loaded_continuation_depth_boundary_and_no_pending_negative() {
    let f = Fixture::with_main(
        MAIN_NO_C_CALL,
        &[
            ("a.js", "export { f } from './b';\n"),
            ("b.js", "export { f } from './c';\nexport const g = 1;\n"),
            ("c.js", BODY),
        ],
    );
    f.run(limits(2, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let first = run_stage(r, &old, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert!(first.frontier.load_order.is_empty());
        assert_eq!(attempts.attempted_files, 2);
        let second = run_stage(r, &first, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert!(second
            .frontier
            .dispositions
            .iter()
            .any(|(origin, path, d)| *origin == 0
                && path == "c.js"
                && *d == Disposition::DepthBound));
        assert!(second.frontier.open.is_empty());
        assert_eq!(attempts.attempted_files, 2);
        assert_eq!(super::driver::terminal(&second, 2), Stop::DepthReached);
        let mut empty = initial(r);
        empty.round = 2;
        assert_eq!(super::driver::terminal(&empty, 2), Stop::FrontierExhausted);
    });
}
#[test]
fn all_skips_with_loaded_continuation_preserves_attempts_and_refusal() {
    let f = Fixture::with_main(
        MAIN_NO_C_CALL,
        &[
            ("a.js", "export { f } from './b';\n"),
            ("b.js", "export { f } from './c';\nexport const g = 1;\n"),
            ("c.js", BODY),
            ("d.js", BODY),
        ],
    );
    std::fs::write(f._dir.path().join("d.js"), [0xff]).unwrap();
    f.run(limits(3, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let mut first = run_stage(r, &old, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert!(first.frontier.load_order.is_empty());
        assert_eq!(attempts.attempted_files, 2);
        pending(&mut first, &[("d.js", "h")]);
        assert_eq!(first.frontier.load_order, ["d.js"]);
        let second = run_stage(r, &first, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert_eq!(second.round, 2);
        assert_eq!(attempts.attempted_files, 3);
        assert!(matches!(attempts.skips[0].1, SkipReason::NotUtf8));
        assert!(!second.context_files.contains("d.js"));
        assert!(second
            .frontier
            .dispositions
            .iter()
            .any(|(_, p, d)| p == "d.js" && *d == Disposition::TargetRefused));
        assert_eq!(second.frontier.load_order, ["c.js"]);
    });
}
#[test]
fn controlled_cycle_never_rereads_loaded_files() {
    let f = Fixture::new(&[
        ("a.js", "export { f } from './b';\n"),
        ("b.js", "export { f } from './a';\n"),
    ]);
    f.run(limits(4, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f")]);
        let mut attempts = AttemptCounters::default();
        let a = run_stage(r, &old, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        let b = run_stage(r, &a, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert_eq!(attempts.attempted_files, 2);
        assert!(b.frontier.open.is_empty());
        assert!(b
            .frontier
            .dispositions
            .iter()
            .any(|(_, _, d)| *d == Disposition::BarrelCycle));
    });
}
#[test]
fn empty_frontier_does_not_start_a_stage() {
    let f = Fixture::new(&[]);
    f.run(limits(2, 4), &Guard::never(), |r| {
        let mut old = initial(r);
        let before = snapshot(&old);
        let mut attempts = AttemptCounters::default();
        let stage = run_stage(r, &old, &mut attempts, &mut RoundStats::default());
        let (b, stop) = settle(old, stage);
        assert_eq!(snapshot(&b), before);
        assert_eq!(stop, Some(Stop::FrontierExhausted));
        assert_eq!(attempts.attempted_files, 0);
    });
}

#[test]
fn guard_every_phase_of_two_rounds_returns_a_completed_snapshot() {
    let f = Fixture::new(&[("a.js", "export { f } from './b';\n"), ("b.js", BODY)]);
    let baseline_guard = Guard::never();
    let snapshots = f.run(limits(2, 8), &baseline_guard, |r| {
        let mut zero = initial(r);
        pending(&mut zero, &[("a.js", "f")]);
        let mut attempts = AttemptCounters::default();
        let one = run_stage(r, &zero, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        let two = run_stage(r, &one, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert!(
            one.context_files.contains("a.js"),
            "inadmissible precondition: first stage admitted a"
        );
        assert!(
            two.context_files.contains("b.js"),
            "inadmissible precondition: second stage admitted b"
        );
        let snapshots = [snapshot(&zero), snapshot(&one), snapshot(&two)];
        assert_ne!(
            snapshots[0], snapshots[1],
            "inadmissible precondition: distinct first completed stage"
        );
        assert_ne!(
            snapshots[1], snapshots[2],
            "inadmissible precondition: distinct second completed stage"
        );
        snapshots
    });
    let consultations = baseline_guard.calls.get();
    assert!(
        consultations > 0,
        "inadmissible precondition: guard was consulted"
    );
    for cause in [StopCause::Time, StopCause::Cancelled] {
        for at in 0..consultations {
            let guard = Guard::at(at, cause);
            f.run(limits(2, 8), &guard, |r| {
                let mut bundle = initial(r);
                pending(&mut bundle, &[("a.js", "f")]);
                let mut attempts = AttemptCounters::default();
                for _ in 0..2 {
                    let stage = run_stage(r, &bundle, &mut attempts, &mut RoundStats::default());
                    let (next, stop) = settle(bundle, stage);
                    bundle = next;
                    if let Some(stop) = stop {
                        assert_eq!(stop, Stop::Guard(cause));
                        break;
                    }
                }
                assert!(snapshots.contains(&snapshot(&bundle)));
                assert!(guard.calls.get() > at);
            });
        }
    }
}
#[test]
fn duplicated_baseline_root_does_not_change_real_expansion() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY), ("c.js", BODY)]);
    f.run(limits(2, 8), &Guard::never(), |r| {
        let base = run_owned(r);
        assert!(
            !base.bundle.context_files.is_empty(),
            "real expansion must admit context"
        );
        let mut trace = r.baseline_trace.clone();
        let root = trace
            .roots
            .first()
            .expect("inadmissible precondition: baseline root")
            .clone();
        trace.roots.reverse();
        trace.roots.push(root);
        let duplicate = expand(ExpansionRequest {
            loader: r.loader,
            inputs: r.inputs,
            baseline_ctx: r.baseline_ctx,
            baseline_trace: &trace,
            config: r.config,
            limits: r.limits,
            ceiling: r.ceiling,
            guard: r.guard,
        });
        assert_eq!(outcome_snapshot(&base), outcome_snapshot(&duplicate));
    });
}

#[test]
fn guarded_loaded_continuation_discards_the_logical_stage() {
    let f = Fixture::with_main(
        MAIN_NO_C_CALL,
        &[
            ("a.js", "export { f } from './b';\n"),
            ("b.js", "export { f } from './c';\nexport const g = 1;\n"),
            ("c.js", BODY),
        ],
    );
    f.run(limits(3, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let first = run_stage(r, &old, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        let before = snapshot(&first);
        assert!(first.frontier.load_order.is_empty());
        let guard = Guard::at(0, StopCause::Time);
        let guarded = ExpansionRequest {
            loader: r.loader,
            inputs: r.inputs,
            baseline_ctx: r.baseline_ctx,
            baseline_trace: r.baseline_trace,
            config: r.config,
            limits: r.limits,
            ceiling: r.ceiling,
            guard: &guard,
        };
        let stage = run_stage(&guarded, &first, &mut attempts, &mut RoundStats::default());
        let (b, stop) = settle(first, stage);
        assert_eq!(snapshot(&b), before);
        assert_eq!(stop, Some(Stop::Guard(StopCause::Time)));
        assert_eq!(attempts.attempted_files, 2);
    });
}
#[test]
fn guard_after_a_bound_discards_even_the_accepted_prefix() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY)]);
    f.run(limits(2, 1), &Guard::at(2, StopCause::Time), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let before = snapshot(&old);
        let mut attempts = AttemptCounters::default();
        let stage = run_stage(r, &old, &mut attempts, &mut RoundStats::default());
        let (b, stop) = settle(old, stage);
        assert_eq!(snapshot(&b), before);
        assert_eq!(stop, Some(Stop::Guard(StopCause::Time)));
        assert_eq!(attempts.attempted_files, 1);
    });
}

#[test]
fn round_zero_projects_away_framework_roots() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY), ("c.js", BODY)]);
    f.run(limits(2, 8), &Guard::never(), |r| {
        let normal = run_owned(r);
        assert!(
            !normal.bundle.context_files.is_empty(),
            "eligible original loading root must admit context"
        );
        let mut trace = r.baseline_trace.clone();
        assert!(
            !trace.roots.is_empty(),
            "inadmissible precondition: baseline root"
        );
        for root in &mut trace.roots {
            root.origin = super::super::super::TaintRootOrigin::Framework;
        }
        let filtered = expand(ExpansionRequest {
            loader: r.loader,
            inputs: r.inputs,
            baseline_ctx: r.baseline_ctx,
            baseline_trace: &trace,
            config: r.config,
            limits: r.limits,
            ceiling: r.ceiling,
            guard: r.guard,
        });
        assert!(filtered.bundle.context_files.is_empty());
        assert_eq!(filtered.stop, Stop::FrontierExhausted);
    });
}

#[test]
fn driver_advances_loaded_continuation_before_reading_next_file() {
    let f = Fixture::with_main(
        MAIN_NO_C_CALL,
        &[
            ("a.js", "export { f } from './b';\n"),
            ("b.js", "export { f } from './c';\nexport const g = 1;\n"),
            ("c.js", BODY),
        ],
    );
    f.run(limits(3, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let first = run_stage(
            r,
            &old,
            &mut AttemptCounters::default(),
            &mut RoundStats::default(),
        )
        .unwrap()
        .bundle;
        assert!(
            first.frontier.load_order.is_empty(),
            "inadmissible precondition: logical no-read stage"
        );
        assert!(first
            .frontier
            .open
            .iter()
            .any(|p| p.target == "b.js" && p.member.as_deref() == Some("f")));
        let result =
            super::super::driver::drive(r, old, AttemptCounters::default(), RoundStats::default());
        assert!(result.bundle.context_files.contains("c.js"));
        assert_eq!(result.bundle.round, 3);
        assert_eq!(result.stats.rounds_started, 3);
        assert_eq!(result.stats.rounds_completed, 3);
        assert_eq!(result.attempts.attempted_files, 3);
    });
}
