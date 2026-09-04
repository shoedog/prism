use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

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
fn test_typescript_parameter_annotation_and_new_constructor_do_not_recover() {
    // P3: `m` must stay OVER the R6 fanout cap (4 owners: Foo/Other/Other2/
    // Other3) so this residue keeps testing what its name says — these
    // receiver shapes do not recover — rather than the P3 candidate path a
    // 2-owner pool would now hit instead.
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nclass Other2 { m() {} }\nclass Other3 { m() {} }\nfunction req(x: Foo) { x.m(); }\nfunction opt(x?: Foo) { x.m(); }\nfunction annotated() { const x: Foo = other(); x.m(); }\nfunction made() { const x = new Foo(); x.m(); }\n",
    );
    for caller in ["req", "opt", "annotated", "made"] {
        let s = site(&cg, caller, "m");
        assert_eq!(s.receiver_type, None, "{caller}");
        assert!(!s.receiver_materialized, "{caller}");
        assert!(cg.resolve_call_site(&s).is_empty(), "{caller}");
    }
}

#[test]
fn test_typescript_bare_factory_call_does_not_recover() {
    // P3: `m` must stay OVER the R6 fanout cap (4 owners) so this residue
    // keeps testing that a bare factory call does not recover, rather than
    // the P3 candidate path a 2-owner pool would now hit instead.
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nclass Other2 { m() {} }\nclass Other3 { m() {} }\nfunction factory() { const x = Foo(); x.m(); }\n",
    );
    let s = site(&cg, "factory", "m");
    assert_eq!(s.receiver_type, None);
    assert!(!s.receiver_materialized);
    assert!(cg.resolve_call_site(&s).is_empty());
}

#[test]
fn test_typescript_import_shadowing_param_suppresses_import_qualified() {
    let cg = graph_files(&[
        ("api.ts", "export function m() {}\n"),
        (
            "svc.ts",
            "import api from './api';\nclass Foo { m() {} }\nfunction run(api: Foo) { api.m(); }\nfunction ok(x: Foo) { x.m(); }\n",
        ),
    ]);

    let shadow = site(&cg, "run", "m");
    let shadow_out = cg.resolve_call_site(&shadow);
    assert_eq!(shadow.receiver_type, None);
    assert!(!shadow.receiver_materialized);
    assert!(shadow_out.iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));

    let ok = site(&cg, "ok", "m");
    let ok_out = cg.resolve_call_site(&ok);
    assert_eq!(ok.receiver_type, None);
    assert!(!ok.receiver_materialized);
    assert!(ok_out.iter().all(|c| {
        c.kind != ResolutionKind::TypedParam && c.kind != ResolutionKind::ConstructorLocal
    }));
}

#[test]
fn test_typescript_imported_type_param_suppresses_import_qualified() {
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
    assert!(!s.receiver_materialized);
    assert!(out.iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));
}

#[test]
fn test_typescript_nested_block_binding_does_not_suppress_import_qualified() {
    let cg = graph_files(&[
        ("api.ts", "export function m() {}\n"),
        (
            "svc.ts",
            "import api from './api';\nclass Foo { m() {} }\nfunction make(): Foo { return new Foo(); }\nfunction run() { { const api: Foo = make(); } api.m(); }\n",
        ),
    ]);

    let s = site(&cg, "run", "m");
    let out = cg.resolve_call_site(&s);
    assert_eq!(s.receiver_type, None);
    assert!(!s.receiver_materialized);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind, ResolutionKind::ImportQualified);
    assert_eq!(out[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out[0].target.file, "api.ts");
}

#[test]
fn test_typescript_lexical_receiver_bindings_respect_scope_and_hoisting() {
    let files: BTreeMap<_, _> = [
        ("api.ts", "export function m() {}\n"),
        (
            "svc.ts",
            "import api from './api';\n\
             class Foo { m() {} }\n\
             function make(): Foo { return new Foo(); }\n\
             function lexical() { const api: Foo = make(); api.m(); }\n\
             function lexicalAfter() { api.m(); let api: Foo; }\n\
             function destructured({ api }: { api: Foo }) { api.m(); }\n\
             function varNested() { { var api: Foo; } api.m(); }\n\
             function varAfter() { api.m(); var api: Foo; }\n\
             function caught() { try { throw 1; } catch (api) { api.m(); } }\n\
             function classBinding() { class api {} api.m(); }\n\
             function functionBinding() { function api() {} api.m(); }\n\
             function sibling() { { const api: Foo = make(); } { api.m(); } }\n\
             function catchEnded() { try { throw 1; } catch (api) {} api.m(); }\n\
             function nestedCallable() { function inner(api: Foo) { return api; } api.m(); }\n\
             function unrelated(other: Foo) { api.m(); }\n",
        ),
    ]
    .into_iter()
    .map(|(path, src)| {
        (
            path.to_string(),
            ParsedFile::parse(path, src, Language::TypeScript).expect("parse ts"),
        )
    })
    .collect();
    let cg = CallGraph::build(&files);

    for caller in [
        "lexical",
        "lexicalAfter",
        "destructured",
        "varNested",
        "varAfter",
        "caught",
        "classBinding",
        "functionBinding",
    ] {
        let call = site(&cg, caller, "m");
        assert!(
            cg.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::ImportQualified
                    || candidate.confidence != ResolutionConfidence::Exact
                    || candidate.target.file != "api.ts"
            }),
            "{caller} resolved through the shadowed module import"
        );
    }

    for caller in ["sibling", "catchEnded", "nestedCallable", "unrelated"] {
        let call = site(&cg, caller, "m");
        assert!(
            cg.resolve_call_site(&call).iter().any(|candidate| {
                candidate.kind == ResolutionKind::ImportQualified
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.target.file == "api.ts"
            }),
            "{caller} lost the visible module import"
        );
    }

    let only = BTreeSet::from(["api.ts".to_string(), "svc.ts".to_string()]);
    let subset = CallGraph::build_direct_subset(&files, &only);
    for caller in ["lexical", "varNested", "caught"] {
        let call = site(&subset, caller, "m");
        assert!(
            subset.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::ImportQualified
                    || candidate.confidence != ResolutionConfidence::Exact
                    || candidate.target.file != "api.ts"
            }),
            "direct subset disagreed for {caller}"
        );
    }
}
