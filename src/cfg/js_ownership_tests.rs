//! #693/#697 option B, slice 1: the JS CFG ownership records and the disposition inventory.
//!
//! Every expectation is derived by hand from the source's syntax and the emitters' rules (which builder
//! draws which edge), never read back from a run. Each source has its `function` header on line 1, so a
//! line number here is a line of the source in the test. Lines are 1-based.
use super::*;
use crate::cfg::js_ownership::{Doubt, Own, Producer, Role};
use crate::cfg::{build_cfg_edges_with_arms, EdgeSink};
use crate::languages::Language;

fn parse(src: &str) -> ParsedFile {
    let parsed = ParsedFile::parse("a.js", src, Language::JavaScript).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{src}");
    parsed
}

/// The ownership of the first (outermost) owner of `src`.
fn own(src: &str) -> Ownership {
    ownership_of(&parse(src)).into_iter().next().unwrap()
}

/// Every entry of `tree` on the line pair `from -> to`, in emission order.
fn on(tree: &TreeInventory, from: usize, to: usize) -> Vec<&Entry> {
    tree.entries
        .iter()
        .filter(|e| (e.record.from_line, e.record.to_line) == (from, to))
        .collect()
}

/// The one entry of `tree` on `from -> to` drawn by `producer`.
fn one(tree: &TreeInventory, from: usize, to: usize, producer: Producer) -> &Entry {
    let hits: Vec<_> = on(tree, from, to)
        .into_iter()
        .filter(|e| e.record.producer == producer)
        .collect();
    assert_eq!(hits.len(), 1, "{from}->{to} by {producer:?}: {hits:?}");
    hits[0]
}

fn no_unauthorized_drop_or_survivor(tree: &TreeInventory) {
    for e in &tree.entries {
        assert!(!e.unauthorized_drop, "{e:?}");
        assert!(!e.surviving_bypass, "{e:?}");
    }
}

use Disposition::{Independent as I, OwnedReplaceable as O, Unresolved as U};

#[test]
fn a_loop_on_distinct_lines_has_every_record_owned_or_independent() {
    // Control for the shared-line cases: the loop's own transitions are owned, the rest independent.
    let o =
        own("function f(n) {\n  let i = 0;\n  while (i < n) {\n    i = g();\n  }\n  sink(i);\n}\n");
    assert_eq!(o.trees.len(), 1);
    let t = &o.trees[0];
    assert!(t.rewired);
    // Main's sequential edge from the body's completion past the loop: a bypass, dropped by the rewrite.
    let bypass = one(t, 4, 6, Producer::Sequential);
    assert_eq!(bypass.record.role, Role::NormalCompletion);
    assert_eq!(bypass.fate, Fate::Dropped);
    assert_eq!(bypass.disposition, O(Owned::BodyBypass));
    // The legacy back edge and exit (loop builders) are the loop's own, dropped and re-emitted.
    let back = one(t, 4, 3, Producer::LoopBack);
    assert_eq!(
        (back.fate, back.disposition),
        (Fate::Dropped, O(Owned::LoopBack))
    );
    assert_eq!(back.record.role, Role::Repetition);
    assert_eq!(
        back.record.depends_on.map(|a| a.kind),
        Some("while_statement")
    );
    let exit = one(t, 3, 6, Producer::LoopExit);
    assert_eq!(
        (exit.fate, exit.disposition),
        (Fate::Dropped, O(Owned::LoopExit))
    );
    // Entering the loop, and header -> first body statement, are independent of its transitions.
    assert_eq!(
        one(t, 2, 3, Producer::Sequential).disposition,
        I(Indep::EntryFromOutside)
    );
    assert_eq!(
        one(t, 3, 4, Producer::Sequential).disposition,
        I(Indep::LoopEntry)
    );
    // The rewrite's own edges.
    for (from, to, role) in [(4, 3, Role::Repetition), (3, 6, Role::LoopExit)] {
        let r = one(t, from, to, Producer::JsLoops);
        assert_eq!((r.fate, r.record.role), (Fate::Survived, role));
        assert_eq!(r.disposition, O(Owned::Replacement));
    }
    no_unauthorized_drop_or_survivor(t);
    assert!(o.statements.dropped.is_empty() && o.statements.covered.is_empty());
}

