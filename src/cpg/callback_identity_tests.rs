//! js-param-defs PR-B ("callback identity"): anonymous JS/TS/TSX callables get
//! their own DFG pass under a synthetic, non-referenceable `<cb@L:C>` owner;
//! every JS/TS reference walk is fenced at nested binders (E3).
//! SPEC: docs/superpowers/plans/2026-10-06-js-param-defs/SPEC-prB.md.

use super::build::CodePropertyGraph;
use super::{CpgEdge, CpgNode, VarAccess};
use crate::ast::{is_synthetic_owner, ParsedFile};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

const JS_TS: [Language; 3] = [Language::JavaScript, Language::TypeScript, Language::Tsx];

fn r5_full_binding_rows(
    cpg: &CodePropertyGraph,
    owner: &str,
    name: &str,
) -> Vec<serde_json::Value> {
    let mut result = Vec::new();
    for e in cpg.graph.edge_indices() {
        let CpgEdge::DataFlow(label) = cpg.graph[e] else {
            continue;
        };
        let (a, b) = cpg.graph.edge_endpoints(e).unwrap();
        if let (
            CpgNode::Variable {
                function,
                path,
                line,
                start_byte,
                end_byte,
                access: VarAccess::Def,
                ..
            },
            CpgNode::Variable {
                function: to_owner,
                line: to_line,
                start_byte: to_byte,
                end_byte: to_end,
                access: VarAccess::Use,
                ..
            },
        ) = (cpg.node(a), cpg.node(b))
        {
            if function == owner && to_owner == owner && path.is_simple() && path.base == name {
                let (doubt, kill) = match label {
                    super::FlowConfidence::Exact => (None, None),
                    super::FlowConfidence::NameOnly(super::FlowDoubt::Killed { kill_line }) => {
                        (Some("killed"), Some(kill_line))
                    }
                    super::FlowConfidence::NameOnly(super::FlowDoubt::SameLine) => {
                        (Some("sameline"), None)
                    }
                    super::FlowConfidence::NameOnly(super::FlowDoubt::CfgIncomplete) => {
                        (Some("cfg_incomplete"), None)
                    }
                    super::FlowConfidence::NameOnly(super::FlowDoubt::AliasUnstable) => {
                        (Some("alias_unstable"), None)
                    }
                    super::FlowConfidence::NameOnly(super::FlowDoubt::CallNameOnly) => {
                        (Some("call_nameonly"), None)
                    }
                };
                result.push(serde_json::json!([
                    line,
                    start_byte,
                    end_byte,
                    to_line,
                    to_byte,
                    to_end,
                    label.level(),
                    doubt,
                    kill
                ]));
            }
        }
    }
    result.sort_by_key(ToString::to_string);
    result
}

