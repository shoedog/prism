//! Source names, not callable display names, establish receiver self bindings.
use prism::{
    ast::ParsedFile,
    call_graph::CallGraph,
    cpg::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess},
    languages::Language,
    resolution::ResolutionConfidence,
};
use std::collections::{BTreeMap, BTreeSet};

const LANGUAGES: [(Language, &str); 3] = [
    (Language::JavaScript, "js"),
    (Language::TypeScript, "ts"),
    (Language::Tsx, "tsx"),
];
const IMPORT: &str = "import * as ns from './origin';";
const ORIGIN: &str = "export function item(input) { return input; }";

fn files(lang: Language, ext: &str, app: &str) -> BTreeMap<String, ParsedFile> {
    [
        ("origin", ORIGIN),
        ("app", app),
        ("decoy", "function item(input) { return input; }"),
    ]
    .into_iter()
    .map(|(p, s)| {
        let p = format!("{p}.{ext}");
        let f = ParsedFile::parse(&p, s, lang).unwrap();
        assert!(!f.tree.root_node().has_error(), "{p}: {s}");
        (p, f)
    })
    .collect()
}

fn receiver<'a>(n: tree_sitter::Node<'a>, parsed: &ParsedFile) -> Option<tree_sitter::Node<'a>> {
    if n.kind() == "member_expression"
        && n.child_by_field_name("property")
            .is_some_and(|p| parsed.node_text(&p) == "item")
    {
        return n.child_by_field_name("object");
    }
    let mut cursor = n.walk();
    let found = n
        .named_children(&mut cursor)
        .find_map(|n| receiver(n, parsed));
    found
}

