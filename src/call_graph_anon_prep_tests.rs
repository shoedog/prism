//! Inject anonymous owners/sites without changing the production emission switch.
use super::*;
use crate::anon_owner::*;
use crate::languages::Language;
use crate::resolution::{LegacyClassifier, ReceiverClassifier, ReceiverCtx};

fn files(source: &str) -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([(
        "a.ts".into(),
        ParsedFile::parse("a.ts", source, Language::TypeScript).unwrap(),
    )])
}

fn template() -> CallSite {
    CallGraph::build_skeleton(&files("function probe(){ f(); }"))
        .calls
        .into_values()
        .flatten()
        .next()
        .unwrap()
}

fn inject(
    graph: &mut CallGraph,
    parsed: &ParsedFile,
    node: &tree_sitter::Node<'_>,
    name: &str,
) -> CallSite {
    let id = identity(parsed, node, "a.ts").unwrap();
    graph
        .functions
        .entry(id.name.clone())
        .or_default()
        .push(id.clone());
    graph
        .js_ts_function_locals
        .insert(id.clone(), shadow_locals(parsed, node));
    let range = parsed.node_line_range(node);
    let meta = parsed
        .function_calls_with_qualifier_and_spans_on_lines(
            node,
            &(range.0..=range.1).collect(),
            &BTreeSet::new(),
        )
        .0
        .into_iter()
        .find(|m| m.callee_name == name)
        .unwrap();
    let mut site = template();
    site.caller = id;
    site.callee_name = name.into();
    site.line = meta.line;
    site.start_byte = meta.start_byte;
    site.end_byte = meta.end_byte;
    site.local_binding = CallGraph::js_local_binding_at(
        parsed,
        site.start_byte,
        site.end_byte,
        name,
        &mut crate::ast::JsBindingCache::for_sites(),
    );
    site
}

#[test]
fn identity_and_disabled_builders_agree() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let source = "function outer(){ sink(() => hidden(), function(){}, function*(){}, (x,x) => x); } const named = () => 2;";
        let parsed = ParsedFile::parse("a.ts", source, lang).unwrap();
        let fs = BTreeMap::from([("a.ts".into(), parsed)]);
        let p = &fs["a.ts"];
        let ids: BTreeSet<_> = candidate_nodes(p, true)
            .iter()
            .filter_map(|n| identity(p, n, "a.ts"))
            .collect();
        let anon: Vec<_> = ids
            .iter()
            .filter(|id| is_anonymous_owner_name(&id.name))
            .collect();
        assert_eq!(anon.len(), 4);
        for id in anon {
            let n = node_for_id(p, id).unwrap();
            assert_eq!(source_bytes(p, id), Some((n.start_byte(), n.end_byte())));
            assert!(emitted_identity(p, &n, "a.ts").is_none());
            let mut wrong = id.clone();
            wrong.name.push('x');
            assert!(node_for_id(p, &wrong).is_none());
        }
        let expected: BTreeSet<_> = ids
            .into_iter()
            .filter(|id| !is_anonymous_owner_name(&id.name))
            .collect();
        for graph in [
            CallGraph::build(&fs),
            CallGraph::build_skeleton(&fs),
            CallGraph::build_direct_subset(&fs, &fs.keys().cloned().collect()),
        ] {
            assert_eq!(
                graph
                    .functions
                    .values()
                    .flatten()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
                expected
            );
            assert!(graph
                .calls
                .values()
                .flatten()
                .all(|s| !is_anonymous_owner_name(&s.caller.name)));
        }
        assert!(CallGraph::build_direct_subset(&fs, &BTreeSet::new())
            .functions
            .is_empty());
    }
    let py = ParsedFile::parse("a.py", "x = lambda p: p", Language::Python).unwrap();
    assert!(py
        .all_functions()
        .iter()
        .all(|n| identity(&py, n, "a.py").is_none()));
}

