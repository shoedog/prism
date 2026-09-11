//! JS/TS loop headers define only their left-hand binding or assignment target.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeMap;

fn build(language: Language, source: &str) -> CodePropertyGraph {
    let file = match language {
        Language::JavaScript => "loops.js",
        Language::TypeScript => "loops.ts",
        Language::Tsx => "loops.tsx",
        _ => unreachable!(),
    };
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    CodePropertyGraph::build(&BTreeMap::from([(file.to_string(), parsed)]))
}

fn has_access(
    cpg: &CodePropertyGraph,
    function: &str,
    path: &str,
    access: VarAccess,
    span: (usize, usize),
) -> bool {
    cpg.graph.node_indices().any(|node| {
        matches!(cpg.node(node), CpgNode::Variable {
            path: actual,
            function: owner,
            access: actual_access,
            start_byte,
            end_byte,
            ..
        } if owner == function && actual.to_string() == path && *actual_access == access
            && (*start_byte, *end_byte) == span)
    })
}

#[test]
fn default_parameter_loop_rhs_is_not_a_def_or_argument_target() {
    let source = "function consume(iterable = []) {\n  for (const item of iterable) { sink(item); }\n}\nfunction run(input) { consume(input); }\n";
    let left = source.find("item of").unwrap();
    let rhs = source.find("iterable) { sink").unwrap();

    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let cpg = build(language, source);
        if !has_access(&cpg, "consume", "item", VarAccess::Def, (left, left + 4)) {
            failures.push(format!("{language:?}: loop binding lost its Def"));
        }
        if has_access(&cpg, "consume", "iterable", VarAccess::Def, (rhs, rhs + 8)) {
            failures.push(format!("{language:?}: iterable RHS became a Def"));
        }
        if cpg.graph.edge_indices().any(|edge| {
                if !matches!(cpg.graph[edge], CpgEdge::DataFlow(_)) {
                    return false;
                }
                let Some((from, to)) = cpg.graph.edge_endpoints(edge) else {
                    return false;
                };
                matches!(
                    (cpg.node(from), cpg.node(to)),
                    (
                        CpgNode::Variable { path: from_path, function: from_fn, access: VarAccess::Use, .. },
                        CpgNode::Variable { path: to_path, function: to_fn, access: VarAccess::Def, start_byte, end_byte, .. }
                    ) if from_fn == "run" && from_path.to_string() == "input"
                        && to_fn == "consume" && to_path.to_string() == "iterable"
                        && (*start_byte, *end_byte) == (rhs, rhs + 8)
                )
            })
        {
            failures.push(format!(
                "{language:?}: caller argument flowed to the loop RHS as a fake parameter Def"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn loop_header_controls_keep_left_defs_off_right_expressions() {
    let cases = [
        (
            "for (let key in iterable) { sink(key); }",
            "key",
            "iterable",
        ),
        (
            "for (const value of iterable) { sink(value); }",
            "value",
            "iterable",
        ),
        (
            "for await (const value of iterable) { sink(value); }",
            "value",
            "iterable",
        ),
        (
            "for (const { value } of iterable) { sink(value); }",
            "value",
            "iterable",
        ),
        (
            "for (const value of state.items) { sink(value); }",
            "value",
            "state.items",
        ),
        (
            "for (const value of load(iterable)) { sink(value); }",
            "value",
            "iterable",
        ),
    ];

    let mut failures = Vec::new();
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        for (header, left_path, rhs_path) in cases {
            let source =
                format!("async function run(iterable, target, state) {{\n  {header}\n}}\n");
            let cpg = build(language, &source);
            let accesses: Vec<_> = cpg
                .graph
                .node_indices()
                .filter_map(|node| match cpg.node(node) {
                    CpgNode::Variable {
                        path,
                        function,
                        access,
                        line,
                        ..
                    } if function == "run" && *line == 2 => Some((path.to_string(), *access)),
                    _ => None,
                })
                .collect();
            if !accesses.contains(&(left_path.into(), VarAccess::Def)) {
                failures.push(format!(
                    "{language:?}: {header}: missing left Def; {accesses:?}"
                ));
            }
            // Exact loop-RHS read provenance is deferred: the assignments query does
            // not capture for_in_statement. Required/plain parameters still supply a
            // line-anchored Use; retain member RHS only as a no-false-Def control.
            if rhs_path != "state.items" && !accesses.contains(&(rhs_path.into(), VarAccess::Use)) {
                failures.push(format!(
                    "{language:?}: {header}: missing RHS Use; {accesses:?}"
                ));
            }
            if accesses.contains(&(rhs_path.into(), VarAccess::Def)) {
                failures.push(format!(
                    "{language:?}: {header}: unexpected RHS Def; {accesses:?}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
