use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{DropReason, ReceiverRecovery, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn graph(srcs: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<_, _> = srcs
        .iter()
        .map(|(path, src)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, src, Language::Python).expect("parse python"),
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
fn test_python_typed_param_constructor_and_annotation_hit() {
    let cg = graph(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\nclass Other:\n    def m(self):\n        pass\ndef typed(x: Foo):\n    x.m()\ndef made():\n    x = Foo()\n    x.m()\ndef annotated():\n    x: Foo\n    x.m()\n",
    )]);
    for caller in ["typed", "made", "annotated"] {
        let s = site(&cg, caller, "m");
        assert_eq!(s.receiver_type.as_deref(), Some("Foo"), "{caller}");
        let r = cg.resolve_call_site(&s);
        assert_eq!(r.len(), 1, "{caller}");
        assert_eq!(r[0].target.start_line, 2, "{caller}");
        assert_eq!(r[0].confidence, ResolutionConfidence::Exact, "{caller}");
        if caller == "typed" {
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
fn test_python_shadow_import_wildcard_and_singleton_external_skip() {
    let shadow = graph(&[(
        "svc.py",
        "class Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x = other()\n    x.m()\n",
    )]);
    assert_eq!(site(&shadow, "run", "m").receiver_type, None);

    let imported = graph(&[(
        "svc.py",
        "from ext import Foo\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
    )]);
    let s = site(&imported, "run", "m");
    assert_eq!(s.receiver_type, None);
    assert!(imported.resolve_call_site(&s).iter().all(
        |c| c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
    ));

    for (path, src) in [
        (
            "before.py",
            "from ext import *\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
        ),
        (
            "after.py",
            "from ext import *\ndef run(x: Foo):\n    x.m()\nclass Foo:\n    def m(self):\n        pass\n",
        ),
    ] {
        let cg = graph(&[(path, src)]);
        let s = site(&cg, "run", "m");
        assert_eq!(s.receiver_type, None, "{path}");
        assert!(cg.resolve_call_site(&s).iter().all(|c| {
            c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
        }));
    }
}

#[test]
fn test_python_r3b_collision_and_local_miss_fallthrough() {
    let collision = graph(&[(
        "svc.py",
        "class x:\n    def m(self):\n        pass\nclass Foo:\n    def m(self):\n        pass\ndef run(x: Foo):\n    x.m()\n",
    )]);
    let s = site(&collision, "run", "m");
    let r = collision.resolve_call_site(&s);
    assert_eq!(s.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].target.start_line, 5);
    assert_eq!(r[0].kind, ResolutionKind::TypedParam);

    let miss = graph(&[(
        "miss.py",
        "class Foo:\n    pass\nclass Other:\n    def missing(self):\n        pass\ndef annotated(x: Foo):\n    x.missing()\ndef plain(x):\n    x.missing()\n",
    )]);
    let annotated = site(&miss, "annotated", "missing");
    let plain = site(&miss, "plain", "missing");
    let annotated_out = miss.resolve_call_site_full(&annotated);
    let plain_out = miss.resolve_call_site_full(&plain);
    assert_eq!(annotated.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(annotated_out.resolved, plain_out.resolved);
    assert_eq!(annotated_out.drop, plain_out.drop);
    assert_ne!(annotated_out.drop, Some(DropReason::ExternalReceiver));
}
