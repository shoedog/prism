//! #660 S1b-2: the matrix rows that name S1b-2 (same-node order relation, D3) as label rows, plus the
//! placement and position rows the S1b-1 review history asked for. Label-only: no edge moves.
//! FIX rows are `exact` on base d891f74d and move to `nameonly/sameline`; controls hold on both.
use super::census_label_tests::labels_of;
use crate::js_write_census::matrix_tests::lang_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../tests/fixtures/js_write_census/variant-matrix.tsv");

const SAME: &str = "nameonly/sameline";
const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";

struct Row {
    fixture: String,
    edge: String,
    expected: String,
    source: String,
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .skip(1)
        .filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 8, "matrix row: {line}");
            c[5].contains("S1b-2").then(|| Row {
                fixture: c[0].to_string(),
                edge: c[1].to_string(),
                expected: c[4].to_string(),
                source: c[7].replace("\\n", "\n"),
            })
        })
        .collect()
}

/// Rows whose head label the design's `expected` column cannot yet give (a later slice, or a
/// contradiction with the design): the head's label is pinned here and the row is listed in the PR.
const PINNED: &[(&str, &str, &str)] = &[];

fn want(r: &Row) -> String {
    PINNED
        .iter()
        .find(|(f, e, _)| *f == r.fixture && *e == r.edge)
        .map_or_else(|| r.expected.clone(), |(_, _, w)| (*w).to_string())
}

