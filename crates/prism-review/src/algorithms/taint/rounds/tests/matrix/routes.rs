use super::super::super::super::source_scope::admission::{slice_in_context, ExclusionReason};
use super::super::super::super::TaintRootOrigin;
use super::*;
use prism::api::{ContextReadOutcome, ReviewContextRound};
use prism::cpg::TaintCrossingPolicy;

#[test]
fn m01_complete_frontier_beyond_ten() {
    let mut main = String::new();
    let mut files: Vec<(String, String)> = Vec::new();
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
    let refs: Vec<_> = files
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect();
    let f = named(&refs, "main.js", &[13]);
    variant(
        &f,
        &TaintConfig::default(),
        limits(1, 16),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            let plan = first_plan(r);
            assert_eq!(
                plan.provenance.len(),
                12,
                "inadmissible precondition: 12 physical eligible sites"
            );
            assert_eq!(
                plan.load_order.len(),
                12,
                "inadmissible precondition: 12 unique targets"
            );
            let result = run_owned(r);
            assert_eq!(result.bundle.context_files.len(), 12);
            assert_eq!(result.attempts.attempted_files, 12);
            assert_eq!(result.bundle.frontier.provenance.len(), 12);
        },
    );
}
#[test]
fn m02_loaded_unresolved_never_becomes_a_finding() {
    let f = named(
        &[
            (
                "main.js",
                "import { h } from './a';\nexport function main(x) { h(x); }\n",
            ),
            ("a.js", "export const other = 1;\n"),
        ],
        "main.js",
        &[2],
    );
    f.run(limits(2, 8), &Guard::never(), |r| {
        assert_eq!(
            first_plan(r).load_order,
            ["a.js"],
            "inadmissible precondition: producer target"
        );
        let result = run_owned(r);
        assert!(result.bundle.context_files.contains("a.js"));
        assert!(result
            .bundle
            .frontier
            .dispositions
            .iter()
            .any(|(_, p, d)| p == "a.js" && *d == Disposition::LoadedUnresolved));
        assert!(result
            .bundle
            .analysis
            .as_ref()
            .unwrap()
            .result
            .findings
            .iter()
            .all(|finding| finding.file != "a.js"));
    });
}
#[test]
fn m03_explicit_mode_suppresses_diff_and_context_roots() {
    let f=named(&[("main.js","import { f } from './a';\nimport { g } from './b';\nexport function main(x,y) {\n f(x);\n g(y);\n}\n"),("a.js","export function f(x) { eval(x); }\n"),("b.js","export const other = 1;\n")],"main.js",&[5]);
    let cfg = TaintConfig {
        sources: vec![("main.js".into(), 4), ("a.js".into(), 1)],
        taint_from_diff: false,
        ..Default::default()
    };
    variant(
        &f,
        &cfg,
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            assert!(
                r.baseline_trace
                    .roots
                    .iter()
                    .any(|root| root.origin == TaintRootOrigin::Explicit
                        && root.seed.file == "main.js"),
                "inadmissible precondition: explicit original root"
            );
            assert!(
                r.baseline_trace
                    .roots
                    .iter()
                    .all(|root| root.origin != TaintRootOrigin::Diff),
                "inadmissible precondition: diff disabled"
            );
            assert_eq!(
                first_plan(r).load_order,
                ["a.js"],
                "inadmissible precondition: explicit target only"
            );
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("a.js"));
            assert!(!out.bundle.context_files.contains("b.js"));
            let a = out.bundle.analysis.as_ref().unwrap();
            assert!(a
                .excluded_explicit
                .iter()
                .any(|x| x.file == "a.js" && x.reason == ExclusionReason::OutsideOriginalFiles));
            assert!(a.trace.roots.iter().all(|root| root.seed.file != "a.js"));
        },
    );
}
#[test]
fn m04_framework_only_cannot_load_but_can_find_after_diff_admission() {
    let main = "from a import helper\ndef lookup_view(request):\n    q = request.GET.get('q')\n    helper(q)\n";
    let f = named(
        &[
            ("main.py", main),
            ("a.py", "import os\ndef helper(p):\n    os.system(p)\n"),
        ],
        "main.py",
        &[4],
    );
    let only = TaintConfig {
        taint_from_diff: false,
        ..Default::default()
    };
    variant(
        &f,
        &only,
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            assert!(
                r.baseline_trace
                    .roots
                    .iter()
                    .any(|x| x.origin == TaintRootOrigin::Framework),
                "inadmissible precondition: framework producer root"
            );
            let original = r.inputs.files.keys().cloned().collect();
            assert!(
                project_loading_trace(r.baseline_trace, &original, &r.inputs.files)
                    .roots
                    .is_empty(),
                "inadmissible precondition: no framework loading roots"
            );
            let mut files = r.inputs.files.clone();
            match r.loader.read("a.py", 1 << 20).outcome {
                ContextReadOutcome::Loaded(x) => {
                    files.insert(x.path, x.parsed);
                }
                _ => panic!("inadmissible precondition: context read"),
            };
            let round = ReviewContextRound::new(r.loader, &files);
            let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
            let context = ["a.py".to_string()].into_iter().collect();
            let policy = TaintCrossingPolicy::new(&context, r.ceiling);
            let observed =
                slice_in_context(&ctx, &r.inputs.diff, r.config, &original, &policy).unwrap();
            assert!(
                observed
                    .result
                    .findings
                    .iter()
                    .any(|x| x.file == "a.py" && x.category.as_deref() == Some("taint_sink")),
                "inadmissible precondition: real framework-only union sink"
            );
            assert!(
                observed
                    .trace
                    .roots
                    .iter()
                    .any(|root| root.origin == TaintRootOrigin::Framework
                        && root.reached.iter().any(|v| v.location.file == "a.py")),
                "inadmissible precondition: framework trace reaches context"
            );
            let off = run_owned(r);
            assert_eq!(off.attempts.attempted_files, 0);
            assert!(off.bundle.context_files.is_empty());
        },
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
                ["a.py"],
                "inadmissible precondition: diff target"
            );
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("a.py"));
            assert!(!sinks(&out, "a.py").is_empty());
            assert!(out
                .bundle
                .analysis
                .as_ref()
                .unwrap()
                .trace
                .roots
                .iter()
                .any(|root| root.origin == TaintRootOrigin::Framework
                    && root.reached.iter().any(|v| v.location.file == "a.py")));
        },
    );
}
#[test]
fn m05_context_sources_do_not_start_roots() {
    let main = "import { f } from './a';\nexport function main(x) { f(x); }\n";
    let context="const express = require('express');\nconst app = express();\napp.get('/', (req,res) => { const q = req.query.q; eval(q); });\nexport function f(x) {}\n";
    let f = named(&[("main.js", main), ("a.js", context)], "main.js", &[2]);
    let cfg = TaintConfig {
        sources: vec![("a.js".into(), 3)],
        ..Default::default()
    };
    variant(
        &f,
        &cfg,
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            assert_eq!(
                first_plan(r).load_order,
                ["a.js"],
                "inadmissible precondition: eligible original target"
            );
            let mut files = r.inputs.files.clone();
            match r.loader.read("a.js", 1 << 20).outcome {
                ContextReadOutcome::Loaded(x) => {
                    files.insert(x.path, x.parsed);
                }
                _ => panic!("inadmissible precondition: context read"),
            };
            let round = ReviewContextRound::new(r.loader, &files);
            let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
            let (_, all) =
                super::super::super::super::slice_with_trace(&ctx, &r.inputs.diff, r.config)
                    .unwrap();
            assert!(
                all.roots
                    .iter()
                    .any(|x| x.seed.file == "a.js" && x.origin == TaintRootOrigin::Framework),
                "inadmissible precondition: real context framework root"
            );
            assert!(
                all.roots
                    .iter()
                    .any(|x| x.seed.file == "a.js" && x.origin == TaintRootOrigin::Explicit),
                "inadmissible precondition: real context explicit root"
            );
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("a.js"));
            let analysis = out.bundle.analysis.as_ref().unwrap();
            assert!(analysis
                .trace
                .roots
                .iter()
                .all(|root| root.seed.file != "a.js"));
            assert!(analysis
                .excluded_explicit
                .iter()
                .any(|x| x.file == "a.js" && x.reason == ExclusionReason::OutsideOriginalFiles));
            assert!(analysis
                .result
                .findings
                .iter()
                .all(|finding| finding.file != "a.js"));
        },
    );
}

