use super::*;
use prism::cpg_cache::{self, CacheResult};

#[test]
fn m07_round_zero_plain_scoped_and_native_cache_hit() {
    let f = named(
        &[
            (
                "main.js",
                "import { h } from './a';\nexport function main(x) { h(x); }\n",
            ),
            ("a.js", "export function h(x) { eval(x); }\n"),
        ],
        "main.js",
        &[2],
    );
    let config = TaintConfig::default();
    let loader = ReviewContextLoader::new(&f.inputs).unwrap();
    let guard = Guard::never();
    let mut scoped = ReviewOptions::new(f._dir.path());
    scoped.scoped_cpg = true;
    let cache = f._dir.path().join("cache");
    let mut cached = ReviewOptions::new(f._dir.path());
    cached.cache_dir = Some(cache.clone());
    drop(build_context(&f.inputs, &cached).unwrap());
    let hashes = cpg_cache::compute_file_hashes(&f.inputs.sources);
    let key =
        cpg_cache::compute_topology_key(&hashes, &f.inputs.scope_graph_inputs.manifest_hashes);
    assert!(
        matches!(
            cpg_cache::load_cache_with_topology(&hashes, &key, f.inputs.type_db.is_some(), &cache),
            CacheResult::Hit(_)
        ),
        "inadmissible precondition: native cache hit"
    );
    let mut snapshots = Vec::new();
    for opts in [&f.options, &scoped, &cached] {
        let built = build_context(&f.inputs, opts).unwrap();
        let (_, trace) =
            super::super::super::super::slice_with_trace(&built.ctx, &f.inputs.diff, &config)
                .unwrap();
        let request = ExpansionRequest {
            loader: &loader,
            inputs: &f.inputs,
            baseline_ctx: &built.ctx,
            baseline_trace: &trace,
            config: &config,
            limits: limits(2, 8),
            ceiling: CrossingCeiling::NameOnly,
            guard: &guard,
        };
        assert_eq!(
            first_plan(&request).load_order,
            ["a.js"],
            "inadmissible precondition: eligible original call"
        );
        let mut union = request.inputs.files.clone();
        let loaded = match request.loader.read("a.js", 1 << 20).outcome {
            prism::api::ContextReadOutcome::Loaded(file) => file,
            _ => panic!("inadmissible precondition: context file read"),
        };
        union.insert(loaded.path, loaded.parsed);
        let round = prism::api::ReviewContextRound::new(request.loader, &union);
        let ctx = round.build_context(request.inputs.type_db.as_ref(), &request.baseline_ctx.types);
        let originals = request.inputs.files.keys().cloned().collect();
        let contexts = ["a.js".to_string()].into_iter().collect();
        let policy = prism::cpg::TaintCrossingPolicy::new(&contexts, request.ceiling);
        let proof = super::super::super::super::source_scope::admission::slice_in_context(
            &ctx,
            &request.inputs.diff,
            request.config,
            &originals,
            &policy,
        )
        .unwrap();
        assert!(
            proof
                .result
                .findings
                .iter()
                .any(|f| f.file == "a.js" && f.category.as_deref() == Some("taint_sink")),
            "inadmissible precondition: actual full union reaches context sink"
        );
        let out = run_owned(&request);
        assert!(out.bundle.context_files.contains("a.js"));
        assert!(!sinks(&out, "a.js").is_empty());
        snapshots.push(outcome_snapshot(&out));
    }
    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[0], snapshots[2]);
}
#[test]
fn m09_go_declaring_only_and_unknown() {
    let main = "package main\nimport \"example.com/app/svc\"\nfunc main1(x string) { svc.H(x) }\n";
    let files = [
        ("go.mod", "module example.com/app\n"),
        ("main.go", main),
        ("svc/a.go", "package svc\nfunc H(x string) {}\n"),
        ("svc/b.go", "package svc\nfunc B() {}\n"),
        ("svc/c.go", "package svc\nfunc C() {}\n"),
    ];
    let f = named(&files, "main.go", &[3]);
    variant(
        &f,
        &TaintConfig::default(),
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            let p = first_plan(r);
            assert_eq!(
                p.load_order,
                ["svc/a.go"],
                "inadmissible precondition: exact Go declaring hint"
            );
            let out = run_owned(r);
            assert_eq!(out.attempts.attempted_files, 1);
            assert_eq!(
                out.bundle.context_files,
                ["svc/a.go".into()].into_iter().collect()
            );
        },
    );
    let absent=named(&[("go.mod","module example.com/app\n"),("main.go","package main\nimport \"example.com/app/svc\"\nfunc main1(x string) { svc.Missing(x) }\n"),("svc/a.go","package svc\nfunc H(x string) {}\n")],"main.go",&[3]);
    variant(
        &absent,
        &TaintConfig::default(),
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            let p = first_plan(r);
            assert!(
                p.dispositions
                    .iter()
                    .any(|(_, _, d)| *d == Disposition::GoDeclarationUnknown),
                "inadmissible precondition: Go unknown declaration"
            );
            let out = run_owned(r);
            assert_eq!(out.attempts.attempted_files, 0);
            assert!(out
                .bundle
                .frontier
                .dispositions
                .iter()
                .any(|(_, _, d)| *d == Disposition::GoDeclarationUnknown));
        },
    );
}
#[test]
fn m10_python_relative_import_context() {
    let f = named(
        &[
            ("pkg/__init__.py", ""),
            ("pkg/main.py", "from .a import h\ndef main(x):\n    h(x)\n"),
            ("pkg/a.py", "def h(x):\n    pass\n"),
        ],
        "pkg/main.py",
        &[3],
    );
    variant(
        &f,
        &TaintConfig::default(),
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            assert_eq!(
                first_plan(r).load_order,
                ["pkg/a.py"],
                "inadmissible precondition: relative import hint"
            );
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("pkg/a.py"));
            assert_eq!(out.bundle.sources["pkg/a.py"], "def h(x):\n    pass\n");
            assert_eq!(out.attempts.attempted_files, 1);
        },
    );
}
// CONTROLLED: retained frontier is constructed; admission/guard/settle are actual D code.
#[test]
fn f4_bound_precedes_loaded_continuation() {
    let f = Fixture::with_main(
        MAIN_NO_C_CALL,
        &[
            ("a.js", "export { f } from './b';\n"),
            ("b.js", "export { f } from './c';\nexport const g = 1;\n"),
            ("c.js", BODY),
            ("d.js", BODY),
        ],
    );
    f.run(limits(3, 8), &Guard::never(), |r| {
        let mut old = initial(r);
        pending(&mut old, &[("a.js", "f"), ("b.js", "g")]);
        let mut attempts = AttemptCounters::default();
        let mut first = run_stage(r, &old, &mut attempts, &mut RoundStats::default())
            .unwrap()
            .bundle;
        assert!(
            first.frontier.load_order.is_empty(),
            "inadmissible precondition: loaded continuation without reads"
        );
        assert!(
            first
                .frontier
                .open
                .iter()
                .any(|p| p.target == "b.js" && p.member.as_deref() == Some("f")),
            "inadmissible precondition: loaded barrel"
        );
        pending(&mut first, &[("d.js", "h")]);
        assert_eq!(first.frontier.load_order, ["d.js"]);
        let before = snapshot(&first);
        let counts = (
            attempts.attempted_files,
            attempts.attempted_read_bytes,
            attempts.skips.len(),
        );
        let capped = ExpansionRequest {
            loader: r.loader,
            inputs: r.inputs,
            baseline_ctx: r.baseline_ctx,
            baseline_trace: r.baseline_trace,
            config: r.config,
            limits: limits(3, first.published.files),
            ceiling: r.ceiling,
            guard: r.guard,
        };
        let out =
            super::super::super::driver::drive(&capped, first, attempts, RoundStats::default());
        assert_eq!(snapshot(&out.bundle), before);
        assert_eq!(
            out.stop,
            Stop::Bound(AdmissionBound::Files {
                first_skipped: 0,
                skipped: 1
            })
        );
        assert_eq!(
            (
                out.attempts.attempted_files,
                out.attempts.attempted_read_bytes,
                out.attempts.skips.len()
            ),
            counts
        );
        assert_eq!(out.stats.rounds_started, 1);
        assert_eq!(out.stats.rounds_completed, 0);
    });
}
#[test]
fn f5_actual_driver_guard_returns_completed_snapshot() {
    let f = Fixture::with_main(MAIN_NO_C_CALL, &[("a.js", BODY)]);
    for cause in [StopCause::Time, StopCause::Cancelled] {
        let guard = Guard::at(1, cause);
        f.run(limits(2, 8), &guard, |r| {
            let mut old = initial(r);
            pending(&mut old, &[("a.js", "f")]);
            let before = snapshot(&old);
            let out = super::super::super::driver::drive(
                r,
                old,
                AttemptCounters::default(),
                RoundStats::default(),
            );
            assert_eq!(out.stop, Stop::Guard(cause));
            assert_eq!(snapshot(&out.bundle), before);
            assert_eq!(out.stats.rounds_started, out.stats.rounds_completed + 1);
            assert_eq!(out.attempts.attempted_files, 1);
            assert!(out.attempts.attempted_read_bytes > 0);
            assert_eq!(out.bundle.published.files, 0);
            assert_eq!(out.bundle.published.bytes, 0);
        });
    }
}
#[test]
fn f5_real_terminal_matches_completed_dispositions() {
    let f = named(
        &[
            (
                "main.js",
                "import { h } from './a';\nexport function main(x) { h(x); }\n",
            ),
            (
                "a.js",
                "import { g } from './b';\nexport function h(x) { g(x); }\n",
            ),
            ("b.js", "export function g(x) {}\n"),
        ],
        "main.js",
        &[2],
    );
    for depth in [1, 2] {
        variant(
            &f,
            &TaintConfig::default(),
            limits(depth, 8),
            CrossingCeiling::NameOnly,
            &Guard::never(),
            |r| {
                assert_eq!(
                    first_plan(r).load_order,
                    ["a.js"],
                    "inadmissible precondition: first target"
                );
                let out = run_owned(r);
                assert!(out.bundle.context_files.contains("a.js"));
                let bounded = out
                    .bundle
                    .frontier
                    .dispositions
                    .iter()
                    .any(|(_, _, d)| *d == Disposition::DepthBound);
                if depth == 1 {
                    assert!(bounded);
                    assert_eq!(out.stop, Stop::DepthReached);
                } else {
                    assert!(out.bundle.context_files.contains("b.js"));
                    assert!(!bounded);
                    assert!(out.bundle.frontier.open.is_empty());
                    assert_eq!(out.stop, Stop::FrontierExhausted);
                }
            },
        );
    }
}
