use super::*;
use crate::ast::ParsedFile;
use crate::call_graph::{CallGraph, CallSiteOrigin};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn files(
    language: Language,
    ext: &str,
    callee: &str,
    caller: &str,
) -> BTreeMap<String, ParsedFile> {
    [
        (format!("callee.{ext}"), callee),
        (format!("caller.{ext}"), caller),
    ]
    .into_iter()
    .map(|(path, code)| {
        (
            path.clone(),
            ParsedFile::parse(&path, code, language).unwrap(),
        )
    })
    .collect()
}
fn outcome(cg: &CallGraph) -> Vec<(String, String, ResolutionKind, ResolutionConfidence)> {
    cg.calls
        .values()
        .flatten()
        .filter(|site| site.caller.name == "caller" && site.callee_name == "f")
        .flat_map(|site| cg.resolve_call_site_full(site).resolved)
        .map(|r| {
            (
                r.target.file.clone(),
                r.target.name.clone(),
                r.kind,
                r.confidence,
            )
        })
        .collect()
}
#[test]
fn e5_unbound_cross_file_targets_refuse_by_declaration_syntax() {
    let mut failures = Vec::new();
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for callee in [
            "const o = { f: (x) => x };",
            "const o = { f(x) { return x; } };",
            "o.f = (x) => x;",
            "const f = wrap((x) => x);",
            "class C { f = (x) => x; }",
            "export function f(x) { return x; }",
        ] {
            let input = files(lang, ext, callee, "function caller(x) { return f(x); }");
            for cg in [
                CallGraph::build(&input),
                CallGraph::build_skeleton(&input),
                CallGraph::build_direct_subset(&input, &input.keys().cloned().collect()),
            ] {
                let actual = outcome(&cg);
                if !actual.is_empty() {
                    failures.push(format!("{ext}: {callee}: {actual:?}"));
                }
            }
        }
    }
    assert_eq!(
        failures,
        Vec::<String>::new(),
        "all shape/language/construction cells observed"
    );
}
#[test]
fn e5_preserves_script_binding_imports_aliases_and_same_file_controls() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let input = files(
            lang,
            ext,
            "function f(x) { return x; }",
            "function caller(x) { return f(x); }",
        );
        assert_eq!(
            outcome(&CallGraph::build(&input)),
            vec![(
                format!("callee.{ext}"),
                "f".into(),
                ResolutionKind::FreeSingle,
                ResolutionConfidence::Exact
            )]
        );
        let input = files(
            lang,
            ext,
            "export function f(x) { return x; }",
            &format!("import {{ f }} from './callee'; function caller(x) {{ return f(x); }}"),
        );
        let out = outcome(&CallGraph::build(&input));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].2, ResolutionKind::ImportMember);
        assert_eq!(out[0].3, ResolutionConfidence::Exact);
        let input = files(
            lang,
            ext,
            "export function f(x) { return x; }",
            "function caller(f, x) { return f(x); }",
        );
        // Existing alias/may-call refusals and fallbacks are preserved; never supply a new target.
        let cg = CallGraph::build(&input);
        for sites in cg.calls.values() {
            for site in sites {
                if site.caller.name == "caller" {
                    assert!(
                        !matches!(&site.local_binding,crate::call_graph::JsLocalBinding::Unproven(r) if r=="unbound")
                    );
                }
            }
        }
        let input = files(
            lang,
            ext,
            "function f(x) { return x; } function caller(x) { return f(x); }",
            "",
        );
        let out = outcome(&CallGraph::build(&input));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].2, ResolutionKind::LocalDef);
        assert_eq!(out[0].3, ResolutionConfidence::Exact);
    }
    let input=files(Language::JavaScript,"js","module.exports = function f(x) { return x; };","function outer(x) { const f = require('./callee'); function caller(x) { return f(x); } return caller(x); }");
    let out = outcome(&CallGraph::build(&input));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].3, ResolutionConfidence::Exact);
}
#[test]
fn e5_non_source_and_non_js_controls_keep_exact_fallback() {
    let input = files(
        Language::JavaScript,
        "js",
        "o.f = (x) => x;",
        "function caller(x) { return f(x); }",
    );
    let cg = CallGraph::build(&input);
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.caller.name == "caller")
        .unwrap();
    let mut synthetic = site.clone();
    synthetic.origin = CallSiteOrigin::IndirectResolution;
    let out = cg.resolve_call_site_full(&synthetic);
    assert_eq!(out.resolved.len(), 1);
    assert_eq!(out.resolved[0].kind, ResolutionKind::FreeSingle);
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    let input = BTreeMap::from([
        (
            "callee.js".into(),
            ParsedFile::parse("callee.js", "o.f = (x) => x;", Language::JavaScript).unwrap(),
        ),
        (
            "caller.py".into(),
            ParsedFile::parse(
                "caller.py",
                "def caller(x):\n    return f(x)\n",
                Language::Python,
            )
            .unwrap(),
        ),
    ]);
    assert_eq!(
        outcome(&CallGraph::build(&input)),
        vec![(
            "callee.js".into(),
            "f".into(),
            ResolutionKind::FreeSingle,
            ResolutionConfidence::Exact
        )]
    );
}
#[test]
fn e5_incremental_replacement_and_serialization_keep_refusal_facts() {
    let input = files(
        Language::JavaScript,
        "js",
        "export function f(x) { return x; }",
        "function caller(x) { return f(x); }",
    );
    let mut cg = CallGraph::build(&input);
    assert_eq!(outcome(&cg), vec![]);
    let restored: CallGraph = bincode::deserialize(&bincode::serialize(&cg).unwrap()).unwrap();
    assert_eq!(outcome(&restored), vec![]);
    let changed = files(
        Language::JavaScript,
        "js",
        "function f(x) { return x; }",
        "function caller(x) { return f(x); }",
    );
    let dirty = BTreeSet::from(["callee.js".to_string()]);
    cg.remove_files(&dirty);
    cg.merge(CallGraph::build_direct_subset(&changed, &dirty));
    assert_eq!(outcome(&cg), outcome(&CallGraph::build(&changed)));
    assert_eq!(outcome(&cg).len(), 1);
}

