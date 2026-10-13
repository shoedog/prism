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
fn e5_module_caller_refuses_module_targets_and_local_alias_singletons() {
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
                if caller == "export {}; function caller(x) { return f(x); }"
                    && !callee.starts_with("export ")
                {
                    continue; // R2: the unbound module-to-script control is below.
                }
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
fn e5_module_source_jsx_alias_and_module_target_refuse_singletons() {
    let mut wrong = Vec::new();
    for (lang, ext) in [(Language::JavaScript, "jsx"), (Language::Tsx, "tsx")] {
        for caller in [
            "export {}; function caller(o) { const F = o.component; return <F />; }",
            "export {}; function caller() { return <F />; }",
        ] {
            let input = files(lang, ext, "export function F() { return null; }", caller);
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
        // Earlier-rung protection: import_member, local_def, import_qualified.
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

fn assert_singleton(cg: &CallGraph, name: &str, file: &str) {
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == "caller" && s.callee_name == name)
        .collect();
    assert_eq!(sites.len(), 1);
    let out = cg.resolve_call_site_full(sites[0]);
    assert_eq!(
        out.drop, None,
        "site {:?}; evidence {:?}",
        sites[0], cg.js_ts_module_binding_evidence
    );
    let targets: Vec<_> = out
        .resolved
        .iter()
        .map(|r| {
            (
                r.target.file.as_str(),
                r.target.name.as_str(),
                r.target.start_line,
                r.target.end_line,
                r.kind,
                r.confidence,
            )
        })
        .collect();
    assert_eq!(
        targets,
        vec![(
            file,
            name,
            1,
            1,
            ResolutionKind::FreeSingle,
            ResolutionConfidence::Exact
        )]
    );
}

fn assert_refused(cg: &CallGraph, name: &str) {
    let sites: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == "caller" && s.callee_name == name)
        .collect();
    assert_eq!(sites.len(), 1);
    let out = cg.resolve_call_site_full(sites[0]);
    assert!(out.resolved.is_empty());
    assert_eq!(
        out.drop,
        Some(DropReason::LocalBindingUnproven),
        "site {:?}",
        sites[0]
    );
}

#[test]
fn e5_import_equals_and_awaited_import_evidence_preserve_fallbacks() {
    for (lang, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        for caller in [
            "import f = require('./callee'); function caller(x) { return f(x); }",
            "import f = require('./callee'); export function caller(x) { return f(x); }",
            "export import f = require('./callee'); function caller(x) { return f(x); }",
        ] {
            let input = files(lang, ext, "function f(x) { return x; } export = f;", caller);
            for cg in [
                CallGraph::build(&input),
                CallGraph::build_skeleton(&input),
                CallGraph::build_direct_subset(&input, &input.keys().cloned().collect()),
            ] {
                assert_singleton(&cg, "f", &format!("callee.{ext}"));
            }
        }
        let input = files(
            lang,
            ext,
            "module.exports = function f(x) { return x; };",
            "import f = require('./callee'); export function caller(x) { return f(x); }",
        );
        assert_singleton(&CallGraph::build(&input), "f", &format!("callee.{ext}"));
        let input = files(lang, ext, "export namespace N { export function f(x) { return x; } }",
            "import { N } from './callee'; import f = N.f; export function caller(x) { return f(x); }");
        assert_singleton(&CallGraph::build(&input), "f", &format!("callee.{ext}"));
        for binding in [
            "import g = require('./callee');",
            "import f = N.g;",
            "export import f = require; ('callee');",
            "export import f = require(value);",
            "export import format = ts.getPathFromPathComponents;",
        ] {
            let name = if binding.contains("format") {
                "format"
            } else {
                "f"
            };
            let input = files(
                lang,
                ext,
                &format!("export function {name}(x) {{ return x; }}"),
                &format!("{binding} export function caller(x) {{ return {name}(x); }}"),
            );
            assert_refused(&CallGraph::build(&input), name);
        }
        let input = files(lang, ext, "function f(x) { return x; } export = f;",
            "import renamed = require('./callee'); export function caller(x) { return renamed(x); }");
        let cg = CallGraph::build(&input);
        assert!(cg.js_ts_module_binding_evidence[&format!("caller.{ext}")].contains("renamed"));
        assert!(!cg.js_ts_module_binding_evidence[&format!("caller.{ext}")].contains("f"));
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "renamed")
            .unwrap();
        assert!(cg.resolve_call_site_full(site).resolved.is_empty()); // evidence proves no target
    }
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        for binding in [
            "const { f } = await import('./callee');",
            "const { g: f } = await import('./callee');",
            "const { f = null } = await import('./callee');",
            "const { nested: { f } } = await import('./callee');",
            "const { nested: [f] } = await import('./callee');",
            "const { ...f } = await import('./callee');",
        ] {
            let input = files(lang, ext, "export function f(x) { return x; }",
                &format!("export {{}}; async function outer() {{ {binding} function caller(x) {{ return f(x); }} }}"));
            assert_singleton(&CallGraph::build(&input), "f", &format!("callee.{ext}"));
        }
        let input = files(lang, ext, "export function f(x) { return x; }",
            "export {}; async function outer() { const { f } = await acquire('./callee'); function caller(x) { return f(x); } }");
        assert_refused(&CallGraph::build(&input), "f");
        let input = files(lang, ext, "export function f(x) { return x; }",
            "export {}; async function outer() { const { g } = await import('./callee'); function caller(x) { return f(x); } }");
        assert_refused(&CallGraph::build(&input), "f");
    }
}