#[test]
fn a_statement_on_the_closing_line_leaves_a_surviving_bypass_unresolved() {
    // sol r2 P1: `4 -> 5` leaves the body's completion for `sink(i)` on the loop's closing line without
    // the header; the rewrite keeps it (5 > end line is false) and the inventory cannot own it.
    let o = own(
        "function f(n) {\n  let i = 0;\n  for (; i < n; i++) {\n    i = g();\n  } sink(i);\n}\n",
    );
    let t = &o.trees[0];
    assert!(t.rewired);
    let bypass = one(t, 4, 5, Producer::Sequential);
    assert_eq!(bypass.fate, Fate::Survived);
    assert_eq!(bypass.disposition, U(Unres::SharedBoundaryLine));
    assert_eq!(bypass.record.own, Own::Known);
    // `sink(i)` is the one statement the closing line holds: it is not a start-line duplicate.
    assert_eq!(o.statements.by_line[&5].len(), 1);
    assert_eq!(o.statements.by_line[&5][0].kind, "expression_statement");
    // The replacement exit targets that shared line too.
    let exit = one(t, 3, 5, Producer::JsLoops);
    assert_eq!(exit.disposition, U(Unres::SharedBoundaryLine));
    // The back edge is the loop's own and does not touch the shared line.
    assert_eq!(
        one(t, 4, 3, Producer::LoopBack).disposition,
        O(Owned::LoopBack)
    );
}

#[test]
fn sibling_loops_sharing_a_line_drop_each_others_edges_unauthorized() {
    // sol r2 P1b: the `while` is rewired, the `do` (its condition shares the body's line) is not, and
    // the while's drop removes the do header's edges `5 -> 6` and `5 -> 7`.
    let o = own(
        "function f(n, k) {\n  let x = 0;\n  while (n--) {\n    touch();\n  } do {\n    use(x); } while (k());\n  sink(x);\n}\n",
    );
    assert_eq!(o.trees.len(), 2);
    let (w, d) = (&o.trees[0], &o.trees[1]);
    assert_eq!((w.root.kind, w.rewired), ("while_statement", true));
    assert_eq!((d.root.kind, d.rewired), ("do_statement", false));
    for t in [w, d] {
        let seq = one(t, 5, 6, Producer::Sequential);
        assert_eq!(seq.fate, Fate::Dropped);
        assert_eq!(seq.disposition, U(Unres::SharedBoundaryLine));
        assert!(seq.unauthorized_drop, "{seq:?}");
        let exit = one(t, 5, 7, Producer::LoopExit);
        assert_eq!(exit.fate, Fate::Dropped);
        assert_eq!(exit.disposition, U(Unres::SharedBoundaryLine));
        assert!(exit.unauthorized_drop);
    }
    // The while's own edges stay owned.
    assert_eq!(
        one(w, 4, 3, Producer::LoopBack).disposition,
        O(Owned::LoopBack)
    );
}

#[test]
fn an_if_and_a_do_on_one_line_keep_the_then_end_shortcut_unresolved() {
    // opus r2 N1: `if (c) { do {` with an else. The then-end `3 -> 9` leaves the do's entry line (the
    // header is empty once the condition is its own statement) and survives the rewrite's keep list.
    let o = own(
        "function w(c) {\n  let x = 0;\n  if (c) { do {\n    g(x);\n  } while ((x = next()));\n  } else {\n    x = 2;\n  }\n  return x;\n}\n",
    );
    let t = o
        .trees
        .iter()
        .find(|t| t.root.kind == "do_statement")
        .unwrap();
    assert!(t.rewired);
    let then_end = one(t, 3, 9, Producer::ThenEnd);
    assert_eq!(then_end.fate, Fate::Survived);
    assert_eq!(then_end.disposition, U(Unres::SharedBoundaryLine));
    assert_eq!(then_end.record.role, Role::NormalCompletion);
    assert_eq!(then_end.record.from.map(|a| a.kind), Some("do_statement"));
    assert_eq!(then_end.record.own, Own::Ambiguous(Doubt::WrapperEntry));
    assert_eq!(
        then_end.record.depends_on.map(|a| a.kind),
        Some("statement_block")
    );
    // Two statements start on line 3 (the `if` and the `do`); the line node keeps the `if`.
    let line3 = &o.statements.by_line[&3];
    assert_eq!(
        line3.iter().map(|s| s.kind.as_str()).collect::<Vec<_>>(),
        ["if_statement", "do_statement"]
    );
    assert_eq!(o.statements.dropped.len(), 1);
    assert_eq!(o.statements.dropped[0].0.kind, "do_statement");
    // The if's branch entries are the `if`'s own, independent of the loop's transitions.
    assert_eq!(
        one(t, 3, 7, Producer::BranchEntry).record.role,
        Role::BranchEntry
    );
}

