//! #674 (G1/G2): a recorded def of a JS/TS/TSX owner kills every recorded def its path `affects`
//! (a prefix write kills the paths below it, `[]` matches any component either way), as the census's
//! kill-only defs already do. Label-only: no edge moves. FIX rows are `exact` on base 3991e252 and
//! move to `nameonly/killed`; CONTROL rows hold on both. Python and Go keep main's equal-path kills.
use super::census_label_tests::labels_of;
use crate::js_write_census::matrix_tests::lang_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../tests/fixtures/js_write_census/variant-matrix.tsv");

const EXACT: &str = "exact";
const KILLED: &str = "nameonly/killed";
const SAME: &str = "nameonly/sameline";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;

struct Case {
    name: &'static str,
    lang: Language,
    edge: &'static str,
    source: &'static str,
    expected: &'static str,
}

/// The two matrix rows this slice flips (o27 = G2, o29 = G1) are FIX rows too.
#[test]
fn flipped_matrix_rows() {
    let mut seen = 0;
    for line in MATRIX.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if !c[5].contains("#674") {
            continue;
        }
        seen += 1;
        assert_eq!(c[2], "exact", "{}: main label", c[0]);
        assert_eq!(c[4], KILLED, "{}: expected label", c[0]);
        let got = labels_of(lang_of(c[0]), &c[7].replace("\\n", "\n"), c[1]);
        assert_eq!(got, [KILLED], "{} {}", c[0], c[1]);
    }
    assert_eq!(seen, 2, "o27 and o29");
}

