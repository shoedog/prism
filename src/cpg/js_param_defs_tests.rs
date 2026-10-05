//! js-param-defs PR-A ("binding shapes"): JS/TS/TSX parameter Defs for the
//! unparenthesised arrow formal (`x => …`) and identifier rest formals
//! (`...xs`). SPEC: docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prA.md.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::navigation::queries::dfg_edge_dump;
use std::collections::{BTreeMap, BTreeSet};

const JS_TS: [Language; 3] = [Language::JavaScript, Language::TypeScript, Language::Tsx];

fn file_name(language: Language) -> &'static str {
    match language {
        Language::JavaScript => "shapes.js",
        Language::TypeScript => "shapes.ts",
        Language::Tsx => "shapes.tsx",
        _ => unreachable!(),
    }
}

fn parse(language: Language, source: &str) -> ParsedFile {
    let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    parsed
}

fn build(language: Language, source: &str) -> CodePropertyGraph {
    let files = BTreeMap::from([(file_name(language).to_string(), parse(language, source))]);
    CodePropertyGraph::build(&files)
}

/// (owner function, parameter path, start byte, end byte) of every Def node
/// pinned to its function's start line (parameter Defs live there).
fn defs(cpg: &CodePropertyGraph) -> BTreeSet<(String, String, usize, usize)> {
    cpg.graph
        .node_indices()
        .filter_map(|node| match cpg.node(node) {
            CpgNode::Variable {
                path,
                function,
                access: VarAccess::Def,
                start_byte,
                end_byte,
                ..
            } => Some((function.clone(), path.to_string(), *start_byte, *end_byte)),
            _ => None,
        })
        .collect()
}

/// (caller argument path, callee parameter path) DataFlow Use->Def edges.
fn argument_edges(cpg: &CodePropertyGraph) -> BTreeSet<(String, String, String)> {
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
                        access: VarAccess::Use,
                        ..
                    },
                    CpgNode::Variable {
                        path: parameter,
                        function,
                        access: VarAccess::Def,
                        ..
                    },
                ) => Some((path.to_string(), function.clone(), parameter.to_string())),
                _ => None,
            }
        })
        .collect()
}

fn dump(language: Language, source: &str) -> Vec<String> {
    dfg_edge_dump(&build(language, source))
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect()
}

fn span(source: &str, needle: &str, offset: usize) -> usize {
    source.find(needle).unwrap() + offset
}

fn has_def(cpg: &CodePropertyGraph, function: &str, name: &str, start: usize) -> bool {
    defs(cpg).contains(&(
        function.to_string(),
        name.to_string(),
        start,
        start + name.len(),
    ))
}

#[test]
fn bare_arrow_formal_gets_a_def_at_its_identifier_bytes() {
    let source = "const run = cmd => {\n  exec(cmd);\n};\n\
                  const later = async job => {\n  await job;\n};\n";
    for language in JS_TS {
        let cpg = build(language, source);
        let cmd = span(source, "cmd =>", 0);
        let job = span(source, "job =>", 0);
        assert!(
            has_def(&cpg, "run", "cmd", cmd),
            "{language:?}: {:?}",
            defs(&cpg)
        );
        assert!(
            has_def(&cpg, "later", "job", job),
            "{language:?}: async bare arrow"
        );
    }
}

#[test]
fn identifier_rest_formal_gets_a_def_but_never_a_positional_argument() {
    let js = "function collect(first, ...rest) {\n  use(rest);\n  return first;\n}\n\
              function run(v) {\n  collect(v, v, v);\n}\n";
    let ts = "function collect(first: any, ...rest: any[]) {\n  use(rest);\n  return first;\n}\n\
              function run(v: any) {\n  collect(v, v, v);\n}\n";
    for (language, source) in [
        (Language::JavaScript, js),
        (Language::TypeScript, ts),
        (Language::Tsx, ts),
    ] {
        let cpg = build(language, source);
        let rest = span(source, "...rest", 3);
        assert!(
            has_def(&cpg, "collect", "rest", rest),
            "{language:?}: {:?}",
            defs(&cpg)
        );
        let edges = argument_edges(&cpg);
        assert!(
            edges.contains(&("v".into(), "collect".into(), "first".into())),
            "{language:?}: {edges:?}"
        );
        // Variadic: slots stop at rest, so Step 5b binds no argument to it.
        assert!(
            !edges.iter().any(|(_, _, parameter)| parameter == "rest"),
            "{language:?}: an argument was bound to the rest formal: {edges:?}"
        );
    }
}

