//! #599: a JS-family owner whose statement universe is one node is labelled
//! by reaching definitions instead of the NoCfgEdges fallback. Label-only: the
//! DataFlow edge set must not move.
use super::*;
use crate::ast::ParsedFile;
use crate::data_flow::test_support::label_snapshot;
use crate::languages::Language;
use std::collections::BTreeMap;

fn labels(language: Language, ext: &str, source: &str) -> (Vec<String>, usize) {
    let name = format!("a.{ext}");
    let file = ParsedFile::parse(&name, source, language).unwrap();
    assert_eq!(file.parse_error_count, 0, "{source}");
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(name, file)]));
    let mut rows: Vec<String> = label_snapshot(&cpg.dfg)
        .into_iter()
        .map(|((from, to), label)| {
            format!(
                "{}:{}@{}->{}@{}:{label:?}",
                from.function, from.path, from.line, to.path, to.line
            )
        })
        .collect();
    rows.sort();
    (rows, cpg.dfg_label_stats.dfg_rd_functions_without_cfg)
}

const LANGS: [(Language, &str); 3] = [
    (Language::JavaScript, "js"),
    (Language::TypeScript, "ts"),
    (Language::Tsx, "tsx"),
];

fn exact(row: &str) -> String {
    format!("{row}:Exact")
}
fn cfg_incomplete(row: &str) -> String {
    format!("{row}:NameOnly(CfgIncomplete)")
}
fn same_line(row: &str) -> String {
    format!("{row}:NameOnly(SameLine)")
}

#[test]
fn one_statement_block_body_is_exact() {
    for (language, ext) in LANGS {
        let (rows, without_cfg) = labels(language, ext, "function f(c) {\n  exec(c);\n}\n");
        assert_eq!(rows, [exact("f:c@1->c@2")], "{language:?}");
        assert_eq!(without_cfg, 0, "{language:?}");
    }
}

#[test]
fn single_return_and_multiline_statement_are_exact() {
    for (language, ext) in LANGS {
        let (rows, _) = labels(language, ext, "function f(c) {\n  return c.x;\n}\n");
        assert_eq!(rows, [exact("f:c@1->c.x@2")], "{language:?}");
        let (rows, _) = labels(language, ext, "function f(c) {\n  exec(\n    c,\n  );\n}\n");
        assert_eq!(rows, [exact("f:c@1->c@3")], "{language:?}");
    }
}

#[test]
fn expression_bodied_arrow_is_exact_across_lines() {
    for (language, ext) in LANGS {
        let (rows, without_cfg) = labels(language, ext, "const f = (c) =>\n  exec(c);\n");
        assert_eq!(rows, [exact("f:c@1->c@2")], "{language:?}");
        assert_eq!(without_cfg, 0, "{language:?}");
    }
}

#[test]
fn one_line_owners_keep_a_doubt_but_not_cfg_incomplete() {
    // The parameter's def line is the statement's line: SameLine, as for any
    // multi-statement function whose first statement shares the signature line.
    for (language, ext) in LANGS {
        let (rows, _) = labels(language, ext, "function f(c) { exec(c); }\n");
        assert_eq!(rows, [same_line("f:c@1->c@1")], "{language:?}");
        let (rows, _) = labels(language, ext, "const f = (c) => exec(c);\n");
        assert_eq!(rows, [same_line("f:c@1->c@1")], "{language:?}");
        // Two statements on one line are one statement line (one line-anchored use).
        let (rows, _) = labels(language, ext, "function f(c) { exec(c); run(c); }\n");
        assert_eq!(rows, [same_line("f:c@1->c@1")], "{language:?}");
    }
}

