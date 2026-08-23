use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::ResolutionKind;
use std::collections::{BTreeMap, BTreeSet};

fn build_go(sources: &[(&str, &str)]) -> CallGraph {
    let files: BTreeMap<String, ParsedFile> = sources
        .iter()
        .map(|(path, source)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, source, Language::Go).expect("parse Go fixture"),
            )
        })
        .collect();
    CallGraph::build(&files)
}

fn fixture() -> CallGraph {
    build_go(&[
        (
            "p/types.go",
            "package p\n\
             type C interface{ M() }\n\
             type Wrong struct{}\n\
             func (*Wrong) M() {}\n\
             func retain() { _ = &Wrong{} }\n",
        ),
        (
            "q/types.go",
            "package q\n\
             type C struct{}\n\
             func (*C) M() {}\n\
             type D struct{}\n\
             func (*D) M() {}\n\
             func Reset(x *C) (*C, error) { return x, nil }\n\
             func Different() (*D, error) { return &D{}, nil }\n",
        ),
        (
            "app/use.go",
            "package app\n\
             import q \"example/q\"\n\
             func sameScope(x *q.C) {\n\
               x, err := q.Reset(x)\n\
               _ = err\n\
               x.M()\n\
             }\n\
             func differentType(x *q.C) {\n\
               x, err := q.Different()\n\
               _ = err\n\
               x.M()\n\
             }\n\
             func nestedShadow(x *q.C) {\n\
               {\n\
                 x := &q.C{}\n\
                 x.M()\n\
               }\n\
             }\n\
             func nestedReuse() {\n\
               {\n\
                 x := &q.C{}\n\
                 x, err := q.Reset(x)\n\
                 _ = err\n\
                 x.M()\n\
               }\n\
             }\n\
             func ifInitializerShadow(x *q.C) {\n\
               if x, err := q.Reset(x); err == nil {\n\
                 x.M()\n\
               }\n\
             }\n",
        ),
    ])
}

fn site<'a>(cg: &'a CallGraph, caller: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|site| site.caller.name == caller && site.callee_name == "M")
        .unwrap_or_else(|| panic!("missing {caller}->M"))
}

fn manifest_route(cg: &CallGraph, call: &CallSite) -> String {
    prism::navigation::queries::interface_dispatch_manifest(cg)["sites"]
        .as_array()
        .expect("manifest sites")
        .iter()
        .find(|entry| entry["file"] == call.caller.file && entry["line"] == call.line)
        .unwrap_or_else(|| panic!("missing manifest site {}:{}", call.caller.file, call.line))
        ["dispatch_route"]
        .as_str()
        .expect("dispatch route")
        .to_string()
}

fn resolved_files(cg: &CallGraph, call: &CallSite) -> BTreeSet<String> {
    cg.resolve_call_site_full(call)
        .resolved
        .iter()
        .map(|resolved| resolved.target.file.clone())
        .collect()
}

#[test]
fn same_scope_multi_name_short_declarations_reuse_the_receiver_binding() {
    let cg = fixture();
    for caller in ["sameScope", "nestedReuse"] {
        let call = site(&cg, caller);
        let outcome = cg.resolve_call_site_full(call);
        assert!(!call.receiver_local_type_shadowed, "{caller}: {call:?}");
        assert_eq!(
            resolved_files(&cg, call),
            BTreeSet::from(["q/types.go".into()]),
            "{caller}: {outcome:?}"
        );
        assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 1);
        assert_eq!(manifest_route(&cg, call), "concrete_direct");
    }
}

#[test]
fn changed_static_type_and_nested_shadow_still_fail_closed_to_r3() {
    let cg = fixture();
    for caller in ["differentType", "nestedShadow", "ifInitializerShadow"] {
        let call = site(&cg, caller);
        let outcome = cg.resolve_call_site_full(call);
        assert!(call.receiver_local_type_shadowed, "{caller}: {call:?}");
        assert_eq!(outcome.telemetry.go_concrete_receiver_direct, 0);
        assert!(
            outcome
                .resolved
                .iter()
                .all(|resolved| resolved.kind == ResolutionKind::InterfaceDispatch),
            "{caller}: {outcome:?}"
        );
        assert_eq!(manifest_route(&cg, call), "interface_dispatch");
    }
}
