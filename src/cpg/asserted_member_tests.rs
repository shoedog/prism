//! Bounded asserted-member binding, preserving the existing occurrence index.
use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
fn build(language: Language, source: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
    let file = match language {
        Language::Tsx => "member.tsx",
        Language::TypeScript => "member.ts",
        Language::JavaScript => "member.js",
        _ => unreachable!(),
    };
    let mut files = BTreeMap::from([(
        file.to_string(),
        ParsedFile::parse(file, source, language).unwrap(),
    )]);
    if language != Language::JavaScript {
        let callee = "export function take(a, b) { sink(a, b); }";
        files.insert(
            "callee.js".into(),
            ParsedFile::parse("callee.js", callee, Language::JavaScript).unwrap(),
        );
    }
    (CodePropertyGraph::build(&files), files)
}
// Include exact source endpoints: path equality alone would miss cross-argument binding.
fn argument_edges(cpg: &CodePropertyGraph) -> BTreeSet<(String, String, usize, usize)> {
    cpg.graph
        .edge_indices()
        .filter_map(|edge| {
            if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
                return None;
            }
            let (from, to) = cpg.graph.edge_endpoints(edge)?;
            match (cpg.node(from), cpg.node(to)) {
                (
                    CpgNode::Variable {
                        path,
                        function,
                        access: VarAccess::Use,
                        start_byte,
                        end_byte,
                        ..
                    },
                    CpgNode::Variable {
                        path: param,
                        function: callee,
                        access: VarAccess::Def,
                        ..
                    },
                ) if function == "run" && callee == "take" => {
                    Some((path.to_string(), param.to_string(), *start_byte, *end_byte))
                }
                _ => None,
            }
        })
        .collect()
}
fn fixture(language: Language, arguments: &str) -> String {
    // TS parameter Def extraction is a separate known gap. Use real JS targets.
    let callee = if language == Language::JavaScript {
        "function take(a, b) { sink(a, b); }"
    } else {
        "import { take } from './callee.js';"
    };
    format!("{callee}\nfunction run(runtime, other) {{\n  runtime.X = source();\n  runtime.Y = source();\n  other.X = source();\n  take({arguments});\n}}\n")
}
#[test]
fn asserted_member_arguments_bind_field_and_base_across_layouts() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expression in [
            "runtime.X",
            "(runtime as any).X",
            "(\n    runtime as any\n  ).X",
            "(runtime satisfies any).X",
        ] {
            let source = fixture(language, &format!("{expression}, other.X"));
            let (cpg, _) = build(language, &source);
            let edges = argument_edges(&cpg);
            let shapes: BTreeSet<_> = edges
                .iter()
                .map(|(path, param, _, _)| (path.as_str(), param.as_str()))
                .collect();
            assert_eq!(
                shapes,
                BTreeSet::from([
                    ("runtime.X", "a"),
                    ("runtime", "a"),
                    ("other.X", "b"),
                    ("other", "b")
                ]),
                "{language:?} {expression}: {edges:?}"
            );
            let start = source.rfind(expression).unwrap();
            for (_, param, from, to) in &edges {
                if param == "a" {
                    assert!(
                        start <= *from && *to <= start + expression.len(),
                        "{edges:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn asserted_member_same_line_duplicates_do_not_cross_argument_spans() {
    for language in [Language::TypeScript, Language::Tsx] {
        let mut observed_shapes = Vec::new();
        for expression in ["runtime.X", "(runtime as any).X"] {
            let source = fixture(language, &format!("{expression}, {expression}"));
            let (cpg, _) = build(language, &source);
            let edges = argument_edges(&cpg);
            assert!(
                !edges.is_empty(),
                "duplicate fixture must retain an indexed endpoint"
            );
            let starts: Vec<_> = source
                .rmatch_indices(expression)
                .map(|(byte, _)| byte)
                .take(2)
                .collect();
            for (path, param, start, end) in &edges {
                let argument_start = match param.as_str() {
                    "a" => starts[1],
                    "b" => starts[0],
                    _ => panic!("unexpected {param}"),
                };
                assert!(
                    argument_start <= *start && *end <= argument_start + expression.len(),
                    "{language:?}: {path} -> {param} crossed its AST argument span: {edges:?}"
                );
            }
            // var_index deliberately collapses same-line same-path nodes. Preserve that
            // baseline behavior, rather than demanding two impossible unique endpoints.
            observed_shapes.push(
                edges
                    .into_iter()
                    .map(|(path, param, _, _)| (path, param))
                    .collect::<BTreeSet<_>>(),
            );
        }
        assert_eq!(observed_shapes[0], observed_shapes[1], "{language:?}");
    }
}
#[test]
fn asserted_member_return_uses_field_endpoint_not_synthetic_fallback() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expression in [
            "runtime.X",
            "(runtime as any).X",
            "(runtime satisfies any).X",
        ] {
            let source = format!("function read(runtime) {{\n runtime.X = source();\n return {expression};\n}}\nfunction run(runtime) {{\n const out = read(runtime);\n sink(out);\n}}\n");
            let (cpg, _) = build(language, &source);
            let returns: Vec<_> = cpg
                .graph
                .edge_indices()
                .filter(|&edge| matches!(cpg.graph[edge], CpgEdge::ReturnFlow { .. }))
                .filter_map(|edge| cpg.graph.edge_endpoints(edge))
                .collect();
            assert_eq!(returns.len(), 1, "{language:?}: {expression}");
            let (from, to) = returns[0];
            assert!(
                matches!(cpg.node(from), CpgNode::Variable {
                path, function, access: VarAccess::Use, ..
            } if path.to_string() == "runtime.X" && function == "read"),
                "{language:?}: {expression}: {:?}",
                cpg.node(from)
            );
            assert!(matches!(cpg.node(to), CpgNode::Variable {
                path, function, access: VarAccess::Def, ..
            } if path.to_string() == "out" && function == "run"));
        }
    }
}
#[test]
fn asserted_member_parallel_serial_step5b_edges_match() {
    for language in [Language::TypeScript, Language::Tsx] {
        for args in [
            "(runtime as any).X, other.X",
            "(\n runtime as any\n).X, other.X",
            "(runtime as any).X, (runtime as any).X",
            "(runtime satisfies any).X, other.X",
        ] {
            let (cpg, files) = build(language, &fixture(language, args));
            let parallel = CodePropertyGraph::collect_step5b_edges(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files,
            );
            let serial = CodePropertyGraph::collect_step5b_edges_reference(
                &cpg.call_graph,
                &cpg.var_index,
                &cpg.graph,
                &files,
            );
            assert!(!parallel.is_empty(), "{language:?}: {args}");
            assert_eq!(parallel, serial, "{language:?}: {args}");
        }
    }
}
#[test]
fn asserted_member_unsupported_receivers_do_not_gain_simple_field_bindings() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expression in [
            "(runtime!).X",
            "(flag ? runtime : other).X",
            "(runtime, other).X",
        ] {
            let (cpg, _) = build(
                language,
                &fixture(language, &format!("{expression}, other.X")),
            );
            let edges = argument_edges(&cpg);
            assert!(
                !edges.iter().any(|(path, param, _, _)| param == "a"
                    && (path == "runtime.X" || path == "other.X")),
                "{language:?}: unsupported {expression} gained normalized field edge: {edges:?}"
            );
            assert!(edges
                .iter()
                .any(|(path, param, _, _)| path == "other.X" && param == "b"));
        }
    }
}

