//! #693/#697 option B, slice 3: record-based replacement. Every expectation is derived by hand from the
//! syntax and the emitters' rules (which builder draws which edge), never read back from a run. Each source
//! has its `function` header on line 1, so a line number is a line of the source.
//!
//! The accounting tests are the slice's gate: over every fixture of the loop matrix and the plan tables,
//! the records standing as the rewrite starts are exactly the records standing after it plus the ones it
//! dropped (nothing else appears or disappears, so outside and sibling records are untouched), every
//! dropped record has an admitted tree that owns it, and that tree emitted its replacement.
use super::*;
use crate::cfg::js_loops::Why;
use crate::cfg::js_ownership::{EdgeRecord, Recorder};
use crate::cfg::{build_function_cfg, EdgeSink};
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;
use std::collections::BTreeSet;

const JS: Language = Language::JavaScript;
const MATRIX: &str = include_str!("../../../tests/fixtures/js_loops/variant-matrix.tsv");

fn parse(lang: Language, src: &str) -> ParsedFile {
    let parsed = ParsedFile::parse("a.js", src, lang).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{src}");
    parsed
}

/// The recorder of every callable of the file after its build, outermost first.
fn recorders<'p>(parsed: &'p ParsedFile) -> Vec<(Node<'p>, Recorder)> {
    let mut sink = EdgeSink::new();
    let mut out = Vec::new();
    for owner in parsed.all_functions() {
        build_function_cfg(owner, parsed, &mut sink);
        out.push((owner, sink.take_records().expect("a JS owner has records")));
        sink.drain().for_each(drop);
    }
    out
}

fn pairs(records: &[EdgeRecord]) -> BTreeSet<(usize, usize)> {
    records.iter().map(|r| (r.from_line, r.to_line)).collect()
}

fn emitted(r: &EdgeRecord) -> bool {
    matches!(r.producer, Producer::JsLoops | Producer::Jump)
}

/// The fixtures the accounting runs over: the matrix sources and the plan tables.
fn extra(name: &str) -> &'static str {
    EXTRA.iter().find(|(n, _)| *n == name).unwrap().1
}

fn corpus() -> Vec<(String, Language, String)> {
    let mut out: Vec<(String, Language, String)> = Vec::new();
    for line in MATRIX.lines().skip(1) {
        let c: Vec<&str> = line.split('\t').collect();
        if out.iter().any(|(n, _, _)| n == c[0]) {
            continue;
        }
        let lang = match c[1] {
            "ts" => Language::TypeScript,
            "tsx" => Language::Tsx,
            _ => JS,
        };
        out.push((c[0].to_string(), lang, c[8].replace("\\n", "\n")));
    }
    for c in crate::cpg::loop_plan_tests::REFUSED {
        out.push((c.name.to_string(), JS, c.src.to_string()));
    }
    for (name, src, _) in crate::cpg::loop_plan_tests::ADMITTED {
        out.push((name.to_string(), JS, src.to_string()));
    }
    for (name, src) in EXTRA {
        out.push((name.to_string(), JS, src.to_string()));
    }
    out
}

