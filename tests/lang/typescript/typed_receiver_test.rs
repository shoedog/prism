use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn graph(src: &str) -> CallGraph {
    graph_files(&[("svc.ts", src)])
}

fn graph_files(srcs: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<_, _> = srcs
        .iter()
        .map(|(path, src)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, src, Language::TypeScript).expect("parse ts"),
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
fn test_typescript_parameter_annotation_and_new_constructor_recover() {
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nfunction req(x: Foo) { x.m(); }\nfunction opt(x?: Foo) { x.m(); }\nfunction annotated() { const x: Foo = other(); x.m(); }\nfunction made() { const x = new Foo(); x.m(); }\n",
    );
    for caller in ["req", "opt", "annotated", "made"] {
        let s = site(&cg, caller, "m");
        assert_eq!(s.receiver_type.as_deref(), Some("Foo"), "{caller}");
        let r = cg.resolve_call_site(&s);
        assert_eq!(r.len(), 1, "{caller}");
        if matches!(caller, "req" | "opt") {
            assert_eq!(s.receiver_recovery, Some(ReceiverRecovery::TypedParam));
            assert_eq!(r[0].kind, ResolutionKind::TypedParam);
        } else {
            assert_eq!(
                s.receiver_recovery,
                Some(ReceiverRecovery::ConstructorLocal)
            );
            assert_eq!(r[0].kind, ResolutionKind::ConstructorLocal);
        }
    }
}

#[test]
fn test_typescript_bare_factory_call_does_not_recover() {
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nfunction factory() { const x = Foo(); x.m(); }\n",
    );
    let s = site(&cg, "factory", "m");
    assert_eq!(s.receiver_type, None);
    assert!(cg.resolve_call_site(&s).is_empty());
}

#[test]
fn test_typescript_import_shadowing_materialized_param_suppresses_import_qualified() {
    let cg = graph_files(&[
        ("api.ts", "export function m() {}\n"),
        (
            "svc.ts",
            "import api from './api';\nclass Foo { m() {} }\nfunction run(api: Foo) { api.m(); }\nfunction ok(x: Foo) { x.m(); }\n",
        ),
    ]);

    let shadow = site(&cg, "run", "m");
    let shadow_out = cg.resolve_call_site(&shadow);
    assert_eq!(shadow.receiver_type.as_deref(), Some("Foo"));
    assert!(shadow.receiver_materialized);
    assert_eq!(shadow_out.len(), 1);
    assert_eq!(shadow_out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(shadow_out[0].confidence, ResolutionConfidence::Exact);
    assert_ne!(shadow_out[0].target.file, "api.ts");

    let ok = site(&cg, "ok", "m");
    let ok_out = cg.resolve_call_site(&ok);
    assert_eq!(ok.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(ok_out.len(), 1);
    assert_eq!(ok_out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(ok_out[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn test_typescript_poisoned_type_materialized_param_suppresses_import_qualified() {
    let cg = graph_files(&[
        ("api.ts", "export function m() {}\n"),
        ("types.ts", "export class Foo {}\n"),
        (
            "svc.ts",
            "import api from './api';\nimport { Foo } from './types';\nfunction run(api: Foo) { api.m(); }\n",
        ),
    ]);

    let s = site(&cg, "run", "m");
    let out = cg.resolve_call_site(&s);
    assert_eq!(s.receiver_type, None);
    assert!(s.receiver_materialized);
    assert!(out.iter().all(|c| {
        c.kind != ResolutionKind::ImportQualified
            || c.confidence != ResolutionConfidence::Exact
            || c.target.file != "api.ts"
    }));
}