#[test]
fn asserted_member_javascript_plain_field_binding_is_unchanged() {
    let (cpg, _) = build(
        Language::JavaScript,
        &fixture(Language::JavaScript, "runtime.X, other.X"),
    );
    let shapes: BTreeSet<_> = argument_edges(&cpg)
        .into_iter()
        .map(|(path, param, _, _)| (path, param))
        .collect();
    let expected = [
        ("runtime.X", "a"),
        ("runtime", "a"),
        ("other.X", "b"),
        ("other", "b"),
    ]
    .into_iter()
    .map(|(path, param)| (path.to_string(), param.to_string()))
    .collect();
    assert_eq!(shapes, expected);
}

#[test]
fn parenthesized_member_arguments_keep_comment_and_wrapper_identity() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expression in [
            "(runtime /* c */ .X)",
            "(runtime. /* c */ X)",
            "((runtime as any).X)",
            "((((((((runtime /* c */ .X))))))))",
            "(((((((runtime as any))))))).X",
        ] {
            let (cpg, _) = build(
                language,
                &fixture(language, &format!("{expression}, other.X")),
            );
            let paths: BTreeSet<_> = argument_edges(&cpg)
                .into_iter()
                .filter(|(_, p, _, _)| p == "a")
                .map(|(p, _, _, _)| p)
                .collect();
            assert_eq!(
                paths,
                BTreeSet::from(["runtime".to_string(), "runtime.X".to_string()]),
                "{expression}"
            );
        }
        for expression in [
            "(((((((((runtime /* c */ .X)))))))))",
            "((((((((runtime as any))))))).X)", // receiver8 + envelope1
            "((runtime as any).X as any)",
            "((runtime as any).X, other.X)",
            "[(runtime as any).X]",
            "(runtime /* c */ .X as any)",
            "[runtime /* c */ .X]",
            "(runtime /* c */ .X, other.X)",
            "...(runtime /* c */ .X)",
        ] {
            let (cpg, _) = build(
                language,
                &fixture(language, &format!("{expression}, other.X")),
            );
            assert!(
                argument_edges(&cpg).iter().all(|(_, p, _, _)| p != "a"),
                "{expression}"
            );
        }
    }
}
