//! T2 (#785): the try-flow admission boundary. Every expectation is derived by hand from the syntax (an owner that
//! contains or overlaps a `try` is refused whole; every surviving pair is `nameonly/cfg_incomplete`, never a fabricated
//! `killed`). The label regressions are RED on a4081dd2 (the merge head before T2), same-environment control in
//! `~/prism-evidence/jsts-lane/693/T2/`. A3 contains EXPLICIT try flow only: `await`, `yield` and iterator close
//! without try syntax are out of model. Each source has its `function` header on line 1.
use super::*;
use crate::cfg::js_loops::Why;
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const JS: Language = Language::JavaScript;
const CFG: &str = "nameonly/cfg_incomplete";

fn parsed(src: &str) -> ParsedFile {
    let p = ParsedFile::parse("a.js", src, JS).unwrap();
    assert_eq!(p.parse_error_count, 0, "{src}");
    p
}

fn why(src: &str) -> Vec<Option<Why>> {
    parsed(src)
        .js_loop_plan()
        .verdicts
        .iter()
        .map(|v| v.why.clone())
        .collect()
}

/// Every tree of the file is refused, each by `TryFlow` or with `try_flow` among its overlapping reasons, and its
/// owner is label-refused (O1). A tree an earlier rule already refuses keeps that primary reason.
fn all_try_flow(src: &str) -> bool {
    let p = parsed(src);
    let plan = p.js_loop_plan();
    !plan.verdicts.is_empty()
        && plan.verdicts.iter().all(|v| {
            v.why.is_some()
                && (v.why == Some(Why::TryFlow) || v.overlaps.contains(&"try_flow"))
                && plan.refuses_owner(v.owner)
        })
}

/// Every given pair is either pruned (#788) or labelled `cfg_incomplete`, and at least one exists.
fn refused_pairs(src: &str, edges: &[&str]) {
    let mut seen = 0;
    for e in edges {
        let l = labels_of(JS, src, e);
        assert!(l.iter().all(|x| x == CFG), "{e}: {l:?}");
        seen += l.len();
    }
    assert!(seen > 0, "no pair survived for {edges:?}");
}

const RET: &str = "function f(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        work();\n      }\n    } catch (e) {\n      return (x = 1);\n    } finally {\n      cleanup();\n    }\n  } else {\n    return;\n  }\n  sink(x);\n}\n";
const THR: &str = "function f(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        work();\n      }\n    } catch (e) {\n      throw (x = 1);\n    } finally {\n      cleanup();\n    }\n  } else {\n    return;\n  }\n  sink(x);\n}\n";
const LC: &str = "function lc(xs) {\n  let x = 0;\n  for (const y of xs) {\n    use(x);\n    try {\n      work();\n    } catch (e) {\n      if ((x = seed())) {\n        x = more();\n      } else {\n        x = other();\n      }\n    } finally {\n      cleanup();\n    }\n  }\n}\n";
const NOFIN: &str = "function f(c, p) {\n  let x = 0;\n  while (c()) {\n    sink(x);\n    try {\n      if (p()) {\n        x = 1;\n      } else {\n        x = 2;\n      }\n    } catch (e) {\n      continue;\n    }\n    x = 3;\n  }\n}\n";
const TC: &str = "function f(c) {\n  let x = 0;\n  if (c) {\n    try {\n      work();\n    } catch (e) {\n      if ((x = seed())) {\n        while ((x = next())) {\n          work();\n        }\n      } else {\n        x = other();\n      }\n    } finally {\n      cleanup();\n    }\n  } else {\n    return;\n  }\n  sink(x);\n}\n";
const LF: &str = "function lf() {\n  let x = 0;\n  try {\n    work();\n  } catch (e) {\n    if ((x = seed())) {\n      x = more();\n    } else {\n      x = other();\n    }\n  } finally {\n    cleanup();\n  }\n  sink(x);\n}\n";
const FIN_DEF: &str = "function f() {\n  let x = 0;\n  try {\n    return;\n  } finally {\n    x = 1;\n  }\n  sink(x);\n}\n";
const NOLOOP: &str = "function f(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      work();\n    } catch (e) {\n      return (x = 1);\n    } finally {\n      cleanup();\n    }\n  } else {\n    return;\n  }\n  sink(x);\n}\n";

