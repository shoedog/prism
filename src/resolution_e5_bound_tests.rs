use super::*;
use crate::ast::ParsedFile;
use crate::call_graph::CallGraph;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn files(lang: Language, ext: &str, callee: &str, caller: &str) -> BTreeMap<String, ParsedFile> {
    [
        (format!("callee.{ext}"), callee),
        (format!("caller.{ext}"), caller),
    ]
    .into_iter()
    .map(|(p, s)| (p.clone(), ParsedFile::parse(&p, s, lang).unwrap()))
    .collect()
}

fn resolutions(cg: &CallGraph) -> Vec<(String, String, ResolutionKind, ResolutionConfidence)> {
    cg.calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == "caller" && matches!(s.callee_name.as_str(), "f" | "F"))
        .flat_map(|s| cg.resolve_call_site_full(s).resolved)
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
fn e5_module_caller_refuses_unbound_and_alias_singletons() {
    let mut wrong = Vec::new();
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for caller in [
            "export {}; function caller(x) { return f(x); }",
            "export {}; function caller(f, x) { return f(x); }",
            "export {}; function caller(o, x) { const f = o.cb; return f(x); }",
            "export {}; function caller({ f }, x) { return f(x); }",
            "import './side-effect'; function caller(f, x) { return f(x); }",
            "import { g } from './other'; function caller(f, x) { return f(x); }",
        ] {
            for callee in [
                "function f(x) { return x; }",
                "export function f(x) { return x; }",
            ] {
                let input = files(lang, ext, callee, caller);
                for cg in [
                    CallGraph::build(&input),
                    CallGraph::build_skeleton(&input),
                    CallGraph::build_direct_subset(&input, &input.keys().cloned().collect()),
                ] {
                    let got = resolutions(&cg);
                    if !got.is_empty() {
                        wrong.push(format!("{ext}: {caller}: {callee}: {got:?}"));
                    }
                }
            }
        }
    }
    assert_eq!(
        wrong,
        Vec::<String>::new(),
        "every constructor/language/binding/target cell"
    );
}

#[test]
fn e5_module_jsx_alias_and_unbound_refuse_singletons() {
    let mut wrong = Vec::new();
    for (lang, ext) in [(Language::JavaScript, "jsx"), (Language::Tsx, "tsx")] {
        for caller in [
            "export {}; function caller(o) { const F = o.component; return <F />; }",
            "export {}; function caller() { return <F />; }",
        ] {
            let input = files(lang, ext, "function F() { return null; }", caller);
            for cg in [
                CallGraph::build(&input),
                CallGraph::build_skeleton(&input),
                CallGraph::build_direct_subset(&input, &input.keys().cloned().collect()),
            ] {
                let got = resolutions(&cg);
                if !got.is_empty() {
                    wrong.push(format!("{ext}: {caller}: {got:?}"));
                }
            }
        }
        let input = files(
            lang,
            ext,
            "export function F() { return null; }",
            "import { F } from './callee'; function caller() { return <F />; }",
        );
        assert_eq!(
            resolutions(&CallGraph::build(&input)),
            vec![(
                format!("callee.{ext}"),
                "F".into(),
                ResolutionKind::ImportMember,
                ResolutionConfidence::Exact
            )]
        );
    }
    assert_eq!(wrong, Vec::<String>::new());
}

