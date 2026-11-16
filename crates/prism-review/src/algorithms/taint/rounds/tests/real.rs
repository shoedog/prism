//! REAL: each row starts from a producer trace and the captured repository loader.
use super::super::super::boundary::collect::collect_round;
use super::super::super::frontier::plan_round;
use super::super::super::source_scope::{
    admission::slice_in_context, loading::project_loading_trace,
};
use super::*;
use prism::api::{ContextReadOutcome, ReviewBoundaryHints, ReviewContextRound};
use prism::cpg::TaintCrossingPolicy;

fn initial_plan(r: &ExpansionRequest<'_>) -> FrontierState {
    let originals = r.inputs.files.keys().cloned().collect();
    let trace = project_loading_trace(r.baseline_trace, &originals, &r.inputs.files);
    let mut hints =
        ReviewBoundaryHints::new(r.inputs).expect("inadmissible precondition: baseline hints");
    let collection = collect_round(&mut hints, r.baseline_ctx, &trace);
    plan_round(
        1,
        r.limits.depth,
        &collection,
        r.baseline_ctx,
        &FrontierState::default(),
        &BTreeSet::new(),
        &mut |b, m| hints.follow_barrel(b, m).map(|h| (h.file, h.member)),
    )
}
#[test]
fn producer_frontier_file_cap_promotes_exact_prefix() {
    let f = Fixture::new(&[("a.js", BODY), ("b.js", BODY), ("c.js", BODY)]);
    f.run(limits(2, 2), &Guard::never(), |r| {
        let plan = initial_plan(r);
        assert_eq!(
            plan.load_order.len(),
            3,
            "inadmissible precondition: three actual producer targets"
        );
        let expected: BTreeSet<_> = plan.load_order[..2].iter().cloned().collect();
        let result = run_owned(r);
        assert_eq!(result.bundle.context_files, expected);
        assert_eq!(
            result.stop,
            Stop::Bound(AdmissionBound::Files {
                first_skipped: 2,
                skipped: 1
            })
        );
        assert_eq!(result.attempts.attempted_files, 2);
        assert!(result.bundle.analysis.is_some());
        assert_eq!(result.stats.rounds_completed, 1);
    });
}

