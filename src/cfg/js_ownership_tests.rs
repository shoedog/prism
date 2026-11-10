//! #693/#697 option B, slice 1: the JS CFG ownership records and the disposition inventory.
//!
//! Every expectation is derived by hand from the source's syntax and the emitters' rules (which builder
//! draws which edge), never read back from a run. Each source has its `function` header on line 1, so a
//! line number here is a line of the source in the test. Lines are 1-based.
use super::*;
use crate::cfg::js_loops::Why;
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

/// The one SURVIVING entry of `tree` on `from -> to` drawn by `producer` (a rewrite emission that repeats a
/// pair is recorded as suppressed beside it).
fn survivor(tree: &TreeInventory, from: usize, to: usize, producer: Producer) -> &Entry {
    let hits: Vec<_> = on(tree, from, to)
        .into_iter()
        .filter(|e| e.record.producer == producer && e.fate == Fate::Survived)
        .collect();
    assert_eq!(hits.len(), 1, "{from}->{to} by {producer:?}: {hits:?}");
    hits[0]
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

/// The disposition as text, so a test can name a class without the enum having to know it.
fn d(e: &Entry) -> String {
    format!("{:?}", e.disposition)
}

fn own_ts(src: &str) -> Vec<Ownership> {
    let parsed = ParsedFile::parse("a.ts", src, Language::TypeScript).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{src}");
    ownership_of(&parsed)
}

/// Every entry of `tree` that touches `line`.
fn touching(tree: &TreeInventory, line: usize) -> Vec<&Entry> {
    tree.entries
        .iter()
        .filter(|e| e.record.from_line == line || e.record.to_line == line)
        .collect()
}

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
    assert!(o.statements.dropped.is_empty() && o.statements.covered().is_empty());
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
fn sibling_loops_sharing_a_line_keep_each_others_edges() {
    // sol r2 P1b: the `while` is rewired, the `do` (its condition shares the body's line) is not. Slice 1
    // showed the while's span rule dropping the do header's edges `5 -> 6` and `5 -> 7`; slice 3 drops by
    // record, so both survive (the while owns neither: they sit on a shared boundary line).
    let o = own(
        "function f(n, k) {\n  let x = 0;\n  while (n--) {\n    touch();\n  } do {\n    use(x); } while (k());\n  sink(x);\n}\n",
    );
    assert_eq!(o.trees.len(), 2);
    let (w, d) = (&o.trees[0], &o.trees[1]);
    assert_eq!((w.root.kind, w.rewired), ("while_statement", true));
    assert_eq!((d.root.kind, d.rewired), ("do_statement", false));
    for t in [w, d] {
        let seq = one(t, 5, 6, Producer::Sequential);
        assert_eq!(seq.fate, Fate::Survived);
        assert_eq!(seq.disposition, U(Unres::SharedBoundaryLine));
        assert!(!seq.unauthorized_drop, "{seq:?}");
        let exit = one(t, 5, 7, Producer::LoopExit);
        assert_eq!(exit.fate, Fate::Survived);
        assert_eq!(exit.disposition, U(Unres::SharedBoundaryLine));
        assert!(!exit.unauthorized_drop);
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
fn a_statement_after_a_one_line_loop_keeps_its_successor() {
    // opus r2 N2: `while (c()) x = f(); g(x);` on one line. Slice 1 showed the span rule removing `g(x)`'s
    // edge to line 3, which the loop does not own; the line holds three statements, one kept. A record-based
    // drop leaves it.
    let o = own("function f(c, x) {\n  while (c()) x = f(); g(x);\n  return x;\n}\n");
    let t = &o.trees[0];
    assert!(t.rewired);
    let seq = one(t, 2, 3, Producer::Sequential);
    assert_eq!(seq.fate, Fate::Survived);
    assert_eq!(seq.disposition, U(Unres::SharedBoundaryLine));
    assert!(!seq.unauthorized_drop);
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
    // sol r2 P1 #2: the then-end `4 -> 11` leaves the label's line, not the loop's completion. The
    // wrapper's completion resolves (the loop's condition), so the tree owns it (slice 3) and replaces it
    // with the condition's edge to `sink(x)`.
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
    assert_eq!(then_end.fate, Fate::Dropped);
    assert_eq!(then_end.disposition, O(Owned::DelegatedThenEnd));
    assert!(!then_end.unauthorized_drop);
    // Replaced by the wrapper's completion: the do's condition (L7) reaches `sink(x)` (L11).
    let replacement = survivor(t, 7, 11, Producer::JsLoops);
    assert_eq!(replacement.record.role, Role::LoopExit);
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
    // The try's completions are the loop's exit (L5) and the catch's last statement (L9): both reach
    // `sink(x)` (L14), so the tree owns the then-end (slice 3).
    assert_eq!(then_end.disposition, O(Owned::DelegatedThenEnd));
    assert_eq!(then_end.fate, Fate::Dropped);
    for from in [5, 9] {
        let r = survivor(t, from, 14, Producer::JsLoops);
        assert_eq!(
            r.record.role,
            if from == 5 {
                Role::LoopExit
            } else {
                Role::NormalCompletion
            }
        );
    }
    // build_try_catch_edges is not modelled: its `4 -> 9` has role Legacy, ownership unknown,
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
    // The record names the `try` it was drawn for, so a refusal can point at it.
    assert_eq!(try_edge.from.map(|a| a.kind), Some("try_statement"));
    assert!(try_edge.to.is_none());
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
    // A `catch` whose first statement is the loop: the `try -> catch` edge ENTERS the tree at its first line
    // from outside. The rewrite changes no edge into the head and owns nothing of it: independent (slice 3;
    // slice 2 called every unmodelled edge that crosses the tree's lines unresolved).
    let src = "function f(k) {\n  try {\n    a();\n  } catch (e) {\n    while (k()) {\n      b();\n    }\n  }\n}\n";
    let entering = own(src);
    let e = one(&entering.trees[0], 2, 5, Producer::Try);
    assert_eq!(e.disposition, I(Indep::EntryFromOutside));
    // With a `finally`, `catch -> finally` leaves the tree from its head: nothing says the loop's exit is
    // not bypassed by it, so it stays a crossing no tree owns.
    let leaving = own(
        "function f(k) {\n  try {\n    a();\n  } catch (e) {\n    while (k()) {\n      b();\n    }\n  } finally {\n    c();\n  }\n}\n",
    );
    let e = one(&leaving.trees[0], 5, 9, Producer::Try);
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
    // Line 3 holds the arrow's declaration, a statement of the OUTER owner too, so the outer RD selects
    // these edges (both endpoint lines are its statements): foreign, admitted, and interior to the loop
    // (independent of its transitions).
    let back = on(t, 3, 3)
        .into_iter()
        .filter(|e| e.record.producer == Producer::LoopBack)
        .collect::<Vec<_>>();
    // Two `3 -> 3` back edges: the one the OUTER build drew for the arrow's loop (nested), and the one the
    // arrow's own build drew (another callable's record the outer RD also selects by its lines).
    assert_eq!(back.len(), 2, "{back:?}");
    let exit = one(t, 3, 4, Producer::LoopExit);
    for e in back.into_iter().chain([exit]) {
        assert!(e.record.nested && e.admitted, "{e:?}");
        assert_eq!(d(e), "Foreign(Admitted)");
        assert!(!e.unauthorized_drop, "{e:?}");
    }
    assert_eq!(exit.record.own, Own::Ambiguous(Doubt::NestedCallable));
    // The arrow's own build draws its loop's edges for itself: not a nested-callable doubt (its one line
    // holds the loop and its body statement, so the line is shared), and the records are the arrow's.
    let inner = &owners[1];
    assert_eq!(inner.trees.len(), 1);
    let backs = on(&inner.trees[0], 3, 3);
    let own_back = backs
        .iter()
        .find(|e| !e.record.nested)
        .expect("the arrow's own record");
    assert_eq!(own_back.record.own, Own::Ambiguous(Doubt::SharedLine));
    assert_eq!(own_back.record.callable, inner.callable);
    assert_eq!(d(own_back), "Unresolved(HeadCollapse)");
    // The outer build's record for the arrow's loop is foreign to the arrow, and its RD selects it too
    // (line 3 is the arrow's statement): it sits on the collapsed head, so it is unresolved.
    let outer_back = backs
        .iter()
        .find(|e| e.record.nested)
        .expect("the outer build's record");
    assert_eq!(d(outer_back), "Unresolved(ForeignAdmitted)");
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
fn records_are_one_per_edge_in_lockstep() {
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
        // The edges the public builder returns are the same sequence the sink held (records are consumed
        // by the loop rewrite since slice 3, so there is no records-off comparison any more).
        let all = build_cfg_edges_with_arms(&parsed);
        assert!(!all.is_empty(), "{src}");
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
    assert_eq!(o.statements.covered().len(), 1);
    let (stmt, covering) = &o.statements.covered()[0];
    assert_eq!((stmt.line, stmt.kind.as_str()), (4, "expression_statement"));
    assert_eq!((covering.line, covering.kind.as_str()), (2, "if_statement"));
    // Two statements on one line: the second is dropped by deduplication, nothing is covered.
    let o = own("function f(a) {\n  a = 1; sink(a);\n  return a;\n}\n");
    assert_eq!(o.statements.dropped.len(), 1);
    assert_eq!(o.statements.dropped[0].0.line, 2);
    assert!(o.statements.covered().is_empty());
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
fn a_switch_or_nested_if_ending_an_if_arm_is_delegated_to_the_tree() {
    // switch: the then-end `4 -> 13` leaves the `switch`'s line, past the loop inside its case.
    let o = own(
        "function f(c, k, a) {\n  let x = 0;\n  if (c) {\n    switch (a) {\n      case 1:\n        while (k()) {\n          x = 1;\n        }\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let e = one(t, 4, 13, Producer::ThenEnd);
    assert_eq!(e.record.from.map(|a| a.kind), Some("switch_statement"));
    // The switch has no `default`: its completions are its header (L4, no case matches) and the loop's
    // exit (the last case's last statement, L6), both reaching `sink(x)` (L13): the tree owns the then-end.
    assert_eq!(
        (e.fate, e.disposition),
        (Fate::Dropped, O(Owned::DelegatedThenEnd))
    );
    assert!(!e.unauthorized_drop);
    for from in [4, 6] {
        assert!(on(t, from, 13)
            .iter()
            .any(|e| e.record.producer == Producer::JsLoops && e.fate == Fate::Survived));
    }
    // nested if/else: the outer if's then-end leaves the inner `if`'s line (a wrapper); the inner `if`'s
    // then-end leaves the loop itself, so the loop owns it (drawn by the branch builder, delegated),
    // and the rewrite drops it.
    let o = own(
        "function f(c, d, k) {\n  let x = 0;\n  if (c) {\n    if (d) {\n      while (k()) {\n        x = 1;\n      }\n    } else {\n      x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let outer = one(t, 4, 14, Producer::ThenEnd);
    assert_eq!(outer.record.from.map(|a| a.kind), Some("if_statement"));
    // The inner `if`'s completions are its two arms' (the loop's exit L5, `x = 2` L9), reaching L14.
    assert_eq!(outer.disposition, O(Owned::DelegatedThenEnd));
    assert_eq!(outer.fate, Fate::Dropped);
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

// ---- slice 1 fold r1 (F1-F6): each test below fails on 8f1ee3b4 ----

#[test]
fn f1_a_replacement_belongs_to_the_tree_that_emitted_it() {
    // Tree 1's replacement exit `2 -> 5` and its `break` edge `3 -> 5` enter tree 2's first line. Tree 1
    // owns them; tree 2's inventory lists the same records as independent.
    let o = own(
        "function f(n) {\n  while (n--) {\n    if (n) break;\n  }\n  while (n++ < 9) {\n    b();\n  }\n  c();\n}\n",
    );
    let (t1, t2) = (&o.trees[0], &o.trees[1]);
    for (from, to, producer) in [(2, 5, Producer::JsLoops), (3, 5, Producer::Jump)] {
        let (a, b) = (one(t1, from, to, producer), one(t2, from, to, producer));
        assert_eq!(a.record.id, b.record.id, "the same record");
        assert_eq!(
            d(a),
            "OwnedReplaceable(Replacement)",
            "{from}->{to} in tree 1"
        );
        assert_eq!(d(b), "Independent(OtherTree)", "{from}->{to} in tree 2");
    }
    // Every replacement a tree claims was emitted by it: its source is on that tree's own lines.
    for (t, (first, last)) in o.trees.iter().zip([(2, 4), (5, 7)]) {
        for e in t
            .entries
            .iter()
            .filter(|e| d(e) == "OwnedReplaceable(Replacement)")
        {
            assert!((first..=last).contains(&e.record.from_line), "{e:?}");
        }
    }
}

#[test]
fn f2_a_header_line_that_also_holds_body_execution_is_a_head_collapse() {
    // `while (c()) { x = f();`: the test and the first body statement share one node.
    let o = own(
        "function s() {\n  let x = 0;\n  while (c()) { x = f();\n    g();\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    assert!(t.rewired);
    assert!(!t.entries.is_empty());
    for e in &t.entries {
        assert_eq!(d(e), "Unresolved(HeadCollapse)", "{e:?}");
    }
    // One-line `while (c()) x = f();`: the self edges equal their replacements; neither is a bypass.
    let o = own("function s() {\n  let x = 0;\n  while (c()) x = f();\n  return x;\n}\n");
    let t = &o.trees[0];
    for e in &t.entries {
        assert_eq!(d(e), "Unresolved(HeadCollapse)", "{e:?}");
        assert!(!e.surviving_bypass, "{e:?}");
    }
    // A one-line `do` has no admitted condition point: collapsed.
    let o =
        own("function s() {\n  let x = 0;\n  do { x = f(); } while ((x = g()));\n  sink(x);\n}\n");
    assert!(!o.trees[0].rewired);
    for e in &o.trees[0].entries {
        assert_eq!(d(e), "Unresolved(HeadCollapse)", "{e:?}");
    }
}

#[test]
fn f2_harmless_sharing_is_not_a_head_collapse() {
    // A `for` header declaration shares the head line with the loop, and a multi-line header spreads over
    // lines: neither puts body execution on the head line.
    let o = own(
        "function h(n) {\n  let x = 0;\n  for (let i = 0;\n       i < n;\n       i++) {\n    x = f(i);\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    assert!(!t.entries.is_empty());
    for e in &t.entries {
        assert_ne!(d(e), "Unresolved(HeadCollapse)", "{e:?}");
    }
    assert_eq!(
        d(one(t, 6, 3, Producer::LoopBack)),
        "OwnedReplaceable(LoopBack)"
    );
    // A two-line `do` (body on the `do` line, condition on its own line) is representable: its `do`
    // header holds nothing once the condition is a statement. Its records carry the informative
    // `SharedLine` and are still owned.
    let o = own("function f(a) {\n  do { a = g(); }\n  while (a-- > 0);\n  sink(a);\n}\n");
    let t = &o.trees[0];
    assert!(t.rewired);
    for e in &t.entries {
        assert!(!d(e).starts_with("Unresolved"), "{e:?}");
    }
    let exit = one(t, 2, 4, Producer::LoopExit);
    assert_eq!(exit.record.own, Own::Ambiguous(Doubt::SharedLine));
    assert_eq!(d(exit), "OwnedReplaceable(LoopExit)");
    // An empty body adds no body execution to the head line.
    let o = own("function h() {\n  let x = 0;\n  while ((x = next())) {}\n  sink(x);\n}\n");
    for e in &o.trees[0].entries {
        assert!(!d(e).starts_with("Unresolved"), "{e:?}");
        assert!(!e.unauthorized_drop && !e.surviving_bypass, "{e:?}");
    }
}

#[test]
fn f2_nested_loops_sharing_a_boundary_line_are_unresolved() {
    let o = own("function f(a, b) {\n  let x = 0;\n  while (a()) { while (b()) {\n    x = 1;\n  } }\n  sink(x);\n}\n");
    let t = &o.trees[0];
    let on3 = touching(t, 3);
    assert!(!on3.is_empty());
    for e in on3 {
        assert!(d(e).starts_with("Unresolved"), "{e:?}");
    }
}

#[test]
fn f3_a_then_end_from_a_wrapper_holding_a_loop_of_the_tree_is_delegated_when_it_resolves() {
    // Opus probes w1 (try), w3 (switch): the wrapper sits inside an outer loop and holds the inner one.
    let w1 = own(
        "function w1(c, a) {\n  let x = 0;\n  while (a()) {\n    if (c) {\n      try {\n        while ((x = next())) {\n          g();\n        }\n      } catch (e) {\n        h();\n      }\n    } else {\n      x = 2;\n    }\n    sink(x);\n  }\n}\n",
    );
    assert_eq!(
        d(one(&w1.trees[0], 5, 15, Producer::ThenEnd)),
        "OwnedReplaceable(DelegatedThenEnd)"
    );
    let w3 = own(
        "function w3(c, a, k) {\n  let x = 0;\n  while (a()) {\n    if (c) {\n      switch (k) {\n        default:\n          while ((x = next())) {\n            g();\n          }\n      }\n    } else {\n      x = 2;\n    }\n    sink(x);\n  }\n}\n",
    );
    assert_eq!(
        d(one(&w3.trees[0], 5, 14, Producer::ThenEnd)),
        "OwnedReplaceable(DelegatedThenEnd)"
    );
    // A label on the loop's own line is its alias: the then-end is the loop's (delegated), not unresolved.
    let w2 = own(
        "function w2(c, a) {\n  let x = 0;\n  while (a()) {\n    if (c) {\n      lbl: while ((x = next())) {\n        g();\n      }\n    } else {\n      x = 2;\n    }\n    sink(x);\n  }\n}\n",
    );
    let e = one(&w2.trees[0], 5, 11, Producer::ThenEnd);
    assert_eq!(d(e), "OwnedReplaceable(DelegatedThenEnd)");
    assert_eq!(e.fate, Fate::Dropped);
    assert!(!e.unauthorized_drop);
}

#[test]
fn f3b_a_wrapper_whose_completion_cannot_be_routed_stays_unresolved() {
    // A `break` of the switch INSIDE the wrapper but outside the loop's tree: no tree routes it, so the
    // wrapper emits its edge itself (slice 3: `case 1`'s end reaches `sink(x)`), and the then-end is
    // delegated. Hand-derived lines: switch L4, loop L6, `break` L9, `x = 2` L11, `sink(x)` L16.
    let o = own(
        "function f(c, k) {\n  let x = 0;\n  if (c) {\n    switch (k) {\n      case 1:\n        while (g()) {\n          x = 1;\n        }\n        break;\n      default:\n        x = 2;\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let e = one(t, 4, 16, Producer::ThenEnd);
    assert_eq!(e.record.from.map(|a| a.kind), Some("switch_statement"));
    assert_eq!(
        (e.fate, e.disposition),
        (Fate::Dropped, O(Owned::DelegatedThenEnd))
    );
    let brk = survivor(t, 9, 16, Producer::Jump);
    assert_eq!(brk.record.role, Role::Break);
    survivor(t, 11, 16, Producer::JsLoops);
    // A `break` out of the wrapper to a labeled block OUTSIDE it, outside every tree, has no edge and no
    // owner: dropping the then-end would lose its flow, so the wrapper is not delegated.
    let o = own(
        "function f(c, k) {\n  let x = 0;\n  outer: {\n    if (c) {\n      switch (k) {\n        case 1:\n          while (g()) {\n            x = 1;\n          }\n          break outer;\n        default:\n          x = 2;\n      }\n    } else {\n      return 0;\n    }\n    sink(x);\n  }\n}\n",
    );
    let e = one(&o.trees[0], 5, 17, Producer::ThenEnd);
    assert_eq!(e.record.from.map(|a| a.kind), Some("switch_statement"));
    assert_eq!(
        (e.fate, e.disposition),
        (Fate::Survived, U(Unres::WrapperEntryThenEnd))
    );
    // A wrapper that holds two trees is claimed by the first of them (one record, one authority); the
    // other sees it as another tree's, and both are delegated.
    let two = own(
        "function g(c, k) {\n  let x = 0;\n  if (c) {\n    try {\n      while (k()) {\n        x = 1;\n      }\n      while (k()) {\n        x = 2;\n      }\n    } finally {\n      h();\n    }\n  } else {\n    return 0;\n  }\n  sink(x);\n}\n",
    );
    assert_eq!(two.trees.len(), 2);
    assert_eq!(
        d(one(&two.trees[0], 4, 17, Producer::ThenEnd)),
        "OwnedReplaceable(DelegatedThenEnd)"
    );
    assert_eq!(
        d(one(&two.trees[1], 4, 17, Producer::ThenEnd)),
        "Independent(OtherTree)"
    );
    assert_eq!(
        one(&two.trees[1], 4, 17, Producer::ThenEnd).fate,
        Fate::Dropped
    );
}

#[test]
fn f4_an_edge_leaving_a_larger_loop_for_a_smaller_sibling_is_a_body_bypass() {
    // e1: the larger loop `4..7` first, the smaller `8..10` second. e2 is the mirror (control).
    let e1 = own(
        "function e1(a) {\n  let x = 0;\n  while (a()) {\n    while ((x = next())) {\n      g();\n      x = f();\n    }\n    while (k()) {\n      m(x);\n    }\n  }\n}\n",
    );
    let e = one(&e1.trees[0], 6, 8, Producer::Sequential);
    assert_eq!(d(e), "OwnedReplaceable(BodyBypass)");
    assert_eq!(e.fate, Fate::Dropped);
    assert!(!e.unauthorized_drop);
    let e2 = own(
        "function e2(a) {\n  let x = 0;\n  while (a()) {\n    while (k()) {\n      m(x);\n    }\n    while ((x = next())) {\n      g();\n      x = f();\n    }\n  }\n}\n",
    );
    assert_eq!(
        d(one(&e2.trees[0], 5, 7, Producer::Sequential)),
        "OwnedReplaceable(BodyBypass)"
    );
}

#[test]
fn f5_nested_callable_records_are_foreign_whatever_else_is_uncertain() {
    // The arrow's `if (y) { if (z) g(); }` emits branch entries and then-ends into the outer owner's list.
    // All are drawn for the arrow and every one is foreign (never owned). The arrow shares line 4 with the
    // outer statement `x = f();`, so the outer RD SELECTS all of them: the `4 -> 4` ones stay inside the
    // loop (independent: foreign, admitted); the `4 -> 6` ones leave the loop from its body line and
    // cannot be owned by it: unresolved, which refuses the tree.
    let src = "function n(arr) {\n  let x = 0;\n  while ((x = next())) {\n    x = f(); arr.forEach(y => { if (y) { if (z) g(); } });\n  }\n  sink(x);\n}\n";
    let o = own(src);
    let t = &o.trees[0];
    let foreign: Vec<_> = t
        .entries
        .iter()
        .filter(|e| matches!(e.record.producer, Producer::BranchEntry | Producer::ThenEnd))
        .filter(|e| e.record.nested)
        .collect();
    assert!(foreign.len() >= 4, "{foreign:?}");
    for e in foreign {
        let want = if e.record.to_line == 4 {
            "Foreign(Admitted)"
        } else {
            assert_eq!(e.record.to_line, 6);
            "Unresolved(ForeignAdmitted)"
        };
        assert_eq!(d(e), want, "{e:?}");
        // Slice 3 drops by record: the `4 -> 6` records are no tree's, so they are not dropped (the plan
        // refuses the tree instead).
        assert!(!e.unauthorized_drop && e.fate == Fate::Survived, "{e:?}");
        assert!(e.admitted, "{e:?}");
    }
    let plan = parse(src).js_loop_plan().verdicts.clone();
    assert_eq!(plan[0].why, Some(Why::Unresolved(Unres::ForeignAdmitted)));
}

#[test]
fn f6_a_body_whose_last_line_holds_two_statements_has_a_body_bypass() {
    let o = own(
        "function n(arr) {\n  let x = 0;\n  while ((x = next())) {\n    x = f(); arr.forEach(y => g(y));\n  }\n  sink(x);\n}\n",
    );
    let e = one(&o.trees[0], 4, 6, Producer::Sequential);
    assert_eq!(d(e), "OwnedReplaceable(BodyBypass)");
    assert!(!e.unauthorized_drop, "{e:?}");
}

#[test]
fn a_wrapper_then_end_inside_the_body_that_leaves_the_loop_is_delegated_to_it() {
    // The 62-record class on TypeScript: the outer arm's last statement is an inner `if`/`else`; main's
    // then-end leaves its entry line past the loop. The arm's real completions are already the loop's, so
    // the loop owns the record (slice 3; slice 2 called it a non-completion crossing and refused the tree).
    let o = own(
        "function f(v, k) {\n  let x = 0;\n  while (k()) {\n    if (v) {\n      if (x) {\n        x = 1;\n      } else {\n        x = 2;\n      }\n    }\n  }\n  sink(x);\n}\n",
    );
    let t = &o.trees[0];
    let e = one(t, 5, 12, Producer::ThenEnd);
    assert_eq!(d(e), "OwnedReplaceable(DelegatedThenEnd)");
    assert_eq!(e.fate, Fate::Dropped);
    assert!(!e.unauthorized_drop);
    // The inner arm's own end is a body completion.
    assert_eq!(
        d(one(t, 6, 12, Producer::ThenEnd)),
        "OwnedReplaceable(BodyBypass)"
    );
    // Straight-line body flow is independent.
    let o = own("function f(k) {\n  while (k()) {\n    a();\n    b();\n  }\n  c();\n}\n");
    assert_eq!(
        d(one(&o.trees[0], 3, 4, Producer::Sequential)),
        "Independent(InteriorFlow)"
    );
}

#[test]
fn methods_and_arrows_are_owners_and_a_static_block_is_not() {
    let owners = own_ts(
        "class K {\n  m() {\n    let x = 0;\n    while ((x = next())) {\n      x = f();\n    }\n    sink(x);\n  }\n  static {\n    let y = 0;\n    while ((y = next())) {\n      y = f();\n    }\n    sink(y);\n  }\n}\nconst a = () => {\n  let x = 0;\n  for (const k of ks) {\n    x = f(k);\n  }\n  sink(x);\n};\n",
    );
    // The method and the arrow own one tree each; the `static` block's loop belongs to no owner
    // (pre-existing: it has no CFG), so its lines appear in no inventory.
    assert_eq!(owners.len(), 2);
    for o in &owners {
        assert_eq!(o.trees.len(), 1);
        assert!(o.trees[0].rewired);
    }
    assert_eq!(
        d(one(&owners[0].trees[0], 5, 4, Producer::LoopBack)),
        "OwnedReplaceable(LoopBack)"
    );
    assert!(owners[0].trees[0]
        .entries
        .iter()
        .all(|e| e.record.from_line < 9 && e.record.to_line < 9));
}
