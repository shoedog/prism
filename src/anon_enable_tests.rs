//! Enabled ownership and persisted-proof parity. Fixtures never execute JS.
use super::*;
use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
use petgraph::visit::EdgeRef;

fn sources(lib: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("a.ts".into(), "import {f} from './lib';\nfunction outer(){ sink(() => f(), (f) => f(), function* (p) { yield f(p); }); }".into()),
        ("lib.ts".into(), lib.into()),
    ])
}
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
fn calls(cpg: &CodePropertyGraph) -> Vec<u8> {
    let mut stored: Vec<_> = cpg
        .graph
        .edge_references()
        .filter(|e| matches!(e.weight(), CpgEdge::Call(_) | CpgEdge::Return(_)))
        .map(|e| {
            format!(
                "{:?}|{:?}|{:?}",
                cpg.node(e.source()),
                cpg.node(e.target()),
                e.weight()
            )
        })
        .collect();
    stored.sort();
    serde_json::to_vec(&(
        crate::navigation::queries::call_site_dump(&cpg.call_graph),
        stored,
    ))
    .unwrap()
}

#[test]
fn enabled_callers_include_anonymous_but_shadowed_sibling_drops() {
    let cpg = CodePropertyGraph::build(&parse(&sources("export function f(p) {return p;}")));
    let f = &cpg.call_graph.functions["f"][0];
    let sites: Vec<_> = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .filter(|s| s.callee_name == "f")
        .collect();
    assert_eq!(sites.len(), 3);
    let mut exact = 0;
    let mut dropped = 0;
    for site in sites {
        assert!(is_anonymous_owner_name(&site.caller.name));
        let out = cpg.call_graph.resolve_call_site_full(site);
        if out.resolved.is_empty() {
            dropped += 1;
        } else {
            assert_eq!(out.resolved.len(), 1);
            assert_eq!(out.resolved[0].target, f);
            assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
            exact += 1;
        }
    }
    assert_eq!((exact, dropped), (2, 1));
}

#[test]
fn anonymous_cold_full_and_partial_cache_hits_are_byte_identical() {
    for lib in [
        "export function f(p) {return p;}",
        "export function f(p) {return p;} eval('x');",
    ] {
        let before = sources("export function f(p) {return p;}");
        let before_files = parse(&before);
        let cold = CodePropertyGraph::build(&before_files);
        assert_eq!(anonymous(&cold.call_graph).len(), 3);
        let cache = tempfile::tempdir().unwrap();
        save_cache(&cold, &compute_file_hashes(&before), false, cache.path()).unwrap();
        let CacheResult::Hit(warm) = load_cache(&compute_file_hashes(&before), false, cache.path())
        else {
            panic!("not a full hit");
        };
        assert_eq!(calls(&cold), calls(&warm), "full hit grades/edges");
        let mut after = sources(lib);
        // Force a real partial hit even for the positive control.
        after
            .get_mut("lib.ts")
            .unwrap()
            .push_str("\n// dependency changed\n");
        let after_files = parse(&after);
        let CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } = load_cache(&compute_file_hashes(&after), false, cache.path())
        else {
            panic!("not a partial hit");
        };
        assert_eq!(changed_files, BTreeSet::from(["lib.ts".into()]));
        let partial = CodePropertyGraph::build_incremental(
            cached_call_graph,
            cached_dfg,
            &changed_files,
            &after_files,
            None,
        );
        let fresh = CodePropertyGraph::build(&after_files);
        assert_eq!(
            calls(&fresh),
            calls(&partial),
            "unchanged caller, changed proof dependency"
        );
        save_cache(&partial, &compute_file_hashes(&after), false, cache.path()).unwrap();
        let CacheResult::Hit(reloaded) =
            load_cache(&compute_file_hashes(&after), false, cache.path())
        else {
            panic!("not a reloaded full hit")
        };
        assert_eq!(calls(&fresh), calls(&reloaded));
    }
}

#[test]
fn generator_duplicate_formals_refuse_even_without_strict_directive() {
    for ext in ["js", "ts", "tsx"] {
        let files = files("sink(function* (p,p) {yield p;});\nsink(function* (q) {let q; yield q;});\nsink(function* (r) {yield r;});",ext);
        let p = files.values().next().unwrap();
        let generators: Vec<_> = p
            .all_functions()
            .into_iter()
            .filter(|n| n.kind() == "generator_function")
            .collect();
        assert_eq!(generators.len(), 3);
        assert!(p.dfg_owner_name(&generators[0]).is_none());
        assert!(p.dfg_owner_name(&generators[1]).is_none());
        assert!(p.dfg_owner_name(&generators[2]).is_some());
        let cpg = CodePropertyGraph::build(&files);
        assert_eq!(anonymous(&cpg.call_graph).len(), 3);
        for n in &generators[..2] {
            let id = identity(p, n, files.keys().next().unwrap()).unwrap();
            assert!(!cpg.node_indices().any(
                |i| matches!(cpg.node(i),CpgNode::Variable{function,..} if *function==id.name)
            ));
        }
    }
}

