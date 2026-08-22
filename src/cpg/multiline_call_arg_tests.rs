//! Regression coverage for Step-5b argument identity across multi-line calls.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn languages() -> [(Language, &'static str); 6] {
    [
        (Language::Python, "m.py"),
        (Language::Go, "m.go"),
        (Language::Rust, "m.rs"),
        (Language::Java, "M.java"),
        (Language::JavaScript, "m.js"),
        (Language::TypeScript, "m.ts"),
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
        (Language::Java, false) => "M.wrapped((Object) wrapped_user);",
        (Language::Java, true) => "M.wrapped(\n        (Object)\n        wrapped_user\n    );",
        (Language::JavaScript, false) => "wrapped(...[wrapped_user]);",
        (Language::JavaScript, true) => {
            "wrapped(\n        ...[\n            wrapped_user\n        ],\n    );"
        }
        (Language::TypeScript, false) => "wrapped(...[wrapped_user]);",
        (Language::TypeScript, true) => {
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
        Language::Java => {
            r#"class M {
    static void direct(Object p) { sink(p); }
    static void wrapped(Object p) { sink(p); }
    static void mixed(Object a, Object b) { sink(a); sink(b); }
    static void mutated(Object a, Object b, Object c) { sink(a); sink(b); sink(c); }
    static void repeated(Object a, Object b) { sink(a); sink(b); }
    static void inner(Object p) { sink(p); }
    static void outer(Object q) { sink(q); }

    static void run() {
        Object direct_user = source();
        Object wrapped_user = source();
        Object mixed_first = source();
        Object mixed_second = source();
        Object mutation_user = source();
        Object pair_obj = source();
        Object nested_user = source();
        M.direct(
            direct_user
        );
        WRAPPED_CALL
        M.mixed(mixed_first,
            mixed_second);
        M.mutated(mutation_user, (mutation_user = source()), (
            mutation_user
        ));
        M.repeated(
            pair_obj.value,
            pair_obj.value
        );
        M.outer(
            M.inner(
                nested_user
            )
        );
    }
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
        Language::TypeScript => {
            r#"function direct(p: unknown) { sink(p); }
function wrapped(p: unknown) { sink(p); }
function mixed(a: unknown, b: unknown) { sink(a); sink(b); }
function mutated(a: unknown, b: unknown, c: unknown) { sink(a); sink(b); sink(c); }
function repeated(a: unknown, b: unknown) { sink(a); sink(b); }
function inner(p: unknown) { sink(p); }
function outer(q: unknown) { sink(q); }

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
fn java_formal_parameters_materialize_step5b_targets() {
    let source = fixture_source(Language::Java, true);
    let cpg = build(Language::Java, "M.java", &source);
    assert!(
        cpg.node_indices().any(|idx| matches!(
            cpg.node(idx),
            CpgNode::Variable {
                path,
                function,
                access: VarAccess::Def,
                ..
            } if path.base == "p" && function == "direct"
        )),
        "Java formal parameter direct.p must materialize a Def target"
    );
}

#[test]
fn typescript_typed_parameters_materialize_step5b_targets() {
    let source = fixture_source(Language::TypeScript, true);
    let cpg = build(Language::TypeScript, "m.ts", &source);
    assert!(
        cpg.node_indices().any(|idx| matches!(
            cpg.node(idx),
            CpgNode::Variable {
                path,
                function,
                access: VarAccess::Def,
                ..
            } if path.base == "p" && function == "direct"
        )),
        "TypeScript typed parameter direct.p must materialize a Def target"
    );
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
        assert_eq!(
            multiline_edges, single_edges,
            "{language:?}: changing only wrapper layout must preserve Step-5b edges"
        );
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
