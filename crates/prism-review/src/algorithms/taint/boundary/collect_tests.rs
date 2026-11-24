use super::super::recovery::RecoveryContext;
use super::*;
use crate::algorithms::taint::{self, TaintConfig};
use prism::{
    access_path::AccessPath,
    api::{ReviewHintBasis, ReviewUnresolvedReason},
};
#[path = "collect_tests/fixtures.rs"]
mod fixtures;
use fixtures::*;
#[path = "collect_tests/policy.rs"]
mod policy;

#[test]
fn actual_imports_use_the_same_run_for_all_five_languages() {
    let mut observed = Vec::new();
    for (path, source, line, target, body, name) in [
        ("a.js", "import { send } from './transport';\nfunction h(req) {\n send(req);\n}", 3, "transport.js", "export function send(x) {}", "send"),
        ("a.ts", "import { send } from './transport';\nfunction h(req: string) {\n send(req);\n}", 3, "transport.ts", "export function send(x: string) {}", "send"),
        ("a.tsx", "import { send } from './transport';\nfunction h(req: string) {\n send(req);\n return <div/>;\n}", 3, "transport.ts", "export function send(x: string) {}", "send"),
        ("a.py", "from transport import send\ndef h(req):\n    send(req)\n", 3, "transport.py", "def send(x):\n    pass\n", "send"),
        ("main.go", "package main\nimport \"example.com/app/transport\"\nfunc h(req string) {\n transport.Send(req)\n}", 4, "transport/out.go", "package transport\nfunc Send(x string) {}", "Send"),
    ] {
        for loaded in [false, true] {
            let files = [(path, source), (target, body), ("go.mod", "module example.com/app\n")];
            let loaded_files = if loaded { vec![path, target] } else { vec![path] };
            let f = Fixture::new(&files, &loaded_files, &[(path, line)]);
            let ctx = f.context();
            let (result, trace) = f.trace(&ctx, &TaintConfig::default());
            let call = site(&ctx, name);
            let qualified = root_match(&ctx, &trace, &call);
            let before = (serde_json::to_value(&result).unwrap(), format!("{:?}", result.evidence));
            let collection = checked(&f, &ctx, &trace);
            assert_eq!(before, (serde_json::to_value(&result).unwrap(), format!("{:?}", result.evidence)));
            eprintln!("LANG {path} loaded={loaded} qualified={qualified} records={:?}", collection.records);
            let target_kept = collection.records.first().is_some_and(|r| matches!(&r.hint, ReviewImportHint::InRepo { files, .. } if files == &[target]));
            observed.push((qualified, collection.availability == Availability::Available, collection.records.len(), target_kept));
        }
    }
    assert!(
        observed.iter().all(|r| r.0),
        "inadmissible actual-root precondition: {observed:?}"
    );
    assert_eq!(
        observed,
        [(true, true, 1, true), (true, true, 0, false)].repeat(5)
    );
}

#[test]
fn six_real_express_sources_keep_the_specific_full_target() {
    let mut observed = Vec::new();
    for (body, named, target) in [
        (" send(req.body); // SOURCE", false, "req"),
        (" send(req.body); // SOURCE", true, "req"),
        (" send(req.query.command); // SOURCE", false, "req"),
        (" send(req.query.command); // SOURCE", true, "req"),
        (
            " req.body = req.query.command; // SOURCE\n send(req.body);",
            false,
            "req.body",
        ),
        (
            " req.query.command = req.body; // SOURCE\n send(req.query.command);",
            false,
            "req.query.command",
        ),
    ] {
        let header = "import { send } from './transport.js';\nconst express = require('express');\nconst app = express();\n";
        let source = if named {
            format!("{header}function handler(req,res) {{\n{body}\n}}\napp.post('/run',handler);")
        } else {
            format!("{header}app.post('/run',(req,res)=>{{\n{body}\n}});")
        };
        let line = source.lines().position(|l| l.contains("SOURCE")).unwrap() + 1;
        let f = Fixture::new(
            &[
                ("a.js", &source),
                ("transport.js", "export function send(x) {}"),
            ],
            &["a.js"],
            &[("a.js", line)],
        );
        let ctx = f.context();
        let (_, mut trace) = f.trace(
            &ctx,
            &TaintConfig {
                taint_from_diff: false,
                ..Default::default()
            },
        );
        let call = site(&ctx, "send");
        trace.roots.retain(|r| {
            r.seed.target.as_ref() == Some(&AccessPath::from_expr(target))
                && r.reached.iter().any(|v| {
                    v.location.line == call.line && v.location.start_byte == v.location.end_byte
                })
        });
        let recovery = RecoveryContext::new(&ctx.files["a.js"]);
        let qualified = trace.roots.iter().any(|r| {
            let prepared = recovery.prepare(r);
            r.location.path == AccessPath::from_expr(target)
                && (0..r.reached.len()).any(|i| {
                    prepared.value(i).is_some_and(|v| {
                        v.path == AccessPath::simple("req") && recovery.matches_argument(&call, v)
                    })
                })
        });
        let count = checked(&f, &ctx, &trace).records.len();
        eprintln!("EXPRESS named={named} target={target} qualified={qualified} count={count}");
        observed.push((qualified, count));
    }
    assert!(
        observed.iter().all(|r| r.0),
        "inadmissible specific source: {observed:?}"
    );
    assert_eq!(observed, [(true, 1); 6]);
}

