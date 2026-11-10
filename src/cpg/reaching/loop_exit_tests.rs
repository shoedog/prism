//! #693/#697: JS/TS/TSX loop exits and the `do` condition, as DataFlow labels. Every row of
//! `tests/fixtures/js_loops/variant-matrix.tsv` is one (fixture, edge) with its label on main
//! (4c8620a2, run by `scripts/matrix693.py`), its label at the first head (8f1deff0) and the label
//! expected now. REFUSED rows (#693 slice 2) are the fixtures whose loop tree the plan refuses
//! (`f6_if_loop_else`: the `if` shares the loop's head line; `one_node_owner`: a one-line owner; slice 3
//! moved `f6_loop_ends_if_arm`, an arm ending in a wrapper holding the loop, to an admitted tree: its 15 rows are
//! HOLD rows, one MOVE row and one RESIDUAL row, `s@4->s@12`, an `exact` main draws from the `if`'s own skip
//! edge to the other arm, which no loop transition owns): main's edges stand and every label of the
//! owner is `nameonly/cfg_incomplete` (option O1), so their `main`/first-head columns are history only.
//! MOVE rows are the ones the rule moves; HOLD rows are controls and moved-neither rows;
//! ABSENT rows (merge of main 72be3b73, #788): rules U and D prune write-only and dominated def->use pairs from the
//! source text before the CFG is built, so 265 rows (HOLD 154, MOVE 16, REFUSED 95) name a pair that no longer exists. A
//! row is ABSENT only when new main 72be3b73 also lacks the pair (checked per row: `scripts/matrix_control.py`, 0
//! rows absent on the head and present on main); the test asserts the pair is absent. No fixture lost all its rows.
//! RESIDUAL rows are shapes of a loop tree that is not rewired (a one-line `do`, a condition sharing a
//! line, a jump through a `finally`, a `for` initializer re-run by a back route, an unrewirable
//! statement) and keep main's edges and labels, pinned as such. Since fold 1 of slice 2 such a tree is
//! classified by the plan too: the fixtures whose tree has a failed placement or an unresolved record
//! (every `do` condition without an execution point, the `finally` jumps, the multi-line `for` header, the
//! unrewirable statement) are REFUSED rows now, no longer RESIDUAL ones. Fold 2 of slice 2 refuses the owner of EVERY tree main's own policy refuses (a `for` initializer hazard, a jump through a `finally`, an unrouted form): such a tree is not rewired and keeps main's bypass edges, so no label of its owner may be `exact`; the 98 former RESIDUAL rows (six fixtures) are REFUSED rows, and no RESIDUAL row remains. FIX rows (round 1 of PR #785) are the
//! rows whose label differs from 8f1deff0's: each is a wrong `exact` or a false kill that commit made.
use super::census_defs_tests::{trace_lines, trace_lines_main_loops};
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const MATRIX: &str = include_str!("../../../tests/fixtures/js_loops/variant-matrix.tsv");

struct Row {
    fixture: String,
    lang: Language,
    edge: String,
    main: String,
    first_head: String,
    expected: String,
    kind: String,
    fix: bool,
    source: String,
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .skip(1)
        .map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 9, "matrix row: {line}");
            Row {
                fixture: c[0].to_string(),
                lang: match c[1] {
                    "ts" => Language::TypeScript,
                    "tsx" => Language::Tsx,
                    _ => Language::JavaScript,
                },
                edge: c[2].to_string(),
                main: c[3].to_string(),
                first_head: c[4].to_string(),
                expected: c[5].to_string(),
                kind: c[6].to_string(),
                fix: c[7] == "FIX",
                source: c[8].replace("\\n", "\n"),
            }
        })
        .collect()
}