#[test]
fn e5_refusal_removes_argument_and_return_modeling_but_keeps_script_controls() {
    use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
    use petgraph::visit::EdgeRef;
    let counts = |input: &BTreeMap<String, ParsedFile>| {
        let cpg = CodePropertyGraph::build(input);
        let mut counts = [0usize; 5];
        for edge in cpg.graph.edge_references() {
            match edge.weight() {
                CpgEdge::Call(_) => counts[0] += 1,
                CpgEdge::Return(_) => counts[1] += 1,
                CpgEdge::DataFlow(_) => {
                    if let (
                        CpgNode::Variable { file: from, .. },
                        CpgNode::Variable { file: to, .. },
                    ) = (cpg.node(edge.source()), cpg.node(edge.target()))
                    {
                        counts[2] += usize::from(from != to);
                    }
                }
                CpgEdge::ReturnFlow { .. } => counts[3] += 1,
                CpgEdge::ReturnInput => counts[4] += 1,
                _ => {}
            }
        }
        counts
    };
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for (body, inputs) in [("x", 0), ("x + 1", 1)] {
            let caller = "function caller(x) { const y = f(x); return y; }";
            let denied = files(
                lang,
                ext,
                &format!("export function f(x) {{ return {body}; }}"),
                caller,
            );
            assert_eq!(counts(&denied), [0, 0, 0, 0, 0], "{ext}: module {body}");
            let allowed = files(
                lang,
                ext,
                &format!("function f(x) {{ return {body}; }}"),
                caller,
            );
            assert_eq!(
                counts(&allowed),
                [1, 1, 1, 1, inputs],
                "{ext}: script {body}"
            );
        }
    }
}
