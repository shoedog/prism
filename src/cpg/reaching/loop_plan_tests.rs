//! #693/#697 option B, slice 2: what the loop-tree plan does to labels and the CFG, and how its foreign-record
//! admission matches the real consumer. Expectations are written by hand from the layout rules (the placement
//! rule, the disposition kinds, option O1), and use only APIs that exist before the plan, so each refusal test
//! is RED on 874ce2c3 (where these trees are rewired and their independent edges stay `exact`).
//!
//! Each source has its `function` header on line 1, so a line number is a line of the source.
//! `z`/`w` are defined before the loop and read after it, never written in between: independent of every loop
//! transition, so `exact` whenever the owner is not refused, and `nameonly/cfg_incomplete` under O1.
use super::*;
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;
use std::collections::BTreeSet;

const JS: Language = Language::JavaScript;
const CFG: &str = "nameonly/cfg_incomplete";

pub(crate) struct Case {
    pub(crate) name: &'static str,
    /// Why the plan refuses the tree (asserted in `cfg::js_loops::plan_tests`).
    pub(crate) why: &'static str,
    pub(crate) src: &'static str,
    /// Edges of the owner: the first is independent of the loop (`exact` on 874ce2c3), all are refused by O1.
    pub(crate) edges: &'static [&'static str],
}

#[rustfmt::skip]
pub(crate) const REFUSED: &[Case] = &[
    // Opus N1: an assignment update on a header continuation line (L6) is a statement of its own, entered from
    // the header: the update would run before the first body. Placement.
    Case { name: "n1_update_on_continuation_line", why: "Placement",
        src: "function f(n) {\n  let z = 1;\n  let x = 0;\n  for (let i = 0;\n       i < n;\n       x = i++) {\n    x = g(i);\n  }\n  sink(x, z);\n}\n",
        edges: &["z@2->z@9", "x@7->x@9", "x@3->x@9"] },
    // Prettier layout: the initializer on its own line (L4) is a statement of its own. Placement.
    Case { name: "prettier_initializer_line", why: "Placement",
        src: "function f(n) {\n  let z = 1;\n  for (\n    let k = n;\n    k > 0;\n    k = k - 1\n  ) {\n    use(k);\n  }\n  sink(z);\n}\n",
        edges: &["z@2->z@10", "k@4->k@8", "k@6->k@8"] },
    // sol: the condition's write sits on a continuation line (L5) that the body statement `work();` owns.
    Case { name: "sol_condition_on_body_line", why: "Placement",
        src: "function f() {\n  let z = 1;\n  let x = 0;\n  while (\n    (x = next())) work();\n  sink(x, z);\n}\n",
        edges: &["z@2->z@6", "x@3->x@6", "x@5->x@6"] },
    Case { name: "sol_condition_on_body_line_braced", why: "Placement",
        src: "function f() {\n  let z = 1;\n  let x = 0;\n  while (\n    (x = next())) { work(); }\n  sink(x, z);\n}\n",
        edges: &["z@2->z@6", "x@3->x@6", "x@5->x@6"] },
    // Head collapse: the body statement starts on the head line, so test and body share one node.
    Case { name: "head_collapse", why: "Unresolved(HeadCollapse)",
        src: "function f(c) {\n  let z = 1;\n  while (c()) x = g();\n  sink(x, z);\n}\n",
        edges: &["z@2->z@4", "c@1->c@3", "x@3->x@4"] },
    // (Merge of #788: the former pair `x@3->x@5` is a dominated def into a write-only line, which rule D/U prunes
    // on main too; the fixture asserts pairs that survive it: reads of `x`, `w` and `arr`.)
    // A foreign record: the arrow shares line 4 with `x = f();`, so the outer RD selects the arrow's branch
    // edges `4 -> 6`, which leave the loop from its body line and cannot be owned by it.
    Case { name: "foreign_admitted", why: "Unresolved(ForeignAdmitted)",
        src: "function n(arr) {\n  let w = 1;\n  let x = 0;\n  while ((x = next())) {\n    x = f(); arr.forEach(y => { if (y) { if (z) g(); } });\n  }\n  sink(x, w);\n}\n",
        edges: &["w@2->w@7", "x@4->x@7", "x@3->x@7", "x@5->x@7", "arr@1->arr.forEach@5"] },
    // A callback's loop is refused (head collapse); the enclosing function holds no loop tree of its own, so
    // O1 refuses the arrow's labels only (`cfg::js_loops::plan_tests::only_the_owner_of_a_refused_tree...`).
    Case { name: "callback_head_collapse", why: "Unresolved(HeadCollapse)",
        src: "function outer(items) {\n  let w = 1;\n  items.forEach(it => {\n    while (it()) x = g();\n    use(x);\n  });\n  sink(w);\n}\n",
        edges: &["it@3->it@4"] },
];

