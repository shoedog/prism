use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

fn graph(src: &str) -> CallGraph {
    graph_files(&[("svc.js", src)])
}

#[test]
fn receiver_authority_regression_matrix() {
    let mut failures = Vec::new();
    for (label, body) in [
        ("outer parameter", "function outer(Foo) { function run() { const x = new Foo(); x.m(); } }"),
        ("outer block", "function outer() { const Foo = Other; function run() { const x = new Foo(); x.m(); } }"),
        ("class expression self", "const Holder = class Foo { run() { const x = new Foo(); x.m(); } };"),
        ("conditional var", "function run(flag) { if (flag) { var x = new Foo(); } x.m(); }"),
        ("iteration assignment", "function run(items) { let x = new Foo(); for (x of items) { x.m(); } }"),
        ("loop carried write", "function run(items) { let x = new Foo(); for (const item of items) { x.m(); x = item; } }"),
    ] {
        let cg = graph(&format!("class Foo {{ m() {{}} }}\nclass Other {{ m() {{}} }}\n{body}"));
        let call = site(&cg, "run", "m");
        let resolved = cg.resolve_call_site(&call);
        if call.receiver_type.is_some() || resolved.iter().any(|r| r.confidence == ResolutionConfidence::Exact) {
            failures.push(format!("{label}: type={:?}, edges={resolved:?}", call.receiver_type));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn receiver_authority_preserves_reaching_origins() {
    for body in [
        "function run() { const x = new Foo(); x.m(); }",
        "function outer() { function run() { const x = new Foo(); x.m(); } }",
        "function outer() { { const Foo = Other; } function run() { const x = new Foo(); x.m(); } }",
        "function run(flag) { if (flag) { const x = new Foo(); x.m(); } }",
        "function run() { { var x = new Foo(); } x.m(); }",
        "function run(items) { const x = new Foo(); for (const item of items) { x.m(); } }",
        "function run(items) { for (const item of items) { let x = new Foo(); x.m(); x = item; } }",
        "function run(items) { const x = new Foo(); for (let x of items) {} x.m(); }",
    ] {
        for (language, file) in [(Language::JavaScript, "svc.js"), (Language::TypeScript, "svc.ts"), (Language::Tsx, "svc.tsx")] {
            let src = format!("class Foo {{ m() {{}} }}\nclass Other {{ m() {{}} }}\n{body}");
            let files = BTreeMap::from([(file.to_string(), ParsedFile::parse(file, &src, language).unwrap())]);
            let cg = CallGraph::build(&files);
            let call = site(&cg, "run", "m");
            let edges = cg.resolve_call_site(&call);
            assert_eq!(edges.len(), 1, "{language:?} {body}: {edges:?}");
            assert_eq!(edges[0].kind, ResolutionKind::ConstructorLocal, "{language:?} {body}");
            assert_eq!(edges[0].confidence, ResolutionConfidence::Exact);
        }
    }
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
fn test_javascript_new_constructor_recovers_but_bare_call_does_not() {
    // P3: `m` must stay OVER the R6 fanout cap (4 owners: Foo/Other/Other2/
    // Other3) so the factory residue cannot resolve through the bounded
    // multi-owner candidate path.
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nclass Other2 { m() {} }\nclass Other3 { m() {} }\nfunction made() { const x = new Foo(); x.m(); }\nfunction factory() { const x = Foo(); x.m(); }\n",
    );
    let made = site(&cg, "made", "m");
    assert!(made.receiver_lexically_bound);
    assert_eq!(made.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(
        made.receiver_recovery,
        Some(ReceiverRecovery::ConstructorLocal)
    );
    assert!(made.receiver_materialized);
    let made_out = cg.resolve_call_site(&made);
    assert_eq!(made_out.len(), 1);
    assert_eq!(made_out[0].target.file, "svc.js");
    assert_eq!(made_out[0].target.name, "m");
    assert_eq!(made_out[0].kind, ResolutionKind::ConstructorLocal);
    assert_eq!(made_out[0].confidence, ResolutionConfidence::Exact);

    let factory = site(&cg, "factory", "m");
    assert!(factory.receiver_lexically_bound);
    assert_eq!(factory.receiver_type, None);
    assert!(factory.receiver_materialized);
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
    assert!(!s.receiver_lexically_bound);
    assert_eq!(s.receiver_type, None);
    assert!(!s.receiver_materialized);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, ResolutionKind::ImportQualified);
    assert_eq!(out[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out[0].target.file, "api.js");
}

#[test]
fn test_javascript_lexical_receiver_bindings_suppress_import_qualified() {
    let cg = graph_files(&[
        ("api.js", "export function m() {}\n"),
        (
            "svc.js",
            "import api from './api';\n\
             class Foo { m() {} }\n\
             function param(api) { api.m(); }\n\
             function destructured({ api }) { api.m(); }\n\
             function lexical() { const api = new Foo(); api.m(); }\n\
             function varNested() { { var api; } api.m(); }\n\
             function sibling() { { const api = new Foo(); } { api.m(); } }\n\
             function nestedCallable() { function inner(api) { return api; } api.m(); }\n\
             function unrelated(other) { api.m(); }\n",
        ),
    ]);

    for caller in ["param", "destructured", "lexical", "varNested"] {
        let call = site(&cg, caller, "m");
        assert!(call.receiver_lexically_bound, "{caller}");
        assert!(
            cg.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::ImportQualified
                    || candidate.confidence != ResolutionConfidence::Exact
                    || candidate.target.file != "api.js"
            }),
            "{caller} resolved through the shadowed module import"
        );
    }

    for caller in ["sibling", "nestedCallable", "unrelated"] {
        let call = site(&cg, caller, "m");
        assert!(!call.receiver_lexically_bound, "{caller}");
        assert!(
            cg.resolve_call_site(&call).iter().any(|candidate| {
                candidate.kind == ResolutionKind::ImportQualified
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.target.file == "api.js"
            }),
            "{caller} lost the visible module import"
        );
    }
}

#[test]
fn test_javascript_receiver_binding_includes_enclosing_parameter() {
    let cg = graph_files(&[
        ("api.js", "export function m() {}\n"),
        (
            "svc.js",
            "import api from './api';\nfunction outer(api) { function inner() { api.m(); } }\n",
        ),
    ]);
    let call = site(&cg, "inner", "m");
    assert!(call.receiver_lexically_bound);
    assert!(cg.resolve_call_site(&call).iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.js"
    }));
}