#[test]
fn the_probes_with_a_loop_are_refused_whole_and_every_pair_is_cfg_incomplete() {
    // `ret`/`thr`: the pending return/throw never reaches `sink` (x@9->x@16 was `exact` on a4081dd2). `lc`: either catch arm
    // overwrites x before the next iteration (x@8->x@4 was `exact`). `nofin`: after the try, `x = 3` runs before the
    // next iteration (x@7->x@4 was `exact`; x@14->x@4 a wrong `killed`). `tc`: x@7->x@20 (slice-3 fold 1).
    for (name, src, edges) in [
        ("ret", RET, &["x@9->x@16", "x@2->x@16"][..]),
        ("thr", THR, &["x@9->x@16", "x@2->x@16"]),
        ("lc", LC, &["x@8->x@4", "x@9->x@4", "x@11->x@4", "x@2->x@4"]),
        (
            "nofin",
            NOFIN,
            &["x@7->x@4", "x@14->x@4", "x@9->x@4", "x@2->x@4"],
        ),
        ("tc", TC, &["x@7->x@20", "x@12->x@20", "x@2->x@20"]),
    ] {
        assert!(all_try_flow(src), "{name}: {:?}", why(src));
        refused_pairs(src, edges);
        for e in edges {
            assert!(
                !labels_of(JS, src, e).contains(&"exact".to_string()),
                "{name} {e}"
            );
        }
    }
}

#[test]
fn loop_free_try_owners_are_main_residuals_not_t2s() {
    // No loop tree, nothing to refuse: `lf` (x@6->x@14) and `fin_def` (x@6->x@8) are wrong `exact` on main too (every
    // path overwrites x / the pending return skips the use); T1 / #802 owns them. Pinned as the main label.
    assert!(why(LF).is_empty() && why(FIN_DEF).is_empty());
    assert_eq!(labels_of(JS, LF, "x@6->x@14"), ["exact"]);
    assert_eq!(labels_of(JS, FIN_DEF, "x@6->x@8"), ["exact"]);
    // `noloop` stays refused by the existing O1/legacy machinery of main: pairs are `cfg_incomplete`.
    assert_eq!(labels_of(JS, NOLOOP, "x@7->x@14"), [CFG]);
}

fn assert_plan(src: &str, want: Option<Why>) {
    let w = why(src);
    assert!(!w.is_empty(), "{src}");
    assert!(w.iter().all(|x| *x == want), "{want:?} vs {w:?}: {src}");
}

#[test]
fn a_try_in_a_nested_callable_that_projects_into_the_owner_refuses_it() {
    // The arrow's `try` is on L4, the line the owner's own statements `xs.forEach(..)` and `x = 1` start on: the owner's
    // selected statement line lies on a try region (rule b), though its own syntax (arrow bodies excluded) holds none.
    let src = "function f(xs, c) {\n  let x = 0;\n  while (c()) {\n    xs.forEach((a) => { try { g(a); } catch (e) {} }); x = 1;\n  }\n  sink(x);\n}\n";
    assert!(why(src).contains(&Some(Why::TryFlow)), "{:?}", why(src));
    let p = parsed(src);
    assert!(p
        .js_loop_plan()
        .verdicts
        .iter()
        .any(|v| v.first.contains("statement line")));
}

#[test]
fn a_try_in_a_nested_callable_with_no_projection_leaves_the_owner_admitted() {
    // The arrow (L6..L10) holds the try; the owner's statement lines are 2, 3, 4, 6 and 11, none on L7..L9: admitted under
    // the existing rules, loop-owner label `x@4->x@11` exact.
    let src = "function f(xs, c) {\n  let x = 0;\n  while (c()) {\n    x = 1;\n  }\n  xs.forEach((a) => {\n    try {\n      g(a);\n    } catch (e) {}\n  });\n  sink(x);\n}\n";
    assert_plan(src, None);
    assert_eq!(labels_of(JS, src, "x@4->x@11"), ["exact"]);
}