#[rustfmt::skip]
pub(crate) const ADMITTED: &[(&str, &str, &[&str])] = &[
    // A multi-line header whose continuation lines hold no statement of their own, a header declaration
    // (`for (let i = 0;`) and a body on its own lines.
    ("ctl_multiline_header", "function f(n) {\n  let z = 1;\n  let x = 0;\n  for (let i = 0;\n       i < n;\n       i++) {\n    x = g(i);\n  }\n  sink(x, z);\n}\n",
        &["z@2->z@9", "x@3->x@9", "x@7->x@9"]),
    ("ctl_while_body_on_its_own_line", "function f(c) {\n  let z = 1;\n  let x = 0;\n  while (c()) {\n    x = g();\n  }\n  sink(x, z);\n}\n",
        &["z@2->z@7", "x@5->x@7", "c@1->c@4"]),
    ("ctl_do_condition_on_its_own_line", "function f(a) {\n  let z = 1;\n  do {\n    a = g();\n  } while (a-- > 0);\n  sink(a, z);\n}\n",
        &["z@2->z@6", "a@4->a@5"]),
    // Slice 3: wrapper then-end. The arm of `if (c)` ends in an inner `if` holding the loop; main's then-end
    // edge leaves the inner `if`'s entry (L5) for L14, skipping the loop. The wrapper's completions (its skip
    // edge L5, the loop's exit L7) are the tree's to emit, so the then-end is replaced, not refused.
    ("wrapper_then_end", "function f(c, xs) {\n  let s = 0;\n  if (c) {\n    s = 1;\n    if (!s) {\n      s = 2;\n      for (const a of xs) {\n        use(a);\n      }\n    }\n  } else {\n    use(s);\n  }\n  done();\n}\n",
        &["c@1->c@3", "s@2->s@12", "a@7->a@8"]),
    // Slice 3: the loop's last statement is an `if` whose inner `if`/`else` is drawn from its entry past the
    // loop, though the arms' completions are the loop's own: the loop owns that record.
    ("non_completion_crossing", "function f(v, k) {\n  let x = 0;\n  let z = 1;\n  while (k()) {\n    if (v) {\n      if (x) {\n        x = 1;\n      } else {\n        x = 2;\n      }\n    }\n  }\n  sink(x, z);\n}\n",
        &["z@3->z@13", "k@1->k@4", "x@2->x@13"]),
    // Slice 3 (the foreign rule): a loop in a callback. The ENCLOSING function's generic collectors drew the
    // loop's legacy back edge `5 -> 4` too, which the arrow's own rewrite could not drop; the enclosing owner
    // now drops its copies of what the arrow's admitted tree owns.
    ("foreign_from_enclosing_callable", "function outer(items) {\n  let w = 1;\n  items.forEach(it => {\n    for (const k of it) {\n      if (k) {\n        use(k);\n      } else {\n        drop(k);\n      }\n    }\n  });\n  sink(w);\n}\n",
        &["it@3->it@4"]),
];

fn edges(src: &str) -> BTreeSet<(usize, usize)> {
    let parsed = ParsedFile::parse("a.js", src, JS).unwrap();
    crate::cfg::build_cfg_edges(&parsed)
        .into_iter()
        .map(|e| (e.from_line, e.to_line))
        .collect()
}