#[test]
fn m05_inline_context_source_sink_is_not_independent() {
    let main = "import { f } from './a';\nexport function main(x) { f(x); }\n";
    let context="import express from 'express';\nimport { load } from 'js-yaml';\nconst app = express();\napp.post('/yaml', (req,res) => { return load(req.body.payload); });\nexport function f(x) {}\n";
    let f = named(&[("main.js", main), ("a.js", context)], "main.js", &[2]);
    variant(
        &f,
        &TaintConfig::default(),
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            assert_eq!(
                first_plan(r).load_order,
                ["a.js"],
                "inadmissible precondition: legitimate context target"
            );
            let mut files = r.inputs.files.clone();
            match r.loader.read("a.js", 1 << 20).outcome {
                ContextReadOutcome::Loaded(x) => {
                    files.insert(x.path, x.parsed);
                }
                _ => panic!("inadmissible precondition: context read"),
            };
            let round = ReviewContextRound::new(r.loader, &files);
            let ctx = round.build_context(r.inputs.type_db.as_ref(), &r.baseline_ctx.types);
            let (all, _) =
                super::super::super::super::slice_with_trace(&ctx, &r.inputs.diff, r.config)
                    .unwrap();
            assert!(
                all.findings
                    .iter()
                    .any(|x| x.file == "a.js" && x.category.as_deref() == Some("taint_sink")),
                "inadmissible precondition: producer recognizes context inline source/sink"
            );
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("a.js"));
            assert!(out
                .bundle
                .analysis
                .as_ref()
                .unwrap()
                .result
                .findings
                .iter()
                .all(|x| x.file != "a.js"));
        },
    );
}

