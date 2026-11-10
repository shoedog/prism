//! #802: JS-family generic try edges are explicit incomplete joins.
use super::*;

const LF: &str = "function lf() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    if ((x = seed())) {\n      x = more();\n    } else {\n      x = other();\n    }\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const CATCH_FINALLY: &str = "function f() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    x = 1;\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const CATCH_ONLY: &str = "function f() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    x = 1;\n  }\n  sink(x);\n}\n";
const FINALLY_ONLY: &str = "function f() {\n  let x = 0;\n  try {\n    work();\n  } finally {\n    x = 1;\n  }\n  sink(x);\n}\n";
const NESTED: &str = "function f() {\n  let x = 0;\n  try {\n    try {\n      work();\n    } catch (e) {\n      x = 1;\n    }\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const FIN_DEF: &str = "function f() {\n  let x = 0;\n  try {\n    return;\n  } finally {\n    x = 1;\n  }\n  sink(x);\n}\n";

/// Label of the edge from the `x` def on `def_line` to the use on `use_line`,
/// with defs on every line of `def_lines`.
fn labels(src: &str, def_lines: &[usize], use_line: usize) -> Vec<(usize, FlowConfidence)> {
    let parsed = parsed(src, Language::JavaScript);
    let func = function(&parsed);
    let defs: Vec<DefSite> = def_lines
        .iter()
        .enumerate()
        .map(|(i, &line)| def(&parsed, i as u32, "x", line, 0, false))
        .collect();
    let edges: Vec<FlowEdge> = defs
        .iter()
        .map(|d| edge(&parsed, &func, d, use_line))
        .collect();
    let (outcome, _) = run(&parsed, &defs, &edges);
    let RdOutcome::Available(result) = outcome else {
        panic!("RD unavailable: {outcome:?}");
    };
    defs.iter()
        .zip(&edges)
        .map(|(d, e)| {
            (
                d.line,
                *result
                    .labels
                    .get(&(e.from.clone(), e.to.clone()))
                    .expect("labelled"),
            )
        })
        .collect()
}

fn incomplete() -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
}

fn exact() -> FlowConfidence {
    FlowConfidence::Exact
}

#[test]
fn catch_overwrites_cannot_be_bypassed_by_the_catch_to_finally_edge() {
    // lf.js: both arms of the catch overwrite `x`; the generic catch-first ->
    // finally edge skips them, and was labelled exact on main.
    assert_eq!(
        labels(LF, &[2, 6, 7, 9], 14),
        vec![
            (2, incomplete()),
            (6, incomplete()),
            (7, exact()),
            (9, exact())
        ]
    );
}

#[test]
fn a_preserving_catch_and_finalizer_loses_its_exact_label_by_design() {
    // Accepted cost: x@6 -> sink is legitimately exact (catch completes, finalizer
    // preserves), but its only route crosses the catch -> finally join.
    assert_eq!(
        labels(CATCH_FINALLY, &[2, 6], 10),
        vec![(2, incomplete()), (6, incomplete())]
    );
}

#[test]
fn try_catch_without_finally_is_unchanged() {
    assert_eq!(
        labels(CATCH_ONLY, &[2, 6], 8),
        vec![(2, incomplete()), (6, exact())]
    );
}

#[test]
fn try_finally_without_catch_is_unchanged() {
    assert_eq!(
        labels(FINALLY_ONLY, &[2, 6], 8),
        vec![
            (
                2,
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 6 })
            ),
            (6, exact())
        ]
    );
}

#[test]
fn nested_try_without_a_catch_to_finally_edge_is_unchanged() {
    assert_eq!(
        labels(NESTED, &[2, 7], 12),
        vec![(2, incomplete()), (7, exact())]
    );
}

#[test]
fn finalizer_created_def_stays_exact_as_a_disclosed_residual() {
    // fin_def.js: `x = 1` in the finalizer cannot reach `sink(x)` after a
    // pending `return`, but the model has no pending completion, so this stays
    // exact. Explicit join provenance only restricts routes that START at a
    // def and cross a try edge; closing this needs pending-completion state
    // (#802 residual, #673).
    assert_eq!(
        labels(FIN_DEF, &[2, 6], 8),
        vec![(2, incomplete()), (6, exact())]
    );
}

fn try_origins(src: &str, language: Language, pair: (usize, usize)) -> Vec<String> {
    let parsed = parsed(src, language);
    crate::cfg::build_cfg_edges_with_arms(&parsed)
        .into_iter()
        .filter(|(e, _)| (e.from_line, e.to_line) == pair)
        .map(|(_, p)| format!("{:?}", p.origin))
        .collect()
}