#[test]
fn compatibility_uses_exact_named_claims_and_strict_scope() {
    let arrangements = [
        ("class C { run(){ sink(() => f()); } }", true),
        ("class C { field = () => { sink(() => f()); }; }", true),
        ("class C { field = wrap(() => f()); }", false),
        (
            "function outer(){ class C { field = wrap(() => f()); } }",
            true,
        ),
        ("const C = class { run(){ sink(() => f()); } };", true),
        (
            "function outer(){ class C { run(){ function inner(){ sink(() => f()); } } } }",
            true,
        ),
        ("sink(() => { f(); sink(() => f()); }, () => f());", false),
        ("function outer(){ sink(() => f()); }", false),
        // Same named FunctionId collision: existing builders refuse both inventories.
        (
            "class C { run(){ sink(() => f()); } run(){ sink(() => f()); } }",
            false,
        ),
    ];
    for (source, claimed) in arrangements {
        let fs = files(&format!("function f(){{}} {source}"));
        let parsed = &fs["a.ts"];
        let claims = named_claims(parsed, true);
        let base = CallGraph::build(&fs);
        let recorded: BTreeSet<_> = base
            .calls
            .values()
            .flatten()
            .map(|s| (s.start_byte, s.end_byte))
            .collect();
        assert_eq!(claims, recorded, "{source}");
        let mut accepted = BTreeSet::new();
        let mut f_count = 0;
        for node in parsed
            .all_functions()
            .into_iter()
            .filter(|n| parsed.language.function_name(n).is_none())
        {
            let range = parsed.node_line_range(&node);
            for meta in parsed
                .function_calls_with_qualifier_and_spans_on_lines(
                    &node,
                    &(range.0..=range.1).collect(),
                    &BTreeSet::new(),
                )
                .0
            {
                if meta.callee_name != "f" {
                    continue;
                }
                let own = parsed.js_ts_span_in_own_scope(&node, meta.start_byte, meta.end_byte);
                let allow = may_mint_call(parsed, &node, meta.start_byte, meta.end_byte, &claims);
                assert_eq!(allow, own && !claimed, "{source}");
                if own {
                    f_count += 1;
                    assert_eq!(claims.contains(&(meta.start_byte, meta.end_byte)), claimed);
                }
                if allow {
                    assert!(
                        accepted.insert((meta.start_byte, meta.end_byte)),
                        "duplicate claimant"
                    );
                }
            }
            assert!(!may_mint_call(parsed, &node, 0, 1, &claims));
        }
        assert!(f_count > 0, "fixture must exercise anonymous calls");
        assert_eq!(accepted.len(), if claimed { 0 } else { f_count });
    }
}

#[test]
fn injected_shadow_chain_owns_parameters_and_isolates_siblings() {
    let mut fs = files("import { f } from './lib';\nfunction outer(decoy){ sink((f) => { const local = 0; f(); }, () => decoy()); }\nfunction clean(){ sink(() => f()); }\n");
    fs.insert(
        "lib.ts".into(),
        ParsedFile::parse(
            "lib.ts",
            "export function f(){} export function decoy(){}",
            Language::TypeScript,
        )
        .unwrap(),
    );
    let p = &fs["a.ts"];
    let nodes: Vec<_> = p
        .all_functions()
        .into_iter()
        .filter(|n| p.language.function_name(n).is_none())
        .collect();
    assert_eq!(
        own_locals(p, &nodes[0]),
        BTreeSet::from(["f".into(), "local".into()])
    );
    assert!(!own_locals(p, &nodes[1]).contains("decoy"));
    assert!(shadow_locals(p, &nodes[1]).contains("decoy"));
    assert!(!shadow_locals(p, &nodes[2]).contains("f"));
    let mut cg = CallGraph::build(&fs);
    for (i, name) in ["f", "decoy", "f"].into_iter().enumerate() {
        let site = inject(&mut cg, p, &nodes[i], name);
        let result = cg.resolve_call_site_full(&site);
        if i < 2 {
            assert!(result.resolved.is_empty(), "{name}: {result:?}");
        } else {
            assert_eq!(result.resolved.len(), 1);
            assert_eq!(
                result.resolved[0].confidence,
                crate::resolution::ResolutionConfidence::Exact
            );
        }
    }
    let outer = p
        .all_functions()
        .into_iter()
        .find(|n| {
            p.language
                .function_name(n)
                .is_some_and(|x| p.node_text(&x) == "outer")
        })
        .unwrap();
    assert_eq!(
        shadow_locals(p, &outer),
        p.js_ts_function_local_bindings(&outer)
    );
    assert!(!shadow_locals(p, &outer).contains("local"));

    let deep = files("function outer(decoy){ sink(() => { const middle=0; sink(() => decoy()); }); } sink(() => clean());");
    let p = &deep["a.ts"];
    let anon: Vec<_> = p
        .all_functions()
        .into_iter()
        .filter(|n| p.language.function_name(n).is_none())
        .collect();
    assert!(own_locals(p, &anon[1]).is_empty());
    assert!(shadow_locals(p, &anon[1]).contains("decoy"));
    assert!(shadow_locals(p, &anon[1]).contains("middle"));
    assert!(!shadow_locals(p, &anon[2]).contains("decoy"));
    assert!(!shadow_locals(p, &anon[2]).contains("middle"));
}

