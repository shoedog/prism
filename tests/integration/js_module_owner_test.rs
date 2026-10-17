//! Module call ownership. Source fixtures are parsed, never executed.
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
use prism::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
use prism::languages::Language;
use prism::resolution::ResolutionConfidence::Exact;
use std::collections::{BTreeMap, BTreeSet};

const MODULE: &str = "<anon@module>";
fn parse(sources: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    sources
        .iter()
        .map(|(p, s)| {
            (
                p.clone(),
                ParsedFile::parse(p, s, Language::from_path(p).unwrap()).unwrap(),
            )
        })
        .collect()
}
fn fixture(source: &str, ext: &str) -> BTreeMap<String, ParsedFile> {
    parse(&BTreeMap::from([(format!("a.{ext}"), source.into())]))
}
fn sites(graph: &CallGraph) -> Vec<(&str, usize, usize, &str)> {
    graph
        .calls
        .values()
        .flatten()
        .map(|s| {
            (
                s.caller.name.as_str(),
                s.start_byte,
                s.end_byte,
                s.callee_name.as_str(),
            )
        })
        .collect()
}

#[test]
fn shapes_and_builders() {
    for ext in ["js", "ts", "tsx"] {
        let source = "function f() {}\nf();\nexport const a = f();\nexport let b = f();\nexport var c = f();\nconst d = f(); let e = f(); var g = f();\nexport default f();\nif (f()) { f(); }\nfor (let i=f(); f(); f()) { f(); break; }\n(() => { f(); })();\nconst nested = () => f();\nconst o = { m() { f(); } };\n";
        let files = fixture(source, ext);
        let full = CallGraph::build(&files);
        let owner = &full.functions[MODULE][0];
        assert_eq!(owner.file, format!("a.{ext}"));
        assert_eq!(owner.start_line, 1);
        let module_sites: Vec<_> = full.calls[owner].iter().collect();
        assert_eq!(module_sites.len(), 15, "{ext}: {module_sites:?}");
        for site in module_sites.iter().filter(|s| s.callee_name == "f") {
            let out = full.resolve_call_site_full(site);
            assert_eq!(out.resolved.len(), 1, "{site:?}: {out:?}");
            assert_eq!(out.resolved[0].confidence, Exact);
            assert_eq!(out.resolved[0].target.name, "f");
        }
        let nested: Vec<_> = full
            .calls
            .values()
            .flatten()
            .filter(|s| s.callee_name == "f" && s.caller.name != MODULE)
            .collect();
        assert_eq!(nested.len(), 3);
        assert!(nested
            .iter()
            .any(|s| s.caller.name.starts_with("<anon@") && s.caller.name != MODULE));
        for other in [
            CallGraph::build_skeleton(&files),
            CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
        ] {
            assert_eq!(sites(&other), sites(&full));
            assert_eq!(other.functions[MODULE], full.functions[MODULE]);
        }
        assert!(!CallGraph::build_direct_subset(&files, &BTreeSet::new())
            .functions
            .contains_key(MODULE));
        let cpg = CodePropertyGraph::build(&files);
        assert!(cpg.node_indices().any(|i| matches!(cpg.node(i), CpgNode::Function { name, start_byte, end_byte, .. } if name == MODULE && *start_byte == 0 && *end_byte == source.len())));
        assert!(cpg.function_at(&format!("a.{ext}"), 2).is_none());
    }
    let files = fixture("function f() {}\nexport = f();", "ts");
    let graph = CallGraph::build(&files);
    assert_eq!(graph.calls[&graph.functions[MODULE][0]].len(), 1);
    let py = BTreeMap::from([(
        "a.py".into(),
        ParsedFile::parse("a.py", "f()", Language::Python).unwrap(),
    )]);
    assert!(!CallGraph::build(&py).functions.contains_key(MODULE));
    assert!(Language::from_path("a.mts").is_none());
    assert!(Language::from_path("a.cts").is_none());
}