#[test]
fn a_statement_after_a_one_line_loop_loses_its_successor_unauthorized() {
    // opus r2 N2: `while (c()) x = f(); g(x);` on one line. The loop's drop removes `g(x)`'s edge to
    // line 3, which the loop does not own; the line holds three statements, one kept.
    let o = own("function f(c, x) {\n  while (c()) x = f(); g(x);\n  return x;\n}\n");
    let t = &o.trees[0];
    assert!(t.rewired);
    let seq = one(t, 2, 3, Producer::Sequential);
    assert_eq!(seq.fate, Fate::Dropped);
    assert_eq!(seq.disposition, U(Unres::SharedBoundaryLine));
    assert!(seq.unauthorized_drop);
    // The #700 inventory: `x = f()` and `g(x)` are dropped by start-line deduplication; the loop is kept.
    let dropped: Vec<_> = o
        .statements
        .dropped
        .iter()
        .map(|(d, k)| (d.kind.as_str(), k.kind.as_str()))
        .collect();
    assert_eq!(
        dropped,
        [
            ("expression_statement", "while_statement"),
            ("expression_statement", "while_statement")
        ]
    );
    // The rewrite's own emissions on a pair main already had are recorded, not emitted: `2 -> 2` is the
    // legacy back edge's pair (the loop builder, a self edge on one line).
    let held = on(t, 2, 2);
    let legacy = held.iter().find(|e| e.fate == Fate::Survived).unwrap();
    assert_eq!(legacy.record.producer, Producer::LoopBack);
    let suppressed: Vec<_> = held.iter().filter(|e| e.fate == Fate::Suppressed).collect();
    assert!(!suppressed.is_empty());
    for s in suppressed {
        assert_eq!(s.record.duplicate_of, Some(legacy.record.id));
        assert_eq!(s.record.producer, Producer::JsLoops);
    }
}