#[test]
fn r6_named_seam_fallback_excludes_nonformal_body_locals() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_param_r6_local_golden.json")).unwrap();
    let mut failures = Vec::new();
    for case in cases.as_array().unwrap() {
        let source = case["source"].as_str().unwrap();
        let name = case["name"].as_str().unwrap();
        for (language, ext) in JS_TS.into_iter().zip(["js", "ts", "tsx"]) {
            let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
            let owner = parsed.all_functions().into_iter().next().unwrap();
            if parsed.js_ts_seam_binding(&owner, name) {
                failures.push(format!(
                    "{}/{ext}: nonformal {name} classified as SEAM",
                    case["id"]
                ));
            }
            let actual = r5_full_binding_rows(&build(language, source), "h", name);
            let mut expected = case["golden"][ext].as_array().unwrap().clone();
            expected.sort_by_key(ToString::to_string);
            if actual != expected {
                failures.push(format!(
                    "{}/{ext}: expected={expected:?}; actual={actual:?}",
                    case["id"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r5_named_seam_rows_equal_frozen_main_goldens() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_param_r5_golden.json")).unwrap();
    let mut failures = Vec::new();
    for c in cases.as_array().unwrap() {
        if c["id"].as_str().unwrap().contains("callback") {
            continue;
        }
        for (language, ext) in JS_TS.into_iter().zip(["js", "ts", "tsx"]) {
            let actual =
                r5_full_binding_rows(&build(language, c["source"].as_str().unwrap()), "h", "f");
            let mut expected = c["main"][ext].as_array().unwrap().clone();
            expected.sort_by_key(ToString::to_string);
            if actual != expected {
                failures.push(format!(
                    "{}/{ext}: expected={expected:?}; actual={actual:?}",
                    c["id"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r5_synthetic_seam_formals_and_parameter_endpoints_are_refused() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_param_r5_golden.json")).unwrap();
    let mut failures = Vec::new();
    for c in cases.as_array().unwrap() {
        let original = c["source"].as_str().unwrap();
        let source = if original.starts_with("function h") {
            original
                .replacen("function h", "register(function", 1)
                .trim_end()
                .strip_suffix('}')
                .unwrap()
                .to_string()
                + "});\n"
        } else {
            original.to_string()
        };
        for language in JS_TS {
            let parsed = ParsedFile::parse(file_name(language), &source, language).unwrap();
            let owner = parsed
                .all_functions()
                .into_iter()
                .find(|f| parsed.js_ts_synthetic_callable_name(f).is_some())
                .unwrap();
            let name = parsed.js_ts_synthetic_callable_name(&owner).unwrap();
            let params = parsed.parameter_binding_region(&owner).unwrap();
            let cpg = build(language, &source);
            for node in cpg.graph.node_indices() {
                if matches!(cpg.node(node), CpgNode::Variable { function, path, start_byte, .. }
                    if function == &name && path.base == "f" && params.start_byte() <= *start_byte && *start_byte < params.end_byte())
                {
                    failures.push(format!(
                        "{}/{language:?}: parameter endpoint {:?}",
                        c["id"],
                        cpg.node(node)
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r5_eval_and_mapped_arguments_refuse_all_formals_with_negative_controls() {
    let mut failures = Vec::new();
    for language in JS_TS {
        for (prefix, signature, body, refused) in [
            ("", "f,q", "eval('f=9');use(f,q);", true),
            ("", "f,q", "function inner(){eval('f=9');}use(f,q);", true),
            ("", "f,q", "arguments[0]=9;use(f,q);", true),
            ("", "f,q", "(()=>arguments[0])();use(f,q);", true),
            ("'use strict';", "f,q", "arguments[0]=9;use(f,q);", false),
            ("export {};", "f,q", "arguments[0]=9;use(f,q);", false),
            (
                "",
                "f,q",
                "function inner(){return arguments[0];}use(f,q);",
                false,
            ),
            ("", "f,q", "o.eval('f=9');use(f,q);", false),
            ("", "f,q", "(0,eval)('f=9');use(f,q);", false),
            ("", "f,q", "eval?.('f=9');use(f,q);", false),
            ("", "f,q=0", "arguments[0]=9;use(f,q);", false),
            ("", "f,d=0", "use(f,d);", false),
            ("", "f,q", "var f;use(f,q);", false),
        ] {
            let source = format!("{prefix}register(function({signature}){{{body}}});");
            let cpg = build(language, &source);
            let parsed = ParsedFile::parse(file_name(language), &source, language).unwrap();
            let f = parsed
                .all_functions()
                .into_iter()
                .find(|f| parsed.js_ts_synthetic_callable_name(f).is_some())
                .unwrap();
            let owner = parsed.js_ts_synthetic_callable_name(&f).unwrap();
            for (name, byte, _) in parsed.function_parameter_occurrences(&f) {
                let admitted = defs(&cpg).contains(&(owner.clone(), name.clone(), byte));
                if admitted == refused {
                    failures.push(format!(
                        "{language:?}: {source}: {name} admitted={admitted}, refused={refused}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r5_seam_body_vars_keep_assignments_and_refuse_default_reads() {
    for language in JS_TS {
        for signature in ["f,d=f", "f=1,d=f", "f,d=()=>f", "f,d=(f=5),e=f"] {
            let source = format!(
                "register(function({signature}){{\n use(f);\n var f;\n f=2;\n use(f);\n}});"
            );
            let cpg = build(language, &source);
            let owner = "<cb@1:10>";
            let actual = r5_full_binding_rows(&cpg, owner, "f");
            let def_byte = source.find("f=2").unwrap();
            let use_byte = source.rfind("use(f)").unwrap() + 4;
            assert_eq!(
                actual,
                vec![serde_json::json!([
                    4,
                    def_byte,
                    def_byte + 1,
                    5,
                    use_byte,
                    use_byte + 1,
                    "exact",
                    null,
                    null
                ])],
                "{language:?}: {source}"
            );
            let parsed = ParsedFile::parse(file_name(language), &source, language).unwrap();
            let function = parsed
                .all_functions()
                .into_iter()
                .find(|f| parsed.js_ts_synthetic_callable_name(f).as_deref() == Some(owner))
                .unwrap();
            let params = parsed.parameter_binding_region(&function).unwrap();
            for node in cpg.graph.node_indices() {
                assert!(
                    !matches!(cpg.node(node), CpgNode::Variable { function, path, start_byte, .. }
                    if function == owner && path.base == "f" && params.start_byte() <= *start_byte && *start_byte < params.end_byte()),
                    "{language:?}: {source}: {:?}",
                    cpg.node(node)
                );
            }
        }
    }
}

fn file_name(language: Language) -> &'static str {
    match language {
        Language::JavaScript => "cb.js",
        Language::TypeScript => "cb.ts",
        Language::Tsx => "cb.tsx",
        Language::Python => "cb.py",
        Language::Go => "cb.go",
        Language::Rust => "cb.rs",
        _ => unreachable!(),
    }
}

fn build(language: Language, source: &str) -> CodePropertyGraph {
    let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{language:?}: {source}");
    let files = BTreeMap::from([(file_name(language).to_string(), parsed)]);
    CodePropertyGraph::build(&files)
}

/// (owner, path, def line, use line, def start byte, use start byte) of every
/// intraprocedural Def->Use DataFlow edge.
type Row = (String, String, usize, usize, usize, usize);

fn rows(cpg: &CodePropertyGraph) -> BTreeSet<Row> {
    cpg.graph
        .edge_indices()
        .filter(|&e| matches!(cpg.graph[e], CpgEdge::DataFlow(_)))
        .filter_map(|e| {
            let (from, to) = cpg.graph.edge_endpoints(e)?;
            match (cpg.node(from), cpg.node(to)) {
                (
                    CpgNode::Variable {
                        function,
                        path,
                        line: def_line,
                        access: VarAccess::Def,
                        start_byte: def_byte,
                        ..
                    },
                    CpgNode::Variable {
                        function: use_fn,
                        line: use_line,
                        access: VarAccess::Use,
                        start_byte: use_byte,
                        ..
                    },
                ) if function == use_fn => Some((
                    function.clone(),
                    path.to_string(),
                    *def_line,
                    *use_line,
                    *def_byte,
                    *use_byte,
                )),
                _ => None,
            }
        })
        .collect()
}

fn defs(cpg: &CodePropertyGraph) -> BTreeSet<(String, String, usize)> {
    cpg.graph
        .node_indices()
        .filter_map(|n| match cpg.node(n) {
            CpgNode::Variable {
                function,
                path,
                access: VarAccess::Def,
                start_byte,
                ..
            } => Some((function.clone(), path.to_string(), *start_byte)),
            _ => None,
        })
        .collect()
}

fn at(source: &str, needle: &str, nth: usize) -> usize {
    source.match_indices(needle).nth(nth).unwrap().0
}

fn edge(rows: &BTreeSet<Row>, owner: &str, path: &str, def_line: usize, use_line: usize) -> bool {
    rows.iter()
        .any(|r| r.0 == owner && r.1 == path && r.2 == def_line && r.3 == use_line)
}

#[test]
fn top_level_callback_formal_gets_a_synthetic_owner_and_def() {
    let src = "http.createServer(function(req, res) {\n  exec(req);\n}).listen(1);\n";
    for language in JS_TS {
        let cpg = build(language, src);
        let rows = rows(&cpg);
        assert!(
            edge(&rows, "<cb@1:19>", "req", 1, 2),
            "{language:?}: {rows:?}"
        );
        assert!(defs(&cpg).contains(&("<cb@1:19>".into(), "req".into(), at(src, "req", 0))));
    }
}

#[test]
fn every_anonymous_parent_kind_gets_its_own_pass() {
    // call argument, `.map`, IIFE, return, assignment RHS, nested statement.
    let src = "list.map(x => {\n  sink(x);\n});\n(function (a) {\n  sink(a);\n})(1);\nfunction mk() {\n  return function (r) {\n    sink(r);\n  };\n}\nobj[k] = function (z) {\n  sink(z);\n};\n";
    for language in JS_TS {
        let rows = rows(&build(language, src));
        assert!(
            edge(&rows, "<cb@1:10>", "x", 1, 2),
            "{language:?} map: {rows:?}"
        );
        assert!(
            edge(&rows, "<cb@4:2>", "a", 4, 5),
            "{language:?} iife: {rows:?}"
        );
        assert!(
            edge(&rows, "<cb@8:10>", "r", 8, 9),
            "{language:?} return: {rows:?}"
        );
        assert!(
            edge(&rows, "<cb@12:10>", "z", 12, 13),
            "{language:?} assign: {rows:?}"
        );
    }
}

#[test]
fn tsx_jsx_handler_gets_a_synthetic_pass() {
    let src = "const v = <b onClick={e => {\n  use(e);\n}} />;\n";
    let rows = rows(&build(Language::Tsx, src));
    assert!(edge(&rows, "<cb@1:23>", "e", 1, 2), "{rows:?}");
}

#[test]
fn name_inference_wins_over_the_synthetic_identity() {
    let src = "const h = (req) => {\n  exec(req);\n};\nexports.g = function (q) {\n  exec(q);\n};\nclass C {\n  handler = (m) => {\n    exec(m);\n  };\n}\n";
    for language in JS_TS {
        let cpg = build(language, src);
        let owners: BTreeSet<String> = defs(&cpg).into_iter().map(|d| d.0).collect();
        assert!(owners.contains("h") && owners.contains("g") && owners.contains("handler"));
        assert!(
            owners.iter().all(|o| !is_synthetic_owner(o)),
            "{language:?}: {owners:?}"
        );
    }
}

#[test]
fn synthetic_spelling_is_unspellable_by_any_real_owner() {
    let src = "f(function (a) {\n  g(a);\n});\nconst o = { \"<cb@1:3>\": (b) => g(b) };\n";
    let cpg = build(Language::JavaScript, src);
    let owners: BTreeSet<String> = defs(&cpg).into_iter().map(|d| d.0).collect();
    assert!(owners.contains("<cb@1:3>"));
    // The string-keyed callable is named by its quoted key, never by the synthetic spelling.
    assert!(owners.contains("\"<cb@1:3>\""), "{owners:?}");
}

#[test]
fn enclosing_walk_never_reaches_nested_binders() {
    // Direction 1 (E3): the enclosing pass's Defs of `y` must not reach a Use
    // whose binding is a nested formal, catch parameter, let/const for head,
    // named function-expression self name, class-expression name, a
    // block-level function declaration or a nested callable's hoisted `var`.
    let cases = [
        ("function h(y) {\n  list.forEach(function (y) {\n    use(y);\n  });\n}\n", 3),
        ("function h(y) {\n  try { a(); } catch (y) {\n    use(y);\n  }\n}\n", 3),
        ("function h(y) {\n  for (const y of ys) {\n    use(y);\n  }\n}\n", 3),
        ("function h(y) {\n  const f = function y() {\n    use(y);\n  };\n}\n", 3),
        ("function h(y) {\n  const K = class y {\n    m() { use(y); }\n  };\n}\n", 3),
        ("function h(y) {\n  if (c) {\n    function y() {}\n    use(y);\n  }\n}\n", 4),
        ("function h(y) {\n  for (let y = 0; y < 2; y++) {\n    use(y);\n  }\n}\n", 3),
        ("function h(y) {\n  g(function () {\n    if (c) { var y = 1; }\n    use(y);\n  });\n}\n", 4),
        ("function h(y) {\n  g(function () {\n    for (var y in o) {}\n    use(y);\n  });\n}\n", 4),
    ];
    for language in JS_TS {
        for (src, use_line) in cases {
            let rows = rows(&build(language, src));
            assert!(
                !edge(&rows, "h", "y", 1, use_line),
                "{language:?} leaked outer y into the nested binder: {src}\n{rows:?}"
            );
        }
    }
}

#[test]
fn enclosing_walk_keeps_captured_reads_and_var_redeclarations() {
    // Direction 2: captures and same-binding `var` re-declarations are not fenced.
    let capture = "function h(x) {\n  list.forEach(function (item) {\n    use(x);\n  });\n}\n";
    let var_loop = "function h(k) {\n  for (var k in o) {\n    use(k);\n  }\n}\n";
    for language in JS_TS {
        let rows = rows(&build(language, capture));
        assert!(edge(&rows, "h", "x", 1, 3), "{language:?}: {rows:?}");
        assert!(
            rows.iter()
                .all(|r| !(is_synthetic_owner(&r.0) && r.1 == "x")),
            "{language:?}: the callback pass re-emitted a captured read: {rows:?}"
        );
        let rows = rows_for(language, var_loop);
        assert!(edge(&rows, "h", "k", 1, 3), "{language:?}: {rows:?}");
        // A legacy owner's line-covered Def declared INSIDE the nested callable
        // is not fenced by that callable's own scope (the Def is in it).
        let inner =
            "function h() {\n  list.forEach(function () {\n    var y = 1;\n    use(y);\n  });\n}\n";
        let rows = rows_for(language, inner);
        assert!(edge(&rows, "h", "y", 3, 4), "{language:?}: {rows:?}");
    }
}

fn rows_for(language: Language, src: &str) -> BTreeSet<Row> {
    rows(&build(language, src))
}

#[test]
fn captured_read_rows_are_byte_identical_to_the_legacy_owner() {
    // The enclosing named pass still owns the capture row with its existing
    // label; the anonymous callback adds rows only for its own bindings.
    let src = "function h(x) {\n  list.forEach(function (item) {\n    use(x, item);\n  });\n}\n";
    for language in JS_TS {
        let dump = crate::navigation::queries::dfg_edge_dump(&build(language, src));
        let x: Vec<_> = dump
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .filter(|r| r.contains("\"base\":\"x\""))
            .collect();
        assert_eq!(x.len(), 1, "{language:?}: {x:?}");
        assert!(
            x[0].contains("\"cfg_incomplete\""),
            "B8 label must stay: {x:?}"
        );
    }
}

#[test]
fn synthetic_pass_owns_only_its_own_scope() {
    // E1 containment: the declarator on the callable's first line and the
    // nested callable's locals belong to other passes.
    let src = "const r = f(list.map(x => {\n  const z = x;\n  g(function () {\n    var w = 1;\n    use(w);\n  });\n  use(z);\n}));\n";
    for language in JS_TS {
        let cpg = build(language, src);
        let d = defs(&cpg);
        let outer = "<cb@1:22>";
        assert!(
            !d.iter().any(|(o, p, _)| o == outer && p == "r"),
            "{language:?}: {d:?}"
        );
        assert!(
            !d.iter().any(|(o, p, _)| o == outer && p == "w"),
            "{language:?}: {d:?}"
        );
        assert!(
            d.iter().any(|(o, p, _)| o == "<cb@3:5>" && p == "w"),
            "{language:?}: {d:?}"
        );
        assert!(edge(&rows(&cpg), outer, "z", 2, 7));
    }
}

#[test]
fn synthetic_pass_never_counts_non_reference_positions() {
    // E7 must not grow: pair keys, JSX attribute names and callee property
    // names are not Uses in a new pass.
    let src = "f(function (a) {\n  q({ a: 1 }, a);\n  console.a(a);\n});\n";
    for language in JS_TS {
        let cpg = build(language, src);
        for byte in [at(src, "a: 1", 0), at(src, ".a", 0) + 1] {
            assert!(
                !cpg.graph.node_indices().any(|n| matches!(cpg.node(n),
                CpgNode::Variable { function, access: VarAccess::Use, start_byte, .. }
                    if function == "<cb@1:3>" && *start_byte == byte)),
                "{language:?}: non-reference Use at {byte}"
            );
        }
        let rows = rows(&cpg);
        for line in [2, 3] {
            let uses: Vec<_> = rows
                .iter()
                .filter(|r| r.0 == "<cb@1:3>" && r.1 == "a" && r.3 == line)
                .collect();
            assert_eq!(uses.len(), 1, "{language:?}: {rows:?}");
            let byte = uses[0].5;
            assert_eq!(
                &src[byte..byte + 3],
                "a);",
                "{language:?} line {line}: {uses:?}"
            );
        }
    }
    let tsx = "f(function (x) {\n  return <C x={x} />;\n});\n";
    let rows = rows(&build(Language::Tsx, tsx));
    let u: Vec<_> = rows.iter().filter(|r| r.1 == "x" && r.3 == 2).collect();
    assert_eq!(u.len(), 1, "{rows:?}");
    assert_eq!(&tsx[u[0].5..u[0].5 + 2], "x}", "{rows:?}");
}

#[test]
fn synthetic_lexical_def_stays_in_its_block() {
    // E10 must not grow: a let/const Def reaches only references inside its block.
    let src = "f(function () {\n  if (c) {\n    const k = g();\n    use(k);\n  } else {\n    use(k);\n  }\n});\n";
    for language in JS_TS {
        let rows = rows(&build(language, src));
        assert!(edge(&rows, "<cb@1:3>", "k", 3, 4), "{language:?}: {rows:?}");
        assert!(
            !edge(&rows, "<cb@1:3>", "k", 3, 6),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn synthetic_for_head_def_stays_in_its_loop() {
    let src = "f(function () {\n  for (const k of ks) {\n    use(k);\n  }\n  use2(k);\n});\n";
    for language in JS_TS {
        let rows = rows(&build(language, src));
        assert!(edge(&rows, "<cb@1:3>", "k", 2, 3), "{language:?}: {rows:?}");
        assert!(
            !edge(&rows, "<cb@1:3>", "k", 2, 5),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn synthetic_pass_counts_reads_only() {
    // E4 must not grow: a plain `=` target or a declaration name is not a Use.
    let src = "f(function () {\n  let r = 0;\n  r = g();\n  use(r);\n});\n";
    for language in JS_TS {
        let rows = rows(&build(language, src));
        assert!(
            !edge(&rows, "<cb@1:3>", "r", 2, 3),
            "{language:?}: {rows:?}"
        );
        assert!(edge(&rows, "<cb@1:3>", "r", 3, 4), "{language:?}: {rows:?}");
    }
    // An erased type position is not a read either.
    let ts = "f(function (key: string) {\n  const m: { [key: string]: number } = {};\n  use(key, m);\n});\n";
    for language in [Language::TypeScript, Language::Tsx] {
        let rows = rows(&build(language, ts));
        assert!(
            !edge(&rows, "<cb@1:3>", "key", 1, 2),
            "{language:?}: {rows:?}"
        );
        assert!(
            edge(&rows, "<cb@1:3>", "key", 1, 3),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn synthetic_preferred_use_is_the_read_occurrence() {
    // `return h = g() || h;` collects the LHS `h` as an rvalue span too; the
    // row's Use must sit on the read `|| h`, not on the write target.
    let src = "f(function () {\n  let h = 0;\n  return h = g() || h;\n});\n";
    let read = at(src, "|| h", 0) + 3;
    for language in JS_TS {
        let cpg = build(language, src);
        let write = at(src, "h = g", 0);
        assert!(
            !cpg.graph.node_indices().any(|n| matches!(cpg.node(n),
            CpgNode::Variable { function, access: VarAccess::Use, start_byte, .. }
                if function == "<cb@1:3>" && *start_byte == write)),
            "{language:?}: write-only Use at {write}"
        );
        let rows = rows(&cpg);
        let u: Vec<_> = rows
            .iter()
            .filter(|r| r.1 == "h" && r.2 == 2 && r.3 == 3)
            .collect();
        assert_eq!(u.len(), 1, "{language:?}: {rows:?}");
        assert_eq!(u[0].5, read, "{language:?}: {u:?}");
    }
}

#[test]
fn synthetic_augmented_assignment_is_not_an_alias() {
    let src = "f(function (d) {\n  let o = 0;\n  o += d;\n  use(d, o);\n});\n";
    for language in JS_TS {
        let d = defs(&build(language, src));
        let line3 = at(src, "o += d", 0);
        assert!(
            !d.contains(&("<cb@1:3>".into(), "d".into(), line3)),
            "{language:?}: {d:?}"
        );
    }
}

#[test]
fn synthetic_this_member_def_stops_at_a_nested_receiver() {
    // `this.x` inside a nested non-arrow function is another receiver; an
    // arrow keeps the owner's `this`.
    let src = "f(function () {\n  this.x = 1;\n  g(function () {\n    use(this.x);\n  });\n  h(() => use(this.x));\n});\n";
    for language in JS_TS {
        let rows = rows(&build(language, src));
        assert!(
            !edge(&rows, "<cb@1:3>", "this.x", 2, 4),
            "{language:?}: {rows:?}"
        );
        assert!(
            edge(&rows, "<cb@1:3>", "this.x", 2, 6),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn synthetic_alias_twin_only_on_the_alias_line() {
    // E11 must not grow: re-assigning the alias name breaks the alias.
    let src = "f(function (a) {\n  var b = a;\n  b = other();\n  use(b, a);\n});\n";
    for language in JS_TS {
        let d = defs(&build(language, src));
        let line3 = at(src, "b = other", 0);
        assert!(
            !d.contains(&("<cb@1:3>".into(), "a".into(), line3)),
            "{language:?}: {d:?}"
        );
        assert!(
            d.contains(&("<cb@1:3>".into(), "b".into(), line3)),
            "{language:?}: {d:?}"
        );
    }
    // A nested callable's aliases never enter the outer synthetic pass: no
    // flow-insensitive `a.x` twin for the outer `b.x` write, and no alias-only
    // `p.n` Def for the nested destructuring.
    let nested = "f(function () {\n  var b = {};\n  g(function (a, p) {\n    b = a;\n    const { n } = p;\n    use(n);\n  });\n  b.x = 2;\n  use(b.x);\n});\n";
    for language in JS_TS {
        let d = defs(&build(language, nested));
        let outer: Vec<_> = d.iter().filter(|(o, _, _)| o == "<cb@1:3>").collect();
        assert!(
            !outer.iter().any(|(_, p, _)| p == "a.x" || p == "p.n"),
            "{language:?}: {outer:?}"
        );
        assert!(
            outer.iter().any(|(_, p, _)| p == "b.x"),
            "{language:?}: {outer:?}"
        );
    }
}

#[test]
fn anonymous_callables_are_never_interprocedural_endpoints() {
    // E5 isolation: no Step-5b argument or return edge enters or leaves a
    // synthetic owner (it is never a callee and never a resolved caller).
    let src = "function f(p) {\n  return p;\n}\nfunction g(cb) {\n  return cb(1);\n}\ng(function (v) {\n  return f(v);\n});\nlist.map(x => f(x));\n";
    for language in JS_TS {
        let cpg = build(language, src);
        for e in cpg.graph.edge_indices() {
            let (a, b) = cpg.graph.edge_endpoints(e).unwrap();
            let owner = |n| match cpg.node(n) {
                CpgNode::Variable { function, .. } => Some(function.clone()),
                _ => None,
            };
            if let (Some(x), Some(y)) = (owner(a), owner(b)) {
                if is_synthetic_owner(&x) || is_synthetic_owner(&y) {
                    assert_eq!(x, y, "{language:?}: cross-owner edge {:?}", cpg.graph[e]);
                }
            }
        }
        assert!(cpg
            .call_graph
            .functions
            .keys()
            .all(|name| !is_synthetic_owner(name)));
    }
}

#[test]
fn non_js_anonymous_callables_get_no_synthetic_owner() {
    for (language, src) in [
        (Language::Python, "f(lambda a: g(a))\n"),
        (
            Language::Go,
            "package p\nfunc h() { f(func(a int) { g(a) }) }\n",
        ),
        (Language::Rust, "fn h() { f(|a| g(a)); }\n"),
    ] {
        let parsed = ParsedFile::parse(file_name(language), src, language).unwrap();
        for node in parsed.all_functions() {
            assert!(parsed.js_ts_synthetic_callable_name(&node).is_none());
        }
        let cpg = build(language, src);
        assert!(
            defs(&cpg).iter().all(|d| !is_synthetic_owner(&d.0)),
            "{language:?}"
        );
    }
}

mod navigation {
    use crate::navigation::{queries, NavigationIndex, NavigationSession};
    use crate::reasoning::seeds::{self, SeedRole, SeedSpec};
    use std::sync::Arc;

    fn session(src: &str) -> (tempfile::TempDir, NavigationSession) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("cb.js"), src).unwrap();
        let repo = Arc::new(crate::repo_loader::load_repo(dir.path()).unwrap());
        let index = Arc::new(NavigationIndex::build(&repo));
        (dir, NavigationSession { repo, index })
    }

    const SRC: &str = "function exec(c) {\n  return c;\n}\nhttp.createServer(function(req, res) {\n  exec(req);\n});\n";

    #[test]
    fn synthetic_identity_never_resolves_as_a_symbol() {
        let (_dir, s) = session(SRC);
        for query in [queries::callers, queries::callees] {
            let err = query(&s, Some("<cb@4:19>"), None, None, 1).unwrap_err();
            assert!(format!("{err:?}").contains("SymbolNotFound"), "{err:?}");
        }
        let err = seeds::resolve(
            &s,
            &[SeedSpec::Symbol {
                name: "<cb@4:19>".into(),
                file: None,
            }],
            SeedRole::Source,
        )
        .unwrap_err();
        assert!(format!("{err:?}").contains("SymbolNotFound"), "{err:?}");
        // The real callee gains no caller from the anonymous registration site.
        let callers = queries::callers(&s, Some("exec"), None, None, 1).unwrap();
        assert!(serde_json::to_string(&callers)
            .unwrap()
            .find("<cb@")
            .is_none());
    }

    #[test]
    fn navigation_hides_synthetic_variables_but_taint_seeds_see_them() {
        let (_dir, s) = session(SRC);
        let evidence = queries::nodes_at(&s, "cb.js", 5);
        assert!(!serde_json::to_string(&evidence).unwrap().contains("<cb@"));
        let ego = queries::ego_graph(&s, None, None, Some("cb.js:4"), 1, &[]);
        assert!(ego
            .map(|e| !serde_json::to_string(&e).unwrap().contains("<cb@"))
            .unwrap_or(true));
        let seeds = seeds::resolve(
            &s,
            &[SeedSpec::Loc {
                file: "cb.js".into(),
                line: 4,
            }],
            SeedRole::Source,
        )
        .unwrap();
        assert!(format!("{seeds:?}").contains("<cb@4:19>"), "{seeds:?}");
    }
}

#[test]
fn synthetic_rows_and_labels_equal_the_named_control() {
    // D2 parity: the synthetic pass is the legacy named pass minus the E4
    // rows D12 refuses (a declaration name or a plain `=` target taken as a
    // Use). Every common row keeps its exact label, including E6 prior-write
    // Exact semantics and loop-carried rows.
    let body = "(x, n) {\n  if (n) {\n    x = n;\n  }\n  let i = 0;\n  while (i < n) {\n    i = i + x;\n  }\n  sink(x, i);\n}";
    let anon = format!("f(function {body});\n");
    let named = format!("f(function g{body});\n");
    // (def line, use line, path) of the four E4 rows the named pass keeps.
    let e4 = [(1, 3, "x"), (3, 1, "n"), (3, 1, "x"), (7, 5, "i")];
    for language in JS_TS {
        let dump = |src: &str| -> BTreeSet<String> {
            crate::navigation::queries::dfg_edge_dump(&build(language, src))
                .iter()
                .map(|r| serde_json::to_string(r).unwrap())
                .collect()
        };
        let (a, n) = (dump(&anon), dump(&named));
        assert!(!a.is_empty() && a.is_subset(&n), "{language:?}: {a:?}");
        let extra: BTreeSet<(usize, usize, String)> = n
            .difference(&a)
            .map(|row| {
                let v: serde_json::Value = serde_json::from_str(row).unwrap();
                (
                    v["from"]["line"].as_u64().unwrap() as usize,
                    v["to"]["line"].as_u64().unwrap() as usize,
                    v["from"]["path"]["base"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        let expected: BTreeSet<_> = e4.iter().map(|(d, u, p)| (*d, *u, p.to_string())).collect();
        assert_eq!(extra, expected, "{language:?}");
    }
}

// R1 regressions use real endpoint bytes as well as the wire line/path keys.
fn r1_label(
    cpg: &CodePropertyGraph,
    owner: &str,
    path_name: &str,
    d: usize,
    u: usize,
) -> super::FlowConfidence {
    let found: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter_map(|e| {
            let CpgEdge::DataFlow(label) = cpg.graph[e] else {
                return None;
            };
            let (a, b) = cpg.graph.edge_endpoints(e)?;
            match (cpg.node(a), cpg.node(b)) {
                (
                    CpgNode::Variable {
                        function,
                        path,
                        line,
                        access: VarAccess::Def,
                        ..
                    },
                    CpgNode::Variable {
                        line: use_line,
                        access: VarAccess::Use,
                        ..
                    },
                ) if function == owner
                    && path.to_string() == path_name
                    && *line == d
                    && *use_line == u =>
                {
                    Some(label)
                }
                _ => None,
            }
        })
        .collect();
    assert_eq!(found.len(), 1, "{owner}.{path_name}@{d}->{u}: {found:?}");
    found[0]
}

#[test]
fn r1_nested_writes_are_kill_only_and_named_parity_is_preserved() {
    let body = "(x) => {\n  let z = x;\n  g(() => {\n    z = 2;\n  });\n  sink(z);\n}";
    for language in JS_TS {
        let anonymous = format!("reg({body});\n");
        let named = format!("const f = {body};\n");
        let a = build(language, &anonymous);
        let n = build(language, &named);
        let label = r1_label(&a, "<cb@1:5>", "z", 2, 6);
        assert_eq!(label, r1_label(&n, "f", "z", 2, 6));
        assert!(matches!(
            label,
            super::FlowConfidence::NameOnly(super::FlowDoubt::Killed { .. })
        ));
        assert!(!defs(&a).contains(&("<cb@1:5>".into(), "z".into(), at(&anonymous, "z = 2", 0))));
        let rebound = anonymous.replace("() =>", "(z) =>");
        assert_eq!(
            r1_label(&build(language, &rebound), "<cb@1:5>", "z", 2, 6),
            super::FlowConfidence::Exact
        );
    }
}

#[test]
fn r1_assignment_defs_stay_with_catch_block_and_nested_formals() {
    let src = "reg((x) => {\n  try { t(); } catch (e) {\n    e = x;\n    sink(e);\n  }\n  sink(e);\n  { let q;\n    q = x;\n    sink(q);\n  }\n  sink(q);\n});\n";
    let nested =
        "function h(y) {\n  a(function(y) {\n    y = 1;\n    use(y);\n  });\n  use(y);\n}\n";
    for language in JS_TS {
        let r = rows(&build(language, src));
        assert!(edge(&r, "<cb@1:5>", "e", 3, 4), "{language:?}: {r:?}");
        assert!(!edge(&r, "<cb@1:5>", "e", 3, 6), "{language:?}: {r:?}");
        assert!(edge(&r, "<cb@1:5>", "q", 8, 9), "{language:?}: {r:?}");
        assert!(!edge(&r, "<cb@1:5>", "q", 8, 11), "{language:?}: {r:?}");
        let r = rows(&build(language, nested));
        assert!(edge(&r, "h", "y", 3, 4), "{language:?}: {r:?}");
        assert!(!edge(&r, "h", "y", 3, 6), "{language:?}: {r:?}");
        assert!(!edge(&r, "h", "y", 3, 1), "{language:?}: {r:?}");
    }
}

#[test]
fn r1_defaults_and_computed_keys_keep_outer_captures() {
    for language in JS_TS {
        for declaration in ["var y", "let y", "var q"] {
            let src = format!("function h(y) {{\n  g(function(z = y) {{\n    {declaration} = 1;\n    use(z);\n  }});\n}}\n");
            let cpg = build(language, &src);
            let r = rows(&cpg);
            assert!(
                r.contains(&(
                    "h".into(),
                    "y".into(),
                    1,
                    2,
                    at(&src, "y", 0),
                    at(&src, "  g", 0)
                )),
                "{language:?}: {r:?}"
            );
            assert_eq!(
                r1_label(&cpg, "h", "y", 1, 2),
                super::FlowConfidence::NameOnly(super::FlowDoubt::CfgIncomplete)
            );
            assert!(!edge(&r, "<cb@2:5>", "y", 3, 2), "{language:?}: {r:?}");
        }
        let key = "function h(y) {\n  const o = {\n    [\n      y\n    ](y) {\n      use(y);\n    }\n  };\n}\n";
        let cpg = build(language, key);
        let r = rows(&cpg);
        assert!(
            r.contains(&(
                "h".into(),
                "y".into(),
                1,
                4,
                at(key, "y", 0),
                at(key, "      y", 0)
            )),
            "{language:?}: {r:?}"
        );
        assert!(!edge(&r, "h", "y", 1, 6), "{language:?}: {r:?}");
        assert_eq!(
            r1_label(&cpg, "h", "y", 1, 4),
            super::FlowConfidence::NameOnly(super::FlowDoubt::CfgIncomplete)
        );
    }
    for language in [Language::TypeScript, Language::Tsx] {
        for annotation in ["y is string", "typeof y"] {
            let src =
                format!("const h = (\n  y: unknown,\n): {annotation} => {{\n  use(y);\n}};\n");
            let r = rows(&build(language, &src));
            assert!(
                r.contains(&(
                    "h".into(),
                    "y".into(),
                    1,
                    3,
                    at(&src, "y", 0),
                    at(&src, "): ", 0)
                )),
                "{language:?}: {r:?}"
            );
        }
    }
}

#[test]
fn r1_member_writes_are_not_value_reads() {
    for language in JS_TS {
        for base in ["this", "obj"] {
            let src = format!(
                "f(function() {{\n  {base}.x = 0;\n  {base}.x = 1;\n  use({base}.x);\n}});\n"
            );
            let cpg = build(language, &src);
            let r = rows(&cpg);
            assert!(
                !edge(&r, "<cb@1:3>", &format!("{base}.x"), 2, 3),
                "{language:?}: {r:?}"
            );
            assert!(
                r.contains(&(
                    "<cb@1:3>".into(),
                    format!("{base}.x"),
                    3,
                    4,
                    at(&src, &format!("{base}.x = 1"), 0),
                    at(&src, &format!("{base}.x"), 2)
                )),
                "{language:?}: {r:?}"
            );
            assert_eq!(
                r1_label(&cpg, "<cb@1:3>", &format!("{base}.x"), 3, 4),
                super::FlowConfidence::Exact
            );
            for operation in ["+= 1", "++"] {
                let read = src.replace("= 1", operation);
                assert!(edge(
                    &rows(&build(language, &read)),
                    "<cb@1:3>",
                    &format!("{base}.x"),
                    2,
                    3
                ));
            }
        }
    }
}

#[test]
fn r1_runtime_enums_fence_only_matching_bindings() {
    for language in [Language::TypeScript, Language::Tsx] {
        for prefix in ["enum", "const enum"] {
            let src = format!(
                "f(function(p) {{\n  {{\n    {prefix} p {{ A }}\n    use(p);\n  }}\n}});\n"
            );
            let r = rows(&build(language, &src));
            assert!(!edge(&r, "<cb@1:3>", "p", 1, 3), "{language:?}: {r:?}");
            assert!(!edge(&r, "<cb@1:3>", "p", 1, 4), "{language:?}: {r:?}");
            let unrelated = src.replace(&format!("{prefix} p"), &format!("{prefix} q"));
            assert!(edge(
                &rows(&build(language, &unrelated)),
                "<cb@1:3>",
                "p",
                1,
                4
            ));
        }
    }
}

#[test]
fn r1_same_line_defs_and_alias_twins_keep_lvalue_bytes() {
    for language in JS_TS {
        let src =
            "f(function(a, c) {\n  { const b = a; } { const b = c;\n    use(b, c);\n  }\n});\n";
        let r = rows(&build(language, src));
        let b: Vec<_> = r
            .iter()
            .filter(|r| r.0 == "<cb@1:3>" && r.1 == "b" && r.2 == 2 && r.3 == 3)
            .collect();
        assert_eq!(b.len(), 1, "{language:?}: {r:?}");
        assert_eq!(b[0].4, at(src, "b = c", 0), "{language:?}: {r:?}");
        for statement in [
            "var b = other(); b = a;",
            "var b = a; b = other();",
            "var b = c; b = a;",
            "var b = a; b = a;",
        ] {
            let src = format!("f(function(a, c) {{\n  {statement}\n  use(a, c, b);\n}});\n");
            let r = rows(&build(language, &src));
            let actual: BTreeSet<_> = r
                .iter()
                .filter(|r| r.0 == "<cb@1:3>" && r.1 == "a" && r.2 == 2)
                .map(|r| r.4)
                .collect();
            let expected: BTreeSet<_> = src.match_indices("b = a").map(|(b, _)| b).collect();
            assert_eq!(actual, expected, "{language:?}: {r:?}");
        }
        let blocks = "f(function() {\n  class C { static { var p = 1; } static { var p = 2;\n    use(p);\n  } }\n});\n";
        let r = rows(&build(language, blocks));
        let actual: BTreeSet<_> = r
            .iter()
            .filter(|r| r.0 == "<cb@1:3>" && r.1 == "p" && r.3 == 3)
            .map(|r| r.4)
            .collect();
        assert_eq!(
            actual,
            BTreeSet::from([at(blocks, "p = 2", 0)]),
            "{language:?}: {r:?}"
        );
    }
}

#[test]
fn r1_decoded_escaped_binders_preserve_unrelated_captures() {
    for language in JS_TS {
        for (escaped, matching) in [
            ("\\u0078", false),
            ("\\u{78}", false),
            ("\\u0079", true),
            ("\\u{79}", true),
        ] {
            let src =
                format!("function h(y) {{\n  g(function({escaped}) {{\n    use(y);\n  }});\n}}\n");
            let cpg = build(language, &src);
            let r = rows(&cpg);
            assert_eq!(edge(&r, "h", "y", 1, 3), !matching, "{language:?}: {r:?}");
            if !matching {
                assert!(r.contains(&(
                    "h".into(),
                    "y".into(),
                    1,
                    3,
                    at(&src, "y", 0),
                    at(&src, "    use", 0)
                )));
                assert_eq!(
                    r1_label(&cpg, "h", "y", 1, 3),
                    super::FlowConfidence::NameOnly(super::FlowDoubt::CfgIncomplete)
                );
            }
        }
    }
}

#[test]
fn r2_body_function_hoisting_shares_simple_parameters() {
    for language in JS_TS {
        for declaration in ["function x() {}", "function q() {}", "var x;", "{ var x; }"] {
            let src = format!("function h(x) {{\n  {declaration}\n  use(x);\n}}\n");
            let cpg = build(language, &src);
            assert!(
                rows(&cpg).contains(&(
                    "h".into(),
                    "x".into(),
                    1,
                    3,
                    at(&src, "x", 0),
                    at(&src, "x);", 0)
                )),
                "{language:?}: {:?}",
                rows(&cpg)
            );
            if declaration.starts_with("function") {
                assert_eq!(
                    r1_label(&cpg, "h", "x", 1, 3),
                    super::FlowConfidence::NameOnly(super::FlowDoubt::CfgIncomplete)
                );
            }
        }
        let src = "function h(x) {\n  { function x() {}\n    use(x);\n  }\n  use(x);\n}\n";
        let r = rows(&build(language, src));
        assert!(!edge(&r, "h", "x", 1, 3), "{language:?}: {r:?}");
        assert!(edge(&r, "h", "x", 1, 5), "{language:?}: {r:?}");
    }
}

#[test]
fn r2_parameter_expressions_separate_body_vars_and_functions() {
    for language in JS_TS {
        for declaration in ["var y = 2;", "function y() {}"] {
            for default in ["y", "() => y"] {
                let src = format!("f(function(\n y = 1,\n z = {default}\n) {{\n {declaration}\n use(y, z);\n}});\n");
                let cpg = build(language, &src);
                let r = rows(&cpg);
                assert!(!edge(&r, "<cb@1:3>", "y", 5, 3), "{language:?}: {r:?}");
                assert!(!edge(&r, "<cb@1:3>", "y", 1, 6), "{language:?}: {r:?}");
                if declaration.starts_with("var") {
                    assert!(
                        r.contains(&(
                            "<cb@1:3>".into(),
                            "y".into(),
                            5,
                            6,
                            at(&src, "y = 2", 0),
                            at(&src, "y, z", 0)
                        )),
                        "{language:?}: {r:?}"
                    );
                    assert_eq!(
                        r1_label(&cpg, "<cb@1:3>", "y", 5, 6),
                        super::FlowConfidence::Exact
                    );
                }
                // This whole non-inert signature deliberately has no default
                // parameter Defs under PR-A; do not invent registration here.
            }
        }
        let direct = "f(function(y = 1, z = y) {\n var y = 2;\n use(y);\n});\n";
        let r = rows(&build(language, direct));
        assert!(!edge(&r, "<cb@1:3>", "y", 2, 1), "{language:?}: {r:?}");
        assert!(edge(&r, "<cb@1:3>", "y", 2, 3), "{language:?}: {r:?}");
        let computed = "f(function(y, {[y]: z}) {\n var y = 2;\n use(y, z);\n});\n";
        let r = rows(&build(language, computed));
        assert!(!edge(&r, "<cb@1:3>", "y", 2, 1), "{language:?}: {r:?}");
        assert!(!edge(&r, "<cb@1:3>", "y", 1, 3), "{language:?}: {r:?}");
        assert!(edge(&r, "<cb@1:3>", "y", 2, 3), "{language:?}: {r:?}");
        if language != Language::JavaScript {
            let typed = "f(function(y: number, z: number) {\n var y = 2;\n use(y, z);\n});\n";
            let r = rows(&build(language, typed));
            assert!(edge(&r, "<cb@1:3>", "y", 1, 3), "{language:?}: {r:?}");
        }
        let simple = direct.replace("y = 1, z = y", "y, z");
        let r = rows(&build(language, &simple));
        assert!(edge(&r, "<cb@1:3>", "y", 1, 3), "{language:?}: {r:?}");
        assert!(edge(&r, "<cb@1:3>", "y", 2, 3), "{language:?}: {r:?}");
    }
}

#[test]
fn r2_use_endpoints_keep_binding_valid_occurrence_bytes() {
    for language in JS_TS {
        for body in [
            "{let b = 2; use(b);} use(b);",
            "use(b); {let b = 2; use(b);}",
        ] {
            let src = format!("f(function() {{\n let b = 1;\n {body}\n}});\n");
            let cpg = build(language, &src);
            let outer = if body.starts_with('{') {
                at(&src, "b);", 1)
            } else {
                at(&src, "b);", 0)
            };
            let r: Vec<_> = rows(&cpg)
                .into_iter()
                .filter(|r| r.1 == "b" && r.2 == 2 && r.3 == 3)
                .collect();
            assert_eq!(
                r,
                vec![(
                    "<cb@1:3>".into(),
                    "b".into(),
                    2,
                    3,
                    at(&src, "b = 1", 0),
                    outer
                )],
                "{language:?}"
            );
            assert_eq!(
                r1_label(&cpg, "<cb@1:3>", "b", 2, 3),
                super::FlowConfidence::Exact
            );
        }
    }
}

#[test]
fn r2_synthetic_with_body_is_refused_but_expression_and_outside_reads_remain() {
    for property in ["p", "q"] {
        let src = format!("f(function(p) {{\n with ({{{property}: 2, q: p}}) {{\n  use(p);\n  g(() => use(p));\n }}\n use(p);\n}});\n");
        let cpg = build(Language::JavaScript, &src);
        let r = rows(&cpg);
        assert!(!edge(&r, "<cb@1:3>", "p", 1, 3), "{r:?}");
        assert!(!edge(&r, "<cb@1:3>", "p", 1, 4), "{r:?}");
        assert!(edge(&r, "<cb@1:3>", "p", 1, 2), "{r:?}");
        assert!(edge(&r, "<cb@1:3>", "p", 1, 6), "{r:?}");
        for byte in [at(&src, "p);", 0), at(&src, "p));", 0)] {
            assert!(!cpg.graph.node_indices().any(|n| matches!(cpg.node(n),
                CpgNode::Variable { function, access: VarAccess::Use, start_byte, .. }
                    if function == "<cb@1:3>" && *start_byte == byte)));
        }
    }
    let block = "f(function(p) {\n {\n  use(p);\n }\n});\n";
    assert!(edge(
        &rows(&build(Language::JavaScript, block)),
        "<cb@1:3>",
        "p",
        1,
        3
    ));
    // A callback created inside with must not certify its object-backed capture.
    let closure = "with ({p: 2}) {\n f(function() {\n  var z = p;\n  use(z, p);\n });\n}\n";
    assert!(!edge(
        &rows(&build(Language::JavaScript, closure)),
        "<cb@2:4>",
        "p",
        3,
        4
    ));
    let member = "f(function(obj) {\n obj.x = 0;\n with ({obj: {x: 2}, q: obj.x}) {\n  use(obj.x);\n }\n use(obj.x);\n});\n";
    let r = rows(&build(Language::JavaScript, member));
    assert!(!edge(&r, "<cb@1:3>", "obj.x", 2, 4), "{r:?}");
    assert!(edge(&r, "<cb@1:3>", "obj.x", 2, 3), "{r:?}");
    assert!(edge(&r, "<cb@1:3>", "obj.x", 2, 6), "{r:?}");
    let named = "function h(p) {\n with ({p: 2}) {\n  use(p);\n }\n}\n";
    assert!(edge(
        &rows(&build(Language::JavaScript, named)),
        "h",
        "p",
        1,
        3
    ));
}

fn r2_wrapped_write_role(wrapper: &str, member: bool) {
    let languages: &[Language] = match wrapper {
        "parens" => &JS_TS,
        "assertion" => &[Language::TypeScript],
        _ => &[Language::TypeScript, Language::Tsx],
    };
    for &language in languages {
        let path = if member { "this.x" } else { "x" };
        let target = match wrapper {
            "parens" => format!("({path})"),
            "nonnull" => format!("{path}!"),
            "as" => format!("({path} as any)"),
            "satisfies" => format!("({path} satisfies any)"),
            "assertion" => format!("(<any>{path})"),
            _ => unreachable!(),
        };
        let def = if member { "this.x = 0" } else { "let x = 0" };
        let src = format!("f(function() {{\n {def};\n {target} = 1;\n use({path});\n}});\n");
        let r = rows(&build(language, &src));
        assert!(
            !edge(&r, "<cb@1:3>", path, 2, 3),
            "{wrapper} {language:?}: {r:?}"
        );
        assert!(
            edge(&r, "<cb@1:3>", path, 2, 4),
            "{wrapper} {language:?}: {r:?}"
        );
        let compound = src.replace(" = 1", " += 1");
        let r = rows(&build(language, &compound));
        assert!(
            edge(&r, "<cb@1:3>", path, 2, 3),
            "{wrapper} {language:?}: {r:?}"
        );
    }
}

macro_rules! r2_wrapper_tests {
    ($ident:ident, $member:ident, $kind:literal) => {
        #[test]
        fn $ident() {
            r2_wrapped_write_role($kind, false);
        }
        #[test]
        fn $member() {
            r2_wrapped_write_role($kind, true);
        }
    };
}
r2_wrapper_tests!(r2_parens_identifier, r2_parens_member, "parens");
r2_wrapper_tests!(r2_nonnull_identifier, r2_nonnull_member, "nonnull");
r2_wrapper_tests!(r2_as_identifier, r2_as_member, "as");
r2_wrapper_tests!(r2_satisfies_identifier, r2_satisfies_member, "satisfies");
r2_wrapper_tests!(r2_assertion_identifier, r2_assertion_member, "assertion");

#[test]
fn r2_with_body_defs_cannot_reach_outside_reads() {
    for property in ["p", "q"] {
        let src = format!(
            "f(function(p) {{\n with ({{{property}: 2}}) {{\n  p = 3;\n }}\n use(p);\n}});\n"
        );
        let cpg = build(Language::JavaScript, &src);
        assert!(!defs(&cpg).contains(&("<cb@1:3>".into(), "p".into(), at(&src, "p = 3", 0))));
        let r = rows(&cpg);
        assert!(!edge(&r, "<cb@1:3>", "p", 3, 5), "{r:?}");
        assert!(
            r.contains(&(
                "<cb@1:3>".into(),
                "p".into(),
                1,
                5,
                at(&src, "p)", 0),
                at(&src, "p);", 0)
            )),
            "{r:?}"
        );
    }
    let member = "f(function(obj) {\n obj.x = 0;\n with ({obj: {x: 2}}) {\n  obj.x = 3;\n }\n use(obj.x);\n});\n";
    let cpg = build(Language::JavaScript, member);
    let r = rows(&cpg);
    assert!(!defs(&cpg).contains(&(
        "<cb@1:3>".into(),
        "obj.x".into(),
        at(member, "obj.x = 3", 0)
    )));
    assert!(!edge(&r, "<cb@1:3>", "obj.x", 4, 6), "{r:?}");
    assert!(edge(&r, "<cb@1:3>", "obj.x", 2, 6), "{r:?}");
    let block = "f(function(p) {\n {\n  p = 3;\n }\n use(p);\n});\n";
    let cpg = build(Language::JavaScript, block);
    assert!(edge(&rows(&cpg), "<cb@1:3>", "p", 3, 5));
    assert_eq!(
        r1_label(&cpg, "<cb@1:3>", "p", 3, 5),
        super::FlowConfidence::Exact
    );
    let named = member
        .replace("f(function(obj)", "function h(obj)")
        .replace("});", "}");
    assert!(edge(
        &rows(&build(Language::JavaScript, &named)),
        "h",
        "obj.x",
        4,
        6
    ));
}

#[test]
fn r2b_rec2_uninitialised_var_keeps_entry_copy() {
    for language in JS_TS {
        for body in [
            " var f;\n return f;",
            " return f;\n var f;",
            " { var f; }\n return f;",
        ] {
            let src = format!("function rec2(f, o, fn=null) {{\n{body}\n}}\n");
            let cpg = build(language, &src);
            assert!(
                edge(
                    &rows(&cpg),
                    "rec2",
                    "f",
                    1,
                    if body.starts_with(" return") { 2 } else { 3 }
                ),
                "{language:?}: {:?}",
                rows(&cpg)
            );
        }
    }
}

#[test]
fn r4_parameter_early_errors_refuse_dfg_passes() {
    let mut failures = Vec::new();
    for language in JS_TS {
        for (head, tail) in [("function h", "}"), ("register(function", "});")] {
            for (signature, body) in [
                ("f", "let f=2; use(f);"),
                ("f,d=0", "const f=2; use(f);"),
                ("f,d=0", "class f {} use(f);"),
                ("f,d=0", "'use strict'; use(f);"),
                ("f,...r", "'use strict'; use(f);"),
                ("f,f=2", "use(f);"),
            ] {
                let src = format!("{head}({signature}) {{{body}{tail}\n");
                let r = rows(&build(language, &src));
                if !r.is_empty() {
                    failures.push(format!("{language:?}: {src}: {r:?}"));
                }
            }
            for body in ["var f; use(f);", "{let f=2; use(f);} use(f);"] {
                let src = format!("{head}(f,d=0) {{{body}{tail}\n");
                let parsed = ParsedFile::parse(file_name(language), &src, language).unwrap();
                assert!(
                    parsed
                        .all_functions()
                        .iter()
                        .all(|f| parsed.dfg_owner_name(f).is_some()),
                    "{language:?}: {src}"
                );
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r4_contextual_strictness_and_decoded_early_errors_are_refused() {
    let mut failures = Vec::new();
    for language in JS_TS {
        for src in [
            "'use strict';\nfunction h(arguments,d=arguments){use(arguments);}",
            "export {};\nregister(function(arguments,d=arguments){use(arguments);});",
            "class C { h(arguments){use(arguments);} }",
            "function h(f,d=0){let \\u0066=2; use(f);}",
            "register(function(f,d=0){class \\u0066 {} use(f);});",
            "function h(f,d=0){try{throw {f:2};}catch({f}){var f=9;}use(f);}",
        ] {
            let r = rows(&build(language, src));
            if !r.is_empty() {
                failures.push(format!("{language:?}: {src}: {r:?}"));
            }
        }
        for src in [
            "'use strict';\nfunction h(f,d=0){var f;use(f);}",
            "export {};\nregister(function(f,d=0){var f;use(f);});",
            "function h(f,d=0){try{throw 2;}catch(f){var f=9;}use(f);}",
        ] {
            let parsed = ParsedFile::parse(file_name(language), src, language).unwrap();
            assert!(
                parsed
                    .all_functions()
                    .iter()
                    .all(|f| parsed.dfg_owner_name(f).is_some()),
                "{language:?}: {src}"
            );
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r4_bound_names_and_comment_insensitive_early_errors() {
    let mut failures = Vec::new();
    for language in JS_TS {
        for src in [
            "function h(f,d=0){/*comment*/'use strict';use(f);}",
            "/*comment*/'use strict';function h(arguments){use(arguments);}",
            "function h(f,d=0){let [f]=[2];use(f);}",
            "register(function(f,d=0){const {f}={f:2};use(f);});",
            "function h({f},d=0){let f=2;use(f);}",
            "function h(f,{f}){use(f);}",
            "'use strict';function h(f,f){use(f);}",
            "const o={h(f,f){use(f);}};",
            "register((f,f)=>{use(f);});",
        ] {
            let r = rows(&build(language, src));
            if !r.is_empty() {
                failures.push(format!("{language:?}: {src}: {r:?}"));
            }
        }
        for src in [
            "function h(f/*comment*/){'use strict';use(f);}",
            "function h(f,{f:g}){use(f);}",
            "function h(f,d=f){use(f);}",
        ] {
            if rows(&build(language, src)).is_empty() {
                failures.push(format!("valid callable refused: {language:?}: {src}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r7_grammar_kind_matrix_has_positive_early_error_proofs() {
    let cells: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/js_param_r7_kind_matrix.json")).unwrap();
    let mut failures = Vec::new();
    for cell in cells.as_array().unwrap() {
        let language = match cell["ext"].as_str().unwrap() {
            "js" => Language::JavaScript,
            "ts" => Language::TypeScript,
            "tsx" => Language::Tsx,
            _ => unreachable!(),
        };
        let source = cell["source"].as_str().unwrap();
        let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
        let owner = parsed.all_functions().into_iter().next().unwrap();
        let actual = parsed.dfg_owner_name(&owner).is_some();
        let expected = cell["admitted"].as_bool().unwrap();
        if actual != expected {
            failures.push(format!(
                "{}: admitted={actual}, expected={expected}",
                cell["id"]
            ));
        }
        let cpg = build(language, source);
        if !expected && !rows(&cpg).is_empty() {
            failures.push(format!("{}: invalid callable has rows", cell["id"]));
        }
        // A supported sibling formal and a body-local def must survive every
        // valid unfamiliar formal kind, including erased TS annotations.
        if expected && rows(&cpg).is_empty() {
            failures.push(format!("{}: valid callable lost all rows", cell["id"]));
        }
        if expected {
            let name = parsed.dfg_owner_name(&owner).unwrap();
            for binding in ["$", "a"] {
                let actual = r5_full_binding_rows(&cpg, &name, binding);
                let mut golden = cell["golden"][binding].as_array().unwrap().clone();
                golden.sort_by_key(ToString::to_string);
                if actual != golden {
                    failures.push(format!(
                        "{}/{binding}: actual={actual:?}, golden={golden:?}",
                        cell["id"]
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r7_undefined_bound_names_reject_only_real_early_errors() {
    let mut failures = Vec::new();
    for language in JS_TS {
        for source in [
            "function h(f,undefined){let undefined=0;use(f);}",
            "function h(f,undefined){class undefined{};use(f);}",
            "function h(f,undefined,undefined){'use strict';use(f);}",
            "register((f,undefined,undefined)=>{use(f);});",
            "function h(f,undefined,...undefined){use(f);}",
        ] {
            if !rows(&build(language, source)).is_empty() {
                failures.push(format!("{language:?}: invalid: {source}"));
            }
        }
        for source in [
            "function h(f,undefined,undefined){use(f);}",
            "function h(f,undefined){var undefined;use(f);}",
            "function h(f,undefined){let other=0;use(f);}",
            "function h(f,undefined){'use strict';use(f);}",
        ] {
            if rows(&build(language, source)).is_empty() {
                failures.push(format!("{language:?}: valid: {source}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn r7_undefined_seam_and_erased_simple_arguments_are_classified() {
    let mut failures = Vec::new();
    for language in JS_TS {
        for (source, name, seam, args, eval) in [
            (
                "register(function(undefined,d=0){var undefined;use(d);});",
                "undefined",
                true,
                false,
                false,
            ),
            (
                "register(function(undefined,d=0){var other;use(d);});",
                "other",
                false,
                false,
                false,
            ),
            (
                "register(function(f,undefined){use(arguments);use(f);});",
                "f",
                false,
                true,
                false,
            ),
            (
                "register(function(f,undefined){'use strict';use(arguments);use(f);});",
                "f",
                false,
                false,
                false,
            ),
            (
                "register(function(f,undefined=0){use(arguments);use(f);});",
                "f",
                false,
                false,
                false,
            ),
            (
                "register(function(f,undefined){eval('f');use(f);});",
                "f",
                false,
                false,
                true,
            ),
            (
                "register(function(f,undefined){obj.eval('f');use(f);});",
                "f",
                false,
                false,
                false,
            ),
        ] {
            let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
            let owner = parsed.all_functions().into_iter().next().unwrap();
            let actual = (
                parsed.js_ts_seam_binding(&owner, name),
                parsed.js_ts_mapped_arguments_possible(&owner),
                parsed.js_ts_direct_eval_anywhere(&owner),
            );
            if actual != (seam, args, eval) {
                failures.push(format!("{language:?}: {source}: {actual:?}"));
            }
        }
        if language != Language::JavaScript {
            for source in [
                "register(function(f, p?: number){use(arguments);use(f);});",
                "register(function(this: unknown, f: number){use(arguments);use(f);});",
            ] {
                let parsed = ParsedFile::parse(file_name(language), source, language).unwrap();
                let owner = parsed.all_functions().into_iter().next().unwrap();
                if !parsed.js_ts_mapped_arguments_possible(&owner) {
                    failures.push(format!("{language:?}: erased simple arguments: {source}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