fn universe(src: &str) -> Vec<(usize, String)> {
    let parsed = ParsedFile::parse("a.js", src, JS).unwrap();
    let owner = parsed.all_functions().into_iter().next().unwrap();
    parsed.statements_in_function(&owner)
}

#[test]
fn a_refused_tree_labels_every_edge_of_its_owner_cfg_incomplete() {
    for c in REFUSED {
        for e in c.edges {
            assert_eq!(labels_of(JS, c.src, e), [CFG], "{} {e}", c.name);
        }
    }
}

#[test]
fn a_refused_tree_keeps_mains_edges_and_statement_universe() {
    for c in REFUSED {
        let main_edges = crate::cfg::js_loops::with_main_loop_cfg(|| edges(c.src));
        assert_eq!(edges(c.src), main_edges, "{}", c.name);
        let main_universe = crate::cfg::js_loops::with_main_loop_cfg(|| universe(c.src));
        assert_eq!(universe(c.src), main_universe, "{}", c.name);
    }
}

#[test]
fn an_admitted_tree_keeps_its_rewrite_and_its_exact_labels() {
    for (name, src, edges_) in ADMITTED {
        for e in *edges_ {
            assert_eq!(labels_of(JS, src, e), ["exact"], "{name} {e}");
        }
        let main_edges = crate::cfg::js_loops::with_main_loop_cfg(|| edges(src));
        assert_ne!(edges(src), main_edges, "{name}: rewired");
    }
}

#[test]
fn a_do_condition_in_a_refused_tree_is_not_a_statement() {
    // The outer loop's body starts on its head line (head collapse), so the whole tree is refused; the inner
    // `do` is geometrically admissible (its condition is past every body statement) but must not be admitted:
    // the universe holds no `do_condition` and the CFG is main's.
    let src = "function f(c) {\n  let z = 1;\n  while (c()) { do {\n    x = g();\n  } while (c()); }\n  sink(x, z);\n}\n";
    assert!(universe(src).iter().all(|(_, k)| k != "do_condition"));
    let main_edges = crate::cfg::js_loops::with_main_loop_cfg(|| edges(src));
    assert_eq!(edges(src), main_edges);
    assert_eq!(labels_of(JS, src, "z@2->z@6"), [CFG]);
    // The same `do` alone is admitted: its condition (L5) is a statement of the universe.
    let alone =
        "function f(c) {\n  let z = 1;\n  do {\n    x = g();\n  } while (c());\n  sink(x, z);\n}\n";
    assert!(universe(alone)
        .iter()
        .any(|(l, k)| *l == 5 && k == "do_condition"));
}

/// The foreign-admission fact is the real consumer's: `FileCfgEdgeIndex::for_owner` selects an edge for an owner
/// exactly when both endpoint lines are statements of its universe, whichever callable's build drew it.
#[test]
fn the_admitted_fact_matches_what_the_owners_rd_selects() {
    let sources = [
        // An arrow sharing line 3 with an outer statement.
        "function f(k) {\n  while (k()) {\n    const h = () => { while (k()) { k--; } };\n    k--;\n  }\n  return k;\n}\n",
        // Arrow callbacks sharing a line with the loop body statement.
        "function n(arr) {\n  let x = 0;\n  while ((x = next())) {\n    x = f(); arr.forEach(y => { if (y) { if (z) g(); } });\n  }\n  sink(x);\n}\n",
        // Two methods overlapping on line 1.
        "class K { m(k) { while (k()) { k--; } } n(k) { while (k()) { k--; } } }\n",
    ];
    let mut some_foreign = false;
    for src in sources {
        let parsed = ParsedFile::parse("a.js", src, JS).unwrap();
        let file_edges = crate::cfg::js_disposition::tentative_file_edges(&parsed);
        let index = FileCfgEdgeIndex::new(&file_edges);
        let owners = parsed.all_functions();
        let own = crate::cfg::js_disposition::ownership_of(&parsed);
        assert_eq!(own.len(), owners.len(), "{src}");
        for (node, o) in owners.iter().zip(&own) {
            let line_index: BTreeMap<usize, usize> = parsed
                .statements_in_function(node)
                .iter()
                .enumerate()
                .map(|(i, (l, _))| (*l, i + 1))
                .collect();
            let selected: BTreeSet<(usize, usize)> = index
                .for_owner(&line_index)
                .iter()
                .map(|(_, _, e, _)| (e.from_line, e.to_line))
                .collect();
            let live = |e: &&crate::cfg::js_disposition::Entry| {
                e.fate == crate::cfg::js_disposition::Fate::Survived
            };
            for e in o.trees.iter().flat_map(|t| &t.entries).filter(live) {
                let pair = (e.record.from_line, e.record.to_line);
                assert_eq!(e.admitted, selected.contains(&pair), "{src}: {e:?}");
                some_foreign |= e.record.nested && e.admitted;
            }
        }
    }
    assert!(
        some_foreign,
        "the fixtures exercise a foreign admitted record"
    );
}