#[test]
fn rebinding_a_name_inside_the_node_keeps_the_fallback() {
    // `c = c + 1` is a second def of `c` in the same node: reaching
    // definitions cannot order them, so the owner stays unlabelled-by-RD.
    for (language, ext) in LANGS {
        let (rows, without_cfg) = labels(language, ext, "function f(c) {\n  c = c + 1;\n}\n");
        assert!(!rows.is_empty(), "{language:?}");
        assert!(
            rows.iter()
                .all(|row| row.ends_with("NameOnly(CfgIncomplete)")),
            "{language:?}: {rows:?}"
        );
        assert_eq!(without_cfg, 1, "{language:?}");
        // An alias def of the parameter's base name counts as a rebinding.
        let (rows, without_cfg) = labels(language, ext, "function f(c) {\n  const d = c;\n}\n");
        assert!(
            rows.iter()
                .all(|row| row.ends_with("NameOnly(CfgIncomplete)")),
            "{language:?}: {rows:?}"
        );
        assert_eq!(without_cfg, 1, "{language:?}");
    }
}

#[test]
fn empty_body_and_capturing_closure_are_unchanged() {
    for (language, ext) in LANGS {
        let (rows, without_cfg) = labels(language, ext, "function f(c) { }\n");
        assert!(rows.is_empty(), "{language:?}");
        assert_eq!(without_cfg, 1, "{language:?}: empty body keeps the refusal");
        // The outer read of `c` inside the nested callable is a capture.
        let src = "function f(c) {\n  return [1].map((x) => x + c);\n}\n";
        let (rows, _) = labels(language, ext, src);
        assert!(
            rows.contains(&cfg_incomplete("f:c@1->c@2")),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn multi_statement_functions_are_unchanged() {
    for (language, ext) in LANGS {
        let (rows, without_cfg) = labels(
            language,
            ext,
            "function f(c) {\n  const d = c;\n  exec(d);\n}\n",
        );
        assert_eq!(without_cfg, 0, "{language:?}");
        assert!(
            rows.contains(&exact("f:d@2->d@3")),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn other_languages_keep_the_refusal() {
    let (rows, without_cfg) = labels(Language::Python, "py", "def f(c):\n    exec(c)\n");
    assert_eq!(rows, [cfg_incomplete("f:c@1->c@2")]);
    assert_eq!(without_cfg, 1);
}

#[test]
fn the_edge_set_does_not_move() {
    // The `{ const d = c; ... }` one-liner emits no edge today (the declarator
    // initialiser is no rvalue span and its use sits on the signature line).
    // Giving the owner a CFG must neither add nor remove edges.
    for (language, ext) in LANGS {
        let (rows, _) = labels(
            language,
            ext,
            "function two(c) { const d = c; exec('ls ' + d); }\n",
        );
        assert!(rows.is_empty(), "{language:?}: {rows:?}");
    }
}

// ---------------------------------------------------------------------------
// Round-1 fold: the one-node path refuses owners with a write the def census
// does not record (hazards a-e) or a deferred read it does not see (hazard f).
// Each hazard input was Exact on 873a6d42 and is NameOnly(CfgIncomplete) on
// fec901a4. Multi-statement owners with the same shapes are unchanged by
// this PR; they are the pre-existing gaps tracked by #660.
// ---------------------------------------------------------------------------

fn assert_no_exact(name: &str, source: &str) {
    assert_no_exact_except(name, source, "");
}

/// `assert_no_exact`, ignoring rows that contain `ignored` (a non-empty marker): #761 makes a parameter
/// default `z = y` the alias `const z = y`, so the nested callable's own `z` def has an exact edge to its body.
fn assert_no_exact_except(name: &str, source: &str, ignored: &str) {
    for (language, ext) in LANGS {
        let (rows, without_cfg) = labels(language, ext, source);
        assert!(!rows.is_empty(), "{name}/{language:?}: no edge in {source}");
        assert!(
            rows.iter()
                .filter(|row| ignored.is_empty() || !row.contains(ignored))
                .all(|row| !row.ends_with(":Exact")),
            "{name}/{language:?}: {rows:?} for {source}"
        );
        let _ = without_cfg;
    }
}

#[test]
fn hazard_a_update_expressions_keep_the_fallback() {
    assert_no_exact(
        "postfix",
        "function f(x) {\n  return (\n    x++,\n    sink(x)\n  );\n}\n",
    );
    assert_no_exact(
        "prefix",
        "function f(x) {\n  return (\n    --x,\n    sink(x)\n  );\n}\n",
    );
    assert_no_exact(
        "closure update",
        "function f(x) {\n  return g(\n    () => x++,\n    sink(x)\n  );\n}\n",
    );
}

#[test]
fn hazard_b_pattern_defaults_keep_the_fallback() {
    assert_no_exact(
        "array",
        "function f(a) {\n  return ([a = 1] = src(), sink(a));\n}\n",
    );
    assert_no_exact(
        "shorthand",
        "function f(a) {\n  return ({ a = 1 } = src(), sink(a));\n}\n",
    );
    assert_no_exact(
        "expression arrow",
        "const f = (a) =>\n  ([a = 1] = src(), sink(a));\n",
    );
}

#[test]
fn hazard_c_hoisted_function_declarations_keep_the_fallback() {
    assert_no_exact(
        "shares param",
        "function h(x) {\n  function x() {}\n  use(x);\n}\n",
    );
    assert_no_exact(
        "after use",
        "function f(a) {\n  return sink(a); function a() {}\n}\n",
    );
    assert_no_exact(
        "non-simple params",
        "function h(f, o, fn = null) {\n  function f() {}\n  use(f);\n}\n",
    );
    // Control: a differently named declaration is not a rebinding.
    for (language, ext) in LANGS {
        let (rows, _) = labels(
            language,
            ext,
            "function h(x) {\n  function q() {}\n  use(x);\n}\n",
        );
        assert_eq!(rows, [exact("h:x@1->x@3")], "{language:?}");
    }
}

#[test]
fn hazard_d_mapped_arguments_keep_the_fallback() {
    assert_no_exact(
        "write",
        "function f(a) {\n  return (\n    arguments[0] = 0,\n    sink(a)\n  );\n}\n",
    );
    assert_no_exact(
        "arrow inherits",
        "function f(a) {\n  return g(() => arguments[0] = 0, sink(a));\n}\n",
    );
    // Control: a nested non-arrow function has its own `arguments`.
    for (language, ext) in LANGS {
        let src =
            "function f(a) {\n  return g(function () { return arguments[0]; }, sink(a));\n}\n";
        let (rows, _) = labels(language, ext, src);
        assert!(
            rows.contains(&exact("f:a@1->a@2")),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn hazard_e_direct_eval_keeps_the_fallback() {
    assert_no_exact(
        "eval",
        "function f(a) {\n  return (eval('a = 1'), sink(a));\n}\n",
    );
}

#[test]
fn hazard_f_nested_callable_defaults_are_deferred_reads() {
    assert_no_exact_except(
        "function expression default",
        "function h(y) {\n  g(function (z = y) {\n    use(z);\n  });\n}\n",
        ":z@",
    );
    assert_no_exact_except(
        "arrow default",
        "function h(y) {\n  g((z = y) => {\n    use(z);\n  });\n}\n",
        ":z@",
    );
    // #761: the permitted rows are exactly the nested callable's own alias binding `z` reaching its use (the
    // default `z = y` is the alias `const z = y`), and nothing else is exact.
    for (language, ext) in LANGS {
        for src in [
            "function h(y) {\n  g(function (z = y) {\n    use(z);\n  });\n}\n",
            "function h(y) {\n  g((z = y) => {\n    use(z);\n  });\n}\n",
        ] {
            let (rows, _) = labels(language, ext, src);
            let exact_rows: Vec<_> = rows.iter().filter(|r| r.ends_with(":Exact")).collect();
            assert_eq!(
                exact_rows,
                [&exact("<anon@2:5>:z@2->z@3")],
                "{language:?}: {rows:?} for {src}"
            );
        }
    }
    // Control: a nested callable without a parameter default.
    for (language, ext) in LANGS {
        let src = "function h(y) {\n  g(function (z) {\n    use(z);\n  }, y);\n}\n";
        let (rows, _) = labels(language, ext, src);
        assert!(
            rows.contains(&exact("h:y@1->y@4")),
            "{language:?}: {rows:?}"
        );
    }
}

#[test]
fn closure_and_member_writes_on_the_formal_keep_the_fallback() {
    // A `for (a of ..)` head is not on the one-node path: its loop edge gives
    // the owner a CFG, so main already runs RD on it.
    assert_no_exact(
        "closure write",
        "function f(a) {\n  return g(() => { a = evil(); }, sink(a));\n}\n",
    );
    assert_no_exact(
        "member write",
        "function f(a) {\n  return (a.x = 1, sink(a));\n}\n",
    );
}

// Probe survivors R657-OPUS-1 and R657-OPUS-4 (round 1).

#[test]
fn multi_line_expression_body_maps_every_body_line() {
    // Kills `end_line: span.line` in `line_span`: the read on the 4th line of the
    // body maps to the body's first line only through the span's end line.
    for (language, ext) in LANGS {
        let (rows, _) = labels(
            language,
            ext,
            "const f = (c) =>\n  exec(\n    1,\n    c,\n  );\n",
        );
        assert_eq!(rows, [exact("f:c@1->c@4")], "{language:?}");
    }
}

#[test]
fn member_path_write_on_the_formal_keeps_the_fallback() {
    // Kills a gate that stops counting member-path defs (`o.x = ..` shares base `o`).
    assert_no_exact(
        "member write then member read",
        "function f(o) {\n  return (\n    o.x = evil(),\n    sink(o.x),\n    sink(o)\n  );\n}\n",
    );
}

// ---------------------------------------------------------------------------
// Round-2 fold: the hazard walk covers the owner's own default values, and a
// direct eval is found by the shared detector (parentheses, comments).
// ---------------------------------------------------------------------------

#[test]
fn hazards_inside_the_owners_own_default_values_keep_the_fallback() {
    assert_no_exact("update", "function f(a, b = a++) {\n  sink(a);\n}\n");
    assert_no_exact("update arrow", "const f = (a, b = a++) =>\n  sink(a);\n");
    assert_no_exact(
        "array pattern default",
        "function f(a, b = ([a = 1] = src())) {\n  return sink(a);\n}\n",
    );
    assert_no_exact(
        "object pattern default",
        "function f(a, b = ({ a = 1 } = src())) {\n  return sink(a);\n}\n",
    );
    assert_no_exact(
        "eval",
        "function f(a, b = eval(\"a = 1\")) {\n  sink(a);\n}\n",
    );
}

#[test]
fn inert_defaults_and_unmapped_arguments_in_defaults_stay_exact() {
    // A non-simple parameter list makes `arguments` unmapped: it cannot alias `a`.
    for (language, ext) in LANGS {
        for src in [
            "function f(a, b = 1) {\n  sink(a);\n}\n",
            "function f(a, b = a + 1) {\n  sink(a);\n}\n",
            "function f(a, b = (arguments[0] = 9)) {\n  return sink(a);\n}\n",
        ] {
            let (rows, _) = labels(language, ext, src);
            assert!(
                rows.contains(&exact("f:a@1->a@2")),
                "{language:?}: {rows:?} for {src}"
            );
        }
    }
}

#[test]
fn parenthesised_and_commented_direct_eval_keep_the_fallback() {
    assert_no_exact(
        "parenthesised",
        "function f(a) {\n  return ((eval)(\"a = 1\"), sink(a));\n}\n",
    );
    assert_no_exact(
        "commented",
        "function f(a) {\n  return ((/* c */ eval /* d */)(\"a = 1\"), sink(a));\n}\n",
    );
}

#[test]
fn indirect_and_optional_eval_are_not_direct_eval() {
    // The shared detector decides: neither can write a local.
    for (language, ext) in LANGS {
        for src in [
            "function f(a) {\n  return ((0, eval)(\"a = 1\"), sink(a));\n}\n",
            "function f(a) {\n  return (eval?.(\"a = 1\"), sink(a));\n}\n",
        ] {
            let (rows, _) = labels(language, ext, src);
            assert!(
                rows.contains(&exact("f:a@1->a@2")),
                "{language:?}: {rows:?} for {src}"
            );
        }
    }
}

#[test]
fn hoisted_generator_declaration_named_like_the_formal_keeps_the_fallback() {
    assert_no_exact(
        "generator",
        "function h(x) {\n  function* x() {}\n  use(x);\n}\n",
    );
}