#[test]
fn isolated_roles_and_adjacent_calls_keep_exact_bytes() {
    let mut observed = Vec::new();
    for (body, expected) in [
        ("send(req);", 1),
        ("send({req});", 1),
        ("send({req:'safe'});", 0),
        ("send({[req]:'safe'});", 0),
        ("send(req); send('safe');", 1),
    ] {
        for explicit in [false, true] {
            let f = js(body);
            let ctx = f.context();
            let config = TaintConfig {
                sources: if explicit {
                    vec![("a.js".into(), 3)]
                } else {
                    vec![]
                },
                taint_from_diff: !explicit,
                ..Default::default()
            };
            let (_, trace) = f.trace(&ctx, &config);
            let actual_use = trace.roots.iter().any(|r| {
                r.location.path == AccessPath::simple("req")
                    && r.location.start_byte < r.location.end_byte
            });
            let records = checked(&f, &ctx, &trace).records;
            let first = site(&ctx, "send");
            let bytes_correct = records.iter().all(|r| {
                r.site.start_byte == first.start_byte && r.site.end_byte == first.end_byte
            });
            eprintln!(
                "ROLE {body} explicit={explicit} actual_use={actual_use} records={records:?}"
            );
            observed.push((actual_use, records.len() == expected && bytes_correct));
        }
    }
    assert!(
        observed.iter().all(|r| r.0),
        "inadmissible role root: {observed:?}"
    );
    assert_eq!(observed, [(true, true); 10]);
}

#[test]
fn go_declaring_sets_remain_distinct_from_package_candidates() {
    let mut observed = Vec::new();
    for (a, b, wanted) in [("Other", "Else", 0), ("Run", "Else", 1), ("Run", "Run", 2)] {
        let a = format!("package util\nfunc {a}(x string) {{}}");
        let b = format!("package util\nfunc {b}(x string) {{}}");
        let f = Fixture::new(&[("go.mod", "module example.com/app\n"), ("main.go", "package main\nimport \"example.com/app/util\"\nfunc h(x string) { util.Run(x) }"), ("util/a.go", &a), ("util/b.go", &b)], &["main.go"], &[("main.go", 3)]);
        let ctx = f.context();
        let (_, trace) = f.trace(&ctx, &TaintConfig::default());
        let qualified = root_match(&ctx, &trace, &site(&ctx, "Run"));
        let records = checked(&f, &ctx, &trace).records;
        let valid = records.first().is_some_and(|r| {
            r.kind == if wanted == 0 { BoundaryKind::GoDeclarationUnknown } else { BoundaryKind::InRepo }
                && matches!(&r.hint, ReviewImportHint::InRepo { files, declaring, .. } if files == &["util/a.go", "util/b.go"] && declaring.len() == wanted)
        });
        observed.push((qualified, valid));
    }
    assert!(
        observed.iter().all(|r| r.0),
        "inadmissible Go root: {observed:?}"
    );
    assert_eq!(observed, [(true, true); 3]);
}

#[test]
fn unresolved_is_visible_but_external_and_unknown_are_silent() {
    let f = Fixture::new(&[("a.js", "import { missing } from './lost.js';\nimport { external } from 'third-party';\nfunction h(req) { missing(req); external(req); mystery(req); }")], &["a.js"], &[("a.js", 3)]);
    let ctx = f.context();
    let (_, trace) = f.trace(&ctx, &TaintConfig::default());
    assert!(["missing", "external", "mystery"]
        .iter()
        .all(|n| root_match(&ctx, &trace, &site(&ctx, n))));
    let records = checked(&f, &ctx, &trace).records;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, BoundaryKind::Unresolved);
    assert_eq!(
        records[0].hint,
        ReviewImportHint::Unresolvable {
            spec: "./lost.js".into(),
            why: ReviewUnresolvedReason::NoFile
        }
    );
}