#[test]
fn matrix_rows_move_to_the_expected_label() {
    let all = rows();
    assert_eq!(all.len(), 45, "S1b-2 matrix rows");
    let mut failures = Vec::new();
    for r in &all {
        let want = want(r);
        let got = labels_of(lang_of(&r.fixture), &r.source, &r.edge);
        if got.is_empty() || got.iter().any(|g| *g != want) {
            failures.push(format!(
                "{} {}: want {want}, got {got:?}",
                r.fixture, r.edge
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The edge set never moves: each row's edge exists with some label on head.
#[test]
fn matrix_rows_keep_their_edges() {
    for r in rows() {
        assert!(
            !labels_of(lang_of(&r.fixture), &r.source, &r.edge).is_empty(),
            "{} {}: edge disappeared",
            r.fixture,
            r.edge
        );
    }
}

struct Case {
    name: &'static str,
    lang: Language,
    edge: &'static str,
    source: &'static str,
    expected: &'static str,
}

const JS: Language = Language::JavaScript;

#[rustfmt::skip]
const CASES: &[Case] = &[
    // Two occurrences of the use path on one line with different order outcomes: fail toward refusal.
    Case { name: "two_reads_around_write", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  sink(a), a++, sink(a);\n}" },
    Case { name: "two_reads_before_write_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  sink(a), sink(a), a++;\n}" },
    // A multi-line statement: the continuation line holds the write (or the read).
    Case { name: "continuation_write_before_read", lang: JS, edge: "a@1->a@5", expected: SAME,
        source: "function f(a) {\n  log();\n  g(\n    a++,\n    sink(a));\n}" },
    Case { name: "continuation_read_before_write_control", lang: JS, edge: "a@1->a@4", expected: EXACT,
        source: "function f(a) {\n  log();\n  g(\n    sink(a),\n    a++);\n}" },
    // JSX attribute then child, and the reverse.
    Case { name: "jsx_attr_write_child_read", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  return <div x={a++}>{sink(a)}</div>;\n}" },
    Case { name: "jsx_attr_read_child_write_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  return <div x={sink(a)}>{a++}</div>;\n}" },
    // Template literal substitutions.
    Case { name: "template_write_then_read", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  t = `${a++}${sink(a)}`;\n}" },
    Case { name: "template_read_then_write_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  t = `${sink(a)}${a++}`;\n}" },
    // Optional chaining: arguments run left to right; a skipped call cannot reorder.
    Case { name: "optional_call_write_then_read", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a, o) {\n  log();\n  o?.m(a++, sink(a));\n}" },
    Case { name: "optional_call_read_then_write_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a, o) {\n  log();\n  o?.m(sink(a), a++);\n}" },
    // `?:` and `&&` can skip a position but never reorder one.
    Case { name: "ternary_skipped_write_before_read", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a, c) {\n  log();\n  (c ? a++ : 0), sink(a);\n}" },
    Case { name: "and_skipped_write_after_read_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a, c) {\n  log();\n  sink(a), c && a++;\n}" },
    // A write in a nested arrow on the line is deferred (D2-c's), not ordered here.
    // (main already refuses it: a write in a nested body is a capture; D3 adds nothing.)
    Case { name: "nested_arrow_write_not_counted", lang: JS, edge: "a@1->a@3", expected: CFG,
        source: "function f(a) {\n  log();\n  h(() => { a++; }, sink(a));\n}" },
    // The single-site operand exception, with and without a second write, in a loop. (#693 slice 2: a
    // trailing `debugger;` is a statement `js_loops` has no completion rule for, so the loop stays
    // unrewired under main's policy: a loop sharing a line with its body is refused by the plan and every
    // label of its owner would be cfg_incomplete, hiding what these rows test.)
    Case { name: "operand_exception_loop", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  while (c()) { a++; debugger; }\n  sink(a);\n}" },
    Case { name: "operand_exception_second_write", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  while (c()) { a++; a = 1; debugger; }\n  sink(a);\n}" },
    Case { name: "operand_exception_member", lang: JS, edge: "o.x@2->o.x@3", expected: EXACT,
        source: "function f(o) {\n  o.x = 0;\n  o.x++;\n  sink(o.x);\n}" },
    // Unknown-kind positions: the byte order alone would prove read -> write, the kind refuses.
    Case { name: "array_pattern_default", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  let b;\n  ([b = sink(a)] = []), a++;\n}" },
    Case { name: "object_pattern_default", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  let b;\n  ({ b = sink(a) } = {}), a++;\n}" },
    Case { name: "pair_pattern_computed_key", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  let b;\n  ({ [sink(a)]: b } = {}), a++;\n}" },
    Case { name: "rest_pattern_subscript", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  let b = [];\n  ({ ...b[sink(a)] } = {}), a++;\n}" },
    Case { name: "class_body_computed_key", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  const C = class { [sink(a)]() {} static x = a++; };\n}" },
    Case { name: "for_in_iterable", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a, o) {\n  log();\n  for (const k in (sink(a), o)) { a++; debugger; }\n}" },
    // (decorators refuse on main already: pinned unchanged.)
    Case { name: "decorator_pair", lang: Language::TypeScript, edge: "a@1->a@3", expected: CFG,
        source: "function f(a: number) {\n  log();\n  @dec(sink(a)) @dec2(a++) class C {}\n}" },
    // Round 1 (Opus W1): reads in a later statement on the line are placed like writes, not dropped.
    Case { name: "later_stmt_write_then_read", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  a = 2; sink(a);\n}" },
    Case { name: "later_stmt_def_line2", lang: JS, edge: "a@2->a@4", expected: SAME,
        source: "function f() {\n  let a = 1;\n  log();\n  a = 2; sink(a);\n}" },
    Case { name: "later_stmt_in_block", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  { a = 2; sink(a); }\n}" },
    Case { name: "later_stmt_after_block", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  { a = 2; } sink(a);\n}" },
    Case { name: "later_stmt_if_read", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  a = src(); if (a) sink(a);\n}" },
    Case { name: "later_stmt_third", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  a = 2; g(); sink(a);\n}" },
    Case { name: "later_stmt_member", lang: JS, edge: "o.x@2->o.x@3", expected: SAME,
        source: "function f(o) {\n  o.x = 0;\n  o.x = 1; sink(o.x);\n}" },
    Case { name: "later_stmt_delete", lang: JS, edge: "o.x@2->o.x@3", expected: SAME,
        source: "function f(o) {\n  o.x = 0;\n  delete o.x; sink(o.x);\n}" },
    // Read first: one statement proves it; two statements cannot be ordered (cost, recovery #670).
    Case { name: "read_first_one_statement_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  sink(a), a = 2;\n}" },
    Case { name: "read_first_two_statements_cost", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  sink(a); a = 2;\n}" },
    // Round 1 (sol W1, Opus OR1/OR4): an unplaced counting write refuses beside placed ones. (Here
    // main's binding facts already kill the edge, and D3 never rewrites a non-`exact` label.)
    Case { name: "unplaced_beside_placed_write", lang: JS, edge: "a@1->a@4", expected: "nameonly/killed",
        source: "function f(a) {\n  log();\n  class C { static x = (a = 5); }\n  sink(a), a++;\n}" },
    Case { name: "placed_write_only_control", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  sink(a), a++;\n}" },
    Case { name: "unplaced_read_provably_first_cost", lang: JS, edge: "a@1->a@3", expected: SAME,
        source: "function f(a) {\n  log();\n  sink(a);\n  class C { static x = (a = 5); }\n}" },
    Case { name: "def_unplaced_formal_on_write_line", lang: JS, edge: "a@1->a@2", expected: SAME,
        source: "function f(a) { a++;\n  sink(a);\n}" },
    // Round 1 (Opus S4): binding checks on the use side.
    Case { name: "shadowed_read_is_not_ds_use", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a, c) {\n  log();\n  if (c) { sink(a); } else { let a = 1; g(a); }\n}" },
    Case { name: "distinct_write_binding_not_counted", lang: JS, edge: "a@1->a@3", expected: EXACT,
        source: "function f(a) {\n  log();\n  if (sink(a)) { let a = 1; g(a); } else h(a);\n}" },
    Case { name: "inner_block_declarator_not_counted", lang: JS, edge: "b@2->b@3", expected: EXACT,
        source: "f(function() {\n let b = 1;\n {let b = 2; use(b);} use(b);\n});\n" },
];

#[test]
fn placement_and_position_rows() {
    let mut failures = Vec::new();
    for c in CASES {
        let got = labels_of(c.lang, c.source, c.edge);
        if got.is_empty() || got.iter().any(|g| g != c.expected) {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                c.name, c.edge, c.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
