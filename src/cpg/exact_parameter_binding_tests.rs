//! Exact JS/TS/TSX argument-to-parameter binding regressions for Step 5b.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;

fn fixture(language: Language, source: &str) -> (CodePropertyGraph, BTreeMap<String, ParsedFile>) {
    let file = match language {
        Language::JavaScript => "exact.js",
        Language::TypeScript => "exact.ts",
        Language::Tsx => "exact.tsx",
        _ => unreachable!(),
    };
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    let files = BTreeMap::from([(file.to_string(), parsed)]);
    (CodePropertyGraph::build(&files), files)
}

fn argument_targets(
    cpg: &CodePropertyGraph,
    caller: &str,
    argument: &str,
    callee: &str,
) -> Vec<(String, usize, usize, usize, String)> {
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
                        ..
                    },
                    CpgNode::Variable {
                        path: target,
                        file,
                        function: owner,
                        function_start_line,
                        access: VarAccess::Def,
                        start_byte,
                        end_byte,
                        ..
                    },
                ) if function == caller && path.to_string() == argument && owner == callee => {
                    Some((
                        file.clone(),
                        *function_start_line,
                        *start_byte,
                        *end_byte,
                        target.to_string(),
                    ))
                }
                _ => None,
            }
        })
        .collect()
}

