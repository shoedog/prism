//! #693/#697: JS/TS/TSX loop exits and the `do` condition, as DataFlow labels. Every row of
//! `tests/fixtures/js_loops/variant-matrix.tsv` is one (fixture, edge) with its label on main
//! (4c8620a2, run by `scripts/matrix693.py`) and the label expected now. MOVE rows are the ones the
//! rule moves; HOLD rows are controls and moved-neither rows; RESIDUAL rows are shapes that keep main's
//! edges (a one-line `do`, a condition sharing a line) and so keep main's labels, pinned as such.
use super::census_defs_tests::trace_lines;
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../../tests/fixtures/js_loops/variant-matrix.tsv");

struct Row {
    fixture: String,
    lang: Language,
    edge: String,
    main: String,
    expected: String,
    kind: String,
    source: String,
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .skip(1)
        .map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 7, "matrix row: {line}");
            Row {
                fixture: c[0].to_string(),
                lang: match c[1] {
                    "ts" => Language::TypeScript,
                    "tsx" => Language::Tsx,
                    _ => Language::JavaScript,
                },
                edge: c[2].to_string(),
                main: c[3].to_string(),
                expected: c[4].to_string(),
                kind: c[5].to_string(),
                source: c[6].replace("\\n", "\n"),
            }
        })
        .collect()
}

#[test]
fn every_matrix_row_has_its_expected_label() {
    let all = rows();
    assert_eq!(all.len(), 232, "matrix rows");
    let mut failures = Vec::new();
    for r in &all {
        let got = labels_of(r.lang, &r.source, &r.edge);
        if got != [r.expected.clone()] {
            failures.push(format!(
                "{} {} ({}; main {}): want {}, got {got:?}",
                r.fixture, r.edge, r.kind, r.main, r.expected
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn the_rule_moves_the_rows_it_names_and_no_others() {
    let all = rows();
    for r in &all {
        match r.kind.as_str() {
            "MOVE" => assert_ne!(r.main, r.expected, "{} {}", r.fixture, r.edge),
            "HOLD" | "RESIDUAL" => assert_eq!(r.main, r.expected, "{} {}", r.fixture, r.edge),
            other => panic!("kind {other}"),
        }
    }
    let moved = all.iter().filter(|r| r.kind == "MOVE").count();
    assert_eq!(moved, 44, "rows the rule moves");
    // Every move to `exact` is justified in the tsv's notes: a `do` condition no longer sits at the
    // header (the first iteration reaches the body entry), and a switch `break` reaches the header.
    let promoted: Vec<String> = all
        .iter()
        .filter(|r| r.kind == "MOVE" && r.expected == "exact")
        .map(|r| format!("{} {}", r.fixture, r.edge))
        .collect();
    assert_eq!(
        promoted,
        [
            "continue_in_do a@1->a@4",
            "do_cond_assign a@1->a@4",
            "do_cond_own_line a@1->a@4",
            "do_cond_write a@1->a@4",
            "switch_in_loop i@5->i@3",
            "ts_do_cond_write a@1->a@4",
            "tsx_do_cond_write a@1->a@4",
        ]
    );
}

fn mechanisms(language: Language, src: &str) -> Vec<(String, String)> {
    trace_lines(language, src)
        .iter()
        .map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c[1].to_string(), c.get(4..).unwrap_or_default().join(" "))
        })
        .filter(|(m, _)| m == "loop-exit" || m == "do-cond")
        .collect()
}

#[test]
fn the_trace_names_the_loop_mechanism_of_each_moved_edge() {
    let js = Language::JavaScript;
    // A `for` update kills a body def before the read after the loop.
    let m = mechanisms(
        js,
        "function f(n) {\n  let i = 0;\n  for (; i < n; i++) {\n    i = g();\n  }\n  sink(i);\n}\n",
    );
    assert!(
        m.iter()
            .any(|(k, e)| k == "loop-exit" && e.starts_with("i@4:") && e.contains("i@6:")),
        "{m:?}"
    );
    // A `do` condition write: the edges touching the `do` are `do-cond`.
    let m = mechanisms(
        js,
        "function f(a) {\n  log();\n  do {\n    a = g();\n  } while (a-- > 0);\n  sink(a);\n}\n",
    );
    assert!(
        m.iter().all(|(k, _)| k == "do-cond") && m.len() >= 3,
        "{m:?}"
    );
    // Controls name nothing: a header that writes nothing the body defines, a one-line `do`, a loop-free owner.
    for src in [
        "function f(n) {\n  log();\n  for (;; n++) {\n    j = g();\n  }\n  sink(j);\n}\n",
        "function f(a) {\n  log();\n  do { a = g(); } while (a-- > 0);\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n",
    ] {
        assert!(
            mechanisms(js, src).is_empty(),
            "{src}: {:?}",
            mechanisms(js, src)
        );
    }
}