/// Slice-3 shapes beyond the tables: a wrapper arm of each kind, a nested callable's `do`, a hoisted
/// function declaration in a body, siblings that share a line.
const EXTRA: &[(&str, &str)] = &[
    ("nested_do_in_callback", "function outer2(xs) {\n  return xs.map((k) => {\n    let x = 0;\n    do {\n      use(x);\n      x = f();\n    } while ((x = g(k)));\n    return x;\n  });\n}\n"),
    ("hoisted_declaration_in_body", "function f(n) {\n  let z = 1;\n  for (let i = 0; i < n; i++) {\n    function g() { return z; }\n    z = g();\n  }\n  sink(z);\n}\n"),
    ("try_arm", "function f(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        x = 1;\n      }\n    } catch (e) {\n      x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n"),
    ("switch_arm", "function f(c, k, a) {\n  let x = 0;\n  if (c) {\n    switch (a) {\n      case 1:\n        while (k()) {\n          x = 1;\n        }\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n"),
    ("wrapper_owned_switch_break", "function f(c, k) {\n  let x = 0;\n  if (c) {\n    switch (k) {\n      case 1:\n        while (g()) {\n          x = 1;\n        }\n        break;\n      default:\n        x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n"),
    ("unrouted_labeled_break", "function f(c, k) {\n  let x = 0;\n  outer: {\n    if (c) {\n      switch (k) {\n        case 1:\n          while (g()) {\n            x = 1;\n          }\n          break outer;\n        default:\n          x = 2;\n      }\n    } else {\n      return 0;\n    }\n    sink(x);\n  }\n}\n"),
    ("two_trees_in_a_wrapper", "function g(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        x = 1;\n      }\n      while (k()) {\n        x = 2;\n      }\n    } finally {\n      h();\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n"),
    ("siblings_sharing_a_line", "function f(n, k) {\n  let x = 0;\n  while (n--) {\n    touch();\n  } do {\n    use(x); } while (k());\n  sink(x);\n}\n"),
    ("one_line_loop_then_statement", "function f(c, x) {\n  while (c()) x = f(); g(x);\n  return x;\n}\n"),
    ("arrow_in_loop_body", "function f(k) {\n  while (k()) {\n    const h = () => { while (k()) { k--; } };\n    k--;\n  }\n  return k;\n}\n"),
    ("loop_in_callback_in_loop", "function f(xs) {\n  for (const a of xs) {\n    a.forEach((b) => {\n      let y = 0;\n      do {\n        use(y);\n        y = g(b);\n      } while (y);\n    });\n  }\n}\n"),
];

#[test]
fn every_dropped_record_has_an_owner_and_every_other_record_stands() {
    let (mut dropped_total, mut nested_total, mut trees) = (0, 0, 0);
    for (name, lang, src) in corpus() {
        let parsed = parse(lang, &src);
        let boundaries = parsed.language.callable_boundary_node_types();
        for (owner, rec) in recorders(&parsed) {
            let pre: BTreeSet<usize> = rec.pre_rewrite.iter().copied().collect();
            let live: BTreeSet<usize> = rec
                .live
                .iter()
                .filter(|r| !emitted(r))
                .map(|r| r.id)
                .collect();
            let dropped: BTreeSet<usize> = rec.dropped.iter().map(|r| r.id).collect();
            assert!(live.is_disjoint(&dropped), "{name}");
            // Nothing appears but the rewrite's emissions, nothing disappears but what it dropped.
            assert_eq!(pre, &live | &dropped, "{name}: the standing records");
            let own_authority = Authority::new(&parsed, owner);
            trees += own_authority.is_some() as usize;
            for r in &rec.dropped {
                dropped_total += 1;
                let owned = if r.nested {
                    nested_total += 1;
                    nested_owner(&parsed, owner, &boundaries, r)
                        .and_then(|n| Authority::new(&parsed, n))
                        .and_then(|a| a.owns(r, true))
                } else {
                    own_authority.as_ref().and_then(|a| a.owns(r, false))
                };
                let Some((root, _)) = owned else {
                    panic!("{name}: dropped without an owner: {r:?}");
                };
                // The tree that owns a record of THIS owner emitted its replacement (a repeated pair is
                // recorded as suppressed beside the surviving record).
                if !r.nested {
                    let range = (root.start, root.end);
                    assert!(
                        rec.live
                            .iter()
                            .chain(&rec.suppressed)
                            .any(|e| emitted(e) && e.tree == Some(range)),
                        "{name}: {r:?} has no replacement of tree {range:?}"
                    );
                }
            }
        }
    }
    // The accounting is not vacuous.
    assert!(trees >= 60, "owners with an admitted tree: {trees}");
    assert!(dropped_total >= 200, "dropped records: {dropped_total}");
    assert!(nested_total >= 2, "dropped nested copies: {nested_total}");
}