#[test]
fn e5_unbound_module_to_script_global_stays_exact_with_negative_twins() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let input = files(
            lang,
            ext,
            "function f(x) { return x; }",
            "export {}; function caller(x) { return f(x); }",
        );
        for cg in [
            CallGraph::build(&input),
            CallGraph::build_skeleton(&input),
            CallGraph::build_direct_subset(&input, &input.keys().cloned().collect()),
        ] {
            assert_singleton(&cg, "f", &format!("callee.{ext}"));
        }
        for caller in [
            "export {}; function outer(o) { const f = o.cb; function caller(x) { return f(x); } }",
        ] {
            assert_refused(
                &CallGraph::build(&files(lang, ext, "function f(x) { return x; }", caller)),
                "f",
            );
        }
        // Parameter protection returns on an earlier lexical rung.
        let input = files(
            lang,
            ext,
            "function f(x) { return x; }",
            "export {}; function caller(f, x) { return f(x); }",
        );
        let cg = CallGraph::build(&input);
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "f")
            .unwrap();
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.is_empty());
        assert_eq!(out.drop, Some(DropReason::UnknownName));
        assert_refused(
            &CallGraph::build(&files(
                lang,
                ext,
                "export function f(x) { return x; }",
                "export {}; function caller(x) { return f(x); }",
            )),
            "f",
        );
    }
    let input = BTreeMap::from([
        (
            "callee.c".into(),
            ParsedFile::parse("callee.c", "int f(int x) { return x; }", Language::C).unwrap(),
        ),
        (
            "caller.js".into(),
            ParsedFile::parse(
                "caller.js",
                "export function caller(x) { return f(x); }",
                Language::JavaScript,
            )
            .unwrap(),
        ),
    ]);
    assert_refused(&CallGraph::build(&input), "f");
    for (lang, ext) in [(Language::JavaScript, "jsx"), (Language::Tsx, "tsx")] {
        let input = files(
            lang,
            ext,
            "function F() { return null; }",
            "export function caller() { return <F />; }",
        );
        assert_singleton(&CallGraph::build(&input), "F", &format!("callee.{ext}"));
        let input = files(lang, ext, "function F() { return null; }",
            "export {}; function outer(o) { const F = o.component; function caller() { return <F />; } }");
        assert_refused(&CallGraph::build(&input), "F");
    }
}

#[test]
fn e5_source_only_preserves_ordinary_and_renamed_jsx_indirect_targets() {
    for (lang, ext, invocation) in [
        (Language::JavaScript, "js", "Box.Item(x)"),
        (Language::JavaScript, "jsx", "<Box.Item />"),
        (Language::Tsx, "tsx", "<Box.Item />"),
    ] {
        let input: BTreeMap<_, _> = [
            (format!("callee.{ext}"), "export function F(x) { return x; }".to_string()),
            (format!("box.{ext}"), "import { F } from './callee'; export function Box() { return null; } Box.Item = F;".to_string()),
            (format!("caller.{ext}"), format!("import {{ Box }} from './box'; export function caller(x) {{ return {invocation}; }}")),
        ].into_iter().map(|(p,s)| (p.clone(),ParsedFile::parse(&p,&s,lang).unwrap())).collect();
        let cg = CallGraph::build(&input);
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.name == "caller" && s.callee_name == "F")
            .unwrap();
        assert_eq!(
            site.origin,
            crate::call_graph::CallSiteOrigin::IndirectResolution
        );
        assert_singleton(&cg, "F", &format!("callee.{ext}"));
    }
}

#[test]
fn e5_arm_a_binding_and_jsx_controls_hold_negative_target_constant() {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        let input = files(
            lang,
            ext,
            "export function f(x) { return x; }",
            "function outer(o) { const f = o.cb; function caller(x) { return f(x); } }",
        );
        assert_singleton(&CallGraph::build(&input), "f", &format!("callee.{ext}"));
    }
    for (lang, ext) in [(Language::JavaScript, "jsx"), (Language::Tsx, "tsx")] {
        let input = files(
            lang,
            ext,
            "export function F() { return null; }",
            "function caller() { return <F />; }",
        );
        assert_singleton(&CallGraph::build(&input), "F", &format!("callee.{ext}"));
    }
}

#[test]
fn e5_target_facts_merge_script_to_module_and_back() {
    let script = files(
        Language::JavaScript,
        "js",
        "function f(x) { return x; }",
        "function caller(x) { return f(x); }",
    );
    let module = files(
        Language::JavaScript,
        "js",
        "export function f(x) { return x; }",
        "function caller(x) { return f(x); }",
    );
    let mut cg = CallGraph::build(&script);
    let dirty = BTreeSet::from(["callee.js".to_string()]);
    for input in [&module, &script, &module, &script] {
        cg.remove_files(&dirty);
        cg.merge(CallGraph::build_direct_subset(input, &dirty));
        let restored: CallGraph = bincode::deserialize(&bincode::serialize(&cg).unwrap()).unwrap();
        for graph in [&cg, &restored, &CallGraph::build(input)] {
            if std::ptr::eq(input, &module) {
                assert_refused(graph, "f");
            } else {
                assert_singleton(graph, "f", "callee.js");
            }
        }
    }
}
