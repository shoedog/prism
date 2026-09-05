use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
use std::collections::{BTreeMap, BTreeSet};

fn graph(src: &str) -> CallGraph {
    graph_files(&[("svc.ts", src)])
}

#[test]
fn receiver_type_namespace_regression_matrix() {
    let mut failures = Vec::new();
    for (label, body) in [
        ("generic", "function run<Foo>(x: Foo) { x.m(); }"),
        (
            "outer generic",
            "function outer<Foo>() { function run(x: Foo) { x.m(); } }",
        ),
        (
            "local interface",
            "function outer() { interface Foo { m(): void } function run(x: Foo) { x.m(); } }",
        ),
        (
            "local alias",
            "function outer() { type Foo = Other; function run(x: Foo) { x.m(); } }",
        ),
        (
            "local class",
            "function outer() { class Foo {} function run(x: Foo) { x.m(); } }",
        ),
        (
            "class expression self",
            "const Holder = class Foo { run(x: Foo) { x.m(); } };",
        ),
    ] {
        for language in [Language::TypeScript, Language::Tsx] {
            let src = format!("class Foo {{ m() {{}} }}\nclass Other {{ m() {{}} }}\n{body}");
            let files = BTreeMap::from([(
                "svc.ts".to_string(),
                ParsedFile::parse("svc.ts", &src, language).unwrap(),
            )]);
            let cg = CallGraph::build(&files);
            let call = site(&cg, "run", "m");
            let resolved = cg.resolve_call_site(&call);
            if call.receiver_type.is_some()
                || resolved
                    .iter()
                    .any(|r| r.confidence == ResolutionConfidence::Exact)
            {
                failures.push(format!(
                    "{language:?} {label}: type={:?}, edges={resolved:?}",
                    call.receiver_type
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn receiver_type_namespace_preserves_visible_module_type() {
    for body in [
        "function run(x: Foo) { x.m(); }",
        "function run(x: Foo) { const Foo = Other; x.m(); }",
        "function outer() { { type Foo = Other; } function run(x: Foo) { x.m(); } }",
        "function unrelated<Foo>() {}\nfunction run(x: Foo) { x.m(); }",
        "function outer() { function unused() { interface Foo {} } function run(x: Foo) { x.m(); } }",
    ] {
        let cg = graph(&format!("class Foo {{ m() {{}} }}\nclass Other {{ m() {{}} }}\n{body}"));
        let call = site(&cg, "run", "m");
        let edges = cg.resolve_call_site(&call);
        assert_eq!(edges.len(), 1, "{body}: {edges:?}");
        assert_eq!(edges[0].kind, ResolutionKind::TypedParam, "{body}");
        assert_eq!(edges[0].confidence, ResolutionConfidence::Exact);
    }
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
fn test_typescript_parameters_and_new_constructor_recover_but_annotation_does_not() {
    // P3: `m` must stay OVER the R6 fanout cap (4 owners: Foo/Other/Other2/
    // Other3) so this residue keeps testing what its name says — these
    // receiver shapes do not recover — rather than the P3 candidate path a
    // 2-owner pool would now hit instead.
    let cg = graph(
        "class Foo { m() {} }\nclass Other { m() {} }\nclass Other2 { m() {} }\nclass Other3 { m() {} }\nfunction req(x: Foo) { x.m(); }\nfunction opt(x?: Foo) { x.m(); }\nfunction defaulted(x: Foo = new Foo()) { x.m(); }\nfunction annotated() { const x: Foo = other(); x.m(); }\nfunction made() { const x = new Foo(); x.m(); }\n",
    );
    for (caller, recovery) in [
        ("req", ReceiverRecovery::TypedParam),
        ("opt", ReceiverRecovery::TypedParam),
        ("defaulted", ReceiverRecovery::TypedParam),
        ("made", ReceiverRecovery::ConstructorLocal),
    ] {
        let s = site(&cg, caller, "m");
        assert!(s.receiver_lexically_bound, "{caller}");
        assert_eq!(s.receiver_type.as_deref(), Some("Foo"), "{caller}");
        assert_eq!(s.receiver_recovery, Some(recovery), "{caller}");
        assert!(s.receiver_materialized, "{caller}");
        let out = cg.resolve_call_site(&s);
        assert_eq!(out.len(), 1, "{caller}: {out:?}");
        assert_eq!(out[0].target.file, "svc.ts", "{caller}");
        assert_eq!(out[0].target.name, "m", "{caller}");
        assert_eq!(
            out[0].kind,
            match recovery {
                ReceiverRecovery::ConstructorLocal => ResolutionKind::ConstructorLocal,
                _ => ResolutionKind::TypedParam,
            }
        );
        assert_eq!(out[0].confidence, ResolutionConfidence::Exact, "{caller}");
    }

    let annotated = site(&cg, "annotated", "m");
    assert!(annotated.receiver_lexically_bound);
    assert_eq!(annotated.receiver_type, None);
    assert!(annotated.receiver_materialized);
    assert!(cg.resolve_call_site(&annotated).is_empty());
}

#[test]
fn test_typescript_recovered_receiver_preempts_qualifier_owner_collision() {
    let cg = graph("class Foo { m() {} }\nclass x { m() {} }\nfunction run(x: Foo) { x.m(); }\n");
    let call = site(&cg, "run", "m");
    assert_eq!(call.receiver_type.as_deref(), Some("Foo"));
    assert!(call.receiver_materialized);
    let out = cg.resolve_call_site(&call);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(out[0].target.start_line, 1, "Foo.m must beat x.m");
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
    assert!(s.receiver_materialized);
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
    assert_eq!(shadow.receiver_type.as_deref(), Some("Foo"));
    assert!(shadow.receiver_materialized);
    assert_eq!(shadow_out.len(), 1, "{shadow_out:?}");
    assert_eq!(shadow_out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(shadow_out[0].target.file, "svc.ts");

    let ok = site(&cg, "ok", "m");
    let ok_out = cg.resolve_call_site(&ok);
    assert!(ok.receiver_lexically_bound);
    assert_eq!(ok.receiver_type.as_deref(), Some("Foo"));
    assert!(ok.receiver_materialized);
    assert_eq!(ok_out.len(), 1, "{ok_out:?}");
    assert_eq!(ok_out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(ok_out[0].target.file, "svc.ts");
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
    assert_eq!(s.receiver_type.as_deref(), Some("Foo"));
    assert!(s.receiver_materialized);
    assert!(out.iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));
    assert!(out.iter().all(|candidate| {
        candidate.kind != ResolutionKind::TypedParam
            && candidate.kind != ResolutionKind::ConstructorLocal
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
             function enumBinding() { enum api { value } api.m(); }\n\
             function abstractClassBinding() { abstract class api {} api.m(); }\n\
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
        "enumBinding",
        "abstractClassBinding",
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
fn test_typescript_value_declarations_bind_but_type_only_and_nested_names_do_not_leak() {
    for (label, declaration) in [
        ("ambient function", "declare function api(): void;"),
        ("namespace", "namespace api {}"),
        ("module", "module api {}"),
        (
            "import alias",
            "declare namespace source { const value: object; } import api = source.value;",
        ),
    ] {
        let source =
            format!("import api from './api';\n{declaration}\nfunction run() {{ api.m(); }}\n");
        let cg = graph_files(&[
            ("api.ts", "export function m() {}\n"),
            ("svc.ts", source.as_str()),
        ]);
        let call = site(&cg, "run", "m");
        assert!(call.receiver_lexically_bound, "{label}");
        assert!(
            cg.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::ImportQualified
                    || candidate.confidence != ResolutionConfidence::Exact
                    || candidate.target.file != "api.ts"
            }),
            "{label} resolved through the shadowed module import"
        );
    }

    for (label, declaration) in [
        ("interface", "interface api {}"),
        ("type alias", "type api = object;"),
        ("nested namespace", "namespace other { const api = {}; }"),
    ] {
        let source =
            format!("import api from './api';\n{declaration}\nfunction run() {{ api.m(); }}\n");
        let cg = graph_files(&[
            ("api.ts", "export function m() {}\n"),
            ("svc.ts", source.as_str()),
        ]);
        let call = site(&cg, "run", "m");
        assert!(!call.receiver_lexically_bound, "{label}");
        assert!(
            cg.resolve_call_site(&call).iter().any(|candidate| {
                candidate.kind == ResolutionKind::ImportQualified
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.target.file == "api.ts"
            }),
            "{label} hid the visible module import"
        );
    }

    let cg = graph_files(&[
        ("api.ts", "export function m() {}\n"),
        (
            "svc.ts",
            "import api from './api';\nnamespace other { const api = {}; export function run() { api.m(); } }\n",
        ),
    ]);
    let call = site(&cg, "run", "m");
    assert!(call.receiver_lexically_bound, "namespace-contained capture");
    assert!(cg.resolve_call_site(&call).iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));
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

#[test]
fn test_typescript_recovery_rejects_unsupported_type_and_owner_shapes() {
    let cg = graph_files(&[
        ("external.ts", "export class External { m() {} }\n"),
        (
            "svc.ts",
            "import { External } from './external';\n\
             class Foo { m() {} }\n\
             class Other { m() {} }\n\
             class Other2 { m() {} }\n\
             class Base { m() {} }\n\
             class Child extends Base {}\n\
             interface Shape { m(): void; }\n\
             declare namespace ns { class Foo { m(): void; } }\n\
             function union(x: Foo | Other) { x.m(); }\n\
             function generic(x: Box<Foo>) { x.m(); }\n\
             function rest(...x: Foo) { x.m(); }\n\
             function qualifiedType(x: ns.Foo) { x.m(); }\n\
             function structural(x: Shape) { x.m(); }\n\
             function imported(x: External) { x.m(); }\n\
             function inherited(x: Child) { x.m(); }\n\
             function reassignedTyped(x: Foo) { x = new Other(); x.m(); }\n\
             function destructuredWrite(x: Foo) { [x] = values; x.m(); }\n\
             function capturedCtor() { const x = new Foo(); function innerCtor() { x.m(); } }\n\
             function outerTyped(x: Foo) { function innerTyped() { x.m(); } }\n",
        ),
    ]);

    for caller in [
        "union",
        "generic",
        "rest",
        "qualifiedType",
        "reassignedTyped",
        "destructuredWrite",
        "innerCtor",
    ] {
        let call = site(&cg, caller, "m");
        assert_eq!(call.receiver_type, None, "{caller}");
        assert!(call.receiver_materialized, "{caller}");
        assert!(
            cg.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::TypedParam
                    && candidate.kind != ResolutionKind::ConstructorLocal
            }),
            "{caller} minted an unsupported recovered edge"
        );
    }

    for (caller, recovered_type) in [("structural", "Shape"), ("inherited", "Child")] {
        let call = site(&cg, caller, "m");
        assert_eq!(
            call.receiver_type.as_deref(),
            Some(recovered_type),
            "{caller}"
        );
        assert!(call.receiver_materialized, "{caller}");
        assert!(
            cg.resolve_call_site(&call).iter().all(|candidate| {
                candidate.kind != ResolutionKind::TypedParam
                    && candidate.kind != ResolutionKind::ConstructorLocal
            }),
            "{caller} bypassed direct-class proof"
        );
    }

    // Named relative imports of directly exported classes now have owner proof.
    let imported = cg.resolve_call_site(&site(&cg, "imported", "m"));
    assert_eq!(imported.len(), 1);
    assert_eq!(imported[0].target.file, "external.ts");
    assert_eq!(imported[0].kind, ResolutionKind::TypedParam);
    assert_eq!(imported[0].confidence, ResolutionConfidence::Exact);

    let captured = site(&cg, "innerTyped", "m");
    assert_eq!(captured.receiver_type.as_deref(), Some("Foo"));
    assert_eq!(
        captured.receiver_recovery,
        Some(ReceiverRecovery::TypedParam)
    );
    let out = cg.resolve_call_site(&captured);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].kind, ResolutionKind::TypedParam);
    assert_eq!(out[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(out[0].target.file, "svc.ts");
}

#[test]
fn test_typescript_instance_receiver_rejects_static_only_method() {
    let files = BTreeMap::from([(
        "svc.ts".to_string(),
        ParsedFile::parse(
            "svc.ts",
            "class Foo { static only() {} }\nfunction run(x: Foo) { x.only(); }\n",
            Language::TypeScript,
        )
        .unwrap(),
    )]);
    let only = BTreeSet::from(["svc.ts".to_string()]);
    for cg in [
        CallGraph::build(&files),
        CallGraph::build_direct_subset(&files, &only),
    ] {
        let call = site(&cg, "run", "only");
        assert_eq!(call.receiver_type.as_deref(), Some("Foo"));
        assert!(call.receiver_materialized);
        assert!(cg.resolve_call_site(&call).iter().all(|candidate| {
            candidate.kind != ResolutionKind::TypedParam
                && candidate.kind != ResolutionKind::ConstructorLocal
        }));
    }
}

#[test]
fn test_typescript_recovery_parse_uncertainty_fails_closed() {
    let parsed = ParsedFile::parse(
        "svc.ts",
        "class Foo { m() {} }\nfunction run(x: Foo) { let = ; x.m(); }\n",
        Language::TypeScript,
    )
    .unwrap();
    assert!(parsed.parse_error_count > 0);
    let cg = CallGraph::build(&BTreeMap::from([("svc.ts".to_string(), parsed)]));
    let call = site(&cg, "run", "m");
    assert_eq!(call.receiver_type, None);
    assert!(call.receiver_materialized);
    assert!(cg.resolve_call_site(&call).iter().all(|candidate| {
        candidate.kind != ResolutionKind::TypedParam
            && candidate.kind != ResolutionKind::ConstructorLocal
    }));
}

#[test]
fn test_typescript_recovery_full_subset_and_incremental_transitions_agree() {
    fn files(caller: &str) -> BTreeMap<String, ParsedFile> {
        let source = format!(
            "class Foo {{ m() {{}} }}\n\
             class Other {{ m() {{}} }}\n\
             class Other2 {{ m() {{}} }}\n\
             class Other3 {{ m() {{}} }}\n{caller}"
        );
        BTreeMap::from([(
            "svc.ts".to_string(),
            ParsedFile::parse("svc.ts", &source, Language::TypeScript).unwrap(),
        )])
    }

    let recovered = "function run() { const x = new Foo(); x.m(); }\n";
    let unsupported = "function run() { const x = makeFoo(); x.m(); }\n";
    let only = BTreeSet::from(["svc.ts".to_string()]);

    let full_files = files(recovered);
    let full = CallGraph::build(&full_files);
    let subset = CallGraph::build_direct_subset(&full_files, &only);
    let full_site = site(&full, "run", "m");
    let subset_site = site(&subset, "run", "m");
    assert_eq!(full_site.receiver_type, subset_site.receiver_type);
    assert_eq!(full_site.receiver_recovery, subset_site.receiver_recovery);
    assert_eq!(
        full_site.receiver_materialized,
        subset_site.receiver_materialized
    );

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
        signature(&full, &full_site),
        signature(&subset, &subset_site)
    );

    for (before, after) in [(unsupported, recovered), (recovered, unsupported)] {
        let before_files = files(before);
        let after_files = files(after);
        let fresh = CallGraph::build(&after_files);
        let mut incremental = CallGraph::build(&before_files);
        incremental.remove_files(&only);
        incremental.merge(CallGraph::build_direct_subset(&after_files, &only));

        let fresh_site = site(&fresh, "run", "m");
        let incremental_site = site(&incremental, "run", "m");
        assert_eq!(fresh_site.receiver_type, incremental_site.receiver_type);
        assert_eq!(
            fresh_site.receiver_recovery,
            incremental_site.receiver_recovery
        );
        assert_eq!(
            fresh_site.receiver_materialized,
            incremental_site.receiver_materialized
        );
        assert_eq!(
            signature(&fresh, &fresh_site),
            signature(&incremental, &incremental_site)
        );
    }
}