#[test]
fn a_try_under_with_label_switch_or_a_bare_block_refuses_the_owner() {
    for (what, src) in [
        ("with", "function f(c, o) {\n  let x = 0;\n  while (c()) {\n    with (o) {\n      try { g(); } catch (e) {}\n    }\n    x = 1;\n  }\n  sink(x);\n}\n"),
        ("label", "function f(c) {\n  let x = 0;\n  while (c()) {\n    blk: {\n      try { g(); } catch (e) {}\n    }\n    x = 1;\n  }\n  sink(x);\n}\n"),
        ("switch", "function f(c, k) {\n  let x = 0;\n  while (c()) {\n    switch (k) {\n      case 1:\n        try { g(); } catch (e) {}\n        break;\n    }\n    x = 1;\n  }\n  sink(x);\n}\n"),
        ("bare block", "function f(c) {\n  let x = 0;\n  while (c()) {\n    {\n      try { g(); } catch (e) {}\n    }\n    x = 1;\n  }\n  sink(x);\n}\n"),
    ] {
        assert!(all_try_flow(src), "{what}: {:?}", why(src));
    }
}

#[test]
fn a_try_in_a_class_static_block_refuses_the_owner() {
    // A static block is not on the callable boundary list: the census is its own syntax scan, not the completion walk.
    let src = "function f(c) {\n  let x = 0;\n  class K {\n    static {\n      try { g(); } catch (e) { x = 1; }\n    }\n  }\n  while (c()) {\n    x = 2;\n  }\n  sink(x);\n}\n";
    assert!(all_try_flow(src), "{:?}", why(src));
}

#[test]
fn a_try_dependent_delegated_wrapper_in_another_owner_refuses_the_projected_owner() {
    // The arrow (own try, its loop wrapped by it) shares L4 with the outer loop body's `x = 2`: the arrow's owner holds a
    // try (a), the outer owner's statement line lies on that try (b). Both refused; every surviving outer pair is
    // `cfg_incomplete`.
    let src = "function f(c, k) {\n  let x = 0;\n  while (c()) {\n    const h = () => { try { while (k()) { g(); } } catch (e) {} }; x = 2;\n  }\n  sink(x);\n}\n";
    assert!(all_try_flow(src), "{:?}", why(src));
    refused_pairs(src, &["x@2->x@6", "x@4->x@6"]);
}

#[test]
fn generator_async_for_await_and_iterator_loops_with_a_try_are_refused() {
    for (what, src) in [
        ("for await", "async function f(xs) {\n  let x = 0;\n  for await (const a of xs) {\n    try {\n      x = a;\n    } catch (e) {}\n  }\n  return x;\n}\n"),
        ("generator", "function* g(xs) {\n  let x = 0;\n  for (const a of xs) {\n    try {\n      yield a;\n    } catch (e) {}\n  }\n  return x;\n}\n"),
        ("async while", "async function f(c) {\n  let x = 0;\n  while (await c()) {\n    try {\n      x = 1;\n    } finally {}\n  }\n  return x;\n}\n"),
        ("iterator for-of", "function f(xs) {\n  let x = 0;\n  for (const a of xs) {\n    try {\n      x = a;\n    } catch (e) {}\n  }\n  return x;\n}\n"),
    ] {
        assert!(all_try_flow(src), "{what}: {:?}", why(src));
    }
}

#[test]
fn the_same_forms_without_a_try_keep_the_existing_policy() {
    // Out of model and untouched: await / yield / iterator close without try syntax. The existing policy decides.
    for (what, src) in [
        ("for await", "async function f(xs) {\n  let x = 0;\n  for await (const a of xs) {\n    x = a;\n  }\n  return x;\n}\n"),
        ("generator", "function* g(xs) {\n  let x = 0;\n  for (const a of xs) {\n    x = a;\n    yield a;\n  }\n  return x;\n}\n"),
        ("while", "function f(c) {\n  let x = 0;\n  while (c()) {\n    x = 1;\n  }\n  return x;\n}\n"),
    ] {
        let w = why(src);
        assert!(w.iter().all(|x| *x != Some(Why::TryFlow)), "{what}: {w:?}");
        assert_eq!(w, [None], "{what}");
    }
}

