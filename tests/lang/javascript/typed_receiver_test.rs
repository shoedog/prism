use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ReceiverRecovery, ResolutionKind};
use std::collections::BTreeMap;

fn graph(src: &str) -> CallGraph {
    let files = BTreeMap::from([(
        "svc.js".to_string(),
        ParsedFile::parse("svc.js", src, Language::JavaScript).expect("parse js"),
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
fn test_javascript_new_constructor_recovers_bare_call_does_not() {
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nfunction made() { const x = new Foo(); x.m(); }\nfunction factory() { const x = Foo(); x.m(); }\n",
    );
    let made = site(&cg, "made", "m");
    assert_eq!(made.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(
        made.receiver_recovery,
        Some(ReceiverRecovery::ConstructorLocal)
    );
    let r = cg.resolve_call_site(&made);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, ResolutionKind::ConstructorLocal);

    let factory = site(&cg, "factory", "m");
    assert_eq!(factory.receiver_type, None);
    assert!(cg.resolve_call_site(&factory).is_empty());
}