#[test]
fn bare_arrow_formal_stays_a_step5b_hole_but_the_parenthesised_control_binds() {
    // SPEC D12: the call ladder admits name-inferred callables as Exact
    // `free_single` targets, so PR-A does not bind arguments to the new
    // bare formal; the pre-existing parenthesised behaviour is unchanged.
    let js = "const take = cmd =>\n  exec(cmd);\nconst paren = (cmd) =>\n  exec(cmd);\n\
              function run(v) {\n  take(v);\n  paren(v);\n}\n";
    let ts = "const take = cmd =>\n  exec(cmd);\nconst paren = (cmd: any) =>\n  exec(cmd);\n\
              function run(v: any) {\n  take(v);\n  paren(v);\n}\n";
    for (language, source) in [
        (Language::JavaScript, js),
        (Language::TypeScript, ts),
        (Language::Tsx, ts),
    ] {
        let cpg = build(language, source);
        let take_cmd = span(source, "cmd =>", 0);
        assert!(has_def(&cpg, "take", "cmd", take_cmd), "{language:?}");
        let edges = argument_edges(&cpg);
        assert!(
            edges.contains(&("v".into(), "paren".into(), "cmd".into())),
            "{language:?}: parenthesised control lost its argument edge: {edges:?}"
        );
        assert!(
            !edges.iter().any(|(_, callee, _)| callee == "take"),
            "{language:?}: an argument was bound to the bare formal: {edges:?}"
        );
    }
}

#[test]
fn new_shapes_refuse_a_formal_rebound_by_a_nested_callable() {
    // SPEC D11: the legacy reference walk does not fence a nested callable's
    // formals, so these Defs would reach Uses of the inner binding.
    let js = "const outer = name => {\n  use(name);\n  list.forEach((entry, name) => use(name));\n};\n\
              function rest(...xs) {\n  use(xs);\n  run(function (xs) { use(xs); });\n}\n\
              const kept = other => {\n  use(other);\n  list.forEach((entry, name) => use(name));\n};\n";
    let ts = "const outer = name => {\n  use(name);\n  list.forEach((entry: any, name: any) => use(name));\n};\n\
              function rest(...xs: any[]) {\n  use(xs);\n  run(function (xs: any) { use(xs); });\n}\n\
              const kept = other => {\n  use(other);\n  list.forEach((entry: any, name: any) => use(name));\n};\n";
    for (language, source) in [
        (Language::JavaScript, js),
        (Language::TypeScript, ts),
        (Language::Tsx, ts),
    ] {
        let all = defs(&build(language, source));
        assert!(
            !all.iter()
                .any(|(owner, path, _, _)| owner == "outer" && path == "name"),
            "{language:?}: {all:?}"
        );
        assert!(
            !all.iter()
                .any(|(owner, path, _, _)| owner == "rest" && path == "xs"),
            "{language:?}: {all:?}"
        );
        let other = span(source, "other =>", 0);
        assert!(
            all.contains(&("kept".into(), "other".into(), other, other + 5)),
            "{language:?}: an unrelated nested formal must not refuse the Def: {all:?}"
        );
    }
}

#[test]
fn bare_arrow_labels_match_the_parenthesised_control() {
    // Same lines, same names; only the parentheses differ. The destructuring
    // read on line 3 is a pre-existing conservative kill for the parenthesised
    // form; the bare form must be classified identically (RD declaration seed).
    let bare = "const o = {\n  fix: context => {\n    const { file } = context;\n    use(file);\n    return go(context);\n  },\n};\n";
    let paren = "const o = {\n  fix: (context) => {\n    const { file } = context;\n    use(file);\n    return go(context);\n  },\n};\n";
    for language in JS_TS {
        let bare_rows = dump(language, bare);
        assert!(
            bare_rows
                .iter()
                .any(|row| row.contains("\"base\":\"context\"")),
            "{language:?}: bare arrow formal produced no rows: {bare_rows:?}"
        );
        assert_eq!(bare_rows, dump(language, paren), "{language:?}");
    }
}

