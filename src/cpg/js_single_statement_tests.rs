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