#[test]
fn default_parameter_never_falls_back_to_later_overwrite_or_var_redeclaration() {
    let cases = [
        (
            "overwrite",
            "function take(value = seed()) {\n  value = clean();\n}\n",
        ),
        (
            "var-redeclaration",
            "function take(value = seed()) {\n  var value = clean();\n}\n",
        ),
        (
            "alias-resolved-local",
            "function take(value = seed()) {\n  let local;\n  local = value;\n  sink(local);\n}\n",
        ),
    ];
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for (case, callee) in cases {
            let source = format!("{callee}function run(input) {{\n  take(input);\n}}\n");
            let (cpg, _) = fixture(language, &source);
            let targets = argument_targets(&cpg, "run", "input", "take");
            if !targets.is_empty() {
                failures.push(format!("{language:?}/{case}: invented targets {targets:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn same_line_default_and_body_assignment_do_not_mint_a_parameter_target() {
    let source = "function take(value = seed()) { value = clean(); }\nfunction run(input) {\n  take(input);\n}\n";
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (cpg, _) = fixture(language, source);
        let targets = argument_targets(&cpg, "run", "input", "take");
        if !targets.is_empty() {
            failures.push(format!("{language:?}: invented targets {targets:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn required_parameters_bind_to_the_exact_identifier_token() {
    let cases = [
        (
            "simple",
            "function take(first, second) { return second; }\n",
        ),
        (
            "multiline-unicode",
            "function take(\n  first,\n  sécond\n) { return sécond; }\n",
        ),
    ];
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for (case, callee) in cases {
            let source = format!("{callee}function run(input) {{\n  take(seed(), input);\n}}\n");
            let token = if case == "simple" {
                "second"
            } else {
                "sécond"
            };
            let start = source.find(token).unwrap();
            let expected = (
                "exact".to_string()
                    + match language {
                        Language::JavaScript => ".js",
                        Language::TypeScript => ".ts",
                        Language::Tsx => ".tsx",
                        _ => unreachable!(),
                    },
                1,
                start,
                start + token.len(),
                token.to_string(),
            );
            let (cpg, _) = fixture(language, &source);
            let targets = argument_targets(&cpg, "run", "input", "take");
            if targets != [expected.clone()] {
                failures.push(format!(
                    "{language:?}/{case}: expected {expected:?}, got {targets:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn required_slot_after_default_keeps_its_original_argument_position() {
    let source = "function take(first, middle = seed(), last) {\n middle = clean();\n return last;\n}\nfunction run(a, b, c) {\n  take(a, b, c);\n}\n";
    let last = source.find("last").unwrap();
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (cpg, _) = fixture(language, source);
        let c_targets = argument_targets(&cpg, "run", "c", "take");
        let b_targets = argument_targets(&cpg, "run", "b", "take");
        if !c_targets.iter().any(|(_, line, start, end, path)| {
            *line == 1 && *start == last && *end == last + 4 && path == "last"
        }) || !b_targets.is_empty()
        {
            failures.push(format!("{language:?}: b={b_targets:?}, c={c_targets:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn ambiguous_recovered_and_destructured_parameters_refuse_binding() {
    let cases = [
        (
            "duplicate",
            "function take(value, value) { value = clean(); }\n",
        ),
        (
            "destructured",
            "function take({ value }) { value = clean(); }\n",
        ),
        (
            "truncated",
            "function take({ value }, later) { later = clean(); }\n",
        ),
        (
            "recovered",
            "function take(value,, later) { later = clean(); }\n",
        ),
    ];
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for (case, callee) in cases {
            let source = format!("{callee}function run(input) {{\n  take(input, input);\n}}\n");
            let (cpg, _) = fixture(language, &source);
            let targets = argument_targets(&cpg, "run", "input", "take");
            if !targets.is_empty() {
                failures.push(format!("{language:?}/{case}: invented targets {targets:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn exact_parameter_step5b_parallel_and_reference_paths_match() {
    let source = "function take(first, middle = seed(), last) { last = clean(last); }\nfunction run(a, b, c) {\n  take(a, b, c);\n}\n";
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (cpg, files) = fixture(language, source);
        let parallel = CodePropertyGraph::collect_step5b_edges(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        let reference = CodePropertyGraph::collect_step5b_edges_reference(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        assert_eq!(
            parallel, reference,
            "{language:?}: Step-5b par != reference"
        );
    }
}

#[test]
fn exact_parameter_missing_entry_def_never_searches_the_body() {
    let source = "function take(value) {\n value = clean();\n return value;\n}\nfunction run(input) {\n take(input);\n}\n";
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (cpg, files) = fixture(language, source);
        assert_eq!(argument_targets(&cpg, "run", "input", "take").len(), 1);
        let mut index = cpg.var_index.clone();
        index.retain(|(_, name, _, line, path, access), _| {
            !(name == "take" && *line == 1 && path.base == "value" && *access == VarAccess::Def)
        });
        assert!(index
            .keys()
            .any(|(_, name, _, line, path, access)| name == "take"
                && *line > 1
                && path.base == "value"
                && *access == VarAccess::Def));
        let edges =
            CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &index, &cpg.graph, &files);
        if !edges.is_empty() {
            failures.push(format!("{language:?}: body fallback survived: {edges:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn optional_parameter_body_definition_is_not_a_supported_token() {
    let source = "function take(value?: unknown) {\n value = clean();\n}\nfunction run(input: unknown) {\n take(input);\n}\n";
    let mut failures = Vec::new();
    for language in [Language::TypeScript, Language::Tsx] {
        let (cpg, files) = fixture(language, source);
        assert_eq!(files.values().next().unwrap().parse_error_count, 0);
        let targets = argument_targets(&cpg, "run", "input", "take");
        if !targets.is_empty() {
            failures.push(format!("{language:?}: optional body target {targets:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn exact_parameter_node_fields_and_genuine_other_owner_cannot_be_substituted() {
    let source = "function take(value) { return value; }\nfunction other(value) { return value; }\nfunction run(input) {\n take(input);\n}\n";
    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let (cpg, files) = fixture(language, source);
        let (key, &target) = cpg
            .var_index
            .iter()
            .find(|((_, owner, _, _, path, access), _)| {
                owner == "take" && path.base == "value" && *access == VarAccess::Def
            })
            .unwrap();
        let other = *cpg
            .var_index
            .iter()
            .find(|((_, owner, _, _, path, access), _)| {
                owner == "other" && path.base == "value" && *access == VarAccess::Def
            })
            .unwrap()
            .1;
        let control = CodePropertyGraph::collect_step5b_edges(
            &cpg.call_graph,
            &cpg.var_index,
            &cpg.graph,
            &files,
        );
        assert_eq!(control.len(), 1);
        assert_eq!(control[0].1, target);
        for field in [
            "file",
            "function",
            "function_start_line",
            "line",
            "path",
            "access",
            "start_byte",
            "end_byte",
            "genuine_other",
        ] {
            let mut graph = cpg.graph.clone();
            let mut index = cpg.var_index.clone();
            if field == "genuine_other" {
                index.insert(key.clone(), other);
            } else if let CpgNode::Variable {
                file,
                function,
                function_start_line,
                line,
                path,
                access,
                start_byte,
                end_byte,
            } = &mut graph[target]
            {
                match field {
                    "file" => file.push_str(".other"),
                    "function" => function.push_str("_other"),
                    "function_start_line" => *function_start_line += 1,
                    "line" => *line += 1,
                    "path" => *path = crate::access_path::AccessPath::simple("other"),
                    "access" => *access = VarAccess::Use,
                    "start_byte" => *start_byte += 1,
                    "end_byte" => *end_byte += 1,
                    _ => unreachable!(),
                }
            } else {
                panic!("parameter control is not a variable");
            }
            let parallel =
                CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &index, &graph, &files);
            let serial = CodePropertyGraph::collect_step5b_edges_reference(
                &cpg.call_graph,
                &index,
                &graph,
                &files,
            );
            if !parallel.is_empty() || !serial.is_empty() {
                failures.push(format!(
                    "{language:?}/{field}: parallel={parallel:?}, serial={serial:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