const MAIN: &str = "import { h } from './a';\nexport function main(x) { h(x); }\n";
const MID: &str = "import { g } from './b';\nexport function h(x) { g(x); }\n";
fn fixture(body: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    for (name, source) in [("main.js", MAIN), ("a.js", MID), ("b.js", body)] {
        std::fs::write(dir.path().join(name), source).unwrap();
    }
    let options = ReviewOptions::new(dir.path());
    let inputs = load_review_inputs(
        &options,
        r#"{"files":[{"file_path":"main.js","modify_type":"Modified","diff_lines":[2]}]}"#,
    )
    .unwrap();
    assert!(
        inputs.files.contains_key("main.js"),
        "inadmissible precondition: original main"
    );
    Fixture {
        _dir: dir,
        inputs,
        options,
    }
}
// Independent P14-style producer precondition: round-one trace reaches the call inside a.js.
fn assert_context_caller_reachable(r: &ExpansionRequest<'_>) {
    let mut files = r.inputs.files.clone();
    let a = match r.loader.read("a.js", 1 << 20).outcome {
        ContextReadOutcome::Loaded(a) => a,
        _ => panic!("inadmissible precondition: a loader"),
    };
    files.insert(a.path, a.parsed);
    let round = ReviewContextRound::new(r.loader, &files);
    let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
    let originals = r.inputs.files.keys().cloned().collect();
    let context = ["a.js".to_string()].into_iter().collect();
    let policy = TaintCrossingPolicy::new(&context, r.ceiling);
    let analysis = slice_in_context(&ctx, &r.inputs.diff, r.config, &originals, &policy).unwrap();
    let mut hints = round.hints().unwrap();
    let collection = collect_round(&mut hints, &ctx, &analysis.loading);
    assert!(collection.records.iter().any(|row| row.site.caller.file == "a.js" && format!("{:?}", row.hint).contains("b.js")), "inadmissible precondition: eligible trace reaches a.js argument and b hint");
}
#[test]
fn producer_depth_one_and_two_follow_context_caller() {
    let f = fixture("export function g(x) {}\n");
    f.run(limits(2, 8), &Guard::never(), |r| {
        assert_eq!(
            initial_plan(r).load_order,
            ["a.js"],
            "inadmissible precondition: real round zero target"
        );
        assert_context_caller_reachable(r);
        let one = expand(ExpansionRequest {
            loader: r.loader,
            inputs: r.inputs,
            baseline_ctx: r.baseline_ctx,
            baseline_trace: r.baseline_trace,
            config: r.config,
            limits: limits(1, 8),
            ceiling: r.ceiling,
            guard: r.guard,
        });
        assert_eq!(
            one.bundle.context_files,
            ["a.js".into()].into_iter().collect()
        );
        assert_eq!(one.stop, Stop::DepthReached);
        assert!(one
            .bundle
            .frontier
            .dispositions
            .iter()
            .any(|(_, p, d)| p == "b.js" && *d == Disposition::DepthBound));
        let two = run_owned(r);
        assert!(two.bundle.context_files.contains("b.js"));
        assert_eq!(two.bundle.round, 2);
    });
}
#[test]
fn admitted_context_changes_analysis_and_produces_evidence() {
    let f = fixture("export function g(x) { eval(x); }\n");
    f.run(limits(2, 8), &Guard::never(), |r| {
        assert_eq!(
            initial_plan(r).load_order,
            ["a.js"],
            "inadmissible precondition: real round zero target"
        );
        assert_context_caller_reachable(r);
        let mut files = r.inputs.files.clone();
        for path in ["a.js", "b.js"] {
            match r.loader.read(path, 1 << 20).outcome {
                ContextReadOutcome::Loaded(file) => {
                    files.insert(file.path, file.parsed);
                }
                _ => panic!("inadmissible precondition: context parse"),
            }
        }
        let round = ReviewContextRound::new(r.loader, &files);
        let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
        let originals = r.inputs.files.keys().cloned().collect();
        let context = ["a.js".to_string(), "b.js".to_string()]
            .into_iter()
            .collect();
        let policy = TaintCrossingPolicy::new(&context, r.ceiling);
        let expected =
            slice_in_context(&ctx, &r.inputs.diff, r.config, &originals, &policy).unwrap();
        let context_sink = expected
            .result
            .findings
            .iter()
            .enumerate()
            .find(|(_, finding)| {
                finding.file == "b.js" && finding.category.as_deref() == Some("taint_sink")
            });
        let (index, _) =
            context_sink.expect("inadmissible precondition: actual union context sink");
        assert!(
            expected.result.evidence[index]
                .as_ref()
                .is_some_and(|e| !e.hops.is_empty()),
            "inadmissible precondition: nonempty context evidence"
        );
        let (baseline, _) =
            super::super::super::slice_with_trace(r.baseline_ctx, &r.inputs.diff, r.config)
                .unwrap();
        assert_ne!(
            format!("{:?}", baseline),
            format!("{:?}", expected.result),
            "inadmissible precondition: union analysis differs from baseline"
        );
        let actual = run_owned(r);
        assert!(actual.bundle.context_files.contains("b.js"));
        let analysis = actual.bundle.analysis.as_ref().expect("completed analysis");
        assert!(
            !analysis.result.findings.is_empty(),
            "completed analysis contains a finding"
        );
        let mut context_hop = false;
        for finding in &analysis.result.findings {
            assert!(actual.bundle.files.contains_key(&finding.file));
        }
        for evidence in analysis.result.evidence.iter().flatten() {
            for hop in &evidence.hops {
                match hop {
                    prism::finding_confidence::EvidenceHop::DataFlow { from, to, .. } => {
                        assert!(actual.bundle.files.contains_key(&from.file));
                        assert!(actual.bundle.files.contains_key(&to.file));
                        context_hop |= from.file == "b.js" || to.file == "b.js";
                    }
                    prism::finding_confidence::EvidenceHop::Call { edge, .. } => {
                        assert!(actual.bundle.files.contains_key(&edge.caller.file));
                        context_hop |= edge.caller.file == "b.js";
                    }
                    _ => {}
                }
            }
        }
        assert!(
            context_hop,
            "inadmissible precondition: evidence reaches admitted b.js"
        );
        assert_eq!(
            format!("{:?}", analysis.result),
            format!("{:?}", expected.result)
        );
        assert_eq!(
            format!("{:?}", analysis.result.evidence),
            format!("{:?}", expected.result.evidence)
        );
    });
}

