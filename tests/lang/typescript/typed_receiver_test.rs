use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ReceiverRecovery, ResolutionKind};
use std::collections::BTreeMap;

fn graph(src: &str) -> CallGraph {
    let files = BTreeMap::from([(
        "svc.ts".to_string(),
        ParsedFile::parse("svc.ts", src, Language::TypeScript).expect("parse ts"),
    )]);
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