#[test]
fn exclusions_and_named_claim_precedence() {
    for ext in ["js", "ts", "tsx"] {
        let source = "function f() {}\nf();\nclass C {\n field = f();\n static field = f();\n static { f(); }\n [f()]() { f(); }\n callback = () => f();\n}\nconst object = { [f()]: 1 };\nnew C();\nfunction outer() { class D { field = f(); } }\n";
        let files = fixture(source, ext);
        for graph in [
            CallGraph::build(&files),
            CallGraph::build_skeleton(&files),
            CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
        ] {
            let module = &graph.functions[MODULE][0];
            assert_eq!(
                graph.calls[module]
                    .iter()
                    .map(|s| s.line)
                    .collect::<Vec<_>>(),
                [2]
            );
            let calls: Vec<_> = graph
                .calls
                .values()
                .flatten()
                .filter(|s| s.callee_name == "f")
                .collect();
            assert_eq!(
                calls.len(),
                5,
                "legacy method/key and class callback owners remain: {calls:?}"
            );
            assert!(calls.iter().any(|s| s.line == 7 && s.caller.name != MODULE));
            assert!(calls
                .iter()
                .any(|s| s.line == 8 && s.caller.name == "callback"));
            assert!(calls
                .iter()
                .any(|s| s.line == 12 && s.caller.name == "outer"));
        }
    }
}

#[test]
fn barrier_vetoes_and_binding_refusals() {
    for suffix in ["", "with (obj) {}", "eval('x');", "const \\u0078 = 1;"] {
        let files = fixture(&format!("function f() {{}}\nf();\n{suffix}"), "js");
        let graph = CallGraph::build(&files);
        let site = graph.calls[&graph.functions[MODULE][0]]
            .iter()
            .find(|s| s.callee_name == "f")
            .unwrap();
        let out = graph.resolve_call_site_full(site);
        assert_eq!(
            out.resolved.iter().any(|r| r.confidence == Exact),
            suffix.is_empty(),
            "{suffix}: {out:?}"
        );
    }
    for source in [
        "const f = external; f();",
        "let f = () => 1; f = external; f();",
        "unknown();",
    ] {
        let files = fixture(source, "ts");
        let graph = CallGraph::build(&files);
        let records = &graph.calls[&graph.functions[MODULE][0]];
        assert_eq!(records.len(), 1);
        for site in records {
            assert!(!graph
                .resolve_call_site_full(site)
                .resolved
                .iter()
                .any(|r| r.confidence == Exact));
        }
    }
}

fn dump(cpg: &CodePropertyGraph) -> Vec<u8> {
    use petgraph::visit::EdgeRef;
    let mut edges: Vec<_> = cpg
        .graph
        .edge_references()
        .filter(|e| {
            matches!(
                e.weight(),
                CpgEdge::Call(_) | CpgEdge::Return(_) | CpgEdge::DataFlow(_)
            )
        })
        .map(|e| {
            format!(
                "{:?}|{:?}|{:?}",
                cpg.node(e.source()),
                cpg.node(e.target()),
                e.weight()
            )
        })
        .collect();
    edges.sort();
    serde_json::to_vec(&(
        prism::navigation::queries::call_site_dump(&cpg.call_graph),
        edges,
    ))
    .unwrap()
}

#[test]
fn cache_parity() {
    for threads in [1, 4] {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let before = BTreeMap::from([
                    (
                        "a.ts".into(),
                        "import { f } from './lib';\nconst x = f();\n".into(),
                    ),
                    ("lib.ts".into(), "export function f() {}".into()),
                ]);
                let cold = CodePropertyGraph::build(&parse(&before));
                assert_eq!(
                    cold.call_graph.calls[&cold.call_graph.functions[MODULE][0]].len(),
                    1
                );
                let cache = tempfile::tempdir().unwrap();
                save_cache(&cold, &compute_file_hashes(&before), false, cache.path()).unwrap();
                let CacheResult::Hit(warm) =
                    load_cache(&compute_file_hashes(&before), false, cache.path())
                else {
                    panic!("full hit required")
                };
                assert_eq!(dump(&cold), dump(&warm));
                for (path, replacement) in [
                    ("lib.ts", "export function f() {} eval('x');"),
                    ("a.ts", "import { f } from './lib';\nf(); f();"),
                ] {
                    let mut after = before.clone();
                    after.insert(path.into(), replacement.into());
                    let CacheResult::PartialHit {
                        cached_call_graph,
                        cached_dfg,
                        changed_files,
                    } = load_cache(&compute_file_hashes(&after), false, cache.path())
                    else {
                        panic!("partial hit required")
                    };
                    assert_eq!(changed_files, BTreeSet::from([path.into()]));
                    let files = parse(&after);
                    let partial = CodePropertyGraph::build_incremental(
                        cached_call_graph,
                        cached_dfg,
                        &changed_files,
                        &files,
                        None,
                    );
                    let fresh = CodePropertyGraph::build(&files);
                    assert_eq!(dump(&partial), dump(&fresh), "{threads}/{path}");
                    let cache_after = tempfile::tempdir().unwrap();
                    save_cache(
                        &partial,
                        &compute_file_hashes(&after),
                        false,
                        cache_after.path(),
                    )
                    .unwrap();
                    let CacheResult::Hit(reloaded) =
                        load_cache(&compute_file_hashes(&after), false, cache_after.path())
                    else {
                        panic!("reloaded hit required")
                    };
                    assert_eq!(dump(&fresh), dump(&reloaded));
                }
            });
    }
}

