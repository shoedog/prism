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
        let rows = rows(&build(language, src));
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
