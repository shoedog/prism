use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn graph(src: &str) -> CallGraph {
    graph_files(&[("svc.js", src)])
}

fn graph_files(srcs: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<_, _> = srcs
        .iter()
        .map(|(path, src)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, src, Language::JavaScript).expect("parse js"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn site(cg: &CallGraph, caller: &str, callee: &str) -> CallSite {
    cg.calls
        .iter()
        .find(|(fid, _)| fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
        .unwrap_or_else(|| panic!("missing {caller}->{callee}"))
        .clone()
}

#[test]
fn test_javascript_new_constructor_and_bare_call_do_not_recover() {
    // P3: `m` must stay OVER the R6 fanout cap (4 owners: Foo/Other/Other2/
    // Other3) so this residue keeps testing what its name says —
    // constructor-local recovery does not engage for JS `new`/bare calls —
    // rather than the P3 candidate path a 2-owner pool would now hit instead.
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nclass Other2 { m() {} }\nclass Other3 { m() {} }\nfunction made() { const x = new Foo(); x.m(); }\nfunction factory() { const x = Foo(); x.m(); }\n",
    );
    let made = site(&cg, "made", "m");
    assert_eq!(made.receiver_type, None);
    assert!(!made.receiver_materialized);
    assert!(cg.resolve_call_site(&made).is_empty());

    let factory = site(&cg, "factory", "m");
    assert_eq!(factory.receiver_type, None);
    assert!(!factory.receiver_materialized);
    assert!(cg.resolve_call_site(&factory).is_empty());
}

#[test]
fn test_javascript_nested_block_binding_does_not_suppress_import_qualified() {
    let cg = graph_files(&[
        ("api.js", "export function m() {}\n"),
        (
            "svc.js",
            "import api from './api';\nclass Foo { m() {} }\nfunction run() { { const api = new Foo(); } api.m(); }\n",
        ),
    ]);

    let s = site(&cg, "run", "m");
    let out = cg.resolve_call_site(&s);
    assert_eq!(s.receiver_type, None);
    assert!(!s.receiver_materialized);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, ResolutionKind::ImportQualified);
    assert_eq!(out[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out[0].target.file, "api.js");
}