#[test]
fn e5_bound_negative_controls_retain_full_resolution_values() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let exact = |kind| {
            vec![(
                format!("callee.{ext}"),
                "f".into(),
                kind,
                ResolutionConfidence::Exact,
            )]
        };
        for caller in [
            "function caller(x) { return f(x); }",
            "function outer(o) { const f = o.cb; function caller(x) { return f(x); } }",
        ] {
            assert_eq!(
                resolutions(&CallGraph::build(&files(
                    lang,
                    ext,
                    "function f(x) { return x; }",
                    caller
                ))),
                exact(ResolutionKind::FreeSingle)
            );
        }
        let input = files(
            lang,
            ext,
            "export function f(x) { return x; }",
            "import { f } from './callee'; function caller(x) { return f(x); }",
        );
        assert_eq!(
            resolutions(&CallGraph::build(&input)),
            exact(ResolutionKind::ImportMember)
        );
        for binding in [
            "const f = require('./callee');",
            "const { f } = require('./callee');",
            "const { g: f } = require('./callee');",
            "const f = require(path);",
        ] {
            let input = files(lang, ext, "module.exports = function f(x) { return x; };", &format!("export {{}}; function outer(path) {{ {binding} function caller(x) {{ return f(x); }} }}"));
            assert_eq!(
                resolutions(&CallGraph::build(&input)),
                exact(ResolutionKind::FreeSingle),
                "{ext}: {binding}"
            );
        }
        let input = files(
            lang,
            ext,
            "",
            "export {}; function f(x) { return x; } function caller(x) { return f(x); }",
        );
        assert_eq!(
            resolutions(&CallGraph::build(&input)),
            vec![(
                format!("caller.{ext}"),
                "f".into(),
                ResolutionKind::LocalDef,
                ResolutionConfidence::Exact
            )]
        );
        let input = files(
            lang,
            ext,
            "export function f(x) { return x; }",
            "import * as o from './callee'; function caller(x) { return o.f(x); }",
        );
        assert_eq!(
            resolutions(&CallGraph::build(&input)),
            exact(ResolutionKind::ImportQualified)
        );
    }
    let input = BTreeMap::from([
        (
            "callee.js".into(),
            ParsedFile::parse(
                "callee.js",
                "function f(x) { return x; }",
                Language::JavaScript,
            )
            .unwrap(),
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
        resolutions(&CallGraph::build(&input)),
        vec![(
            "callee.js".into(),
            "f".into(),
            ResolutionKind::FreeSingle,
            ResolutionConfidence::Exact
        )]
    );
}

#[test]
fn e5_caller_facts_survive_serialization_and_replace_in_both_directions() {
    let script = files(
        Language::JavaScript,
        "js",
        "function f(x) { return x; }",
        "function outer(o) { const f = o.cb; function caller(x) { return f(x); } }",
    );
    let module = files(
        Language::JavaScript,
        "js",
        "function f(x) { return x; }",
        "export {}; function outer(o) { const f = o.cb; function caller(x) { return f(x); } }",
    );
    let imported = files(Language::JavaScript, "js", "function f(x) { return x; }", "export {}; function outer() { const f = require('./callee'); function caller(x) { return f(x); } }");
    let dirty = BTreeSet::from(["caller.js".to_string()]);
    let mut cg = CallGraph::build(&script);
    assert_eq!(resolutions(&cg).len(), 1);
    for input in [&module, &imported, &module, &script] {
        cg.remove_files(&dirty);
        cg.merge(CallGraph::build_direct_subset(input, &dirty));
        let restored: CallGraph = bincode::deserialize(&bincode::serialize(&cg).unwrap()).unwrap();
        let expected = if std::ptr::eq(input, &module) {
            vec![]
        } else {
            vec![(
                "callee.js".into(),
                "f".into(),
                ResolutionKind::FreeSingle,
                ResolutionConfidence::Exact,
            )]
        };
        assert_eq!(resolutions(&cg), expected);
        assert_eq!(resolutions(&restored), expected);
        assert_eq!(resolutions(&CallGraph::build(input)), expected);
    }
}

#[test]
fn e5_bound_refusal_removes_only_interprocedural_consumers() {
    use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
    use petgraph::visit::EdgeRef;
    let counts = |input: &BTreeMap<String, ParsedFile>| {
        let cpg = CodePropertyGraph::build(input);
        let mut n = [0; 5];
        for e in cpg.graph.edge_references() {
            match e.weight() {
                CpgEdge::Call(_) => n[0] += 1,
                CpgEdge::Return(_) => n[1] += 1,
                CpgEdge::DataFlow(_) if matches!((cpg.node(e.source()), cpg.node(e.target())), (CpgNode::Variable { file: a, .. }, CpgNode::Variable { file: b, .. }) if a != b) => {
                    n[2] += 1
                }
                CpgEdge::ReturnFlow { .. } => n[3] += 1,
                CpgEdge::ReturnInput => n[4] += 1,
                _ => {}
            }
        }
        n
    };
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for (prefix, expected) in [("", [1, 1, 1, 1, 0]), ("export {};", [0; 5])] {
            let input = files(lang, ext, "function f(x) { return x; }", &format!("{prefix} function outer(o) {{ const f = o.cb; function caller(x) {{ const y = f(x); return y; }} }}"));
            assert_eq!(counts(&input), expected, "{ext}: {prefix}");
        }
    }
}
