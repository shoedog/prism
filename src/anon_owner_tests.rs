//! S2a: nodes, strict ownership, fallback A1 and conservative lexical facts.
use super::*;
use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, FunctionId};
use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
use crate::languages::Language;
use crate::resolution::ResolutionConfidence;
use std::collections::{BTreeMap, BTreeSet};

fn files(src: &str, ext: &str) -> BTreeMap<String, ParsedFile> {
    let file = format!("a.{ext}");
    BTreeMap::from([(
        file.clone(),
        ParsedFile::parse(&file, src, Language::from_path(&file).unwrap()).unwrap(),
    )])
}

fn anonymous(cg: &CallGraph) -> BTreeSet<FunctionId> {
    cg.functions
        .values()
        .flatten()
        .filter(|f| is_anonymous_owner_name(&f.name) && !is_module_owner_name(&f.name))
        .cloned()
        .collect()
}

#[test]
fn builders_agree_including_same_line_and_parameter_early_errors() {
    for ext in ["js", "ts", "tsx"] {
        let files = files(
            "function outer() { sink(() => hit(), function() { hit(); }, function* () { hit(); }); sink((x,x) => hit()); }",
            ext,
        );
        let full = CallGraph::build(&files);
        let expected = anonymous(&full);
        assert_eq!(expected.len(), 4, "{ext}");
        assert_eq!(anonymous(&CallGraph::build_skeleton(&files)), expected);
        assert_eq!(
            anonymous(&CallGraph::build_direct_subset(
                &files,
                &files.keys().cloned().collect()
            )),
            expected
        );
        let records = |g: &CallGraph| -> BTreeSet<_> {
            g.calls
                .values()
                .flatten()
                .map(|s| {
                    (
                        s.caller.clone(),
                        s.callee_name.clone(),
                        s.line,
                        s.start_byte,
                        s.end_byte,
                    )
                })
                .collect()
        };
        let want = records(&full);
        assert_eq!(records(&CallGraph::build_skeleton(&files)), want);
        assert_eq!(
            records(&CallGraph::build_direct_subset(
                &files,
                &files.keys().cloned().collect()
            )),
            want
        );
        assert!(anonymous(&CallGraph::build_direct_subset(&files, &BTreeSet::new())).is_empty());
        let cpg = CodePropertyGraph::build(&files);
        let ids: BTreeSet<_> = cpg
            .node_indices()
            .filter_map(|idx| match cpg.node(idx) {
                CpgNode::Function {
                    file,
                    name,
                    start_line,
                    end_line,
                    ..
                } if is_anonymous_owner_name(name) && !is_module_owner_name(name) => {
                    Some(FunctionId {
                        file: file.clone(),
                        name: name.clone(),
                        start_line: *start_line,
                        end_line: *end_line,
                    })
                }
                _ => None,
            })
            .collect();
        assert_eq!(ids, expected);
    }
    // Other languages do not gain anonymous call-graph nodes.
    assert!(anonymous(&CallGraph::build(&files(
        "def f():\n  return lambda x: x\n",
        "py"
    )))
    .is_empty());
}

#[test]
fn nested_field_callbacks_are_strict_but_class_methods_keep_a1() {
    let src = "function f() {}\nclass C {\n run() { sink(() => { f(); sink(() => f()); }); }\n field = wrap(() => { f(); sink(() => f()); });\n}\nfunction outer() { sink(() => f()); }\n";
    let files = files(src, "js");
    for cg in [
        CallGraph::build(&files),
        CallGraph::build_skeleton(&files),
        CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
    ] {
        assert_eq!(anonymous(&cg).len(), 5);
        let sites: Vec<_> = cg
            .calls
            .values()
            .flatten()
            .filter(|s| s.callee_name == "f")
            .collect();
        assert_eq!(sites.len(), 5, "no duplicate or missing f calls");
        assert_eq!(
            sites
                .iter()
                .filter(|s| s.line == 3 && s.caller.name == "run")
                .count(),
            2
        );
        assert_eq!(
            sites
                .iter()
                .filter(|s| s.line == 4 && is_anonymous_owner_name(&s.caller.name))
                .count(),
            2
        );
        assert_ne!(
            sites.iter().find(|s| s.line == 4).unwrap().caller,
            sites.iter().filter(|s| s.line == 4).nth(1).unwrap().caller
        );
    }
}

#[test]
fn lexical_chain_shadows_imports_and_decoys_without_poisoning_siblings() {
    let source = "import { f } from './lib';\nfunction outer(decoy) { sink(() => { sink((f) => f()); sink(() => decoy()); }); }\nfunction clean() { sink(() => f()); }\n";
    let mut files = files(source, "ts");
    files.insert(
        "lib.ts".into(),
        ParsedFile::parse(
            "lib.ts",
            "export function f() {}\nexport function decoy() {}",
            Language::TypeScript,
        )
        .unwrap(),
    );
    let cg = CallGraph::build(&files);
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| ["f", "decoy"].contains(&s.callee_name.as_str()))
        .collect();
    assert_eq!(sites.len(), 3);
    for site in sites {
        assert!(is_anonymous_owner_name(&site.caller.name));
        let outcome = cg.resolve_call_site_full(site);
        if site.line == 2 {
            assert!(outcome.resolved.is_empty(), "shadowed {}", site.callee_name);
        } else {
            assert_eq!(outcome.resolved.len(), 1);
            assert_eq!(outcome.resolved[0].confidence, ResolutionConfidence::Exact);
        }
    }
}

