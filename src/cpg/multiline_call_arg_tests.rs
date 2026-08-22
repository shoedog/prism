//! Regression coverage for Step-5b argument identity across multi-line calls.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn languages() -> [(Language, &'static str); 4] {
    [
        (Language::Python, "m.py"),
        (Language::Go, "m.go"),
        (Language::Rust, "m.rs"),
        (Language::JavaScript, "m.js"),
    ]
}

pub(super) fn fixture_source(language: Language, multiline_wrapper: bool) -> String {
    let wrapper = match (language, multiline_wrapper) {
        (Language::Python, false) => "wrapped((wrapped_user))",
        (Language::Python, true) => {
            "wrapped(\n        (\n            wrapped_user\n        )\n    )"
        }
        (Language::Go, false) => "wrapped((wrapped_user))",
        (Language::Go, true) => "wrapped(\n        (\n            wrapped_user\n        ),\n    )",
        (Language::Rust, false) => "wrapped(&wrapped_user);",
        (Language::Rust, true) => "wrapped(\n        &\n        wrapped_user,\n    );",
        (Language::JavaScript, false) => "wrapped(...[wrapped_user]);",
        (Language::JavaScript, true) => {
            "wrapped(\n        ...[\n            wrapped_user\n        ],\n    );"
        }
        _ => unreachable!("fixture languages are fixed above"),
    };
    let template = match language {
        Language::Python => {
            r#"def direct(p):
    sink(p)

def wrapped(p):
    sink(p)

def mixed(a, b):
    sink(a)
    sink(b)

def mutated(a, b, c):
    sink(a)
    sink(b)
    sink(c)

def repeated(a, b):
    sink(a)
    sink(b)

def inner(p):
    sink(p)

def outer(q):
    sink(q)

def run():
    direct_user = source()
    wrapped_user = source()
    mixed_first = source()
    mixed_second = source()
    mutation_user = source()
    pair_obj = source()
    nested_user = source()
    direct(
        direct_user,
    )
    WRAPPED_CALL
    mixed(mixed_first,
        mixed_second)
    mutated(mutation_user, (mutation_user := source()), (
        mutation_user
    ))
    repeated(
        pair_obj.value,
        pair_obj.value,
    )
    outer(
        inner(
            nested_user,
        ),
    )
"#
        }
        Language::Go => {
            r#"package main

func direct(p string) { sink(p) }
func wrapped(p string) { sink(p) }
func mixed(a string, b string) { sink(a); sink(b) }
func mutated(a string, b string, c string) { sink(a); sink(b); sink(c) }
func repeated(a string, b string) { sink(a); sink(b) }
func inner(p string) { sink(p) }
func outer(q string) { sink(q) }

func run() {
    direct_user := source()
    wrapped_user := source()
    mixed_first := source()
    mixed_second := source()
    mutation_user := source()
    pair_obj := source()
    nested_user := source()
    direct(
        direct_user,
    )
    WRAPPED_CALL
    mixed(mixed_first,
        mixed_second,
    )
    mutated(mutation_user, func() string {
        mutation_user = source()
        return mutation_user
    }(), (
        mutation_user
    ))
    repeated(
        pair_obj.value,
        pair_obj.value,
    )
    outer(
        inner(
            nested_user,
        ),
    )
}
"#
        }
        Language::Rust => {
            r#"fn direct(p: String) { sink(p); }
fn wrapped(p: &String) { sink(p); }
fn mixed(a: String, b: String) { sink(a); sink(b); }
fn mutated(a: String, b: String, c: String) { sink(a); sink(b); sink(c); }
fn repeated(a: String, b: String) { sink(a); sink(b); }
fn inner(p: String) { sink(p); }
fn outer(q: String) { sink(q); }

fn run() {
    let direct_user = source();
    let wrapped_user = source();
    let mixed_first = source();
    let mixed_second = source();
    let mut mutation_user = source();
    let pair_obj = source();
    let nested_user = source();
    direct(
        direct_user,
    );
    WRAPPED_CALL
    mixed(mixed_first,
        mixed_second,
    );
    mutated(mutation_user, mutation_user = source(), (
        mutation_user
    ));
    repeated(
        pair_obj.value,
        pair_obj.value,
    );
    outer(
        inner(
            nested_user,
        ),
    );
}
"#
        }
        Language::JavaScript => {
            r#"function direct(p) { sink(p); }
function wrapped(p) { sink(p); }
function mixed(a, b) { sink(a); sink(b); }
function mutated(a, b, c) { sink(a); sink(b); sink(c); }
function repeated(a, b) { sink(a); sink(b); }
function inner(p) { sink(p); }
function outer(q) { sink(q); }

function run() {
    const direct_user = source();
    const wrapped_user = source();
    const mixed_first = source();
    const mixed_second = source();
    let mutation_user = source();
    const pair_obj = source();
    const nested_user = source();
    direct(
        direct_user,
    );
    WRAPPED_CALL
    mixed(mixed_first,
        mixed_second,
    );
    mutated(mutation_user, (mutation_user = source()), (
        mutation_user
    ));
    repeated(
        pair_obj.value,
        pair_obj.value,
    );
    outer(
        inner(
            nested_user,
        ),
    );
}
"#
        }
        _ => unreachable!("fixture languages are fixed above"),
    };
    template.replace("WRAPPED_CALL", wrapper)
}

