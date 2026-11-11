//! T2 (#785): the try-flow admission boundary. A refused owner (one that contains or overlaps a `try`) keeps EXACTLY
//! main's geometry and labels (final refusal policy): every label asserted for a refused owner below is MAIN
//! 8cc4985d's, taken from that binary's output (`O1final/out/tryflow-main-labels.json`); a wrong `exact` main has
//! there (`ckey`, `pdec`, ...) is pinned as main's open residual. The plan verdicts are the T2 contract. History: the
//! interim O1 forced `cfg_incomplete`; see `~/prism-evidence/jsts-lane/693/T2/`. A3 contains EXPLICIT try flow only: `await`, `yield` and iterator close
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

/// Every `(pair, label)` equals main's label (the owner is refused, so it keeps main's labels).
fn main_pairs(src: &str, pairs: &[(&str, &str)]) {
    for (e, want) in pairs {
        assert_eq!(labels_of(JS, src, e), [*want], "{e}");
    }
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
fn the_probes_with_a_loop_are_refused_whole_and_keep_mains_labels() {
    // Plan: every tree refused (TryFlow). Labels: main's (8cc4985d binary). #803 made main flag the try joins, so each
    // surviving try-crossing pair is `cfg_incomplete` on main already (ret/thr x@9->16, lc x@8->4, nofin x@7->4 and
    // x@14->4, tc x@7->20); the pairs that do not cross the try stay `exact`.
    for (name, src, pairs) in [
        ("ret", RET, &[("x@9->x@16", CFG), ("x@2->x@16", CFG)][..]),
        ("thr", THR, &[("x@9->x@16", CFG), ("x@2->x@16", CFG)]),
        (
            "lc",
            LC,
            &[
                ("x@8->x@4", CFG),
                ("x@9->x@4", CFG),
                ("x@11->x@4", CFG),
                ("x@2->x@4", "exact"),
            ],
        ),
        (
            "nofin",
            NOFIN,
            &[
                ("x@7->x@4", CFG),
                ("x@14->x@4", CFG),
                ("x@9->x@4", CFG),
                ("x@2->x@4", "exact"),
            ],
        ),
        (
            "tc",
            TC,
            &[("x@7->x@20", CFG), ("x@12->x@20", CFG), ("x@2->x@20", CFG)],
        ),
    ] {
        assert!(all_try_flow(src), "{name}: {:?}", why(src));
        main_pairs(src, pairs);
    }
}

#[test]
fn loop_free_try_owners_are_main_residuals_not_t2s() {
    // No loop tree, nothing to refuse: `lf` (x@6->x@14) and `fin_def` (x@6->x@8) are wrong `exact` on main too (every
    // path overwrites x / the pending return skips the use); T1 / #802 owns them. Pinned as the main label.
    assert!(why(LF).is_empty() && why(FIN_DEF).is_empty());
    // `lf` x@6->x@14 is `cfg_incomplete` on main since #803 (T1 closed it); `fin_def` x@6->x@8 is still a wrong `exact`.
    assert_eq!(labels_of(JS, LF, "x@6->x@14"), [CFG]);
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
    main_pairs(src, &[("x@2->x@6", "exact"), ("x@4->x@6", "exact")]);
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
    // Final policy: the owner keeps main's labels, and main's `x@2->x@11` is `exact` (the static block throws): main's
    // open residual, not a T2 regression any more.
    main_pairs(&src, &[("x@2->x@11", "exact")]);
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
    main_pairs(src, &[("x@6->x@8", "exact")]);
}

const TS: Language = Language::TypeScript;

fn why_ts(src: &str) -> Vec<Option<Why>> {
    let p = ParsedFile::parse("a.ts", src, TS).unwrap();
    assert_eq!(p.parse_error_count, 0, "{src}");
    p.js_loop_plan()
        .verdicts
        .iter()
        .map(|v| v.why.clone())
        .collect()
}

fn pdec(body: &str) -> String {
    format!(
        "function f(c, dec) {{\n  const x = 0;\n  class C {{\n    m(@dec(class {{\n      static {{\n        {body}\n      }}\n    }}) p: unknown) {{}}\n  }}\n  while (c()) {{\n    sink(x);\n  }}\n}}\n"
    )
}

#[test]
fn a_parameter_decorator_of_a_nested_method_is_eager() {
    // sol T2r2 W. The decorator on `m`'s parameter runs when the class is defined, in the owner: a try in the class
    // static block inside it refuses the owner (probe `pdec.ts`). RED on 62645bff: only direct decorators of a nested
    // callable were visited, the parameter decorator sits under `formal_parameters`, `x@2->x@11` was `exact`.
    let src = pdec("try { throw 1; } finally {}");
    assert_eq!(why_ts(&src), [Some(Why::TryFlow)]);
    // main's label (a wrong `exact`, the static block throws) stands in the refused owner.
    assert_eq!(labels_of(TS, &src, "x@2->x@11"), ["exact"]);
    // The throw-only variant has no try: a disclosed main residual (alongside `ckey_notry`), stays admitted and exact.
    let no_try = pdec("throw 1;");
    assert_eq!(why_ts(&no_try), [None]);
    assert_eq!(labels_of(TS, &no_try, "x@2->x@11"), ["exact"]);
}

#[test]
fn a_decorator_on_a_method_with_a_deferred_body_try_stays_admitted() {
    // `@dec` on `m` is eager but holds no try; the try is in the deferred body (the nested owner's, not projected).
    let src = "function f(c, dec) {\n  const x = 0;\n  class C {\n    @dec\n    m() {\n      try {\n        g();\n      } catch (e) {}\n    }\n  }\n  while (c()) {\n    sink(x);\n  }\n}\n";
    assert_eq!(why_ts(src), [None]);
    assert_eq!(labels_of(TS, src, "x@2->x@12"), ["exact"]);
}

#[test]
fn instance_field_initializers_are_deferred_and_static_ones_are_eager() {
    // sol T2r2 (immaterial): an instance field's initializer runs per instance, not when the owner's default is
    // evaluated. Probe `instfield.js`: admitted again, `x@6->x@8` exact (RED on 62645bff: refused).
    let inst = "function f(c, K = class {\n  field = class { static {\n    try {} finally {}\n  }};\n}) {\n  const x = 0;\n  while (c()) {\n    sink(x);\n  }\n}\n";
    assert_plan(inst, None);
    assert_eq!(labels_of(JS, inst, "x@6->x@8"), ["exact"]);
    // A STATIC field initializer runs when the class is built: refused.
    let stat = "function f(c, K = class {\n  static field = class { static {\n    try {} finally {}\n  }};\n}) {\n  const x = 0;\n  while (c()) {\n    sink(x);\n  }\n}\n";
    assert!(all_try_flow(stat), "{:?}", why(stat));
    main_pairs(stat, &[("x@6->x@8", "exact")]);
    // A computed key of an instance field is eager.
    let key = "function f(c, K = class {\n  [class { static {\n    try {} finally {}\n  }}] = 1;\n}) {\n  const x = 0;\n  while (c()) {\n    sink(x);\n  }\n}\n";
    assert!(all_try_flow(key), "{:?}", why(key));
}
