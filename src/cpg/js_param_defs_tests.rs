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
    // `y => …` is anonymous (no name inference from an arrow body). PR-B
    // (intended re-pin): it owns `y` under its synthetic `<cb@L:C>` identity;
    // the outer named arrow still gains `x` only.
    let source = "const curry = x => y =>\n  sink(x, y);\n";
    for language in JS_TS {
        let cpg = build(language, source);
        let x = span(source, "x =>", 0);
        let y = span(source, "y =>", 0);
        assert!(has_def(&cpg, "curry", "x", x), "{language:?}");
        assert!(
            has_def(&cpg, &format!("<cb@1:{}>", y + 1), "y", y),
            "{language:?}"
        );
        assert!(
            !defs(&cpg)
                .iter()
                .any(|(owner, name, _, _)| name == "y" && owner == "curry"),
            "{language:?}: the named outer arrow owns the inner formal: {:?}",
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
            // R4: a non-simple duplicate list is an ECMAScript early error.
            // Refuse the entire callable, including the plain formal.
            let m: Vec<_> = all
                .iter()
                .filter(|(owner, path, _, _)| owner == "twice" && path == "m")
                .collect();
            assert!(m.is_empty(), "{all:?}");
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

fn refuses_competing_binding(body: &str) {
    for language in JS_TS {
        for signature in ["const outer = x =>", "function outer(...x)"] {
            let source = format!("{signature} {{\n use(x);\n {body}\n}}\n");
            let cpg = build(language, &source);
            let start = source
                .find(if signature.starts_with("const") {
                    "x =>"
                } else {
                    "...x"
                })
                .unwrap()
                + if signature.starts_with("const") { 0 } else { 3 };
            assert!(
                !has_def(&cpg, "outer", "x", start),
                "{language:?}: {source}: {:?}",
                defs(&cpg)
            );
        }
    }
}

#[test]
fn r1_catch_binding_refused() {
    refuses_competing_binding("try { fail(); } catch (x) { sink(x); }");
}
#[test]
fn r1_for_head_bindings_refused() {
    for kind in ["const", "let", "var"] {
        for op in ["of", "in"] {
            refuses_competing_binding(&format!("for ({kind} x {op} ys) {{ sink(x); }}"));
        }
    }
}
#[test]
fn r1_named_function_binding_refused() {
    refuses_competing_binding("return function x() { sink(x); };");
}
#[test]
fn r1_named_class_binding_refused() {
    refuses_competing_binding("return class x { method() { sink(x); } };");
}
#[test]
fn r1_block_bindings_refused() {
    for body in [
        "{ let x = other; sink(x); }",
        "{ const x = other; sink(x); }",
        "{ class x {} sink(x); }",
        "{ function x() {} sink(x); }",
        "{ let {value: x} = other; sink(x); }",
    ] {
        refuses_competing_binding(body);
    }
    // The block and callable begin on the same line (line-based scope leak).
    for language in JS_TS {
        let source = "const outer = x => { { let x = other;\n sink(x); } };";
        assert!(!has_def(
            &build(language, source),
            "outer",
            "x",
            source.find("x =>").unwrap()
        ));
    }
}
#[test]
fn r1_escaped_bindings_refused() {
    refuses_competing_binding(r"{ let \u0078 = other; sink(x); }");
    refuses_competing_binding(r"return function (\u0078) { sink(x); };");
    // Same-line owners and two different Uses on a line (W3).
    refuses_competing_binding(r"{ let \u0078 = other; sink(x); } use(x);");
}
#[test]
fn r1_unrelated_bindings_and_captures_kept() {
    for language in JS_TS {
        let source = "const outer = x => {\n try {} catch (y) { sink(x); }\n for (const y of ys) { sink(x); }\n const inner = function y(z) { sink(x, z); };\n { let y = other; sink(x); }\n return x;\n};";
        assert!(has_def(
            &build(language, source),
            "outer",
            "x",
            source.find("x =>").unwrap()
        ));
    }
}
#[test]
fn r1_rest_trailing_comma_refused() {
    for language in JS_TS {
        for suffix in [",", " /* comment */, /* after */"] {
            let source = format!("function outer(...x{suffix}) {{\n sink(x);\n}}");
            assert!(!has_def(
                &build(language, &source),
                "outer",
                "x",
                source.find("...x").unwrap() + 3
            ));
        }
        let source = "function outer(...x /* valid */) {\n sink(x);\n}";
        assert!(has_def(
            &build(language, source),
            "outer",
            "x",
            source.find("...x").unwrap() + 3
        ));
    }
}

fn source_reaches_inferred(
    language: Language,
    signature: &str,
    body: &str,
    inferred: bool,
) -> bool {
    use petgraph::visit::EdgeRef;
    let input = format!(
        "{}{signature}\n {body}",
        if inferred {
            ""
        } else {
            "import { Array } from './callee';\n"
        }
    );
    let target = if inferred {
        "const obj = { Array: (s) =>\n sink(s)\n};"
    } else {
        "export function Array(s) {\n sink(s);\n}"
    };
    let ext = if language == Language::JavaScript {
        "js"
    } else if language == Language::Tsx {
        "tsx"
    } else {
        "ts"
    };
    let files = BTreeMap::from([
        (
            format!("input.{ext}"),
            ParsedFile::parse(&format!("input.{ext}"), &input, language).unwrap(),
        ),
        (
            format!("callee.{ext}"),
            ParsedFile::parse(&format!("callee.{ext}"), target, language).unwrap(),
        ),
    ]);
    let cpg = CodePropertyGraph::build(&files);
    let start = input
        .find("input =>")
        .or_else(|| input.find("...input").map(|n| n + 3))
        .unwrap();
    let sources: Vec<_> = cpg.graph.node_indices().filter(|&i| matches!(cpg.node(i), CpgNode::Variable { file, access: VarAccess::Def, start_byte, .. } if file.starts_with("input.") && *start_byte == start)).collect();
    assert_eq!(sources.len(), 1, "valid source binding must remain");
    let mut seen = BTreeSet::new();
    let mut stack = sources;
    while let Some(n) = stack.pop() {
        if !seen.insert(n) {
            continue;
        }
        if matches!(cpg.node(n), CpgNode::Variable { file, line: 2, access: VarAccess::Use, path, .. } if file.starts_with("callee.") && path.base == "s")
        {
            return true;
        }
        stack.extend(
            cpg.graph
                .edges(n)
                .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
                .map(|e| e.target()),
        );
    }
    false
}
#[test]
fn r1_e5_new_source_direct_and_local_paths_refused() {
    for language in JS_TS {
        for (signature, body) in [
            ("const entry = input =>", "Array(input);"),
            (
                "const entry = input => {",
                "const local = input;\n Array(local);\n}",
            ),
            (
                "function entry(...input) {",
                "const local = input;\n Array(local);\n}",
            ),
        ] {
            assert!(
                !source_reaches_inferred(language, signature, body, true),
                "{language:?}: {body}"
            );
        }
    }
}
#[test]
fn r1_e5_referenceable_target_kept() {
    for language in JS_TS {
        assert!(source_reaches_inferred(
            language,
            "const entry = input =>",
            "Array(input);",
            false
        ));
    }
}
#[test]
fn r1_unreachable_rest_is_plain_formal_base_parity() {
    for language in JS_TS {
        assert_eq!(
            dump(language, "function outer(...x) {\n return;\n sink(x);\n}"),
            dump(language, "function outer(x) {\n return;\n sink(x);\n}")
        );
    }
}

#[test]
fn r1_ts_runtime_declaration_bindings_refused() {
    for language in [Language::TypeScript, Language::Tsx] {
        for binding in [
            "abstract class x {}",
            "enum x { A }",
            "namespace x { export const y = 1; }",
        ] {
            let source = format!("const outer = x => {{\n use(x);\n {{ {binding} sink(x); }}\n}};");
            assert!(
                !has_def(
                    &build(language, &source),
                    "outer",
                    "x",
                    source.find("x =>").unwrap()
                ),
                "{language:?}: {binding}"
            );
        }
        let source = "const outer = x => {\n { enum y { A } sink(x); }\n return x;\n};";
        assert!(has_def(
            &build(language, source),
            "outer",
            "x",
            source.find("x =>").unwrap()
        ));
    }
}

fn r2_target_reached(language: Language, input: &str, target: &str, callee: &str) -> bool {
    use crate::resolution::{ResolutionConfidence, ResolutionKind};
    use petgraph::visit::EdgeRef;
    let ext = file_name(language).rsplit('.').next().unwrap();
    let files = BTreeMap::from([
        (
            format!("input.{ext}"),
            ParsedFile::parse(&format!("input.{ext}"), input, language).unwrap(),
        ),
        (
            format!("callee.{ext}"),
            ParsedFile::parse(&format!("callee.{ext}"), target, language).unwrap(),
        ),
    ]);
    assert!(files.values().all(|p| p.parse_error_count == 0));
    let cpg = CodePropertyGraph::build(&files);
    let site = cpg
        .call_graph
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == callee)
        .unwrap();
    let resolved = cpg.call_graph.resolve_call_site(site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(
        resolved[0].kind,
        if input.starts_with("import ") {
            ResolutionKind::ImportMember
        } else {
            ResolutionKind::FreeSingle
        }
    );
    assert_ne!(resolved[0].target.file, site.caller.file);
    let start = input
        .find("input =>")
        .or_else(|| input.find("...input").map(|n| n + 3))
        .unwrap();
    let sources: Vec<_> = cpg
        .graph
        .node_indices()
        .filter(|&i| {
            matches!(cpg.node(i),
        CpgNode::Variable { file, access: VarAccess::Def, start_byte, .. }
        if file.starts_with("input.") && *start_byte == start)
        })
        .collect();
    assert_eq!(sources.len(), 1, "the static source Def must remain");
    let mut seen = BTreeSet::new();
    let mut stack = sources;
    while let Some(node) = stack.pop() {
        if !seen.insert(node) {
            continue;
        }
        if matches!(cpg.node(node), CpgNode::Variable {
            file, line: 2, access: VarAccess::Use, path, ..
        } if file.starts_with("callee.") && path.base == "s")
        {
            return true;
        }
        stack.extend(
            cpg.graph
                .edges(node)
                .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
                .map(|e| e.target()),
        );
    }
    false
}

fn r2_w2m(language: Language) {
    let target = "const obj = { Array(s) {\n sink(s);\n} };";
    for input in [
        "const entry = input =>\n Array(input);",
        "const entry = input => {\n const local = input;\n Array(local);\n};",
        "function entry(...input) {\n const local = input;\n Array(local);\n}",
    ] {
        assert!(
            !r2_target_reached(language, input, target, "Array"),
            "{language:?}: {input}"
        );
    }
}

fn r2_w2d(language: Language) {
    let target = "export function escape(s) {\n sink(s);\n}";
    for input in [
        "export const entry = input =>\n escape(input);",
        "export const entry = input => {\n const local = input;\n escape(local);\n};",
        "export function entry(...input) {\n const local = input;\n escape(local);\n}",
    ] {
        assert!(
            !r2_target_reached(language, input, target, "escape"),
            "{language:?}: {input}"
        );
    }
}

#[test]
fn r2_w2m_js() {
    r2_w2m(Language::JavaScript);
}
#[test]
fn r2_w2m_ts() {
    r2_w2m(Language::TypeScript);
}
#[test]
fn r2_w2m_tsx() {
    r2_w2m(Language::Tsx);
}
#[test]
fn r2_w2d_js() {
    r2_w2d(Language::JavaScript);
}
#[test]
fn r2_w2d_ts() {
    r2_w2d(Language::TypeScript);
}
#[test]
fn r2_w2d_tsx() {
    r2_w2d(Language::Tsx);
}

#[test]
fn r2_e5_imported_target_kept() {
    for language in JS_TS {
        assert!(r2_target_reached(
            language,
            "import { escape } from './callee';\nexport const entry = input =>\n escape(input);",
            "export function escape(s) {\n sink(s);\n}",
            "escape"
        ));
    }
}

#[test]
fn r2_e5_nameonly_targets_kept() {
    use crate::resolution::ResolutionConfidence;
    for language in JS_TS {
        let ext = file_name(language).rsplit('.').next().unwrap();
        let input = "const entry = program =>\n cb(program);";
        let files = BTreeMap::from([
            (
                format!("input.{ext}"),
                ParsedFile::parse(&format!("input.{ext}"), input, language).unwrap(),
            ),
            (
                format!("callee.{ext}"),
                ParsedFile::parse(
                    &format!("callee.{ext}"),
                    "const one = { cb: (s) =>\n sink(s) };\nconst two = { cb: (s) =>\n sink(s) };",
                    language,
                )
                .unwrap(),
            ),
        ]);
        let cpg = CodePropertyGraph::build(&files);
        let site = cpg
            .call_graph
            .calls
            .values()
            .flatten()
            .find(|s| s.callee_name == "cb")
            .unwrap();
        let resolved = cpg.call_graph.resolve_call_site(site);
        assert_eq!(resolved.len(), 2);
        assert!(resolved
            .iter()
            .all(|r| r.confidence == ResolutionConfidence::NameOnly));
        assert!(
            dfg_edge_dump(&cpg)
                .iter()
                .any(|r| r.from.path.base == "program"
                    && r.from.access == "def"
                    && r.to.access == "use"),
            "NameOnly callee candidates must not suppress correct source rows"
        );
    }
}

#[test]
fn r2_e5_same_file_target_kept() {
    for language in JS_TS {
        let source = "function escape(s) {\n sink(s);\n}\nconst entry = input =>\n escape(input);";
        let cpg = build(language, source);
        assert!(dfg_edge_dump(&cpg)
            .iter()
            .any(|r| r.from.path.base == "input"
                && r.from.access == "def"
                && r.to.access == "use"));
    }
}

#[test]
fn r2_e7_nonreference_positions_are_plain_formal_parity() {
    for suffix in [
        "q(<C x={1} />, function () {\n return x; });",
        "q({ x: 1 }, function () {\n return x; });",
    ] {
        let bare = format!("const g = x => {suffix}");
        let plain = format!("const g = (x) => {suffix}");
        assert_eq!(dump(Language::Tsx, &bare), dump(Language::Tsx, &plain));
        let key = bare
            .find(if suffix.starts_with("q(<") {
                "x={"
            } else {
                "x: "
            })
            .unwrap();
        let cpg = build(Language::Tsx, &bare);
        assert!(cpg.graph.edge_indices().any(|e| {
            let (from, to) = cpg.graph.edge_endpoints(e).unwrap();
            matches!(cpg.graph[e], CpgEdge::DataFlow(_))
                && matches!(cpg.node(from), CpgNode::Variable { access: VarAccess::Def, path, .. } if path.base == "x")
                && matches!(cpg.node(to), CpgNode::Variable { access: VarAccess::Use, start_byte, end_byte, .. } if (*start_byte, *end_byte) == (key, key + 1))
        }), "the disclosed non-reference Use must be represented at its exact bytes");
    }
}
