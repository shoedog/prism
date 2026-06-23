use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind, ResolutionOutcome};
use std::collections::BTreeMap;

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs
        .iter()
        .map(|(path, source)| {
            let lang = Language::from_path(path).expect("known extension");
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, lang).expect("parse"),
            )
        })
        .collect()
}

fn resolve_call<'a>(
    cg: &'a CallGraph,
    caller_file: &str,
    caller_name: &str,
    callee: &str,
) -> ResolutionOutcome<'a> {
    let caller = cg
        .functions
        .get(caller_name)
        .and_then(|ids| ids.iter().find(|id| id.file == caller_file))
        .expect("caller function");
    let site = cg
        .calls
        .get(caller)
        .and_then(|sites| sites.iter().find(|site| site.callee_name == callee))
        .expect("call site");
    cg.resolve_call_site_full(site)
}

#[test]
fn javascript_decorated_method_resolves_once() {
    let cg = CallGraph::build(&files(&[(
        "svc.js",
        "class Svc {\n  @memo\n  runOnce() { return 1; }\n  step() { return this.runOnce(); }\n}\n",
    )]));

    assert_eq!(
        cg.functions.get("runOnce").map(Vec::len),
        Some(1),
        "JS decorated method should have one function capture"
    );
    let out = resolve_call(&cg, "svc.js", "step", "runOnce");
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
}