#[test]
fn rest_labels_match_the_plain_parameter_control() {
    let rest = "function r(a, ...xs) {\n  xs = a;\n  use(xs);\n  return xs;\n}\n";
    let plain = "function r(a, xs) {\n  xs = a;\n  use(xs);\n  return xs;\n}\n";
    let rest_ts = "function r(a: any, ...xs: any[]) {\n  xs = a;\n  use(xs);\n  return xs;\n}\n";
    let plain_ts = "function r(a: any, xs: any[]) {\n  xs = a;\n  use(xs);\n  return xs;\n}\n";
    for (language, rest, plain) in [
        (Language::JavaScript, rest, plain),
        (Language::TypeScript, rest_ts, plain_ts),
        (Language::Tsx, rest_ts, plain_ts),
    ] {
        let rest_rows = dump(language, rest);
        assert!(
            rest_rows.iter().any(|row| row.contains("\"base\":\"xs\"")),
            "{language:?}: {rest_rows:?}"
        );
        assert_eq!(rest_rows, dump(language, plain), "{language:?}");
    }
}

#[test]
fn curried_arrows_bind_only_the_named_outer_formal() {
    // `y => …` is anonymous (no name inference from an arrow body); its
    // callable identity is PR-B. The outer named arrow gains `x` only.
    let source = "const curry = x => y =>\n  sink(x, y);\n";
    for language in JS_TS {
        let cpg = build(language, source);
        let x = span(source, "x =>", 0);
        assert!(has_def(&cpg, "curry", "x", x), "{language:?}");
        assert!(
            !defs(&cpg).iter().any(|(_, name, _, _)| name == "y"),
            "{language:?}: anonymous inner arrow gained a Def: {:?}",
            defs(&cpg)
        );
    }
}

#[test]
fn refused_shapes_gain_no_def() {
    let js = "function arr(...[p, q]) {\n  use(p, q);\n}\n\
              function obj(...{ p }) {\n  use(p);\n}\n\
              function twice(m, ...m) {\n  use(m);\n}\n";
    let ts = "function arr(...[p, q]: any[]) {\n  use(p, q);\n}\n\
              function obj(...{ p }: any) {\n  use(p);\n}\n\
              function self(this: Foo, ...items: any[]) {\n  use(this, items);\n}\n\
              const esc = \\u0061 => use(a);\n";
    for (language, source) in [
        (Language::JavaScript, js),
        (Language::TypeScript, ts),
        (Language::Tsx, ts),
    ] {
        let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
        let files = BTreeMap::from([(file_name(language).to_string(), parsed)]);
        let all = defs(&CodePropertyGraph::build(&files));
        for (function, name) in [("arr", "p"), ("arr", "q"), ("obj", "p"), ("self", "this")] {
            assert!(
                !all.iter()
                    .any(|(owner, path, _, _)| owner == function && path == name),
                "{language:?}: refused {function}::{name} gained a Def: {all:?}"
            );
        }
        if language == Language::JavaScript {
            // Duplicate bindings: the rest formal is refused (the list is
            // invalid); the pre-existing plain-formal occurrence is untouched.
            let m: Vec<_> = all
                .iter()
                .filter(|(owner, path, _, _)| owner == "twice" && path == "m")
                .collect();
            let first_m = source.find("m, ...m").unwrap();
            assert_eq!(
                m.iter().map(|(_, _, start, _)| *start).collect::<Vec<_>>(),
                vec![first_m],
                "{all:?}"
            );
        } else {
            let items = span(source, "...items", 3);
            assert!(
                all.contains(&("self".into(), "items".into(), items, items + 5)),
                "{language:?}: a `this` pseudo-parameter must not block the rest Def: {all:?}"
            );
            assert!(
                !all.iter()
                    .any(|(owner, path, _, _)| owner == "esc" && path == "a"),
                "{language:?}: escaped identifier spelling is not canonical identity: {all:?}"
            );
        }
    }
}

#[test]
fn ts_duplicate_rest_binding_refuses_the_whole_list() {
    let source = "function twice(m: any, ...m: any[]) {\n  use(m);\n}\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
        let files = BTreeMap::from([(file_name(language).to_string(), parsed)]);
        let all = defs(&CodePropertyGraph::build(&files));
        assert!(
            !all.iter()
                .any(|(owner, path, _, _)| owner == "twice" && path == "m"),
            "{language:?}: {all:?}"
        );
    }
}
