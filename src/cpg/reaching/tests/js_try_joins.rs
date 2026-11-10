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