#[test]
fn every_tree_of_an_affected_owner_is_refused_including_legacy_ones() {
    // Two loops in one owner holding a try: the second is a `for` initializer hazard (main's own policy, Legacy). Both are
    // refused, the legacy one keeps its primary reason and records the overlap.
    let src = "function f(n, c) {\n  let a = 0;\n  try { g(); } catch (e) {}\n  while (c()) {\n    a = 1;\n  }\n  for (a = 1; a < n; n--) {\n    a = 2;\n  }\n  sink(a);\n}\n";
    let p = parsed(src);
    let v = &p.js_loop_plan().verdicts;
    assert_eq!(v.len(), 2);
    assert_eq!(v[0].why, Some(Why::TryFlow));
    assert!(matches!(v[1].why, Some(Why::Legacy(_))), "{:?}", v[1]);
    assert!(v[1].overlaps.contains(&"try_flow") && v[1].legacy_o1 == Some(Why::TryFlow));
    assert!(p.js_loop_plan().refuses_owner(v[0].owner));
    let c = p.js_loop_plan().counters();
    assert_eq!(c.get("refused.try_flow"), Some(&1), "{c:?}");
    assert_eq!(c.get("overlap.try_flow"), Some(&2), "{c:?}");
}

fn ckey(key_body: &str) -> String {
    format!(
        "function f(c) {{\n  const x = 0;\n  while (c()) {{\n    const obj = {{\n      [class {{\n        static {{\n          {key_body}\n        }}\n      }}]() {{}}\n    }};\n    sink(x);\n  }}\n}}\n"
    )
}

#[test]
fn an_eager_computed_key_is_executed_by_the_owner_even_though_the_method_is_not() {
    // sol T2r1 W. The class in the method's computed key is built when `obj` is, so its static block (and the try in
    // it) runs in the owner. RED on 66a42599: the scan skipped the whole `method_definition`, the try lines (L7) are
    // none of the owner's statement lines (2, 3, 4, 11), and `x@2->x@11` was `exact`.
    let src = ckey("try { throw 1; } finally {}");
    assert!(all_try_flow(&src), "{:?}", why(&src));
    refused_pairs(&src, &["x@2->x@11"]);
    assert!(!labels_of(JS, &src, "x@2->x@11").contains(&"exact".to_string()));
    // A try-free computed key keeps the existing policy: admitted.
    assert_plan(&ckey("g();"), None);
    assert_eq!(labels_of(JS, &ckey("g();"), "x@2->x@11"), ["exact"]);
}

#[test]
fn an_always_throwing_eager_block_without_a_try_is_a_disclosed_main_residual() {
    // The same witness with NO try: `static { throw 1; }` always throws while `obj` is built, so `sink` is never
    // reached, yet `x@2->x@11` is `exact` on main too. An exception-flow gap outside the explicit-try contract (A3),
    // pinned as such: it stays admitted and `exact`.
    let src = ckey("throw 1;");
    assert_plan(&src, None);
    assert_eq!(labels_of(JS, &src, "x@2->x@11"), ["exact"]);
}

#[test]
fn a_try_in_a_method_body_that_is_not_projected_stays_admitted() {
    // The method body is deferred (the nested owner's); its try lines (L6..L8) are none of the owner's statement lines.
    let src = "function f(c) {\n  const x = 0;\n  while (c()) {\n    const obj = {\n      m() {\n        try {\n          g();\n        } catch (e) {}\n      },\n    };\n    sink(x);\n  }\n}\n";
    assert_plan(src, None);
    assert_eq!(labels_of(JS, src, "x@2->x@11"), ["exact"]);
}

#[test]
fn a_try_in_the_owners_own_parameter_default_refuses_it() {
    // The default runs at every call of the owner: its class static block's try is the owner's. A try inside an
    // arrow in the default would be that arrow's (deferred) and is not the owner's.
    let src = "function f(c, K = class {\n  static {\n    try { throw 1; } finally {}\n  }\n}) {\n  const x = 0;\n  while (c()) {\n    sink(x);\n  }\n}\n";
    assert!(all_try_flow(src), "{:?}", why(src));
    refused_pairs(src, &["x@6->x@8"]);
}