fn check(id: &str, body: &str, supported: bool, graph: bool) {
    let mut failures = Vec::new();
    for (lang, ext) in LANGUAGES {
        let app = format!("{IMPORT}\n{body}");
        let files = files(lang, ext, &app);
        let parsed = &files[&format!("app.{ext}")];
        let bound = parsed.js_ts_receiver_lexically_bound_at_call(
            &parsed.tree.root_node(),
            receiver(parsed.tree.root_node(), parsed),
        );
        println!("RECEIVER_SELF_RAW {id}/{ext} bound={bound}");
        if bound == supported {
            failures.push(format!("{ext} raw bound={bound}"));
        }
        // Generator property callbacks are raw-guard tests, not eager call-site gains.
        if !graph {
            continue;
        }
        let mut subset = CallGraph::build_direct_subset(&files, &files.keys().cloned().collect());
        subset.apply_js_export_resolution();
        for (mode, cg) in [("full", CallGraph::build(&files)), ("subset", subset)] {
            let sites: Vec<_> = cg
                .calls
                .values()
                .flatten()
                .filter(|s| s.caller.file == format!("app.{ext}") && s.callee_name == "item")
                .collect();
            let expected_sites = if matches!(
                id,
                "receiver_self_nested_declaration" | "receiver_self_outer_parameter"
            ) {
                2
            } else {
                1
            };
            assert_eq!(sites.len(), expected_sites, "{id}/{ext}/{mode}");
            for (index, site) in sites.into_iter().enumerate() {
                let exact: Vec<_> = cg
                    .resolve_call_site_full(site)
                    .resolved
                    .into_iter()
                    .filter(|r| r.confidence == ResolutionConfidence::Exact)
                    .map(|r| {
                        (
                            r.target.file.clone(),
                            r.target.name.clone(),
                            r.target.start_line,
                        )
                    })
                    .collect();
                let want = if supported {
                    vec![(format!("origin.{ext}"), "item".into(), 1)]
                } else {
                    vec![]
                };
                println!("RECEIVER_SELF {id}/{ext}/{mode}/{index} bound={bound} {exact:?}");
                if exact != want {
                    failures.push(format!("{ext}/{mode}: {exact:?}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{id}: {}", failures.join("; "));
}

macro_rules! case {
    ($id:ident, $source:expr, $supported:expr) => {
        #[test]
        fn $id() {
            check(stringify!($id), $source, $supported, true);
        }
    };
}
macro_rules! raw_case {
    ($id:ident, $source:expr, $supported:expr) => {
        #[test]
        fn $id() {
            check(stringify!($id), $source, $supported, false);
        }
    };
}
case!(
    receiver_self_anonymous_property,
    "const obj={ns:null}; obj.ns=function(value){return ns.item(value);};",
    true
);
case!(
    receiver_self_async_property,
    "const obj={ns:null}; obj.ns=async function(value){return ns.item(value);};",
    true
);
raw_case!(
    receiver_self_generator_property,
    "const obj={ns:null}; obj.ns=function*(value){yield ns.item(value);};",
    true
);
raw_case!(
    receiver_self_async_generator_property,
    "const obj={ns:null}; obj.ns=async function*(value){yield ns.item(value);};",
    true
);
case!(
    receiver_self_object_pair,
    "const obj={ns:function(value){return ns.item(value);}};",
    true
);
case!(
    receiver_self_nested_callback,
    "const obj={ns:null}; obj.ns=function(){return function(value){return ns.item(value);};};",
    true
);
case!(
    receiver_self_different_property,
    "const obj={callback:null}; obj.callback=function(value){return ns.item(value);};",
    true
);
case!(
    receiver_self_arrow_property,
    "const obj={ns:null}; obj.ns=(value)=>ns.item(value);",
    true
);
case!(
    receiver_self_object_method,
    "const obj={ns(value){return ns.item(value);}};",
    true
);
case!(
    receiver_self_different_explicit_name,
    "const obj={ns:null}; obj.ns=function callback(value){return ns.item(value);};",
    true
);
case!(
    receiver_self_explicit_name,
    "const obj={ns:null}; obj.ns=function ns(value){return ns.item(value);};",
    false
);
raw_case!(
    receiver_self_explicit_generator,
    "const obj={ns:null}; obj.ns=function* ns(value){yield ns.item(value);};",
    false
);
case!(
    receiver_self_named_nested_callback,
    "const obj={ns:null}; obj.ns=function ns(){return function(value){return ns.item(value);};};",
    false
);
case!(
    receiver_self_parameter,
    "const obj={ns:null}; obj.ns=function(ns){return ns.item(1);};",
    false
);
case!(
    receiver_self_destructured_parameter,
    "const obj={ns:null}; obj.ns=function({ns}){return ns.item(1);};",
    false
);
case!(
    receiver_self_local_before,
    "const obj={ns:null}; obj.ns=function(value){const ns={}; return ns.item(value);};",
    false
);
case!(
    receiver_self_local_after,
    "const obj={ns:null}; obj.ns=function(value){ns.item(value); let ns;};",
    false
);
case!(
    receiver_self_var_after,
    "const obj={ns:null}; obj.ns=function(value){ns.item(value); var ns;};",
    false
);
case!(
    receiver_self_catch,
    "const obj={ns:null}; obj.ns=function(value){try{}catch(ns){return ns.item(value);}};",
    false
);
case!(
    receiver_self_loop_binding,
    "const obj={ns:null}; obj.ns=function(value){for(const ns of []){ns.item(value);}};",
    false
);
case!(
    receiver_self_nested_declaration,
    "function run(value){function ns(){return ns.item(value);}}",
    false
);
case!(
    receiver_self_outer_parameter,
    "function run(ns){const obj={ns:null}; obj.ns=function(value){return ns.item(value);};}",
    false
);
case!(
    receiver_self_unrelated_block,
    "const obj={ns:null}; obj.ns=function(value){{let ns;}return ns.item(value);};",
    true
);
case!(
    receiver_self_unrelated_callable,
    "const obj={ns:null}; obj.ns=function(value){function other(ns){}return ns.item(value);};",
    true
);

fn flow(
    cpg: &CodePropertyGraph,
    files: &BTreeMap<String, ParsedFile>,
    app: &str,
) -> BTreeSet<(String, String)> {
    cpg.graph
        .edge_indices()
        .filter_map(|edge| {
            if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
                return None;
            }
            let (a, b) = cpg.graph.edge_endpoints(edge).unwrap();
            if let (
                CpgNode::Variable {
                    file,
                    path,
                    access: VarAccess::Use,
                    ..
                },
                CpgNode::Variable {
                    file: to,
                    function,
                    path: param,
                    access: VarAccess::Def,
                    start_byte,
                    end_byte,
                    ..
                },
            ) = (cpg.node(a), cpg.node(b))
            {
                if file == app && path.to_string() == "value" && to != app {
                    assert_eq!(&files[to].source[*start_byte..*end_byte], "input");
                    assert_eq!(param.to_string(), "input");
                    return Some((to.clone(), function.clone()));
                }
            }
            None
        })
        .collect()
}

#[test]
fn receiver_self_source_epochs() {
    for (lang, ext) in LANGUAGES {
        let mut previous: Option<CodePropertyGraph> = None;
        let mut previous_sources: BTreeMap<String, String> = BTreeMap::new();
        for (epoch, (name, expected)) in [
            (" callback", true),
            (" ns", false),
            ("", true),
            (" callback", true),
            (" ns", false),
            ("", true),
        ]
        .into_iter()
        .enumerate()
        {
            let app=format!("{IMPORT}\nconst obj={{ns:null}}; obj.ns=function{name}(value){{return ns.item(value);}};");
            let files = files(lang, ext, &app);
            let app_path = format!("app.{ext}");
            let want = if expected {
                BTreeSet::from([(format!("origin.{ext}"), "item".into())])
            } else {
                BTreeSet::new()
            };
            let full = CodePropertyGraph::build(&files);
            assert_eq!(epoch_targets(&full, &app_path), want, "{ext}/full/{epoch}");
            // Positive namespace boundary dataflow remains an observed gap, not
            // authority claimed by this receiver-lookup repair.
            let observed_flow = flow(&full, &files, &app_path);
            println!("RECEIVER_EPOCH {ext}/full/{epoch} targets={want:?} flow={observed_flow:?}");
            if !expected {
                assert!(observed_flow.is_empty());
            }
            let current = if let Some(old) = previous {
                let changed: BTreeSet<_> = files
                    .iter()
                    .filter(|(p, f)| previous_sources.get(*p) != Some(&f.source))
                    .map(|(p, _)| p.clone())
                    .collect();
                assert_eq!(changed, BTreeSet::from([app_path.clone()]));
                let incremental = CodePropertyGraph::build_incremental(
                    old.call_graph,
                    old.dfg,
                    &changed,
                    &files,
                    None,
                );
                assert_eq!(
                    epoch_targets(&incremental, &app_path),
                    want,
                    "{ext}/incremental/{epoch}"
                );
                let observed_flow = flow(&incremental, &files, &app_path);
                println!("RECEIVER_EPOCH {ext}/incremental/{epoch} targets={want:?} flow={observed_flow:?}");
                if !expected {
                    assert!(observed_flow.is_empty());
                }
                incremental
            } else {
                full
            };
            previous = Some(current);
            previous_sources = files
                .iter()
                .map(|(p, f)| (p.clone(), f.source.clone()))
                .collect();
        }
    }
}

fn epoch_targets(cpg: &CodePropertyGraph, app: &str) -> BTreeSet<(String, String)> {
    let sites: Vec<_> = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.file == app && s.callee_name == "item")
        .collect();
    assert_eq!(sites.len(), 1);
    cpg.call_graph
        .resolve_call_site_full(sites[0])
        .resolved
        .into_iter()
        .filter(|r| r.confidence == ResolutionConfidence::Exact)
        .map(|r| {
            assert_eq!(r.target.start_line, 1);
            (r.target.file.clone(), r.target.name.clone())
        })
        .collect()
}

fn write_case(id: &str, callback: &str, invoke: &str, supported: bool) {
    let mut failures = Vec::new();
    for (lang, ext) in LANGUAGES {
        let path = format!("app.{ext}");
        let source=format!("class Client {{ item(input) {{ return input; }} }}\nfunction run(value) {{ const client=new Client(); const obj={{client:null}}; obj.client={callback}; {invoke} return client.item(value); }}");
        let parsed = ParsedFile::parse(&path, &source, lang).unwrap();
        assert!(!parsed.tree.root_node().has_error());
        let files = BTreeMap::from([(path.clone(), parsed)]);
        for (mode, cg) in [
            ("full", CallGraph::build(&files)),
            (
                "subset",
                CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
            ),
        ] {
            let sites: Vec<_> = cg
                .calls
                .values()
                .flatten()
                .filter(|s| s.caller.file == path && s.callee_name == "item")
                .collect();
            assert_eq!(sites.len(), 1);
            let exact: Vec<_> = cg
                .resolve_call_site_full(sites[0])
                .resolved
                .into_iter()
                .filter(|r| r.confidence == ResolutionConfidence::Exact)
                .map(|r| {
                    (
                        r.target.file.clone(),
                        r.target.name.clone(),
                        r.target.start_line,
                    )
                })
                .collect();
            println!("RECEIVER_WRITE {id}/{ext}/{mode} {exact:?}");
            let want = if supported {
                vec![(path.clone(), "item".into(), 1)]
            } else {
                vec![]
            };
            if exact != want {
                failures.push(format!("{ext}/{mode} {exact:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{id}: {}", failures.join("; "));
}
#[test]
fn receiver_self_anonymous_member_write() {
    write_case(
        "anonymous-member",
        "function(){client.item=()=>0;}",
        "obj.client();",
        false,
    );
}
#[test]
fn receiver_self_named_member_write() {
    write_case(
        "named-member",
        "function client(){client.item=()=>0;}",
        "obj.client();",
        true,
    );
}
#[test]
fn receiver_self_parameter_member_write() {
    write_case(
        "parameter-member",
        "function(client){client.item=()=>0;}",
        "obj.client({});",
        true,
    );
}
#[test]
fn receiver_self_different_display_member_write() {
    write_case(
        "different-display-member",
        "function callback(){client.item=()=>0;}",
        "obj.client();",
        false,
    );
}

fn typed_case(id: &str, body: &str, expected_sites: usize, supported: bool) {
    for (lang, ext) in [(Language::TypeScript, "ts"), (Language::Tsx, "tsx")] {
        let path = format!("app.{ext}");
        let source = format!("class Client {{ item(input) {{ return input; }} }}\n{body}");
        let parsed = ParsedFile::parse(&path, &source, lang).unwrap();
        assert!(!parsed.tree.root_node().has_error());
        let files = BTreeMap::from([(path.clone(), parsed)]);
        for (mode, cg) in [
            ("full", CallGraph::build(&files)),
            (
                "subset",
                CallGraph::build_direct_subset(&files, &files.keys().cloned().collect()),
            ),
        ] {
            let sites: Vec<_> = cg
                .calls
                .values()
                .flatten()
                .filter(|s| s.caller.file == path && s.callee_name == "item")
                .collect();
            assert_eq!(sites.len(), expected_sites, "{id}/{ext}/{mode}");
            for (index, site) in sites.into_iter().enumerate() {
                let exact: Vec<_> = cg
                    .resolve_call_site_full(site)
                    .resolved
                    .into_iter()
                    .filter(|r| r.confidence == ResolutionConfidence::Exact)
                    .map(|r| {
                        (
                            r.target.file.clone(),
                            r.target.name.clone(),
                            r.target.start_line,
                        )
                    })
                    .collect();
                let want = if supported {
                    vec![(path.clone(), "item".into(), 1)]
                } else {
                    vec![]
                };
                println!("RECEIVER_TYPED {id}/{ext}/{mode}/{index} {exact:?}");
                assert_eq!(exact, want, "{id}/{ext}/{mode}/{index}");
            }
        }
    }
}
#[test]
fn receiver_self_typed_outer_parameter() {
    typed_case("typed-outer","function run(client:Client){const obj={client:null};obj.client=function(value){return client.item(value);};}",2,true);
}
#[test]
fn receiver_self_typed_inner_parameter() {
    typed_case(
        "typed-inner",
        "const obj={client:null};obj.client=function(client:Client){return client.item(1);};",
        1,
        true,
    );
}
#[test]
fn receiver_self_typed_named_shadow() {
    typed_case("typed-named-shadow","function run(client:Client){const obj={client:null};obj.client=function client(value){return client.item(value);};}",2,false);
}
#[test]
fn receiver_self_constructor_same_display() {
    typed_case("constructor-same-display","const obj={client:null};obj.client=function(value){const client=new Client();return client.item(value);};",1,true);
}
#[test]
fn receiver_self_constructor_identifier_display() {
    typed_case("constructor-identifier-display","const obj={Client:null};obj.Client=function(value){const client=new Client();return client.item(value);};",1,true);
}
#[test]
fn receiver_self_constructor_identifier_named_shadow() {
    typed_case("constructor-identifier-named-shadow","const obj={Client:null};obj.Client=function Client(value){const client=new Client();return client.item(value);};",1,false);
}
#[test]
fn receiver_self_constructor_identifier_parameter_shadow() {
    typed_case("constructor-identifier-parameter-shadow","const obj={Client:null};obj.Client=function(Client){const client=new Client();return client.item(1);};",1,false);
}