#[test]
fn every_matrix_row_has_its_expected_label() {
    let all = rows();
    assert_eq!(all.len(), 741, "matrix rows");
    let mut failures = Vec::new();
    for r in &all {
        // `labels_of` spells the alias doubt as its debug form; the matrix uses the CLI's.
        let got: Vec<String> = labels_of(r.lang, &r.source, &r.edge)
            .into_iter()
            .map(|l| match l.as_str() {
                "NameOnly(AliasUnstable)" => "nameonly/alias_unstable".to_string(),
                _ => l,
            })
            .collect();
        // ABSENT rows: the pair no longer exists (see the module doc); they assert exactly that.
        let ok = if r.kind == "ABSENT" {
            got.is_empty()
        } else {
            got == [r.expected.clone()]
        };
        if !ok {
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
            "REFUSED" => assert_eq!(r.expected, "nameonly/cfg_incomplete", "{}", r.fixture),
            "ABSENT" => assert_eq!(r.expected, "-", "{} {}", r.fixture, r.edge),
            other => panic!("kind {other}"),
        }
    }
    let moved = all.iter().filter(|r| r.kind == "MOVE").count();
    assert_eq!(moved, 78, "rows the rule moves");
    assert_eq!(
        all.iter().filter(|r| r.kind == "REFUSED").count(),
        104,
        "rows of refused trees"
    );
    assert_eq!(
        all.iter().filter(|r| r.kind == "ABSENT").count(),
        265,
        "rows whose pair #788 removed"
    );
    for r in all
        .iter()
        .filter(|r| r.kind != "REFUSED" && r.kind != "ABSENT")
    {
        assert_eq!(
            r.fix,
            r.first_head != r.expected,
            "{} {}: a FIX row is exactly a row whose 8f1deff0 label differs",
            r.fixture,
            r.edge
        );
    }
    assert_eq!(
        all.iter().filter(|r| r.fix && r.kind != "ABSENT").count(),
        37,
        "round-1 FIX rows that still have a pair"
    );
    // Every move to `exact` was read by hand: a `do` condition no longer sits at the header (the first
    // iteration reaches the body entry), a `break`/`continue`/switch `break` reaches the continue point
    // or the code after the loop, an end-of-body arm reaches the next iteration, a `for`..`in`/`of`
    // `continue` reaches the code after the loop without a head write.
    let promoted: Vec<String> = all
        .iter()
        .filter(|r| r.kind == "MOVE" && r.expected == "exact")
        .map(|r| format!("{} {}", r.fixture, r.edge))
        .collect();
    assert_eq!(
        promoted,
        [
            "f1_ctl_jump_in_finally_block x@7->x@11",
            "f1_ctl_try_without_finally x@5->x@11",
            "f2_ctl_do_rewirable x@2->x@4",
            "f2_do_else_empty_stmt x@2->x@4",
            "f4_destructuring_head a@5->a@4",
            "f4_destructuring_head a@5->a@8",
            "f4_for_await_continue x@4->x@3",
            "f4_for_await_continue x@4->x@7",
            "f4_forin_member_head o.p@4->o.p@7",
            "f4_forof_continue x@4->x@3",
            "f4_forof_continue x@4->x@7",
            "f4_labeled_continue_from_inner x@5->x@3",
            "f4_labeled_continue_from_inner x@5->x@9",
            "f5_comment_only_catch y@7->y@4",
            "f5_empty_catch y@7->y@4",
            "f5_empty_else y@7->y@4",
            "f5_empty_finally x@6->x@4",
            "f5_empty_then y@8->y@4",
            "s1_supported_labeled_continue x@5->x@10",
            "switch_in_loop i@5->i@3",
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
    // A one-line `do` is not rewired (its condition has no execution point): under main's loop CFG it names
    // nothing. Under the plan its owner is refused by label (the trace names those edges), pinned by REFUSED rows.
    let one_line = "function f(a) {\n  log();\n  do { a = g(); } while (a-- > 0);\n  sink(a);\n}\n";
    let names: Vec<_> = trace_lines_main_loops(js, one_line)
        .iter()
        .filter(|l| {
            l.split('\t')
                .nth(1)
                .is_some_and(|m| m == "loop-exit" || m == "do-cond")
        })
        .cloned()
        .collect();
    assert!(names.is_empty(), "{names:?}");
    assert!(
        !mechanisms(js, one_line).is_empty(),
        "the refusal moves the owner's labels"
    );
    // Controls name nothing: a header that writes nothing the body defines, a loop-free owner.
    for src in [
        "function f(n) {\n  log();\n  for (;; n++) {\n    j = g();\n  }\n  sink(j);\n}\n",
        "function f(a) {\n  log();\n  a++;\n  sink(a);\n}\n",
    ] {
        assert!(
            mechanisms(js, src).is_empty(),
            "{src}: {:?}",
            mechanisms(js, src)
        );
    }
}