/// Fold 2 (Opus W1): a tree main's own policy refuses (`Legacy`) keeps main's edges, but the plan classifies
/// it under the final admission too, and an unresolved record or a failed placement refuses its owner by
/// label. Each is the wrapper-then-end shape of `wrapper_then_end` made legacy: a labelled `do` ends an `if`
/// arm (`else return`), so the `do` always runs its body once and `x = 1` kills `x = 0`; main's then-end edge
/// from the label line past the loop is a bypass of it, a wrong `exact` for `x@2 -> x@12`.
pub(crate) const LEGACY_REFUSED: &[(&str, &str, &str)] = &[
    // The loop is alone in its arm: unresolved record only, a jump through a `finally` is main's own refusal.
    ("legacy_finally", "Legacy(Finally)",
        "function lw2(c, k) {\n  let x = 0;\n  if (c) {\n    again:\n    do {\n      x = 1;\n      try { if (k()) break; } finally { g(); }\n    } while (k());\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n"),
    // `debugger;` has no completion rule: main's `Unrouted`.
    ("legacy_unrouted", "Legacy(Unrouted)",
        "function lw3(c, k) {\n  let x = 0;\n  if (c) {\n    again:\n    do {\n      x = 1;\n      debugger;\n    } while (k());\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n"),
];

#[test]
fn a_legacy_refused_tree_with_an_unresolved_record_refuses_its_owner() {
    for (name, why, src) in LEGACY_REFUSED {
        let parsed = ParsedFile::parse("a.js", src, JS).unwrap();
        let p = parsed.js_loop_plan();
        let v = &p.verdicts[0];
        assert_eq!(format!("{:?}", v.why.as_ref().unwrap()), *why, "{name}");
        assert!(
            v.legacy_o1.is_some(),
            "{name}: classified under the final admission"
        );
        assert!(p.refuses_owner(v.owner), "{name}");
        // Main's edges stand (no rewrite), the label is the owner-wide refusal.
        let main = crate::cfg::js_loops::with_main_loop_cfg(|| edges(src));
        assert_eq!(edges(src), main, "{name}");
        assert_eq!(labels_of(JS, src, "x@2->x@12"), [CFG], "{name}");
    }
}

/// Fold 3 (Opus W1b): a tree main's own policy refuses keeps main's bypass and exit edges whatever its records
/// say, so its owner is label-refused: no `exact` from a tree that is not rewired. Probe `r2/lg`: the header
/// of `fi` writes `i` through the update, `fin`'s `break` leaves before `x = f()`; main's bypass edge made both
/// a wrong `exact`. `ctl` (no legacy form) is admitted and keeps its truthful `killed`.
#[test]
fn a_legacy_refused_tree_with_resolved_records_refuses_its_owner() {
    let cases = [
        ("fi", "Legacy(ForInit)", "function fi(n) {\n  let x = 0, i = 0;\n  for (x = 0; i < n; i++) {\n    x = f();\n    i = g();\n  }\n  sink(i);\n}\n", "i@5->i@7"),
        ("fin", "Legacy(Finally)", "function fin(c) {\n  let x = 0;\n  while ((x = next())) {\n    try { if (c()) break; } finally { h(); }\n    x = f();\n  }\n  sink(x);\n}\n", "x@5->x@7"),
        // A `for` initializer hazard in a plain layout: every record is owned or independent.
        ("h", "Legacy(ForInit)", "function h(n) {\n  let z = 1;\n  let a = 0;\n  for (a = 1; a < n; n--) {\n    a = 2;\n  }\n  sink(a, z);\n}\n", "z@2->z@7"),
    ];
    for (name, why, src, edge) in cases {
        let parsed = ParsedFile::parse("a.js", src, JS).unwrap();
        let p = parsed.js_loop_plan();
        assert_eq!(
            format!("{:?}", p.verdicts[0].why.as_ref().unwrap()),
            why,
            "{name}"
        );
        assert!(p.refuses_owner(p.verdicts[0].owner), "{name}");
        assert_eq!(labels_of(JS, src, edge), [CFG], "{name} {edge}");
    }
    let ctl = "function ctl() {\n  let x = 0;\n  while ((x = next())) {\n    x = f();\n  }\n  sink(x);\n}\n";
    assert_eq!(labels_of(JS, ctl, "x@4->x@6"), ["nameonly/killed"]);
}