#[test]
fn every_copy_of_a_try_line_pair_is_a_try_join() {
    // catch line 6 -> finally line 8 is emitted by the try builder and again by
    // the sequential loop (consecutive statements): both copies must carry it.
    let origins = try_origins(CATCH_FINALLY, Language::JavaScript, (6, 8));
    assert!(
        origins.len() >= 2,
        "expected a duplicate pair, got {origins:?}"
    );
    assert!(origins.iter().all(|o| o == "JsTryJoin"), "{origins:?}");
    // header -> catch and header -> finally.
    for pair in [(3, 6), (3, 8)] {
        let o = try_origins(CATCH_FINALLY, Language::JavaScript, pair);
        assert!(
            !o.is_empty() && o.iter().all(|o| o == "JsTryJoin"),
            "{pair:?}: {o:?}"
        );
    }
}

#[test]
fn typescript_and_tsx_are_js_family_and_java_is_not() {
    for language in [Language::TypeScript, Language::Tsx] {
        let o = try_origins(CATCH_FINALLY, language, (6, 8));
        assert!(
            !o.is_empty() && o.iter().all(|o| o == "JsTryJoin"),
            "{language:?}: {o:?}"
        );
    }
    let java = "class A {\n  void f() {\n    int x = 0;\n    try {\n      work();\n    } catch (Exception e) {\n      x = 1;\n    } finally {\n      cleanup();\n    }\n    sink(x);\n  }\n}\n";
    let parsed = parsed(java, Language::Java);
    let origins: Vec<String> = crate::cfg::build_cfg_edges_with_arms(&parsed)
        .into_iter()
        .map(|(_, p)| format!("{:?}", p.origin))
        .collect();
    assert!(!origins.is_empty());
    assert!(origins.iter().all(|o| o != "JsTryJoin"), "{origins:?}");
}

const NESTED_CATCH: &str = "function f() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    if ((x = seed())) {\n      if (p()) {\n        x = more();\n      } else {\n        x = other();\n      }\n    } else {\n      x = third();\n    }\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const NESTED_BODY: &str = "function f() {\n  let x = 0;\n  try {\n    if ((x = seed())) {\n      if (p()) {\n        x = more();\n      } else {\n        x = other();\n      }\n    } else {\n      x = third();\n    }\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const NESTED_CATCH_NO_FINALLY: &str = "function f() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    if ((x = seed())) {\n      if (p()) {\n        x = more();\n      } else {\n        x = other();\n      }\n    } else {\n      x = third();\n    }\n  }\n  sink(x);\n}\n";
const NESTED_NO_TRY: &str = "function f() {\n  let x = 0;\n  if ((x = seed())) {\n    if (p()) {\n      x = more();\n    } else {\n      x = other();\n    }\n  } else {\n    x = third();\n  }\n  sink(x);\n}\n";
const ONE_LINE_TRY: &str =
    "function f() {\n  let x = 0;\n  try { x = a(); } catch (e) {\n    sink(x);\n  }\n}\n";

#[test]
fn nested_if_arm_completion_into_the_finalizer_is_a_try_join() {
    // nested_catch.js: the catch's `if` arm ends in a nested `if`, so the branch
    // completion builder draws 7 -> 16 (nested header -> finally entry), skipping
    // every overwrite. Only compound-header completions into the entry are
    // flagged, so the ordinary else-arm exact (x@13) survives.
    let inc = incomplete();
    assert_eq!(
        labels(NESTED_CATCH, &[2, 6, 8, 10, 13], 18),
        vec![
            (2, inc),
            (6, inc),
            (
                8,
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 13 })
            ),
            (10, inc),
            (13, exact())
        ]
    );
}

#[test]
fn nested_if_arm_completion_of_the_try_body_into_the_finalizer_is_a_try_join() {
    // The try-body analog: body-last `if` -> nested `if` -> finally entry.
    let inc = incomplete();
    assert_eq!(
        labels(NESTED_BODY, &[2, 4, 6, 8, 11], 16),
        vec![
            (2, inc),
            (4, inc),
            (
                6,
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 11 })
            ),
            (8, inc),
            (11, exact())
        ]
    );
}

