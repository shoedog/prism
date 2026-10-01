use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, JsLocalBinding},
    languages::Language,
};
use std::collections::BTreeMap;
const LIB: &str = "function helper() {\n  function f() { return 99; }\n  return f;\n}\nexport function f() { return 1; }\n";
const APP: &str = "import * as ns from './lib';\nexport function run() { return ns.f(); }\n";
const RENAME: &str = "function make() {\n function f() { return 1; }\n return f;\n}\nconst g=make();\nexport {g as f};";
fn parsed(ext: &str, sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(p, s)| {
            let path = format!("{p}.{ext}");
            let f = ParsedFile::parse(&path, s, Language::from_path(&path).unwrap()).unwrap();
            (path, f)
        })
        .collect()
}
fn graph(ext: &str, sources: &[(&str, &str)]) -> CallGraph {
    CallGraph::build(&parsed(ext, sources))
}
fn rows(cg: &CallGraph, caller: &str, member: &str) -> Vec<String> {
    cg.calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == caller && s.callee_name == member)
        .map(|s| {
            let out = cg.resolve_call_site_full(s);
            if let Some(d) = out.drop {
                return format!("drop {d:?}");
            }
            out.resolved
                .iter()
                .map(|r| {
                    format!(
                        "{}:{}@{}-{} {:?}/{}",
                        r.target.file,
                        r.target.name,
                        r.target.start_line,
                        r.target.end_line,
                        r.confidence,
                        r.kind.as_str()
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .collect()
}
#[test]
fn d1_direct_and_directory_decoys() {
    for ext in ["jsx", "tsx"] {
        let cg = graph(ext, &[("lib", LIB), ("other/lib", LIB), ("app", APP)]);
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 5)]);
        let f = cg.functions["f"]
            .iter()
            .find(|f| f.file == format!("lib.{ext}") && f.start_line == 5)
            .unwrap();
        assert_eq!(cg.resolved_caller_edges(f).len(), 1);
        let decoy = cg.functions["f"]
            .iter()
            .find(|f| f.file == format!("lib.{ext}") && f.start_line == 2)
            .unwrap();
        assert!(cg.resolved_caller_edges(decoy).is_empty());
    }
}
#[test]
fn d2_wrapped_call_and_jsx() {
    for ext in ["jsx", "tsx"] {
        let cg=graph(ext,&[("lib","import { memo } from 'react';\nexport const f = memo(() => <div/>);\n"),("app","import * as ns from './lib';\nexport function run() { ns.f(); return <ns.f/>; }\n")]);
        assert_eq!(
            rows(&cg, "run", "f"),
            [exact(ext, "lib", "f", 2), exact(ext, "lib", "f", 2)]
        );
    }
}
#[test]
fn d3_rename_named_and_star_barrels() {
    for ext in ["jsx", "tsx"] {
        for barrel in [
            "export { 'g' as f } from './impl';",
            "export * from './impl';",
        ] {
            let implementation = if barrel.contains("'g'") {
                "function actual() { return 1; }\nexport { actual as 'g' };"
            } else {
                "function actual() { return 1; }\nexport { actual as f };"
            };
            let cg = graph(
                ext,
                &[("impl", implementation), ("lib", barrel), ("app", APP)],
            );
            // Rule 4: renamed terminal is outside base R3 candidates.
            assert_eq!(rows(&cg, "run", "f"), ["drop UnknownName"]);
        }
    }
}
#[test]
fn d4_scope_write_recovery_and_positions() {
    for ext in ["jsx", "tsx"] {
        for (app,want) in [
            ("import * as ns from './lib';\nexport function run(a = ns.f()) { var ns=0; return a; }",true),
            ("import * as ns from './lib';\nfunction write(ns) { ns=0; }\nexport function run() { return ns.f(); }",true),
            ("import * as ns from './lib';\nfunction broken() { let bad = ; }\nexport function run() { return ns.f(); }",true),
            ("import * as ns from './lib';\nexport function run() { return { [ns.f()](ns) {} }; }",true),
            ("import * as ns from './lib';\nexport function run() { with(ns.f()) { return 0; } }",true),
            ("import * as ns from './lib';\nconst broken = ;\nexport function run() { return ns.f(); }",false),
            ("import * as ns from './lib';\nimport * as ns from './lib';\nexport function run() { return ns.f(); }",false),
            ("import * as ns from './lib';\nexport function run(o) { with(o) { return ns.f(); } }",false),
            ("import * as ns from './lib';\nexport function run() { const ns=0; return ns.f(); }",false),
        ] {
            let cg=graph(ext,&[("lib",LIB),("app",app)]);
            let site=cg.calls.values().flatten().find(|s| s.caller.name=="run" && s.callee_name=="f").unwrap();
            assert_eq!(matches!(site.local_binding,JsLocalBinding::NamespaceImport{..}),want,"{ext}: {app}");
            if want {
                let base=base_rows(&cg,"f");
                if site.receiver_lexically_bound || site.receiver_materialized { assert_eq!(rows(&cg,"run","f"),base,"{app}"); }
                else { assert_eq!(rows(&cg,"run","f"),[exact(ext,"lib","f",5)],"{app}"); }
            }
            else if app.contains("const ns=0") { assert!(cg.resolve_call_site_full(site).resolved.iter().all(|r| r.kind.as_str()!="import_qualified"),"{app}"); }
        }
    }
}
#[test]
fn d5_authoritative_missing_member_and_fallback() {
    for ext in ["jsx", "tsx"] {
        let wrapped = "import {memo} from 'react';\nexport const f=memo(() => null);";
        let cg = graph(
            ext,
            &[
                ("a/lib", wrapped),
                ("b/lib", "export function f(){}"),
                ("app", APP),
            ],
        );
        assert_eq!(
            rows(&cg, "run", "f"),
            [format!("a/lib.{ext}:f@2-2 NameOnly/import_qualified, b/lib.{ext}:f@1-1 NameOnly/import_qualified")]
        );
        let cg = graph(ext, &[("a/lib", wrapped), ("app", APP)]);
        assert_eq!(
            rows(&cg, "run", "f"),
            [format!("a/lib.{ext}:f@2-2 NameOnly/import_qualified")]
        );
        let cg = graph(
            ext,
            &[("lib", "export {};"), ("other/lib", LIB), ("app", APP)],
        );
        assert_eq!(
            rows(&cg, "run", "f"),
            [format!(
                "{}, {}",
                exact(ext, "other/lib", "f", 2),
                exact(ext, "other/lib", "f", 5)
            )]
        );
        let cg = graph(ext, &[("other/lib", LIB), ("app", APP)]);
        assert_eq!(
            rows(&cg, "run", "f"),
            [format!("other/lib.{ext}:f@2-2 NameOnly/import_qualified, other/lib.{ext}:f@5-5 NameOnly/import_qualified")]
        );
        let cg = graph(
            ext,
            &[("lib", LIB), ("app", &APP.replace("'./lib'", "'./lib.js'"))],
        );
        let grade = if ext == "tsx" { "Exact" } else { "NameOnly" };
        assert_eq!(
            rows(&cg, "run", "f"),
            [if ext == "tsx" {
                format!("lib.{ext}:f@5-5 {grade}/import_qualified")
            } else {
                format!("lib.{ext}:f@2-2 {grade}/import_qualified, lib.{ext}:f@5-5 {grade}/import_qualified")
            }]
        );
    }
}
#[test]
fn d6_non_namespace_imports_keep_base() {
    for ext in ["jsx", "tsx"] {
        let cg=graph(ext,&[("lib",LIB),("app","import * as ns from './lib'; export function run(){ ns['f'](); ns.inner.f(); new ns.f(); }")]);
        assert!(cg
            .calls
            .values()
            .flatten()
            .filter(|s| s.caller.name == "run")
            .all(|s| s.local_binding == JsLocalBinding::Unchecked));
        for import in [
            "import ns from './lib';",
            "import { thing as ns } from './lib';",
            "const ns = require('./lib');",
            "import type * as ns from './lib';",
            "import ns = require('./lib');",
        ] {
            let cg = graph(
                ext,
                &[
                    ("lib", LIB),
                    (
                        "app",
                        &format!("{import}\nexport function run() {{ return ns.f(); }}"),
                    ),
                ],
            );
            let site = cg
                .calls
                .values()
                .flatten()
                .find(|s| s.caller.name == "run" && s.callee_name == "f")
                .unwrap();
            assert_eq!(
                site.local_binding,
                JsLocalBinding::Unchecked,
                "{ext}: {import}"
            );
        }
    }
}
#[test]
fn d9_written_import_and_export_keep_base() {
    for ext in ["jsx", "tsx"] {
        for app in [
            "import * as ns from './lib'; export function run(other){ class ns { static f(){} } ns=other; return ns.f(); }",
            "import * as ns from './lib'; export function run(ns){ ns=other; return ns.f(); }",
            "import * as ns from './lib'; export function run(other){ for(var ns of other){ ns=other; ns.f(); } }",
        ] {
            let cg=graph(ext,&[("lib",LIB),("app",app)]);
            let site=cg.calls.values().flatten().find(|s| s.caller.name=="run" && s.callee_name=="f").unwrap();
            assert!(matches!(&site.local_binding,JsLocalBinding::MayCall(r) if r=="may_call"));
            assert_eq!(prism::navigation::queries::call_stats(&cg)["local_binding_may_call"]["may_call"],1);
        }
        let cg = graph(
            ext,
            &[
                ("lib", LIB),
                (
                    "app",
                    &APP.replace("return ns.f()", "ns=other; return ns.f()"),
                ),
            ],
        );
        let site = cg
            .calls
            .values()
            .flatten()
            .find(|s| s.caller.name == "run")
            .unwrap();
        assert!(matches!(&site.local_binding,JsLocalBinding::MayCall(r) if r=="may_call"));
        assert_eq!(cg.resolve_call_site_full(site).resolved.len(), 2);
        let cg = graph(ext, &[("lib", &format!("{LIB}f=other;")), ("app", APP)]);
        assert_eq!(
            rows(&cg, "run", "f")[0],
            format!(
                "lib.{ext}:f@2-2 Exact/import_qualified, lib.{ext}:f@5-5 Exact/import_qualified"
            )
        );
    }
}
#[test]
fn d10_alias_opacity_direct_named_star_forwarded_and_d4_pin() {
    for ext in ["jsx", "tsx"] {
        for alias in ["function make() {\n function f() { return 1; }\n return f;\n}\nexport const f=make();", "function make() {\n function f() { return 1; }\n return f;\n}\nconst f=make();\nexport {f};", RENAME] {
            for barrel in [None,Some("export {f} from './impl';"),Some("export * from './impl';"),Some("import {f as alias} from './impl'; export {alias as f};")] {
                let barrel_src=barrel.map(|b| format!("function holder(){{ function f(){{}} return f; }}\n{b}"));
                let sources=if let Some(barrel)=barrel_src.as_deref() { vec![("impl",alias),("lib",barrel),("app",APP)] } else { vec![("lib",alias),("app",APP)] };
                let mut sources=sources;
                sources.push(("consumer","import {f} from './lib'; export function direct(){ return f(); }"));
                let cg=graph(ext,&sources);
                let direct=cg.calls.values().flatten().find(|s| s.caller.name=="direct" && s.callee_name=="f").unwrap();
                assert!(cg.resolve_call_site_full(direct).resolved.is_empty());
                // This pure forwarding barrel cannot expose its private decoy.
                assert_eq!(rows(&cg,"run","f"),if barrel.is_none() {vec![exact(ext,"lib","f",2)]} else {vec![exact(ext,"lib","f",1)]});
                let producer=if barrel.is_none() {"lib"} else {"impl"};
                assert!(!cg.js_ts_resolved_exports.get(&format!("{producer}.{ext}")).is_some_and(|e| e.contains_key("f")));
                assert!(!cg.js_ts_namespace_exports.get(&format!("lib.{ext}")).is_some_and(|e|e.contains_key("f")));
            }
            for barrel in ["export {f} from './impl';", "export * from './impl';", "import {f as alias} from './impl'; export {alias as f};"] {
                let cg=graph(ext,&[("lib/index",barrel),("lib/impl",alias),("app",APP)]);
                assert_eq!(rows(&cg,"run","f"),[exact(ext,"lib/impl","f",2)]);
            }
        }
        let sources = [("lib", CROSS_BAR), ("impl", CROSS_IMPL), ("app", APP)];
        let cg = graph(ext, &sources);
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 1)]);
    }
}
#[test]
fn d11_incomplete_exports_and_depth_keep_base() {
    for ext in ["jsx", "tsx"] {
        for (lib, line) in [
            (
                "const base=require('./base');\nfunction f(){}\nmodule.exports={...base,f};",
                2,
            ),
            ("function f(){}\nObject.assign(module.exports,{f});", 1),
            ("exports.f=function f(){};", 1),
            ("module.exports.f=()=>1;", 1),
            ("function f(){}\nexports.f=f;", 1),
            ("function f(){}\nexport = {f};", 1),
            (
                "namespace N { export function f(){} }\nexport import f=N.f;",
                1,
            ),
        ] {
            let cg = graph(ext, &[("lib", lib), ("app", APP)]);
            assert_eq!(
                rows(&cg, "run", "f"),
                [exact(ext, "lib", "f", line)],
                "{ext}: {lib}"
            );
        }
        let cg = graph(
            ext,
            &[
                ("lib/index", "export * from './a';"),
                ("lib/a", "export * from './b';"),
                ("lib/b", "export * from './c';"),
                ("lib/c", "export function f(){}"),
                ("app", APP),
            ],
        );
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib/c", "f", 1)]);
        let cg = graph(
            ext,
            &[
                ("lib", "export * from './a'; export * from './b';"),
                ("a", "export function f(){}"),
                ("b", "export function f(){}"),
                ("app", APP),
            ],
        );
        assert_eq!(rows(&cg, "run", "f"), ["drop ImportExternal"]);
        let cg = graph(
            ext,
            &[
                ("lib", "export * from './a';"),
                ("a", "export * from './lib';"),
                ("other/lib", "export function f(){}"),
                ("app", APP),
            ],
        );
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "other/lib", "f", 1)]);
    }
}
#[test]
fn d12_pattern_alias_and_skipped_maycall_and_bare_alias() {
    for ext in ["jsx", "tsx"] {
        for lib in [
            "const o={ f(){return 1;} };\nexport const {f}=o;",
            "const arr=[function f(){}];\nexport const [f]=arr;",
            "function wrap(x){return x;}\nexport const f=wrap(function f(){});",
        ] {
            let cg = graph(ext, &[("lib", lib), ("app", APP)]);
            let line = if lib.contains("wrap") { 2 } else { 1 };
            assert_eq!(
                rows(&cg, "run", "f"),
                [exact(ext, "lib", "f", line)],
                "{ext}: {lib}"
            );
        }
        let lib = "function make(){\n function f(){}\n return f;\n}\nexport const f=make();";
        for lib in [lib, RENAME] {
            for path in ["lib", "other/lib"] {
                let cg = graph(
                    ext,
                    &[(path, lib), ("app", &APP.replace("'./lib'", "'pkg/lib'"))],
                );
                assert_eq!(
                    rows(&cg, "run", "f"),
                    [format!("{path}.{ext}:f@2-2 NameOnly/import_qualified")]
                );
            }
        }
    }
}
#[test]
fn d13_b0_and_nonproving_refusals_keep_base() {
    for ext in ["jsx", "tsx"] {
        let app = format!("{APP}function g(){{ const a\\u0062c=1; }}");
        let cg = graph(ext, &[("lib", "export function f(){}"), ("app", &app)]);
        assert_eq!(rows(&cg, "run", "f"), [exact(ext, "lib", "f", 1)]);
        let app = app.replace("* as ns", "{f}").replace("ns.f()", "f()");
        let cg = graph(ext, &[("lib", "export function f(){}"), ("app", &app)]);
        assert_eq!(
            rows(&cg, "run", "f"),
            [format!("lib.{ext}:f@1-1 Exact/import_member")]
        );
        // Each non-proving reason must leave the same base rung available.
        for reason in [
            "escaped_identifier",
            "unclassified_kind",
            "parse_recovery",
            "import_parse_recovery",
            "with",
            "import",
            "unbound",
        ] {
            let mut cg = graph(ext, &[("lib", "export function f(){}"), ("app", APP)]);
            for sites in cg.calls.values_mut() {
                *sites = std::mem::take(sites)
                    .into_iter()
                    .map(|mut s| {
                        s.local_binding = JsLocalBinding::Unproven(reason.into());
                        s
                    })
                    .collect();
            }
            assert_eq!(
                rows(&cg, "run", "f"),
                [exact(ext, "lib", "f", 1)],
                "{reason}"
            );
        }
        let cg = graph(
            ext,
            &[
                ("lib", "export function f(){}"),
                (
                    "app",
                    &APP.replace("return ns.f()", "const ns=0; return ns.f()"),
                ),
            ],
        );
        assert!(rows(&cg, "run", "f")[0].starts_with("drop "));
        let using_app = APP.replace("return ns.f()", "using ns=null; return ns.f()");
        let cg = graph(
            ext,
            &[("lib", "export function f(){}"), ("app", &using_app)],
        );
        assert!(rows(&cg, "run", "f")[0].starts_with("drop "));
        if ext == "tsx" {
            let site = cg.calls.values().flatten().next().unwrap();
            assert_eq!(
                site.local_binding,
                JsLocalBinding::Unproven("not_callable".into())
            );
            assert!(!site.receiver_lexically_bound && !site.receiver_materialized);
        }
    }
}
#[test]
fn d14_jsx_specifier_tsx_sibling() {
    for ext in ["jsx", "tsx"] {
        let mut files = parsed(ext, &[("app", &APP.replace("'./lib'", "'./lib.jsx'"))]);
        files.extend(parsed("tsx", &[("lib", "export function f(){}")]));
        assert_eq!(
            rows(&CallGraph::build(&files), "run", "f"),
            [exact("tsx", "lib", "f", 1)]
        );
    }
}
#[test]
fn d10_opaque_conflict_never_authority() {
    for ext in ["jsx", "tsx"] {
        for lib in [
            "export const f=make();\nexport const f=make();",
            "export const f=make();\nexport function f() {}",
            "export function f() {}\nexport const f=make();",
        ] {
            let cg = graph(ext, &[("lib", lib), ("app", APP)]);
            assert!(!cg
                .js_ts_namespace_exports
                .get(&format!("lib.{ext}"))
                .is_some_and(|e| e.contains_key("f")));
        }
    }
}
#[test]
fn d8_serde_cache_and_incremental_epochs() {
    use prism::cpg::CodePropertyGraph;
    use prism::cpg_cache::{self, CacheResult};
    use std::collections::BTreeSet;
    for ext in ["jsx", "tsx"] {
        let mut previous: Option<CodePropertyGraph> = None;
        let mut last_sources: Option<(String, String)> = None;
        for (lib, app) in [
            (LIB, APP),
            (
                LIB,
                "import * as ns from './lib';\nexport function run() { ns=other; return ns.f(); }",
            ),
            (LIB, APP),
            (
                "function make(){ function f(){} return f; }\nexport const f=make();",
                APP,
            ),
            (LIB, APP),
        ] {
            let files = parsed(ext, &[("lib", lib), ("app", app)]);
            let full = CodePropertyGraph::build(&files);
            let fresh = rows(&full.call_graph, "run", "f");
            let cpg = if let Some(old) = previous {
                let (old_lib, old_app) = last_sources.as_ref().unwrap();
                let mut changed = BTreeSet::new();
                if old_lib != lib {
                    changed.insert(format!("lib.{ext}"));
                }
                if old_app != app {
                    changed.insert(format!("app.{ext}"));
                }
                let inc = CodePropertyGraph::build_incremental(
                    old.call_graph,
                    old.dfg,
                    &changed,
                    &files,
                    None,
                );
                assert_eq!(rows(&inc.call_graph, "run", "f"), fresh);
                assert_eq!(
                    inc.call_graph.js_ts_namespace_exports,
                    full.call_graph.js_ts_namespace_exports
                );
                inc
            } else {
                full
            };
            let sources = BTreeMap::from([
                (format!("lib.{ext}"), lib.to_string()),
                (format!("app.{ext}"), app.to_string()),
            ]);
            let hashes = cpg_cache::compute_file_hashes(&sources);
            let dir = tempfile::tempdir().unwrap();
            cpg_cache::save_cache(&cpg, &hashes, false, dir.path()).unwrap();
            let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
                panic!("cache miss");
            };
            assert_eq!(rows(&hit.call_graph, "run", "f"), fresh);
            let bytes = bincode::serialize(&cpg.call_graph).unwrap();
            let back: CallGraph = bincode::deserialize(&bytes).unwrap();
            assert_eq!(
                back.js_ts_namespace_exports,
                cpg.call_graph.js_ts_namespace_exports
            );
            previous = Some(cpg);
            last_sources = Some((lib.to_string(), app.to_string()));
        }
        // Same spelling, same file, different positions; proof cannot be reused.
        let cg=graph(ext,&[("lib",LIB),("app","import * as ns from './lib';\nexport function run(a=ns.f()){ var ns=0; return ns.f(); }")]);
        let sites: Vec<_> = cg
            .calls
            .values()
            .flatten()
            .filter(|s| s.caller.name == "run" && s.callee_name == "f")
            .collect();
        assert_eq!(sites.len(), 2);
        assert!(matches!(
            sites[0].local_binding,
            JsLocalBinding::NamespaceImport { .. }
        ));
        assert!(matches!(
            sites[1].local_binding,
            JsLocalBinding::Unproven(_)
        ));
        let mut old = serde_json::to_value(sites[0]).unwrap();
        old.as_object_mut().unwrap().remove("local_binding");
        assert_eq!(
            serde_json::from_value::<prism::call_graph::CallSite>(old)
                .unwrap()
                .local_binding,
            JsLocalBinding::Unchecked
        );
    }
}
#[test]
fn d10_barrel_conflict_cycle_final_depth_keeps_base() {
    for ext in ["jsx", "tsx"] {
        let alias = "function make(){ function f(){} return f; } export const f=make();";
        for sources in [
            vec![
                (
                    "lib",
                    "function h(){function f(){}} export * from './a'; export * from './b';",
                ),
                ("a", alias),
                ("b", "export function f(){}"),
                ("app", APP),
            ],
            vec![
                ("lib", "function h(){function f(){}} export {f} from './a';"),
                ("a", "export {f} from './lib';"),
                ("app", APP),
            ],
            vec![
                ("lib", "function h(){function f(){}} export * from './a';"),
                ("a", "export * from './b';"),
                ("b", "export * from './c';"),
                ("c", alias),
                ("app", APP),
            ],
        ]
        .into_iter()
        {
            let cg = graph(ext, &sources);
            assert_eq!(rows(&cg, "run", "f"), vec![exact(ext, "lib", "f", 1)]);
        }
    }
}
include!("js_binding_namespace/spec_r2.rs");
include!("js_binding_namespace/repair_r1.rs");