fn build(language: Language, file: &str, source: &str) -> CodePropertyGraph {
    let mut files = BTreeMap::new();
    files.insert(
        file.to_string(),
        ParsedFile::parse(file, source, language).unwrap(),
    );
    CodePropertyGraph::build(&files)
}

fn last_byte(source: &str, needle: &str) -> usize {
    source
        .rmatch_indices(needle)
        .next()
        .map(|(byte, _)| byte)
        .unwrap_or_else(|| panic!("missing `{needle}`"))
}

fn all_bytes(source: &str, needle: &str) -> Vec<usize> {
    source.match_indices(needle).map(|(byte, _)| byte).collect()
}

fn line_at(source: &str, byte: usize) -> usize {
    source.as_bytes()[..byte]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
        + 1
}

fn has_edge_from_byte_to_param(
    cpg: &CodePropertyGraph,
    source: &str,
    source_byte: usize,
    source_path: &str,
    callee: &str,
    parameter: &str,
) -> bool {
    let source_line = line_at(source, source_byte);
    cpg.graph.edge_indices().any(|edge| {
        cpg.graph[edge] == CpgEdge::DataFlow
            && cpg.graph.edge_endpoints(edge).is_some_and(|(from, to)| {
                matches!(
                    cpg.node(from),
                    CpgNode::Variable {
                        path,
                        line,
                        access: VarAccess::Use,
                        start_byte,
                        ..
                    } if path.to_string() == source_path
                        && *line == source_line
                        && *start_byte == source_byte
                ) && matches!(
                    cpg.node(to),
                    CpgNode::Variable {
                        path,
                        function,
                        access: VarAccess::Def,
                        ..
                    } if path.base == parameter && function == callee
                )
            })
    })
}

fn has_edge_from_line_to_param(
    cpg: &CodePropertyGraph,
    source_line: usize,
    source_path: &str,
    callee: &str,
    parameter: &str,
) -> bool {
    cpg.graph.edge_indices().any(|edge| {
        cpg.graph[edge] == CpgEdge::DataFlow
            && cpg.graph.edge_endpoints(edge).is_some_and(|(from, to)| {
                matches!(
                    cpg.node(from),
                    CpgNode::Variable {
                        path,
                        line,
                        function,
                        access: VarAccess::Use,
                        ..
                    } if *line == source_line && path.to_string() == source_path && function == "run"
                ) && matches!(
                    cpg.node(to),
                    CpgNode::Variable {
                        path,
                        function,
                        access: VarAccess::Def,
                        ..
                    } if path.base == parameter && function == callee
                )
            })
    })
}

fn arg_param_shapes(cpg: &CodePropertyGraph, callee: &str, parameter: &str) -> BTreeSet<String> {
    cpg.graph
        .edge_indices()
        .filter(|&edge| cpg.graph[edge] == CpgEdge::DataFlow)
        .filter_map(|edge| {
            let (from, to) = cpg.graph.edge_endpoints(edge)?;
            match (cpg.node(from), cpg.node(to)) {
                (
                    CpgNode::Variable {
                        path: source_path,
                        function: source_function,
                        access: VarAccess::Use,
                        ..
                    },
                    CpgNode::Variable {
                        path: target_path,
                        function: target_function,
                        access: VarAccess::Def,
                        ..
                    },
                ) if source_function == "run"
                    && target_function == callee
                    && target_path.base == parameter =>
                {
                    Some(source_path.to_string())
                }
                _ => None,
            }
        })
        .collect()
}

#[test]
fn multiline_direct_and_mixed_arguments_bind_exact_occurrences_in_each_language() {
    for (language, file) in languages() {
        let source = fixture_source(language, true);
        let cpg = build(language, file, &source);
        assert!(
            has_edge_from_byte_to_param(
                &cpg,
                &source,
                last_byte(&source, "direct_user"),
                "direct_user",
                "direct",
                "p",
            ),
            "{language:?}: multiline direct argument must bind direct_user -> direct.p"
        );
        assert!(
            has_edge_from_byte_to_param(
                &cpg,
                &source,
                last_byte(&source, "mixed_first"),
                "mixed_first",
                "mixed",
                "a",
            ),
            "{language:?}: first call-line argument must bind mixed_first -> mixed.a"
        );
        assert!(
            has_edge_from_byte_to_param(
                &cpg,
                &source,
                last_byte(&source, "mixed_second"),
                "mixed_second",
                "mixed",
                "b",
            ),
            "{language:?}: second multiline argument must bind mixed_second -> mixed.b"
        );
    }
}