#[test]
fn nested_arms_without_a_finalizer_or_a_try_keep_their_labels() {
    // Residual-bearing fixtures, not clean controls: labels are pinned in full
    // (the retag does not apply: no try, or no finalizer entry).
    // No try: x@3 `if ((x = seed()))` is overwritten on every arm, so x@3->12 is
    // wrong; the nested-if exit does not enter the else arm (#796-family residual).
    assert_eq!(
        labels(NESTED_NO_TRY, &[2, 3, 5, 7, 10], 12),
        vec![
            (
                2,
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })
            ),
            (3, exact()), // wrong on main too (#796 family)
            (
                5,
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 10 })
            ),
            (7, incomplete()),
            (10, exact())
        ]
    );
    // Catch with no finally: no finalizer entry to complete into. x@6 `if ((x =
    // seed()))` is overwritten on every arm, so x@6->16 is wrong (#796 family).
    assert_eq!(
        labels(NESTED_CATCH_NO_FINALLY, &[2, 6, 8, 10, 13], 16),
        vec![
            (2, incomplete()),
            (6, exact()), // wrong on main too (#796 family)
            (
                8,
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 13 })
            ),
            (10, incomplete()),
            (13, exact())
        ]
    );
}

#[test]
fn one_line_try_header_edge_is_flagged() {
    // `try { x = a(); } catch (e) {` : x@3 -> sink@4 is only reachable through the
    // header -> catch role; exact on main.
    assert_eq!(
        labels(ONE_LINE_TRY, &[2, 3], 4),
        vec![(2, incomplete()), (3, incomplete())]
    );
}

#[test]
fn java_nested_catch_completion_is_not_retagged() {
    let java = "class A {\n  void f(boolean p) {\n    int x = 0;\n    try {\n      work();\n    } catch (Exception e) {\n      if (p) {\n        if (p) {\n          x = 1;\n        }\n      }\n    } finally {\n      cleanup();\n    }\n    sink(x);\n  }\n}\n";
    let parsed = parsed(java, Language::Java);
    let origins: Vec<String> = crate::cfg::build_cfg_edges_with_arms(&parsed)
        .into_iter()
        .map(|(_, p)| format!("{:?}", p.origin))
        .collect();
    assert!(origins.iter().all(|o| o != "JsTryJoin"), "{origins:?}");
}

const CATCH_LOOP_LAST: &str = "function f(c) {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    log(e);\n    while (c()) {\n      if ((x = seed())) {\n        x = more();\n      } else {\n        x = other();\n      }\n    }\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const CATCH_DOWHILE: &str = "function f() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    x = 1;\n    do {\n      x = 2;\n    } while (false);\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const ENTRY_COLLISION: &str = "function f(c, d) {\n  let x = 0;\n  try {\n    if (c) {\n      x = 1;\n      if (d) {\n        return;\n      }\n    }\n    sink(x); } catch (e) { handle(); }\n}\n";

#[test]
fn residual_loop_ending_a_catch_stays_exact() {
    // Residual (#693): the `while` ending the catch completes into the finalizer
    // through the loop-exit rewrite, not a then-arm completion, so it is not
    // retagged. x@8 `x = seed()` is overwritten on both arms of the body `if`
    // whenever the loop runs, so x@8->sink is wrong; the zero-iteration exit
    // (x@6 is not a def) keeps the pair alive. The same wrong `exact` exists with
    // no try on main (no-try loop x@4->x@10). #785 closes it for admitted loops.
    let got = labels(CATCH_LOOP_LAST, &[2, 8, 9, 11], 17);
    assert_eq!(
        got,
        vec![(2, incomplete()), (8, exact()), (9, exact()), (11, exact())]
    );
}

#[test]
fn residual_do_while_in_a_catch_stays_exact() {
    // Residual (#697): x@6 is overwritten by the do body (x@8) which always runs
    // once, yet x@6->sink is `exact` through the do-while completion into the
    // finalizer (the same wrong `exact` exists with no try on main: x@3->x@7).
    let got = labels(CATCH_DOWHILE, &[2, 6, 8], 13);
    assert_eq!(got, vec![(2, incomplete()), (6, exact()), (8, exact())]);
}

#[test]
fn retagging_matches_by_line_pair_so_an_entry_line_collision_is_retagged() {
    // Retagging matches (from, to) LINE PAIRS: a pair seeded by an arm completion
    // retags every copy, so a header's legitimate false edge, or a loop exit, that
    // shares the pair is retagged too. Here the inner `if (d)` header's false edge
    // goes to the line `sink(x); } catch (e) { handle(); }`, which also holds the
    // catch entry; the route stays inside the try, but x@5->sink is retagged.
    // Accepted cost (closed collision class, bounded to entry LINES).
    assert_eq!(
        labels(ENTRY_COLLISION, &[2, 5], 10),
        vec![(2, exact()), (5, incomplete())]
    );
}