#[test]
fn m02_renamed_barrel_continuation_preserves_member() {
    let f = named(
        &[
            (
                "main.js",
                "import { h } from './a';\nexport function main(x) { h(x); }\n",
            ),
            ("a.js", "export { g as h } from './b';\n"),
            ("b.js", "export function g(x) {}\n"),
        ],
        "main.js",
        &[2],
    );
    variant(
        &f,
        &TaintConfig::default(),
        limits(2, 8),
        CrossingCeiling::NameOnly,
        &Guard::never(),
        |r| {
            let plan = first_plan(r);
            assert_eq!(
                plan.load_order,
                ["a.js"],
                "inadmissible precondition: barrel target"
            );
            let mut files = r.inputs.files.clone();
            match r.loader.read("a.js", 1 << 20).outcome {
                ContextReadOutcome::Loaded(x) => {
                    files.insert(x.path, x.parsed);
                }
                _ => panic!("inadmissible precondition: barrel parse"),
            };
            let round = ReviewContextRound::new(r.loader, &files);
            let mut hints = round.hints().unwrap();
            let hop = hints
                .follow_barrel("a.js", "h")
                .expect("inadmissible precondition: named export forwarding");
            assert_eq!(
                (hop.file.as_str(), hop.member.as_str()),
                ("b.js", "g"),
                "inadmissible precondition: renamed hop"
            );
            let mut zero = initial(r);
            zero.frontier = plan;
            let first = run_stage(r, &zero, &mut AttemptCounters::default())
                .unwrap()
                .bundle;
            assert!(first
                .frontier
                .open
                .iter()
                .any(|request| request.target == "b.js" && request.member.as_deref() == Some("g")));
            let out = run_owned(r);
            assert!(out.bundle.context_files.contains("b.js"));
        },
    );
}