#[test]
fn anonymous_bytes_and_dfg_contains_are_exact() {
    let source = "function outer() { sink((x) => x + 1, function* (p) { yield p; }); }";
    let files = files(source, "js");
    let parsed = &files["a.js"];
    let cpg = CodePropertyGraph::build(&files);
    let mut count = 0;
    for idx in cpg.node_indices() {
        let CpgNode::Function {
            name,
            start_byte,
            end_byte,
            ..
        } = cpg.node(idx)
        else {
            continue;
        };
        if !is_anonymous_owner_name(name) || is_module_owner_name(name) {
            continue;
        }
        count += 1;
        let node = parsed
            .all_functions()
            .into_iter()
            .find(|n| parsed.js_ts_synthetic_callable_name(n).as_ref() == Some(name))
            .unwrap();
        assert_eq!(
            (*start_byte, *end_byte),
            (node.start_byte(), node.end_byte())
        );
        let children: Vec<_> = cpg
            .graph
            .edges(idx)
            .filter(|e| *e.weight() == CpgEdge::Contains)
            .collect();
        assert!(!children.is_empty(), "{name} needs owned DFG rows");
    }
    assert_eq!(count, 2);
}

#[test]
fn generator_dfg_owns_formals_but_refuses_early_errors() {
    for ext in ["js", "ts", "tsx"] {
        let files = files("sink(function* (p) { yield p; sink(() => p); });\nsink(function* (p,p) { 'use strict'; yield p; });", ext);
        let parsed = files.values().next().unwrap();
        let generators: Vec<_> = parsed
            .all_functions()
            .into_iter()
            .filter(|n| n.kind() == "generator_function")
            .collect();
        assert_eq!(generators.len(), 2);
        let owner = parsed
            .dfg_owner_name(&generators[0])
            .expect("generator DFG owner");
        assert!(is_anonymous_owner_name(&owner));
        assert!(parsed.dfg_owner_name(&generators[1]).is_none());
        let cg = CallGraph::build(&files);
        assert_eq!(anonymous(&cg).len(), 3, "early error still gets a node");
        let cpg = CodePropertyGraph::build(&files);
        assert!(cpg.node_indices().any(|i| matches!(cpg.node(i), CpgNode::Variable { function, access: VarAccess::Def, path, .. } if function == &owner && path.base == "p")));
        let refused = parsed
            .js_ts_synthetic_callable_name(&generators[1])
            .unwrap();
        assert!(!cpg.node_indices().any(
            |i| matches!(cpg.node(i), CpgNode::Variable { function, .. } if function == &refused)
        ));
    }
}

#[test]
fn arrow_this_stays_dropped_without_lexical_method_inheritance() {
    let cg = CallGraph::build(&files(
        "class C { m() {} field = wrap(() => this.m()); }",
        "js",
    ));
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "m")
        .expect("anonymous this site");
    assert!(is_anonymous_owner_name(&site.caller.name));
    assert!(cg.resolve_call_site_full(site).resolved.is_empty());
}

#[test]
fn base_legacy_inventory_also_claims_field_callbacks() {
    let source = "function f() {}\nclass C { field = () => { sink(() => f()); }; }\nfunction outer() { class D { field = wrap(() => f()); } return D; }\n";
    let files = files(source, "js");
    let cg = CallGraph::build(&files);
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.callee_name == "f")
        .map(|s| (s.line, s.caller.name.clone()))
        .collect();
    eprintln!("base legacy field callers: {sites:?}");
    assert!(sites.contains(&(2, "field".into())));
    assert!(sites.contains(&(3, "outer".into())));
    let parsed = &files["a.js"];
    let mut overlaps = 0;
    for node in parsed.all_functions() {
        let Some(anon) = parsed.js_ts_synthetic_callable_name(&node) else {
            continue;
        };
        let (start, end) = parsed.node_line_range(&node);
        let (candidate_sites, _) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &node,
            &(start..=end).collect(),
            &BTreeSet::new(),
        );
        for candidate in candidate_sites.iter().filter(|s| s.callee_name == "f") {
            let existing = cg
                .calls
                .values()
                .flatten()
                .find(|s| s.start_byte == candidate.start_byte && s.end_byte == candidate.end_byte)
                .expect("same source call is already claimed");
            eprintln!(
                "same-span candidate: bytes {}..{}, existing {}, anonymous {anon}",
                candidate.start_byte, candidate.end_byte, existing.caller.name
            );
            overlaps += 1;
        }
    }
    assert_eq!(overlaps, 2);
}

#[path = "anon_enable_tests.rs"]
mod enable_tests;

#[path = "anon_eval_tests.rs"]
mod eval_tests;