#[test]
fn injected_jsx_parameter_cannot_resolve_to_same_file_component() {
    let source = "function F(){} function outer(){ sink((F) => <F />); }";
    let fs = BTreeMap::from([(
        "a.ts".into(),
        ParsedFile::parse("a.ts", source, Language::Tsx).unwrap(),
    )]);
    let p = &fs["a.ts"];
    let anon = p
        .all_functions()
        .into_iter()
        .find(|n| p.language.function_name(n).is_none())
        .unwrap();
    let mut cg = CallGraph::build(&fs);
    let mut site = inject(&mut cg, p, &anon, "F");
    site.jsx_element = true;
    let mut cache = crate::ast::JsBindingCache::for_sites();
    site.local_binding =
        CallGraph::js_local_binding_at(p, site.start_byte, site.end_byte, "F", &mut cache);
    assert_eq!(
        site.local_binding,
        JsLocalBinding::Unproven("not_callable".into())
    );
    assert!(shadows(&cg, &site.caller, "F"));
    let outcome = cg.resolve_call_site_full(&site);
    assert!(outcome.resolved.is_empty(), "{outcome:?}");
    let source = "function F(){} const cb = (F) => <F />;";
    let fs = BTreeMap::from([(
        "a.ts".into(),
        ParsedFile::parse("a.ts", source, Language::Tsx).unwrap(),
    )]);
    for cg in [
        CallGraph::build(&fs),
        CallGraph::build_skeleton(&fs),
        CallGraph::build_direct_subset(&fs, &fs.keys().cloned().collect()),
    ] {
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "F")
            .unwrap();
        assert_eq!(
            site.local_binding,
            JsLocalBinding::Unproven("not_callable".into())
        );
        assert!(cg.resolve_call_site_full(site).resolved.is_empty());
    }
}

#[test]
fn receiver_and_binder_facts_use_actual_anonymous_ast() {
    let fs = files("import * as ns from './lib'; function outer(r: C){ sink(() => r.m(), (r) => r.m(), () => { const x = new C(); x.m(); }, () => ns.f()); }");
    let p = &fs["a.ts"];
    let nodes: Vec<_> = p
        .all_functions()
        .into_iter()
        .filter(|n| p.language.function_name(n).is_none())
        .collect();
    for (i, node) in nodes.iter().enumerate() {
        let range = p.node_line_range(node);
        let meta = p
            .function_calls_with_qualifier_and_spans_on_lines(
                node,
                &(range.0..=range.1).collect(),
                &BTreeSet::new(),
            )
            .0
            .into_iter()
            .find(|m| ["m", "f"].contains(&m.callee_name.as_str()))
            .unwrap();
        let facts = LegacyClassifier.classify(ReceiverCtx {
            receiver_expr: meta.receiver_node,
            qualifier: meta.qualifier.as_deref(),
            fn_node: *node,
            call_line: meta.line,
            call_start_byte: meta.start_byte,
            parsed: p,
            recv_var: None,
            file_imports: None,
            proven_imported_receiver_types: None,
        });
        match i {
            0 => assert_eq!(facts.recovered.unwrap().static_type, "C"),
            1 | 2 => {
                assert!(facts.recovered.is_none());
                assert!(facts.materialized);
            }
            _ => assert!(facts.recovered.is_none()),
        }
        assert_eq!(
            p.js_ts_receiver_lexically_bound_at_call(node, meta.receiver_node),
            i != 3
        );
    }
    let fs = files("function f(){} function owner(){ const h=f; h(); sink(() => { const alias=f; alias(); }); }");
    let p = &fs["a.ts"];
    let anon = p
        .all_functions()
        .into_iter()
        .find(|n| p.language.function_name(n).is_none())
        .unwrap();
    let range = p.node_line_range(&anon);
    let m = p
        .function_calls_with_qualifier_and_spans_on_lines(
            &anon,
            &(range.0..=range.1).collect(),
            &BTreeSet::new(),
        )
        .0
        .into_iter()
        .find(|s| s.callee_name == "alias")
        .unwrap();
    let mut cache = crate::ast::JsBindingCache::for_sites();
    assert!(matches!(
        CallGraph::js_local_binding_at(p, m.start_byte, m.end_byte, "alias", &mut cache),
        JsLocalBinding::AliasCallable(_)
    ));
    for cg in [
        CallGraph::build(&fs),
        CallGraph::build_skeleton(&fs),
        CallGraph::build_direct_subset(&fs, &fs.keys().cloned().collect()),
    ] {
        assert!(cg
            .calls
            .values()
            .flatten()
            .any(|s| s.callee_name == "h"
                && matches!(s.local_binding, JsLocalBinding::AliasCallable(_))));
    }
}