#[test]
fn navigation_reports_module_level_without_function_symbol() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.js"), "function f() {}\nf();\n").unwrap();
    let mut options = prism::api::NavOptions::default();
    options.no_cache = true;
    let session = prism::api::nav_session(root.path(), &options).unwrap();
    let evidence = prism::navigation::queries::callers(&session, Some("f"), None, None, 1).unwrap();
    assert_eq!(evidence.items.len(), 1);
    let item = &evidence.items[0];
    assert!(
        matches!(item.symbol, Some(prism::navigation::types::SymbolRef::Statement { ref kind, line: 2, .. }) if kind == "module-level")
    );
    assert!(format!("{:?}", item.why).contains("module-level"));
    assert!(prism::navigation::seed::resolve_fn(&session, Some(MODULE), None, None).is_err());
    assert!(prism::navigation::seed::resolve_fn(&session, None, None, Some("a.js:2")).is_err());
    let visible = prism::navigation::queries::nodes_at(&session, "a.js", 2);
    assert!(!format!("{visible:?}").contains(MODULE));
    let ego = prism::navigation::queries::ego_graph(&session, Some("f"), None, None, 1, &["Call"])
        .unwrap();
    let graph = ego.graph.unwrap();
    assert!(graph.nodes.iter().any(|n| matches!(n.symbol, Some(prism::navigation::types::SymbolRef::Statement { ref kind, .. }) if kind == "module-level")));
    assert!(!format!("{graph:?}").contains(MODULE));
}

#[test]
fn tsx_expression_and_top_level_await() {
    for (ext, source) in [
        ("tsx", "function f() {}\nconst view = <div>{f()}</div>;"),
        ("ts", "function f() {}\nawait f();"),
        ("js", "function f() {}\nawait f();"),
    ] {
        let graph = CallGraph::build(&fixture(source, ext));
        let calls = &graph.calls[&graph.functions[MODULE][0]];
        let site = calls.iter().find(|s| s.callee_name == "f").unwrap();
        let out = graph.resolve_call_site_full(site);
        assert_eq!(out.resolved.len(), 1);
        assert_eq!(out.resolved[0].confidence, Exact);
    }
}

#[test]
fn navigation_sidecar_cache_parity() {
    for threads in ["1", "4"] {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let file = root.path().join("a.js");
        std::fs::write(&file, "function f() {}\nf();").unwrap();
        let query = |cached| {
            let mut command = assert_cmd::Command::cargo_bin("prism").unwrap();
            command
                .env("RAYON_NUM_THREADS", threads)
                .env("PRISM_NAV_EDGE_CACHE_LOAD_DIRTY", "1")
                .arg("nav");
            if cached {
                command.arg("--cache-dir").arg(cache.path());
            } else {
                command.arg("--no-cache");
            }
            command
                .args(["callers", "--symbol", "f", "--format", "json", "--repo"])
                .arg(root.path());
            command.assert().success().get_output().clone()
        };
        let cold = query(true);
        let warm = query(true);
        assert_eq!(cold.stdout, warm.stdout);
        assert!(String::from_utf8_lossy(&cold.stderr).contains("CPG cache miss"));
        assert!(!String::from_utf8_lossy(&warm.stderr).contains("rebuilding"));
        std::fs::write(&file, "function f() {}\nf(); f();").unwrap();
        let partial = query(true);
        assert!(String::from_utf8_lossy(&partial.stderr).contains("partial CPG cache hit"));
        assert_ne!(cold.stdout, partial.stdout);
        assert_eq!(partial.stdout, query(false).stdout);
    }
}