/// The one-line shape of the original `a20` (a `for` initializer write inside a `do` condition on the `do`'s
/// own line): a legacy tree, so its owner is refused.
#[test]
fn a_for_init_inside_a_do_on_one_line_is_a_refused_row() {
    let src = "function a20(n) {\n  let z = 1, a = 0;\n  do { for (a = 10; false;) { a = 1; } } while (n--);\n  sink(a, z);\n}\n";
    let parsed = ParsedFile::parse("a.js", src, JS).unwrap();
    let p = parsed.js_loop_plan();
    assert!(p.refuses_owner(p.verdicts[0].owner));
    assert_eq!(labels_of(JS, src, "z@2->z@4"), [CFG]);
}

/// Fold 3 (sol W): availability is decided on the owner's all-admitted universe at both seams. The refused
/// outer-while/inner-do tree of `a_do_condition_in_a_refused_tree_is_not_a_statement`, padded with separate
/// statements: the standing universe is `n` lines and the all-admitted one holds the inner `do` condition too,
/// `n + 1`. With the standing universe at exactly `RD_MAX_LINES` the owner is over the cap, as on 874ce2c3.
fn padded_refused(standing_lines: usize) -> (String, usize) {
    let src = |pad: usize| {
        format!(
            "function f(c) {{\n  let z = 1;\n{}  while (c()) {{ do {{\n    x = g();\n  }} while (c()); }}\n  sink(x, z);\n}}\n",
            "  log();\n".repeat(pad)
        )
    };
    let base = universe(&src(0)).len();
    let pad = standing_lines - base;
    let source = src(pad);
    assert_eq!(universe(&source).len(), standing_lines);
    (source, pad)
}

fn both_seams(source: &str) -> (Option<RdUnavailable>, Option<RdUnavailable>) {
    let parsed = ParsedFile::parse("a.js", source, JS).unwrap();
    let cfg = crate::cfg::build_cfg_edges_with_arms(&parsed);
    let index = FileCfgEdgeIndex::new(&cfg);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    assert!(parsed
        .js_loop_plan()
        .refuses_owner((owner.start_byte(), owner.end_byte())));
    let solver = match reaching_definitions_with_exact(&parsed, &owner, &[], &[], &[], &index) {
        RdOutcome::Available(_) => None,
        RdOutcome::Unavailable(reason) => Some(reason),
    };
    (
        solver,
        unavailable_before_edges(&parsed, &owner, &[], &index),
    )
}

#[test]
fn the_line_cap_of_a_refused_owner_is_decided_on_its_all_admitted_universe() {
    let (source, _) = padded_refused(RD_MAX_LINES);
    let over = Some(RdUnavailable::StatementLinesCapExceeded {
        actual: RD_MAX_LINES + 1,
    });
    assert_eq!(both_seams(&source), (over, over));
}

#[test]
fn a_refused_owner_whose_all_admitted_universe_fits_the_cap_stays_available() {
    let (source, _) = padded_refused(RD_MAX_LINES - 1);
    assert_eq!(both_seams(&source), (None, None));
}