#[test]
fn the_record_rewrite_changes_no_edge_of_a_loop_free_owner() {
    let src = "function f(c, xs) {\n  let s = 0;\n  if (c) {\n    s = 1;\n  } else {\n    s = 2;\n  }\n  for (const a of xs) {\n    s = a;\n  }\n  return s;\n}\nfunction g(c) {\n  if (c) {\n    return 1;\n  }\n  return 2;\n}\n";
    let parsed = parse(JS, src);
    let g = recorders(&parsed).pop().unwrap().1;
    assert!(g.dropped.is_empty() && g.live.iter().all(|r| !emitted(r)));
    // The same edges as main's loop CFG for the loop-free owner.
    let main = crate::cfg::js_loops::with_main_loop_cfg(|| {
        let parsed = parse(JS, src);
        pairs(&recorders(&parsed).pop().unwrap().1.live)
    });
    assert_eq!(pairs(&g.live), main);
}

#[test]
fn an_enclosing_owner_drops_its_copies_of_a_nested_admitted_trees_records() {
    // Hand-derived. The arrow's `do` (L4..L7) is admitted. The enclosing function's generic loop collector
    // also walks the arrow's body and draws its legacy back edge: the body's last statement (L6) to the `do`
    // (L4), a bypass of the condition. That is the arrow tree's own record, so the enclosing owner drops it.
    // (The collector's exit edge needs a later line in the ENCLOSING function's own statement universe, which
    // holds only L2 here, so there is none to drop.)
    let src = extra("nested_do_in_callback");
    let parsed = parse(JS, src);
    let all = recorders(&parsed);
    let (outer, arrow) = (&all[0].1, &all[1].1);
    let gone: BTreeSet<(usize, usize)> = outer
        .dropped
        .iter()
        .inspect(|r| assert!(r.nested, "{r:?}"))
        .map(|r| (r.from_line, r.to_line))
        .collect();
    assert_eq!(gone, BTreeSet::from([(6, 4)]));
    assert_eq!(outer.dropped[0].producer, Producer::LoopBack);
    assert!(outer
        .live
        .iter()
        .all(|r| !(r.nested && matches!(r.producer, Producer::LoopBack | Producer::LoopExit))));
    // The arrow keeps its own: the do's condition (L7) reaches the body entry (L5) and the code after (L8).
    let own: BTreeSet<_> = arrow
        .live
        .iter()
        .filter(|r| emitted(r))
        .map(|r| (r.from_line, r.to_line))
        .collect();
    assert!(own.contains(&(7, 5)) && own.contains(&(7, 8)), "{own:?}");
    assert!(!arrow
        .live
        .iter()
        .any(|r| (r.from_line, r.to_line) == (6, 4)));
    // Labels (RED on befd8add, which refused the tree as `foreign_admitted`): the condition rewrites `x`
    // every iteration, so the body's `x = f()` (L6) never reaches the next `use(x)` (L5): killed at L7,
    // while the condition's write (L7) and the initial `x = 0` (L3) reach it. The enclosing function's own
    // view of the arrow's lines is a separate (callable-crossing) label that stays `cfg_incomplete`.
    let l65 = labels_of(JS, src, "x@6->x@5");
    assert!(!l65.contains(&"exact".to_string()), "{l65:?}");
    assert!(l65.contains(&"nameonly/killed".to_string()), "{l65:?}");
    assert!(labels_of(JS, src, "x@7->x@5").contains(&"exact".to_string()));
    assert!(labels_of(JS, src, "x@3->x@5").contains(&"exact".to_string()));
    assert!(labels_of(JS, src, "x@7->x@8").contains(&"exact".to_string()));
}

