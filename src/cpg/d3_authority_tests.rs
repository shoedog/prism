//! #696: D3 asks RD which node a position is in. The line-shape matrix: a multi-line statement ends on
//! the line where another starts, in every spelling and in both byte orders (the tail shorter or
//! longer than the statement that starts on the line), against the writes and reads that follow.
//! Expected labels are the order of the real execution; `exact` only where one span proves it.
use super::census_label_tests::labels_of;
use crate::languages::Language;

const SAME: &str = "nameonly/sameline";
const EXACT: &str = "exact";
/// A comment inside the tail's first line: the tail is longer than the statement that follows it.
const PAD: &str = " /*-----------------------------------------------------------*/";

/// A statement that ends on its last line, in the spellings the family needs (first lines, last line).
const TAILS: &[(&str, &str)] = &[
    ("call", "  g(x,{pad}\n    y);"),
    ("object", "  const o = {{pad}\n    k: 1 };"),
    ("array", "  const z = [{pad}\n    1];"),
    ("arrow", "  h(() =>{pad}\n    1);"),
    ("template", "  t = `x{pad}\ny`;"),
    ("jsx", "  r = <div>{pad}\n  </div>;"),
    ("if-paren", "  if (c{pad}\n  ) k();"),
    ("block", "  if (c) {{pad}\n    x();\n  }"),
    ("nested-block", "  {{pad}\n    {\n      x();\n    }\n  }"),
    ("comment", "  g(x,{pad}\n    y); /* c */"),
];

/// What follows the tail on its last line: (name, source, label of `a@1 -> the last line`).
const RESTS: &[(&str, &str, &str)] = &[
    ("write-read", " a = 2, sink(a);", SAME),
    ("update-read", " a++, sink(a);", SAME),
    ("two-statements", " a = 2; sink(a);", SAME),
    ("later-statement", " log2(); a = 2, sink(a);", SAME),
    (
        "read-write-control",
        " sink(a), a = 2, more(1, 2, 3);",
        EXACT,
    ),
    (
        "read-update-control",
        " sink(a), a++, more(1, 2, 3);",
        EXACT,
    ),
];

fn source(tail: &str, pad: &str, rest: &str) -> (String, String) {
    let tail = tail.replace("{pad}", pad);
    let body = format!("function f(a) {{\n  log();\n{tail}{rest}\n}}");
    let line = body.lines().count() - 1;
    (body, format!("a@1->a@{line}"))
}

#[test]
fn a_statement_starting_where_another_ends_is_ordered_in_every_shape() {
    let mut failures = Vec::new();
    for (shape, tail) in TAILS {
        for pad in ["", PAD] {
            for (name, rest, want) in RESTS {
                // `k();` keeps the line's one statement slot: the statement after it has no span of its
                // own, so even the read-first control is unproven (the #670 cost, not the authority's).
                let want = if *shape == "if-paren" { SAME } else { want };
                let (src, edge) = source(tail, pad, rest);
                let got = labels_of(Language::JavaScript, &src, &edge);
                if got.is_empty() || got.iter().any(|g| g != want) {
                    failures.push(format!(
                        "{shape}{} {name}: want {want}, got {got:?}\n{src}",
                        if pad.is_empty() { "" } else { "+pad" }
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

struct Row {
    name: &'static str,
    lang: Language,
    edge: &'static str,
    expected: &'static str,
    source: &'static str,
}

const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;

#[rustfmt::skip]
const ROWS: &[Row] = &[
    // Opus r2 g6/k2/k4/h3 and the control: the write and the read are in different spans, or in one.
    Row { name: "tail_then_write_read", lang: JS, edge: "a@1->a@4", expected: SAME,
        source: "function f(a) {\n  log();\n  g(x,\n    y); a = 2, sink(a);\n}" },
    Row { name: "tail_then_write_read_ts", lang: TS, edge: "a@1->a@4", expected: SAME,
        source: "function f(a: number) {\n  log();\n  g(x,\n    y); a = 2, sink(a);\n}" },
    Row { name: "write_in_tail_read_after", lang: JS, edge: "a@1->a@4", expected: SAME,
        source: "function f(a) {\n  log();\n  g(x,\n    a = 2); sink(a);\n}" },
    Row { name: "object_tail_write_read_after", lang: JS, edge: "a@1->a@4", expected: SAME,
        source: "function f(a) {\n  log();\n  const o = {\n    k: 1, v: (a = 2) }; sink(a);\n}" },
    Row { name: "object_tail_write_read_after_ts", lang: TS, edge: "a@1->a@4", expected: SAME,
        source: "function f(a: number) {\n  log();\n  const o = {\n    k: 1, v: (a = 2) }; sink(a);\n}" },
    Row { name: "tail_then_read_first_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(x,\n    y); sink(a), a = 2, more(1, 2, 3);\n}" },
    // Opus r2 u1: an unplaced write on another line refuses the base beside a placed one.
    Row { name: "unplaced_beside_placed_write_u1", lang: JS, edge: "a@1->a@6", expected: SAME,
        source: "function f(a, c) {\n  log();\n  if (c) {\n    class C { static x = (a = 5); }\n  }\n  sink(a), a++;\n}" },
    // Def side: D is in the tail and a write in the statement that starts on the tail's last line.
    Row { name: "def_in_tail_write_after_on_its_line", lang: JS, edge: "a@4->a@5", expected: SAME,
        source: "function f() {\n  let a = 0;\n  g(x,\n    a = 2); a++, more(1, 2, 3, 4, 5);\n  sink(a);\n}" },
    Row { name: "def_then_update_after_tail", lang: JS, edge: "a@4->a@5", expected: SAME,
        source: "function f() {\n  let a = 0;\n  g(x,\n    y); a = 2, a++;\n  sink(a);\n}" },
    Row { name: "update_then_def_after_tail_control", lang: JS, edge: "a@4->a@5", expected: EXACT,
        source: "function f() {\n  let a = 0;\n  g(x,\n    y); a++, a = 1;\n  sink(a);\n}" },
];

#[test]
fn issue_696_rows() {
    let mut failures = Vec::new();
    for r in ROWS {
        let got = labels_of(r.lang, r.source, r.edge);
        if got.is_empty() || got.iter().any(|g| g != r.expected) {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                r.name, r.edge, r.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Known limit, not D3's: RD maps every line to the smallest statement covering it, so a statement that
/// starts on the last line of a shorter multi-line one (here the `if`) has no node of its own, and its
/// defs and kills (`a = 2`, `a++`) land in the `if`'s node, before the `a = 1` inside it. The recorded def
/// is placed the same way as the kill-only one, so the authority rule does not move it. The head label is
/// `exact` and is WRONG (`a++` always runs after `a = 1`); a fix is RD's line-to-node rule (follow-up).
#[test]
fn rd_line_to_node_limit_is_pinned() {
    for update in ["a++", "a = 2"] {
        let src = format!(
            "function f(c) {{\n  let a = 0;\n  if (c) {{\n    a = 1;\n  }} {update}, sink2(1, 2, 3, 4, 5, 6);\n  sink(a);\n}}"
        );
        assert_eq!(labels_of(JS, &src, "a@4->a@6"), [EXACT], "{src}");
    }
}