#[test]
fn transparent_wrappers_are_line_invariant_in_each_language() {
    for (language, file) in languages() {
        let single = fixture_source(language, false);
        let multiline = fixture_source(language, true);
        let single_edges = arg_param_shapes(&build(language, file, &single), "wrapped", "p");
        let multiline_edges = arg_param_shapes(&build(language, file, &multiline), "wrapped", "p");
        match language {
            Language::Python | Language::Go | Language::Rust => {
                assert!(
                    !single_edges.is_empty(),
                    "{language:?}: single-line wrapper must bind before layout invariance is checked"
                );
                assert_eq!(
                    multiline_edges, single_edges,
                    "{language:?}: changing only wrapper layout must preserve the positive Step-5b edge"
                );
            }
            Language::JavaScript => {
                // Characterization: AccessPath::from_expr does not unwrap this spread form.
                assert!(
                    single_edges.is_empty(),
                    "{language:?}: current AccessPath::from_expr scope does not normalize this wrapper"
                );
                assert!(
                    multiline_edges.is_empty(),
                    "{language:?}: multiline layout must not invent a wrapper edge outside AccessPath::from_expr scope"
                );
            }
            _ => unreachable!("fixture languages are fixed above"),
        }
    }
}

#[test]
fn later_multiline_argument_never_binds_the_call_line_occurrence() {
    for (language, file) in languages() {
        let source = fixture_source(language, true);
        let cpg = build(language, file, &source);
        let call_line = line_at(&source, last_byte(&source, "mutated("));
        assert!(
            !has_edge_from_line_to_param(&cpg, call_line, "mutation_user", "mutated", "c"),
            "{language:?}: c must bind its contained occurrence or be dropped, never a call-line mutation_user"
        );
    }
}

#[test]
fn repeated_path_arguments_keep_their_own_endpoint_lines() {
    for (language, file) in languages() {
        let source = fixture_source(language, true);
        let cpg = build(language, file, &source);
        let occurrences = all_bytes(&source, "pair_obj.value");
        assert_eq!(
            occurrences.len(),
            2,
            "fixture must contain exactly two field occurrences"
        );
        assert!(
            has_edge_from_byte_to_param(
                &cpg,
                &source,
                occurrences[0],
                "pair_obj.value",
                "repeated",
                "a",
            ),
            "{language:?}: first pair_obj.value must bind repeated.a"
        );
        assert!(
            has_edge_from_byte_to_param(
                &cpg,
                &source,
                occurrences[1],
                "pair_obj.value",
                "repeated",
                "b",
            ),
            "{language:?}: second pair_obj.value must bind repeated.b"
        );
    }
}

#[test]
fn nested_call_adds_only_the_inner_argument_to_parameter_edge() {
    for (language, file) in languages() {
        let source = fixture_source(language, true);
        let cpg = build(language, file, &source);
        let nested_user = last_byte(&source, "nested_user");
        assert!(
            has_edge_from_byte_to_param(&cpg, &source, nested_user, "nested_user", "inner", "p"),
            "{language:?}: nested user must bind inner.p"
        );
        assert!(
            !has_edge_from_byte_to_param(&cpg, &source, nested_user, "nested_user", "outer", "q"),
            "{language:?}: nested user must not bypass inner return flow into outer.q"
        );
    }
}

#[test]
fn distinct_name_nested_multiline_trace_descends_only_to_inner_parameter() {
    let source = r#"def h(p):
    sink(p)

def g(q):
    sink(q)

def f():
    user = input()
    g(h(
        user
    ))
"#;
    let cpg = build(Language::Python, "m.py", source);
    let trace = cpg.taint_trace(&[("m.py".to_string(), 8)]);
    let h_param = cpg
        .nodes_at("m.py", 1)
        .into_iter()
        .find(|&node| {
            cpg.to_var_location(node)
                .is_some_and(|loc| loc.path.to_string() == "p")
        })
        .expect("inner parameter");
    let g_param = cpg
        .nodes_at("m.py", 4)
        .into_iter()
        .find(|&node| {
            cpg.to_var_location(node)
                .is_some_and(|loc| loc.path.to_string() == "q")
        })
        .expect("outer parameter");

    assert!(
        trace.in_frontier(h_param),
        "nested call must descend to h.p"
    );
    assert!(
        !trace.in_frontier(g_param),
        "nested call must not invent direct return flow into g.q"
    );
}

#[test]
fn same_name_nested_multiline_trace_refuses_ambiguous_containing_calls() {
    let source = r#"def g(p):
    sink(p)

def f():
    user = input()
    g(g(
        user
    ))
"#;
    let cpg = build(Language::Python, "m.py", source);
    let trace = cpg.taint_trace(&[("m.py".to_string(), 5)]);
    let g_param = cpg
        .nodes_at("m.py", 1)
        .into_iter()
        .find(|&node| {
            cpg.to_var_location(node)
                .is_some_and(|loc| loc.path.to_string() == "p")
        })
        .expect("callee parameter");

    // The inner Step-5b edge (continuation-line `user` → g.p) MUST exist, so the
    // non-descent below is provably the trace gate's ambiguity refusal (two
    // containing same-name `g` spans), not a missing edge. (Review SMELL-1.)
    assert!(
        has_edge_from_byte_to_param(&cpg, source, last_byte(source, "user"), "user", "g", "p"),
        "inner multiline argument must still bind user -> g.p"
    );
    assert!(
        !trace.in_frontier(g_param),
        "same-name nested spans are ambiguous and must not descend"
    );
}