#[test]
fn a_nested_tree_that_is_refused_keeps_the_enclosing_copies() {
    // The arrow's loop shares its line with its body (head collapse): its tree is refused, no tree owns the
    // enclosing owner's copies of its legacy edges, and they stay.
    let parsed = parse(JS, extra("arrow_in_loop_body"));
    let all = recorders(&parsed);
    // The enclosing owner's own loop is admitted and drops its own records; it drops no nested copy.
    assert!(
        all[0].1.dropped.iter().all(|r| !r.nested),
        "{:?}",
        all[0].1.dropped
    );
    assert!(all[0]
        .1
        .live
        .iter()
        .any(|r| r.nested && r.producer == Producer::LoopBack));
}

#[test]
fn a_hoisted_function_declaration_in_a_body_is_not_a_body_occurrence() {
    // RED on befd8add: the declaration's line (L4) holds no statement of the universe, so its row resolved to
    // the head (L3) and the tree was refused as a placement violation. It executes nothing where it stands.
    let src = extra("hoisted_declaration_in_body");
    assert_eq!(parse(JS, src).js_loop_plan().verdicts[0].why, None);
    // Hand-derived: `z` is written once per iteration (L5) and read after the loop (L7); the loop may not run,
    // so the initial `z = 1` (L2) also reaches L7: both `exact`.
    assert_eq!(labels_of(JS, src, "z@2->z@7"), ["exact"]);
    assert_eq!(labels_of(JS, src, "z@5->z@7"), ["exact"]);
    // A declaration does not hide a real violation: a body statement on the head line still collapses it.
    let bad = "function f(c) {\n  let z = 1;\n  while (c()) { z = g();\n    function h() { return z; }\n  }\n  sink(z);\n}\n";
    assert!(matches!(
        parse(JS, bad).js_loop_plan().verdicts[0].why,
        Some(Why::Unresolved(_)) | Some(Why::Placement)
    ));
    // An arrow EXPRESSION on its own line stays an occurrence (it is evaluated there): control.
    let expr = "function f(n) {\n  let z = 1;\n  for (let i = 0; i < n; i++) {\n    const g = function () { return z; };\n    z = g();\n  }\n  sink(z);\n}\n";
    assert_eq!(parse(JS, expr).js_loop_plan().verdicts[0].why, None);
}

#[test]
fn a_wrapper_arm_is_replaced_by_its_completions_and_an_unrouted_one_is_refused() {
    // try / switch arms: delegated (see `js_ownership_tests`); labels: after the arm `x` is 1 or 2 or 0.
    for name in ["try_arm", "switch_arm"] {
        let src = EXTRA.iter().find(|(n, _)| *n == name).unwrap().1;
        assert_eq!(
            parse(JS, src).js_loop_plan().verdicts[0].why,
            None,
            "{name}"
        );
    }
    // A `break` of the switch inside the wrapper: the wrapper emits its edge, so the tree is admitted.
    let owned = EXTRA
        .iter()
        .find(|(n, _)| *n == "wrapper_owned_switch_break")
        .unwrap()
        .1;
    assert_eq!(parse(JS, owned).js_loop_plan().verdicts[0].why, None);
    // A labeled `break` out of the wrapper to a block outside it has no edge and no owner: refused.
    let unrouted = EXTRA
        .iter()
        .find(|(n, _)| *n == "unrouted_labeled_break")
        .unwrap()
        .1;
    assert_eq!(
        parse(JS, unrouted).js_loop_plan().verdicts[0].why,
        Some(Why::Unresolved(
            crate::cfg::js_disposition::Unres::WrapperEntryThenEnd
        ))
    );
    // Two trees in one wrapper: the first claims the record, both are admitted.
    let two = EXTRA
        .iter()
        .find(|(n, _)| *n == "two_trees_in_a_wrapper")
        .unwrap()
        .1;
    let p = parse(JS, two).js_loop_plan().clone();
    assert!(
        p.verdicts.iter().all(|v| v.why.is_none()),
        "{:?}",
        p.verdicts
    );
}