#[test]
fn injected_indexes_survive_merge_remove_subset_and_serialization() {
    let fs = files("function outer(){ sink(() => f()); }");
    let p = &fs["a.ts"];
    let anon = p
        .all_functions()
        .into_iter()
        .find(|n| p.language.function_name(n).is_none())
        .unwrap();
    let mut part = CallGraph::build_direct_subset(&fs, &fs.keys().cloned().collect());
    let site = inject(&mut part, p, &anon, "f");
    let id = site.caller.clone();
    part.calls
        .entry(id.clone())
        .or_default()
        .insert(site.clone());
    part.callers
        .entry("f".into())
        .or_default()
        .push(site.clone());
    let bytes = bincode::serialize(&part).unwrap();
    let part: CallGraph = bincode::deserialize(&bytes).unwrap();
    let mut cg = CallGraph::empty();
    cg.merge(part);
    assert!(cg.calls[&id].contains(&site));
    assert!(cg.callers["f"].contains(&site));
    assert!(cg.js_ts_function_locals.contains_key(&id));
    cg.remove_files(&BTreeSet::from(["unrelated.ts".into()]));
    assert!(cg.calls[&id].contains(&site));
    cg.remove_files(&BTreeSet::from(["a.ts".into()]));
    assert!(!cg.calls.contains_key(&id));
    assert!(!cg.callers.values().flatten().any(|s| s.caller == id));
    assert!(!cg.functions.values().flatten().any(|f| f == &id));
    assert!(!cg.js_ts_function_locals.contains_key(&id));
}

#[test]
fn anonymous_source_is_exact_and_legacy_indirect_declines() {
    let class =
        files("class C { m(){} field=wrap(() => this.m()); gen=function*(){ this.m(); }; }");
    let p = &class["a.ts"];
    for node in candidate_nodes(p, true)
        .into_iter()
        .filter(|n| p.language.function_name(n).is_none())
    {
        assert_eq!(
            CallGraph::method_metadata(p, &node),
            (None, None, None, None)
        );
    }
    let fs = files("function f(){} function g(){} function outer(){ let fp=f; sink(() => { fp(); obj.cb(); }, () => { let fp=g; }); } obj.cb=f;");
    let p = &fs["a.ts"];
    let nodes: Vec<_> = p
        .all_functions()
        .into_iter()
        .filter(|n| p.language.function_name(n).is_none())
        .collect();
    let mut cg = CallGraph::build(&fs);
    let mut site = inject(&mut cg, p, &nodes[0], "fp");
    let id = site.caller.clone();
    assert_eq!(source_before(p, &id, site.start_byte), Some("() => { "));
    assert!(source_before(p, &id, nodes[1].start_byte()).is_none());
    assert!(!permits_legacy_indirect(&id));
    cg.calls.entry(id.clone()).or_default().insert(site.clone());
    site.callee_name = "cb".into();
    site.qualifier = Some("obj".into());
    cg.calls.entry(id.clone()).or_default().insert(site);
    assert!(!cg
        .compute_indirect_call_sites(&fs, false)
        .0
        .iter()
        .any(|(caller, _)| caller == &id));
    let outer = cg.functions["outer"][0].clone();
    assert!(permits_legacy_indirect(&outer));
}

#[path = "call_graph_anon_authority_tests.rs"]
mod authority_tests;
