use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolutionOutcome};
use std::collections::BTreeMap;

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(p, s)| {
            let lang = Language::from_path(p).expect("known extension");
            (
                (*p).to_string(),
                ParsedFile::parse(p, s, lang).expect("parse"),
            )
        })
        .collect()
}

fn resolve_self_call<'a>(
    cg: &'a CallGraph,
    caller_file: &str,
    caller_name: &str,
    callee: &str,
) -> ResolutionOutcome<'a> {
    let caller = cg
        .functions
        .get(caller_name)
        .and_then(|v| v.iter().find(|f| f.file == caller_file))
        .expect("caller fn");
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|s| s.callee_name == callee))
        .expect("call site");
    cg.resolve_call_site_full(site)
}

#[test]
fn javascript_same_class_this_call_resolves_exact() {
    let cg = CallGraph::build(&files(&[(
        "svc.js",
        "class Svc {\n  step() { return this.runOnce(); }\n  runOnce() { return 1; }\n}\n",
    )]));
    let out = resolve_self_call(&cg, "svc.js", "step", "runOnce");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}

#[test]
fn javascript_cross_file_collision_resolves_exact_to_caller_class() {
    let cg = CallGraph::build(&files(&[
        (
            "a.js",
            "class C {\n  m() { return 1; }\n  run() { return this.m(); }\n}\n",
        ),
        ("b.js", "class C {\n  m() { return 2; }\n}\n"),
    ]));
    let out = resolve_self_call(&cg, "a.js", "run", "m");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].target.file, "a.js");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}