#[test]
fn test_javascript_new_recovery_respects_scope_mutation_and_static_owner() {
    let cg = graph(
        "class Foo { m() {} static s() {} }\n\
         class Other { m() {} }\n\
         class Other2 { m() {} }\n\
         class Other3 { m() {} }\n\
         function letMade() { let x = new Foo(); x.m(); }\n\
         function varMade() { var x = new Foo(); x.m(); }\n\
         function prewrite() { x = other(); { const x = new Foo(); x.m(); } }\n\
         function ended() { { const x = other(); } const x = new Foo(); x.m(); }\n\
         function reassigned() { let x = new Foo(); x = other(); x.m(); }\n\
         function after() { x.m(); const x = new Foo(); }\n\
         function captured() { const x = new Foo(); function inner() { x.m(); } }\n\
         function active() { const x = new Foo(); { const x = other(); x.m(); } }\n\
         function qualified() { const x = new ns.Foo(); x.m(); }\n\
         function shadowedCtor() { const Foo = Other; const x = new Foo(); x.m(); }\n\
         function capturedWrite() { const x = new Foo(); function mutate() { x = other(); } x.m(); }\n\
         function shadowWrite() { const x = new Foo(); { let x; x = other(); } x.m(); }\n\
         function staticOwner() { Foo.s(); }\n",
    );

    for caller in ["shadowWrite", "letMade", "varMade", "prewrite", "ended"] {
        let call = site(&cg, caller, "m");
        assert_eq!(call.receiver_type.as_deref(), Some("Foo"), "{caller}");
        assert_eq!(
            call.receiver_recovery,
            Some(ReceiverRecovery::ConstructorLocal),
            "{caller}"
        );
        assert!(call.receiver_materialized, "{caller}");
        let out = cg.resolve_call_site(&call);
        assert_eq!(out.len(), 1, "{caller}: {out:?}");
        assert_eq!(out[0].kind, ResolutionKind::ConstructorLocal, "{caller}");
        assert_eq!(out[0].confidence, ResolutionConfidence::Exact, "{caller}");
        assert_eq!(out[0].target.file, "svc.js", "{caller}");
    }

    for caller in [
        "reassigned",
        "after",
        "inner",
        "active",
        "qualified",
        "shadowedCtor",
        "capturedWrite",
    ] {
        let call = site(&cg, caller, "m");
        assert_eq!(call.receiver_type, None, "{caller}");
        assert!(call.receiver_materialized, "{caller}");
        assert!(
            cg.resolve_call_site(&call).is_empty(),
            "{caller} minted an unsupported recovered edge"
        );
    }

    let static_owner = site(&cg, "staticOwner", "s");
    assert_eq!(static_owner.receiver_type, None);
    assert!(!static_owner.receiver_materialized);
    let out = cg.resolve_call_site(&static_owner);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].kind, ResolutionKind::QualifierOwner);
    assert_eq!(out[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn test_javascript_shadowed_constructor_owner_fails_closed() {
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nfunction run() { const Foo = Other; const x = new Foo(); x.m(); }\n",
    );
    let call = site(&cg, "run", "m");
    assert_eq!(call.receiver_type, None);
    assert!(call.receiver_materialized);
    assert!(cg
        .resolve_call_site(&call)
        .iter()
        .all(|candidate| { candidate.kind != ResolutionKind::ConstructorLocal }));
}

#[test]
fn test_javascript_named_function_self_does_not_resolve_same_named_class() {
    let cg = graph("class Foo { static m() {} }\nconst holder = function Foo() { Foo.m(); };\n");
    let call = site(&cg, "Foo", "m");
    assert_eq!(call.receiver_type, None);
    assert!(call.receiver_materialized);
    assert!(cg
        .resolve_call_site(&call)
        .iter()
        .all(|candidate| { candidate.kind != ResolutionKind::QualifierOwner }));
}