#[test]
fn a_labeled_do_ending_an_if_arm_is_a_wrapper_entry_then_end() {
    // sol r2 P1 #2: the then-end `4 -> 11` leaves the label's line, not the loop's completion, and the
    // rewrite never sees it (it is outside the do's lines).
    let o = own(
        "function f(c, k) {\n  let x = 0;\n  if (c) {\n    again:\n    do {\n      x = 1;\n    } while (k());\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    assert_eq!((t.root.kind, t.rewired), ("do_statement", true));
    let then_end = one(t, 4, 11, Producer::ThenEnd);
    assert_eq!(
        then_end.record.from.map(|a| a.kind),
        Some("labeled_statement")
    );
    assert_eq!(then_end.record.own, Own::Ambiguous(Doubt::WrapperEntry));
    assert_eq!(then_end.fate, Fate::Survived);
    assert_eq!(then_end.disposition, U(Unres::WrapperEntryThenEnd));
    // The if's branch entries are present and independent (the label is the consequence's first statement).
    assert_eq!(
        one(t, 3, 4, Producer::BranchEntry).disposition,
        I(Indep::BranchEntry)
    );
}

#[test]
fn a_try_ending_an_if_arm_is_a_wrapper_entry_then_end_and_legacy_edges_are_unknown() {
    let o = own(
        "function f(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        x = 1;\n      }\n    } catch (e) {\n      x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let then_end = one(t, 4, 14, Producer::ThenEnd);
    assert_eq!(then_end.record.from.map(|a| a.kind), Some("try_statement"));
    assert_eq!(then_end.disposition, U(Unres::WrapperEntryThenEnd));
    // build_try_catch_edges is not modelled: its `4 -> 9` has role Legacy, no anchors, ownership unknown,
    // and (no endpoint on the loop's lines) is in no inventory.
    let try_edge = o
        .records
        .live
        .iter()
        .find(|r| r.producer == Producer::Try)
        .expect("a try -> catch record");
    assert_eq!((try_edge.from_line, try_edge.to_line), (4, 9));
    assert_eq!(try_edge.role, Role::Legacy);
    assert_eq!(try_edge.own, Own::Unknown(Doubt::LegacyEmitter));
    assert!(try_edge.from.is_none() && try_edge.to.is_none());
    assert!(on(t, 4, 9).is_empty());
}

#[test]
fn an_unmodelled_edge_is_interior_inside_the_tree_and_a_crossing_outside_it() {
    // `try`/`finally` inside the body: both endpoints on the loop's lines -> interior.
    let inside = own(
        "function f(k) {\n  while (k()) {\n    try {\n      a();\n    } finally {\n      b();\n    }\n  }\n  c();\n}\n",
    );
    let t = &inside.trees[0];
    let e = one(t, 3, 6, Producer::Try);
    assert_eq!(
        (e.record.role, e.disposition),
        (Role::Legacy, I(Indep::InteriorLegacy))
    );
    // A `catch` whose first statement is the loop: the `try -> catch` edge enters the tree from outside.
    let crossing = own(
        "function f(k) {\n  try {\n    a();\n  } catch (e) {\n    while (k()) {\n      b();\n    }\n  }\n}\n",
    );
    let e = one(&crossing.trees[0], 2, 5, Producer::Try);
    assert_eq!(e.disposition, U(Unres::LegacyCrossing));
}

#[test]
fn a_loop_in_a_nested_callable_emits_unresolved_records_for_the_outer_owner() {
    // The outer owner's loop builders also walk the arrow's body (no callable filter): their edges are
    // drawn for the arrow's loop, and their endpoints sit on the outer loop's lines.
    let src = "function f(k) {\n  while (k()) {\n    const h = () => { while (k()) { k--; } };\n    k--;\n  }\n  return k;\n}\n";
    let owners = ownership_of(&parse(src));
    assert_eq!(owners.len(), 2, "the function and the arrow");
    let outer = &owners[0];
    assert!(outer.callable.1 - outer.callable.0 > owners[1].callable.1 - owners[1].callable.0);
    // One tree: the outer loop (the arrow's loop belongs to the arrow).
    assert_eq!(outer.trees.len(), 1);
    let t = &outer.trees[0];
    assert_eq!(
        src[..t.root.start].matches('\n').count() + 1,
        2,
        "the outer loop"
    );
    let back = one(t, 3, 3, Producer::LoopBack);
    let exit = one(t, 3, 4, Producer::LoopExit);
    for e in [back, exit] {
        assert_eq!(e.record.own, Own::Ambiguous(Doubt::NestedCallable));
        assert_eq!(e.disposition, U(Unres::NestedCallable));
    }
    // The arrow's own build draws its loop's edges for itself: not a nested-callable doubt (its one line
    // holds the loop and its body statement, so the line is shared), and the records are the arrow's.
    let inner = &owners[1];
    assert_eq!(inner.trees.len(), 1);
    let own_back = one(&inner.trees[0], 3, 3, Producer::LoopBack);
    assert_eq!(own_back.record.own, Own::Ambiguous(Doubt::SharedLine));
    assert_eq!(own_back.record.callable, inner.callable);
}

#[test]
fn duplicate_line_pairs_keep_separate_records() {
    let o = own("function f(k) {\n  if (k) {\n    k--;\n  }\n  g(k);\n}\n");
    let pairs = |from: usize, to: usize| -> Vec<Producer> {
        o.records
            .live
            .iter()
            .filter(|r| (r.from_line, r.to_line) == (from, to))
            .map(|r| r.producer)
            .collect()
    };
    // `if (k) {` -> `k--`: the sequential builder and the branch builder.
    assert_eq!(pairs(2, 3), [Producer::Sequential, Producer::BranchEntry]);
    // `k--` -> `g(k)`: the sequential builder and the then-end builder.
    assert_eq!(pairs(3, 5), [Producer::Sequential, Producer::ThenEnd]);
    let ids: std::collections::BTreeSet<_> = o.records.live.iter().map(|r| r.id).collect();
    assert_eq!(ids.len(), o.records.live.len(), "ids are unique");
}

#[test]
fn records_are_one_per_edge_in_lockstep_and_change_no_edge() {
    let srcs = [
        "function f(n, k) {\n  let x = 0;\n  while (n--) {\n    touch();\n  } do {\n    use(x); } while (k());\n  sink(x);\n}\n",
        "function f(c, k) {\n  let x = 0;\n  if (c) {\n    again:\n    do {\n      x = 1;\n    } while (k());\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
        "function f(c, x) {\n  while (c()) x = f(); g(x);\n  return x;\n}\n",
        "function f(xs) {\n  for (const x of xs) {\n    if (x) continue;\n    try { a(); } finally { b(); }\n  }\n  end();\n}\n",
    ];
    for src in srcs {
        let parsed = parse(src);
        let mut sink = EdgeSink::new();
        for owner in parsed.all_functions() {
            super::super::build_function_cfg(owner, &parsed, &mut sink);
            let rec = sink.rec.as_ref().expect("a JS owner has records");
            assert_eq!(rec.live.len(), sink.edges.len(), "{src}");
            for (r, e) in rec.live.iter().zip(&sink.edges) {
                assert_eq!((r.from_line, r.to_line), (e.from_line, e.to_line), "{src}");
            }
            sink.drain().for_each(drop);
        }
        // The same edges, origins and order with the records off.
        let on = build_cfg_edges_with_arms(&parsed);
        let off = crate::cfg::js_ownership::without_records(|| build_cfg_edges_with_arms(&parsed));
        assert_eq!(on, off, "{src}");
    }
}

#[test]
fn a_non_js_file_builds_no_records() {
    let parsed = ParsedFile::parse(
        "a.c",
        "void f(int n) {\n  while (n--) {\n    g();\n  }\n}\n",
        Language::C,
    )
    .unwrap();
    let mut sink = EdgeSink::new();
    for owner in parsed.all_functions() {
        super::super::build_function_cfg(owner, &parsed, &mut sink);
        assert!(sink.rec.is_none());
    }
}

#[test]
fn a_continue_and_a_break_are_jump_records_with_their_target() {
    let o = own(
        "function f(xs) {\n  for (const x of xs) {\n    if (x) {\n      continue;\n    }\n    if (x > 3) {\n      break;\n    }\n    use(x);\n  }\n  end();\n}\n",
    );
    let t = &o.trees[0];
    let cont = one(t, 4, 2, Producer::Jump);
    assert_eq!(cont.record.role, Role::Continue);
    assert_eq!(
        cont.record.depends_on.map(|a| a.kind),
        Some("for_in_statement")
    );
    assert_eq!(cont.disposition, O(Owned::Replacement));
    let brk = one(t, 7, 11, Producer::Jump);
    assert_eq!(brk.record.role, Role::Break);
    assert_eq!(brk.record.from.map(|a| a.kind), Some("break_statement"));
}

#[test]
fn the_700_mechanisms_are_inventoried_including_a_loop_free_control() {
    // The issue's example: `a++, sink2(..)` starts on the closing line of the shorter `if`, so RD's
    // lookup gives line 4 to the `if`'s node: the retained statement is covered, none is deduplicated.
    let o = own("function f(c, a) {\n  if (c) {\n    a = 1;\n  } a++, sink2(1, 2, 3, 4, 5, 6);\n  sink(a);\n}\n");
    assert!(o.trees.is_empty());
    assert!(o.statements.dropped.is_empty());
    assert_eq!(o.statements.covered.len(), 1);
    let (stmt, covering) = &o.statements.covered[0];
    assert_eq!((stmt.line, stmt.kind.as_str()), (4, "expression_statement"));
    assert_eq!((covering.line, covering.kind.as_str()), (2, "if_statement"));
    // Two statements on one line: the second is dropped by deduplication, nothing is covered.
    let o = own("function f(a) {\n  a = 1; sink(a);\n  return a;\n}\n");
    assert_eq!(o.statements.dropped.len(), 1);
    assert_eq!(o.statements.dropped[0].0.line, 2);
    assert!(o.statements.covered.is_empty());
    // The sequential record names the line's statements through the kept anchor.
    let seq = o
        .records
        .live
        .iter()
        .find(|r| (r.from_line, r.to_line) == (2, 3))
        .unwrap();
    assert_eq!(
        seq.from_kept,
        Some((
            o.statements.by_line[&2][0].start,
            o.statements.by_line[&2][0].end
        ))
    );
    assert_eq!(seq.own, Own::Ambiguous(Doubt::SharedLine));
}

#[test]
fn the_kept_statements_are_exactly_the_retained_universe() {
    let src = "function f(n, k) {\n  let x = 0;\n  while (n--) {\n    touch();\n  } do {\n    use(x); } while (k());\n  sink(x); sink(x);\n}\n";
    let parsed = parse(src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let o = own(src);
    let kept: Vec<_> = o
        .statements
        .by_line
        .values()
        .map(|v| (v[0].line, v[0].start, v[0].end))
        .collect();
    let retained: Vec<_> = parsed
        .statement_spans_in_function(&owner)
        .iter()
        .map(|s| (s.line, s.start_byte, s.end_byte))
        .collect();
    assert_eq!(kept, retained);
}

#[test]
fn a_switch_or_nested_if_ending_an_if_arm_is_a_wrapper_entry_then_end() {
    // switch: the then-end `4 -> 13` leaves the `switch`'s line, past the loop inside its case.
    let o = own(
        "function f(c, k, a) {\n  let x = 0;\n  if (c) {\n    switch (a) {\n      case 1:\n        while (k()) {\n          x = 1;\n        }\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let e = one(t, 4, 13, Producer::ThenEnd);
    assert_eq!(e.record.from.map(|a| a.kind), Some("switch_statement"));
    assert_eq!(
        (e.fate, e.disposition),
        (Fate::Survived, U(Unres::WrapperEntryThenEnd))
    );
    // nested if/else: the outer if's then-end leaves the inner `if`'s line (a wrapper); the inner `if`'s
    // then-end leaves the loop itself, so the loop owns it (drawn by the branch builder, delegated),
    // and the rewrite drops it.
    let o = own(
        "function f(c, d, k) {\n  let x = 0;\n  if (c) {\n    if (d) {\n      while (k()) {\n        x = 1;\n      }\n    } else {\n      x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let outer = one(t, 4, 14, Producer::ThenEnd);
    assert_eq!(outer.record.from.map(|a| a.kind), Some("if_statement"));
    assert_eq!(outer.disposition, U(Unres::WrapperEntryThenEnd));
    let inner = one(t, 5, 12, Producer::ThenEnd);
    assert_eq!(inner.record.from.map(|a| a.kind), Some("while_statement"));
    assert_eq!(inner.record.own, Own::Ambiguous(Doubt::WrapperEntry));
    assert_eq!(
        (inner.fate, inner.disposition),
        (Fate::Dropped, O(Owned::DelegatedThenEnd))
    );
    assert!(!inner.unauthorized_drop);
}

#[test]
fn sibling_loops_on_distinct_lines_own_their_own_edges_only() {
    // Control for the shared-line sibling case: loop 1's bypass `3 -> 5` is owned by loop 1 and is an
    // independent entry for loop 2.
    let o = own(
        "function f(n) {\n  while (n--) {\n    a();\n  }\n  while (n++ < 9) {\n    b();\n  }\n  c();\n}\n",
    );
    assert_eq!(o.trees.len(), 2);
    assert_eq!(
        one(&o.trees[0], 3, 5, Producer::Sequential).disposition,
        O(Owned::BodyBypass)
    );
    assert_eq!(
        one(&o.trees[1], 3, 5, Producer::Sequential).disposition,
        I(Indep::EntryFromOutside)
    );
    for t in &o.trees {
        assert!(t.rewired);
        no_unauthorized_drop_or_survivor(t);
    }
}

#[test]
fn an_if_without_else_ending_a_body_has_its_skip_and_then_end_edges_owned_by_the_loop() {
    // The `if`'s skip edge `4 -> 8` and the then-end `5 -> 8` are main's sorted-list targets past the
    // loop: the body's completions bypassing the header, whichever builder drew them. The branch entry
    // `4 -> 5` stays the `if`'s own.
    let o = own(
        "function f(n, c) {\n  let i = 0;\n  while (i < n) {\n    if (c) {\n      i = g();\n    }\n  }\n  sink(i);\n}\n",
    );
    let t = &o.trees[0];
    for (from, to, producer) in [
        (4, 8, Producer::BranchEntry),
        (5, 8, Producer::ThenEnd),
        (5, 8, Producer::Sequential),
    ] {
        let e = one(t, from, to, producer);
        assert_eq!(
            (e.fate, e.disposition),
            (Fate::Dropped, O(Owned::BodyBypass)),
            "{e:?}"
        );
    }
    assert_eq!(
        one(t, 4, 5, Producer::BranchEntry).disposition,
        I(Indep::BranchEntry)
    );
    no_unauthorized_drop_or_survivor(t);
}
