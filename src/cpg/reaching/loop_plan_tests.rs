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
    // A foreign record: the arrow shares line 4 with `x = f();`, so the outer RD selects the arrow's branch
    // edges `4 -> 6`, which leave the loop from its body line and cannot be owned by it.
    Case { name: "foreign_admitted", why: "Unresolved(ForeignAdmitted)",
        src: "function n(arr) {\n  let w = 1;\n  let x = 0;\n  while ((x = next())) {\n    x = f(); arr.forEach(y => { if (y) { if (z) g(); } });\n  }\n  sink(x, w);\n}\n",
        edges: &["w@2->w@7", "x@4->x@7", "x@3->x@5"] },
    // Wrapper then-end: the arm of `if (c)` ends in an inner `if` holding the loop; the then-end edge leaves
    // the inner `if`'s entry (L5) for L14, skipping the loop.
    Case { name: "wrapper_then_end", why: "Unresolved(WrapperEntryThenEnd)",
        src: "function f(c, xs) {\n  let s = 0;\n  if (c) {\n    s = 1;\n    if (!s) {\n      s = 2;\n      for (const a of xs) {\n        use(a);\n      }\n    }\n  } else {\n    use(s);\n  }\n  done();\n}\n",
        edges: &["c@1->c@3", "s@2->s@12", "a@7->a@8"] },
    // Non-completion crossing: the loop's last statement is an `if` whose inner `if`/`else` is drawn from its
    // entry past the loop, though the arms' completions are the loop's own.
    Case { name: "non_completion_crossing", why: "Unresolved(NonCompletionCrossing)",
        src: "function f(v, k) {\n  let x = 0;\n  let z = 1;\n  while (k()) {\n    if (v) {\n      if (x) {\n        x = 1;\n      } else {\n        x = 2;\n      }\n    }\n  }\n  sink(x, z);\n}\n",
        edges: &["z@3->z@13", "k@1->k@4", "x@2->x@13"] },
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