#[rustfmt::skip]
const FIX: &[Case] = &[
    // G1: a prefix write kills the paths below it.
    Case { name: "prefix_write_straight_line", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o = {};\n  sink(o.x);\n}" },
    Case { name: "two_deep_prefix", lang: JS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o) {\n  o.a.b = 1;\n  o.a = {};\n  sink(o.a.b);\n}" },
    Case { name: "root_write_kills_two_deep", lang: JS, edge: "o.a.b@2->o.a.b@4", expected: KILLED,
        source: "function f(o) {\n  o.a.b = 1;\n  o = {};\n  sink(o.a.b);\n}" },
    Case { name: "this_prefix", lang: JS, edge: "this.a.b@2->this.a.b@4", expected: KILLED,
        source: "function f() {\n  this.a.b = 1;\n  this.a = {};\n  sink(this.a.b);\n}" },
    Case { name: "prefix_both_branches", lang: JS, edge: "o.x@2->o.x@8", expected: KILLED,
        source: "function f(o, c) {\n  o.x = 1;\n  if (c) {\n    o = {};\n  } else {\n    o = {};\n  }\n  sink(o.x);\n}" },
    Case { name: "prefix_write_in_the_loop_body", lang: JS, edge: "o.x@2->o.x@5", expected: KILLED,
        source: "function f(o, xs) {\n  o.x = 0;\n  for (const x of xs) {\n    o = {};\n    sink(o.x);\n  }\n}" },
    Case { name: "optional_chain_read", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o = {};\n  sink(o?.x);\n}" },
    Case { name: "ts_prefix", lang: TS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o: any) {\n  o.x = 1;\n  o = {} as any;\n  sink(o.x);\n}" },
    Case { name: "tsx_prefix", lang: Language::Tsx, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o: any) {\n  o.x = 1;\n  o = {};\n  return <b>{sink(o.x)}</b>;\n}" },
    // The prefix write sits in a later statement than the def (D3 sees both nodes clean).
    Case { name: "prefix_write_in_a_shorter_node", lang: JS, edge: "o.x@3->o.x@5", expected: KILLED,
        source: "function f(o) {\n  log();\n  g(o.x = 1,\n    o = {}); y();\n  sink(o.x);\n}" },
    // G2: `[]` is a wildcard component, both directions.
    Case { name: "subscript_write_kills_dot_def", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o[\"x\"] = 2;\n  sink(o.x);\n}" },
    Case { name: "dot_write_kills_subscript_def", lang: JS, edge: "o[]@2->o[]@4", expected: KILLED,
        source: "function f(o, k) {\n  o[k].x = 1;\n  o.a = 2;\n  sink(o[k].x);\n}" },
    Case { name: "subscript_in_the_middle", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: KILLED,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o.a[k] = 2;\n  sink(o.a.x);\n}" },
    Case { name: "root_write_kills_subscript_def", lang: JS, edge: "o[]@2->o[]@4", expected: KILLED,
        source: "function f(o, k) {\n  o[k].x = 1;\n  o = {};\n  sink(o[k].x);\n}" },
];

#[rustfmt::skip]
const CONTROLS: &[Case] = &[
    // A write at or below the def's path does not kill it more than main did.
    Case { name: "below_the_def_path", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  o.x.y = 2;\n  sink(o.x);\n}" },
    Case { name: "sibling_field", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  o.y = 2;\n  sink(o.x);\n}" },
    Case { name: "other_base", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o, p) {\n  o.x = 1;\n  p = {};\n  sink(o.x);\n}" },
    Case { name: "longer_write_below_subscript_def", lang: JS, edge: "o[]@2->o[]@4", expected: EXACT,
        source: "function f(o, k) {\n  o[k].x = 1;\n  o.x.y = 2;\n  sink(o[k].x);\n}" },
    Case { name: "sibling_under_subscript", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: EXACT,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o.b[k] = 2;\n  sink(o.a.x);\n}" },
    // Main's equal-path kills are a subset.
    Case { name: "equal_path_kill", lang: JS, edge: "o.x@2->o.x@4", expected: KILLED,
        source: "function f(o) {\n  o.x = 1;\n  o.x = 2;\n  sink(o.x);\n}" },
    Case { name: "equal_subscript_kill", lang: JS, edge: "o[]@2->o[]@4", expected: KILLED,
        source: "function f(o, k, j) {\n  o[k].x = 1;\n  o[j].x = 2;\n  sink(o[k].x);\n}" },
    // Already `killed` on main (a census kill-only def): the recorded relation must not change it.
    Case { name: "subscript_writer_kills_dot_def_already_killed", lang: JS, edge: "o.a.x@2->o.a.x@4", expected: KILLED,
        source: "function f(o, k) {\n  o.a.x = 1;\n  o[k].x = 2;\n  sink(o.a.x);\n}" },
    // A prefix write on one branch only: the other branch still carries the def.
    Case { name: "prefix_on_one_branch", lang: JS, edge: "o.x@2->o.x@6", expected: EXACT,
        source: "function f(o, c) {\n  o.x = 1;\n  if (c) {\n    o = {};\n  }\n  sink(o.x);\n}" },
    // Another binding of the base: the block `let o` is not the def's binding (flat kills it, bound does not).
    Case { name: "shadowing_let_in_a_block", lang: JS, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "function f(o) {\n  o.x = 1;\n  { let o = {}; o = {}; }\n  sink(o.x);\n}" },
    // The write is in the def's own statement node: no kill there (`affects` kills from other nodes only).
    Case { name: "prefix_in_the_def_node", lang: JS, edge: "o.x@2->o.x@3", expected: SAME,
        source: "function f(o) {\n  g(o.x = 1, o = {});\n  sink(o.x);\n}" },
    // Other languages keep main's equal-path kills.
    Case { name: "python_prefix", lang: Language::Python, edge: "o.x@2->o.x@4", expected: EXACT,
        source: "def f(o):\n    o.x = 1\n    o = {}\n    sink(o.x)\n" },
    Case { name: "go_prefix", lang: Language::Go, edge: "o.x@4->o.x@6", expected: EXACT,
        source: "package p\n\nfunc f(o T) {\n\to.x = 1\n\to = T{}\n\tsink(o.x)\n}\n" },
];

fn run(cases: &[Case]) -> Vec<String> {
    let mut failures = Vec::new();
    for c in cases {
        let got = labels_of(c.lang, c.source, c.edge);
        if got.is_empty() || got.iter().any(|g| g != c.expected) {
            failures.push(format!(
                "{} {}: want {}, got {got:?}",
                c.name, c.edge, c.expected
            ));
        }
    }
    failures
}

#[test]
fn fix_rows_move_to_killed() {
    let failures = run(FIX);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn control_rows_hold() {
    let failures = run(CONTROLS);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
