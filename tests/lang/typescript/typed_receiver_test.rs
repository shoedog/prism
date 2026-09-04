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
        assert!(s.receiver_lexically_bound, "{caller}");
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
    assert!(s.receiver_lexically_bound);
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
    assert!(shadow.receiver_lexically_bound);
    assert_eq!(shadow.receiver_type, None);
    assert!(!shadow.receiver_materialized);
    assert!(shadow_out.iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));

    let ok = site(&cg, "ok", "m");
    let ok_out = cg.resolve_call_site(&ok);
    assert!(ok.receiver_lexically_bound);
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
    assert!(s.receiver_lexically_bound);
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
    assert!(!s.receiver_lexically_bound);
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
        assert!(call.receiver_lexically_bound, "{caller}");
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
        assert!(!call.receiver_lexically_bound, "{caller}");
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
        assert!(call.receiver_lexically_bound, "direct subset {caller}");
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

#[test]
fn test_typescript_receiver_bindings_include_enclosing_scopes_and_self_names() {
    let cg = graph_files(&[
        ("api.ts", "export function m() {}\n"),
        (
            "svc.ts",
            "import api from './api';\n\
             class Foo { m() {} }\n\
             function make(): Foo { return new Foo(); }\n\
             function outer(api: Foo) { function capturedParam() { api.m(); } }\n\
             { const api: Foo = make(); function capturedBlock() { api.m(); } }\n\
             const named = function api() { api.m(); };\n\
             const Holder = class api { run() { api.m(); } };\n\
             function loop(items: Foo[]) { for (const api of items) { api.m(); } }\n\
             function loopEnded(items: Foo[]) { for (const api of items) {} api.m(); }\n\
             function switched() { switch (1) { case 1: const api: Foo = make(); api.m(); } }\n\
             function switchEnded() { switch (1) { case 1: { const api: Foo = make(); } } api.m(); }\n",
        ),
    ]);

    for caller in [
        "capturedParam",
        "capturedBlock",
        "api",
        "run",
        "loop",
        "switched",
    ] {
        let call = site(&cg, caller, "m");
        assert!(call.receiver_lexically_bound, "{caller}");
        assert!(
            cg.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::ImportQualified
                    || candidate.confidence != ResolutionConfidence::Exact
                    || candidate.target.file != "api.ts"
            }),
            "{caller} resolved through the shadowed module import"
        );
    }

    for caller in ["loopEnded", "switchEnded"] {
        let call = site(&cg, caller, "m");
        assert!(!call.receiver_lexically_bound, "{caller}");
        assert!(cg.resolve_call_site(&call).iter().any(|candidate| {
            candidate.kind == ResolutionKind::ImportQualified
                && candidate.confidence == ResolutionConfidence::Exact
                && candidate.target.file == "api.ts"
        }));
    }
}

#[test]
fn test_typescript_receiver_binding_incremental_transitions_match_fresh_builds() {
    fn files(caller: &str) -> BTreeMap<String, ParsedFile> {
        [("api.ts", "export function m() {}\n"), ("svc.ts", caller)]
            .into_iter()
            .map(|(path, source)| {
                (
                    path.to_string(),
                    ParsedFile::parse(path, source, Language::TypeScript).unwrap(),
                )
            })
            .collect()
    }

    let visible = "import api from './api';\nfunction run(other: object) { api.m(); }\n";
    let shadowed = "import api from './api';\nfunction run(api: object) { api.m(); }\n";
    let changed = BTreeSet::from(["svc.ts".to_string()]);

    for (before, after, expected_bound) in [(visible, shadowed, true), (shadowed, visible, false)] {
        let before_files = files(before);
        let after_files = files(after);
        let fresh = CallGraph::build(&after_files);
        let mut incremental = CallGraph::build(&before_files);
        incremental.remove_files(&changed);
        incremental.merge(CallGraph::build_direct_subset(&after_files, &changed));

        let fresh_site = site(&fresh, "run", "m");
        let incremental_site = site(&incremental, "run", "m");
        assert_eq!(fresh_site.receiver_lexically_bound, expected_bound);
        assert_eq!(incremental_site.receiver_lexically_bound, expected_bound);

        let signature = |graph: &CallGraph, call: &CallSite| {
            graph
                .resolve_call_site(call)
                .into_iter()
                .map(|candidate| {
                    (
                        candidate.target.file.clone(),
                        candidate.target.name.clone(),
                        candidate.confidence,
                        candidate.kind,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            signature(&fresh, &fresh_site),
            signature(&incremental, &incremental_site)
        );
    }
}

#[test]
fn test_typescript_receiver_binding_parse_recovery_is_scope_bounded() {
    let source = "import api from './api';\n\
                  function recovered() { let = ; api.m(); }\n\
                  function outer() { function broken() { let = ; } api.m(); }\n";
    let caller = ParsedFile::parse("svc.ts", source, Language::TypeScript).unwrap();
    assert!(caller.parse_error_count > 0);
    let files = BTreeMap::from([
        (
            "api.ts".to_string(),
            ParsedFile::parse("api.ts", "export function m() {}\n", Language::TypeScript).unwrap(),
        ),
        ("svc.ts".to_string(), caller),
    ]);
    let graph = CallGraph::build(&files);

    let recovered = site(&graph, "recovered", "m");
    assert!(recovered.receiver_lexically_bound);
    assert!(graph.resolve_call_site(&recovered).iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));

    let outer = site(&graph, "outer", "m");
    assert!(!outer.receiver_lexically_bound);
    assert!(graph.resolve_call_site(&outer).iter().any(|candidate| {
        candidate.kind == ResolutionKind::ImportQualified
            && candidate.confidence == ResolutionConfidence::Exact
            && candidate.target.file == "api.ts"
    }));
}
