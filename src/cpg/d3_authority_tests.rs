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

/// Known limit, not D3's (#700): RD maps every line to the smallest statement covering it, so a statement
/// that starts on the last line of a shorter multi-line one (here the `if`) has no node of its own, and its
/// defs and kills (`a = 2`, `a++`) land in the `if`'s node, before the `a = 1` inside it. The head label
/// is `exact` and is WRONG (`a++` always runs after `a = 1`); the fix is RD's line-to-node rule, tracked
/// as #700, and that issue flips this row.
#[test]
fn rd_line_to_node_limit_is_pinned() {
    for update in ["a++", "a = 2"] {
        let src = format!(
            "function f(c) {{\n  let a = 0;\n  if (c) {{\n    a = 1;\n  }} {update}, sink2(1, 2, 3, 4, 5, 6);\n  sink(a);\n}}"
        );
        assert_eq!(labels_of(JS, &src, "a@4->a@6"), [EXACT], "{src}");
    }
}

/// #699 round 1 (W1): a kill-only write enters RD at its E18 statement-start line, a recorded def at the
/// target's line. D3 keys each write by the line RD is handed for that very site, so the def and the write
/// meet in one node whenever RD puts them in one. Every row's true answer: the later write always runs
/// between the def and the sink, so `exact` is wrong. All but `w1_preceding_tail_inherited` (#700's layout,
/// `exact` on c6393854 too) were already non-exact on c6393854.
#[rustfmt::skip]
const W1_ROWS: &[Row] = &[
    Row { name: "w1_update_in_tail", lang: JS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  g(a = 1,\n    a++); x();\n  sink(a);\n}" },
    Row { name: "w1_update_in_tail_ts", lang: TS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a: number) {\n  log();\n  g(a = 1,\n    a++); x();\n  sink(a);\n}" },
    Row { name: "w1_decrement_in_tail", lang: JS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  g(a = 1,\n    a--); x();\n  return a;\n}" },
    Row { name: "w1_pattern_default_in_tail", lang: JS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  g(a = 1,\n    [a = 2] = s()); x();\n  sink(a);\n}" },
    Row { name: "w1_delete_member_in_tail", lang: JS, edge: "o.x@3->o.x@5", expected: SAME,
        source: "function f(o) {\n  log();\n  g(o.x = 1,\n    delete o.x); y();\n  sink(o.x);\n}" },
    Row { name: "w1_update_in_declarator_tail", lang: JS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  let b = (a = 1,\n    a++, 0); x();\n  sink(a);\n}" },
    Row { name: "w1_update_after_nested_tail", lang: JS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  g(a = 1, h(1,\n    2), a++); x();\n  sink(a);\n}" },
    Row { name: "w1_sol_update_two_lines_below", lang: JS, edge: "a@4->a@6", expected: SAME,
        source: "function f() {\n  let a = 0;\n  g(\n    a = 2,\n    a++); q();\n  sink(a);\n}" },
    Row { name: "w1_sol_padding_changes_the_winner", lang: JS, edge: "a@4->a@6", expected: SAME,
        source: "function f() {\n  let a = 0;\n  g(\n    a = 2,\n    a++); qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq();\n  sink(a);\n}" },
    Row { name: "w1_preceding_tail_inherited", lang: JS, edge: "a@4->a@6", expected: SAME,
        source: "function f(a) {\n  log();\n  p(y,\n    z); g(a = 1,\n    a++); x();\n  sink(a);\n}" },
    // Reversed order: the update runs first, the plain write last, so the def does reach the sink.
    Row { name: "w1_sol_reversed_order_control", lang: JS, edge: "a@5->a@6", expected: EXACT,
        source: "function f() {\n  let a = 0;\n  g(\n    a++,\n    a = 2); q();\n  sink(a);\n}" },
    // Moves toward `exact` against c6393854 (its E18 keying over-refused them): the use is anchored at a plain
    // write's own target (no read to order), or is the operand read of the node's one counting write (E2).
    Row { name: "toward_exact_use_at_write_target", lang: JS, edge: "a@3->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(a = 1,\n    a = 2); x();\n  sink(a);\n}" },
    Row { name: "toward_exact_compound_operand_read", lang: JS, edge: "a@3->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(a = 1,\n    a += 2); x();\n  sink(a);\n}" },
    // r2: without ` x();` no shorter span covers line 4 and base already refused.
    Row { name: "w1_r2_without_follower_control", lang: JS, edge: "a@3->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  g(a = 1,\n    a++);\n  sink(a);\n}" },
];

#[test]
fn kill_only_writes_are_keyed_by_the_line_rd_is_given() {
    let mut failures = Vec::new();
    for r in W1_ROWS {
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

/// The def-side axis of the shape matrix: a def and a later write both inside a multi-line tail (the write
/// kill-only or recorded, on the tail's last line or below it), with the sink after a follower of each
/// length. The later write always runs after the def, so the def never reaches the sink: not `exact`.
#[test]
fn a_def_and_a_later_write_inside_the_tail_never_reach_the_sink_exact() {
    const WRITES: &[(&str, &str)] = &[
        ("update", "a++"),
        ("decrement", "a--"),
        ("pattern", "[a = 2] = s()"),
        ("plain", "a = 2"),
        ("compound", "a += 2"),
    ];
    const FOLLOWERS: &[&str] = &["", " x();", " more(1, 2, 3, 4, 5, 6, 7, 8);"];
    let mut failures = Vec::new();
    for pre in ["", "  p(y,\n    z);\n"] {
        for (name, w) in WRITES {
            for pad in ["", PAD] {
                for follower in FOLLOWERS {
                    let src = format!(
                        "function f(a) {{\n  log();\n{pre}  g(a = 1,{pad}\n    {w});{follower}\n  sink(a);\n}}"
                    );
                    let line = src.lines().count() - 1;
                    let def = if pre.is_empty() { 3 } else { 5 };
                    let got = labels_of(JS, &src, &format!("a@{def}->a@{line}"));
                    if got.is_empty() || got.iter().any(|g| g == EXACT) {
                        failures.push(format!(
                            "{name}{} {got:?}\n{src}",
                            if pad.is_empty() { "" } else { "+pad" }
                        ));
                    }
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Known WRONG `exact`, owned by #674 (G1/G2, DESIGN 5.4): RD's recorded defs kill only by path equality, so a
/// recorded prefix write (`o = {}`) never kills `o.x`. c6393854 refused this edge only because E18 put the
/// prefix write in the def's node; D3 now keys it where RD puts it (the `y();` node), both nodes are clean, and
/// the label is `exact` although `o = {}` always runs between the def and the sink. The straight-line
/// `o.x = 1; o = {}; sink(o.x)` is `exact` on main for the same reason. The row flips when #674 lands.
#[test]
fn prefix_write_in_a_shorter_node_is_the_g1_gap() {
    let src = "function f(o) {\n  log();\n  g(o.x = 1,\n    o = {}); y();\n  sink(o.x);\n}";
    assert_eq!(labels_of(JS, src, "o.x@3->o.x@5"), [EXACT], "{src}");
}