#[test]
fn producer_depth_attempts_p14_python_and_go() {
    let cases: &[(&str, usize, &str, &str, &[(&str, &str)])] = &[
        ("main.py", 3, "a.py", "b.py", &[("main.py", "from a import h\ndef main(x):\n    h(x)\n"), ("a.py", "from b import g\ndef h(x):\n    g(x)\n"), ("b.py", "def g(x):\n    pass\n")]),
        ("main.go", 3, "svc/a.go", "leaf/b.go", &[("go.mod", "module example.com/app\n"), ("main.go", "package main\nimport \"example.com/app/svc\"\nfunc main1(x string) { svc.H(x) }\n"), ("svc/a.go", "package svc\nimport \"example.com/app/leaf\"\nfunc H(x string) { leaf.G(x) }\n"), ("leaf/b.go", "package leaf\nfunc G(x string) {}\n")]),
    ];
    for &(original, line, context_path, target, files) in cases {
        let dir = tempfile::tempdir().unwrap();
        for (name, source) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, source).unwrap();
        }
        let options = ReviewOptions::new(dir.path());
        let diff = serde_json::json!({"files":[{"file_path":original,"modify_type":"Modified","diff_lines":[line]}]});
        let inputs = load_review_inputs(&options, &diff.to_string()).unwrap();
        let f = Fixture {
            _dir: dir,
            inputs,
            options,
        };
        f.run(limits(2, 8), &Guard::never(), |r| {
            assert_eq!(
                initial_plan(r).load_order,
                [context_path],
                "inadmissible precondition: {original} producer first target"
            );
            let mut union = r.inputs.files.clone();
            let loaded = match r.loader.read(context_path, 1 << 20).outcome {
                ContextReadOutcome::Loaded(file) => file,
                _ => panic!("inadmissible precondition: {context_path} admitted"),
            };
            union.insert(loaded.path, loaded.parsed);
            let round = ReviewContextRound::new(r.loader, &union);
            let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
            let originals = r.inputs.files.keys().cloned().collect();
            let context = [context_path.to_string()].into_iter().collect();
            let policy = TaintCrossingPolicy::new(&context, r.ceiling);
            let analysis =
                slice_in_context(&ctx, &r.inputs.diff, r.config, &originals, &policy).unwrap();
            let mut hints = round.hints().unwrap();
            let collection = collect_round(&mut hints, &ctx, &analysis.loading);
            assert!(
                collection
                    .records
                    .iter()
                    .any(|row| row.site.caller.file == context_path
                        && format!("{:?}", row.hint).contains(target)),
                "inadmissible precondition: {original} producer context reach"
            );
            let shallow = expand(ExpansionRequest {
                loader: r.loader,
                inputs: r.inputs,
                baseline_ctx: r.baseline_ctx,
                baseline_trace: r.baseline_trace,
                config: r.config,
                limits: limits(1, 8),
                ceiling: r.ceiling,
                guard: r.guard,
            });
            assert_eq!(shallow.stop, Stop::DepthReached);
            assert!(shallow
                .bundle
                .frontier
                .dispositions
                .iter()
                .any(|(_, path, d)| path == target && *d == Disposition::DepthBound));
            let deep = run_owned(r);
            assert!(deep.bundle.context_files.contains(target));
            assert_eq!(deep.bundle.round, 2);
        });
    }
}
