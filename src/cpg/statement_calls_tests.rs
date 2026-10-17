use super::*;
use crate::ast::{call_names::take_query_count, ParsedFile};
use crate::languages::Language;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::BTreeMap;

type Statements = (
    Vec<String>,
    BTreeMap<(String, usize), NodeIndex>,
    BTreeMap<(String, usize), Vec<NodeIndex>>,
);

fn statements(files: &BTreeMap<String, ParsedFile>, reference: bool) -> Statements {
    let mut graph = DiGraph::new();
    let mut locations = BTreeMap::new();
    let index = if reference {
        CodePropertyGraph::assemble_step7_reference(files, &mut graph, &mut locations)
    } else {
        CodePropertyGraph::assemble_step7(files, &mut graph, &mut locations)
    };
    (
        graph.node_weights().map(|n| format!("{n:?}")).collect(),
        index,
        locations,
    )
}

#[test]
fn step7_queries_at_most_once_per_file() {
    let mut files = BTreeMap::new();
    for n in [1, 32, 128] {
        let path = format!("batch-cost-{n}.js");
        let source = format!("function f() {{\n{}\n}}", "first(); second();\n".repeat(n));
        files.insert(
            path.clone(),
            ParsedFile::parse(&path, &source, Language::JavaScript).unwrap(),
        );
    }
    // Exercise the real parallel Step 7 in a dedicated pool; the observer is
    // keyed by path, so unrelated concurrent tests cannot affect these counts.
    let nodes = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| statements(&files, false));
    assert_eq!(nodes.0.len(), 161);
    let counts: Vec<_> = files
        .keys()
        .map(|p| (p.clone(), take_query_count(p)))
        .collect();
    assert!(
        counts.iter().all(|(_, n)| *n == 1),
        "full-file query executions: {counts:?}"
    );
}

#[test]
fn step7_identity_matches_main() {
    let sources = [
        ("multi.js", "function f() {\n outer(inner()); second();\n (() => { nested(); })(); tail();\n const x = assigned();\n x = replaced();\n if (condition()) { branch(); }\n while (check()) { looped(); }\n return returned();\n 42;\n}\n", Language::JavaScript),
        ("view.jsx", "function view() {\n <Widget value={inner()} />; later();\n tag`hello ${inside()}`; after();\n new Widget();\n}\n", Language::JavaScript),
        ("view.tsx", "function view() {\n <Widget value={inner()} />; later();\n tag`hello ${inside()}`; after();\n new Widget();\n}\n", Language::Tsx),
        ("typed.ts", "function f() {\n outer(inner()); second();\n new Widget();\n tag`hello ${inside()}`;\n}\n", Language::TypeScript),
        ("python.py", "def f():\n    outer(inner()); second()\n    return last()\n", Language::Python),
        ("rust.rs", "fn f() {\n outer(inner()); second();\n println!(\"x\");\n return last();\n}\n", Language::Rust),
        ("go.go", "package p\nfunc f() {\n outer(inner()); second()\n return\n}\n", Language::Go),
        ("c.c", "void f() {\n outer(inner()); second();\n goto end;\n end: return;\n}\n", Language::C),
        ("cpp.cpp", "void f() {\n outer(inner()); second();\n}\n", Language::Cpp),
        ("java.java", "class C { void f() {\n outer(inner()); second();\n new C();\n} }\n", Language::Java),
        ("lua.lua", "function f()\n outer(inner()); second()\nend\n", Language::Lua),
        ("bash.sh", "f() {\n first; second\n}\n", Language::Bash),
        ("terraform.tf", "resource \"x\" \"y\" {\n value = outer(inner())\n}\n", Language::Terraform),
    ];
    for (path, source, lang) in sources {
        let files = BTreeMap::from([(
            path.to_string(),
            ParsedFile::parse(path, source, lang).unwrap(),
        )]);
        let actual = statements(&files, false);
        assert_eq!(actual, statements(&files, true), "{path}");
        if lang == Language::Terraform {
            assert!(
                actual.0.is_empty(),
                "Terraform has no Step-7 statement nodes"
            );
        } else {
            assert!(!actual.0.is_empty(), "no statement coverage: {path}");
        }
    }
}

#[test]
fn step7_empty_and_no_call_files() {
    for lang in Language::all() {
        let path = format!("batch-empty-{lang:?}");
        let files = BTreeMap::from([(path.clone(), ParsedFile::parse(&path, "", lang).unwrap())]);
        assert_eq!(statements(&files, false), statements(&files, true));
        assert!(statements(&files, false).0.is_empty());
        assert_eq!(take_query_count(&path), 0);
    }
    let path = "batch-no-calls.js";
    let files = BTreeMap::from([(
        path.into(),
        ParsedFile::parse(
            path,
            "function f() {\n return 1;\n}\n",
            Language::JavaScript,
        )
        .unwrap(),
    )]);
    assert_eq!(statements(&files, false), statements(&files, true));
    assert_eq!(take_query_count(path), 0);
}