#[test]
fn d12_registered_bare_and_rest_lose_cfg_incomplete_edges_with_parenthesized_control() {
    for ext in ["js", "ts", "tsx"] {
        for (formal, refused) in [("x", true), ("...x", true), ("(x)", false)] {
            let formal = if formal == "...x" { "(...x)" } else { formal };
            let source = format!("import {{bridge}} from './bridge';\nsink({formal} => {{\n use(x);\n bridge(x);\n}});");
            let mut fs = files(&source, ext);
            for (name, source) in [
                ("bridge", "export function bridge(s) {\n Array(s);\n}"),
                // Keep this parameter-timing control independent of #443's property refusal.
                ("callee", "function Array(s) {\n sink(s)\n}"),
            ] {
                let path = format!("{name}.{ext}");
                fs.insert(
                    path.clone(),
                    ParsedFile::parse(&path, source, Language::from_path(&path).unwrap()).unwrap(),
                );
            }
            let cpg = CodePropertyGraph::build(&fs);
            let defs: Vec<_> = cpg.node_indices().filter(|&i| matches!(cpg.node(i), CpgNode::Variable { function, path, access: VarAccess::Def, .. } if is_anonymous_owner_name(function) && path.base == "x")).collect();
            assert_eq!(defs.len(), 1, "formal Def must remain");
            let edges: Vec<_> = cpg
                .graph
                .edges(defs[0])
                .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
                .collect();
            assert_eq!(edges.len(), if refused { 0 } else { 2 }, "{ext}/{formal}");
            assert!(edges.iter().all(|e| matches!(
                e.weight(),
                CpgEdge::DataFlow(crate::cpg::FlowConfidence::Exact)
            )));
        }
    }
}

#[test]
fn generator_inventory_preserves_named_claims_and_legacy_enclosing_seeds() {
    for ext in ["js", "ts", "tsx"] {
        let source = "function f(){}\nfunction outer(){\n class C{field=function* named(){f();};}\n sink(function* (p){yield f(p);});\n}";
        let fs = files(source, ext);
        let p = fs.values().next().unwrap();
        let mut manual = Vec::new();
        p.collect_functions_manual(p.tree.root_node(), &mut manual);
        let spans = |nodes: Vec<tree_sitter::Node<'_>>| {
            nodes
                .into_iter()
                .map(|n| (n.start_byte(), n.end_byte(), n.kind_id()))
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(spans(manual), spans(p.all_functions()));
        let generators: Vec<_> = p
            .all_functions()
            .into_iter()
            .filter(|n| n.kind() == "generator_function")
            .collect();
        assert_eq!(
            generators.len(),
            1,
            "only anonymous generator expressions enter the new inventory"
        );
        assert!(p.language.function_name(&generators[0]).is_none());
        let selected = p.enclosing_function(4).unwrap();
        assert_eq!(
            p.node_text(&p.language.function_name(&selected).unwrap()),
            "outer"
        );
        for cg in [
            CallGraph::build(&fs),
            CallGraph::build_skeleton(&fs),
            CallGraph::build_direct_subset(&fs, &fs.keys().cloned().collect()),
        ] {
            let sites: Vec<_> = cg
                .calls
                .values()
                .flatten()
                .filter(|s| s.callee_name == "f")
                .collect();
            assert_eq!(sites.len(), 2);
            assert_eq!(
                sites
                    .iter()
                    .filter(|s| s.line == 3 && s.caller.name == "outer")
                    .count(),
                1
            );
            assert_eq!(
                sites
                    .iter()
                    .filter(|s| s.line == 4 && is_anonymous_owner_name(&s.caller.name))
                    .count(),
                1
            );
        }
    }
}

#[test]
fn generator_formals_refuse_suspension_and_reserved_names_but_keep_nested_values() {
    for lang in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for prefix in ["", "async "] {
            for (params, admitted) in [
                ("yield", false),
                ("y\\u0069eld", false),
                ("x=yield 1", false),
                ("x=await q", false),
                ("x={ [yield 1](){} }", false),
                ("await", false), // grammar recovery, even where script syntax permits await
                ("a\\u0077ait", prefix.is_empty()),
                ("{yield:x}", true),
                ("x=function*(){yield 1}", true),
                ("x=async()=>await q", true),
                ("x={ m(){return function*(){yield 1};} }", true),
            ] {
                let source = format!(
                    "sink({prefix}function* ({params}) {{\n let local=1;\n yield local;\n}});"
                );
                let fs = BTreeMap::from([(
                    "a.js".into(),
                    ParsedFile::parse("a.js", &source, lang).unwrap(),
                )]);
                let p = &fs["a.js"];
                let node = p
                    .all_functions()
                    .into_iter()
                    .find(|n| n.kind() == "generator_function")
                    .expect("generator inventory");
                let owner = p.js_ts_synthetic_callable_name(&node).unwrap();
                assert_eq!(
                    p.dfg_owner_name(&node).is_some(),
                    admitted,
                    "{lang:?}/{prefix}/{params}"
                );
                let cpg = CodePropertyGraph::build(&fs);
                assert!(
                    cpg.call_graph.functions.contains_key(&owner),
                    "refusal must retain node"
                );
                let rows = cpg.node_indices().filter(|&i| matches!(cpg.node(i), CpgNode::Variable { function, .. } if *function == owner)).count();
                assert_eq!(rows > 0, admitted, "{lang:?}/{prefix}/{params}: {rows}");
            }
        }
    }
}
