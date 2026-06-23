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
fn python_same_class_self_call_resolves_exact() {
    let cg = CallGraph::build(&files(&[(
        "svc.py",
        "class Svc:\n    def step(self):\n        return self.run_once()\n    def run_once(self):\n        return 1\n",
    )]));
    let out = resolve_self_call(&cg, "svc.py", "step", "run_once");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}

#[test]
fn merged_graph_still_narrows_self_calls() {
    let base = CallGraph::build(&files(&[(
        "b.py",
        "class C:\n    def m(self):\n        return 2\n",
    )]));
    let mut cg = base;
    let fresh = CallGraph::build(&files(&[(
        "a.py",
        "class C:\n    def m(self):\n        return 1\n    def run(self):\n        return self.m()\n",
    )]));
    cg.merge(fresh);
    let out = resolve_self_call(&cg, "a.py", "run", "m");
    assert_eq!(
        out.resolved.len(),
        1,
        "method_class_span survived merge -> still narrows"
    );
    assert_eq!(out.resolved[0].target.file, "a.py");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}
