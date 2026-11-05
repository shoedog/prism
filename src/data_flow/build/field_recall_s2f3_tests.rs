//! S2 fold 3 (#639, PR #665 review round 2): deferred and `go` receivers see
//! only contributions that reach the statement, wide branch-arm ids, and a
//! forward "after" set for escaping literals. Parsed, never executed.
use super::field_recall_s2f2_tests::{base, check, rows};
use std::collections::BTreeSet;

macro_rules! body {
    ($($line:literal),* $(,)?) => { concat!("func f(p *T, q *T, s *H, x int, c bool, n int) {\n", $($line, "\n",)* "}\n") };
}

macro_rules! cases {
    ($($test:ident => ($body:expr, $expected:expr)),* $(,)?) => { $(
        #[test] fn $test() { check(stringify!($test), $body, &$expected); }
    )* };
}

cases! {
    // Review sol r2 #1: the pointer was captured at the defer.
    s2f3_defer_pointer_rebound_then_written_no_edge => (
        body!("r := &T{}", "defer r.M()", "r = &T{}", "r.f = x"), []),
    s2f3_go_pointer_rebound_then_written_no_edge => (
        body!("r := &T{}", "go r.M()", "r = &T{}", "r.f = x"), []),
    // Review sol r2 #2: a value receiver is copied at the statement.
    s2f3_defer_value_receiver_write_after_no_edge => (
        body!("r := T{}", "defer r.M()", "r.f = x"), []),
    s2f3_go_value_receiver_write_after_no_edge => (
        body!("r := T{}", "go r.M()", "r.f = x"), []),
    // A write before the statement stays, graded cfg_incomplete.
    s2f3_go_write_before_is_cfg_incomplete => (
        body!("p.f = x", "go p.M()"), [(2, 3, "cfg")]),
    s2f3_write_before_then_rebound_after_keeps_edge => (
        body!("p.f = x", "defer p.M()", "p = q"), [(2, 3, "cfg")]),
    s2f3_write_before_then_rebound_after_keeps_edge_go => (
        body!("p.f = x", "go p.M()", "p = q", "p.f = 0"), [(2, 3, "cfg")]),
    // `defer x.mu.Unlock()`: a nested receiver path.
    s2f3_defer_nested_receiver_write_after_no_edge => (
        body!("defer p.a.Unlock()", "p.a.b = x"), []),
    s2f3_defer_nested_receiver_write_before_edge => (
        body!("p.a.b = x", "defer p.a.Unlock()"), [(2, 3, "cfg")]),
    // Review opus r2 N2: the other write overwrote it on the only path.
    s2f3_go_receiver_overwritten_on_branch_path => (
        body!("p.f = x", "if c {", "p.f = 0", "go p.M()", "}", "done()"), [(4, 5, "cfg")]),
}

cases! {
    // Review opus r2 N2 (escaping literal): a write on a branch that bypasses
    // the creation point and joins after it reaches nothing.
    s2f3_escaping_bypassing_branch_write_no_edge => (
        body!("if c {", "s.cb = func() { use(p) }", "} else {", "p.f = x", "}", "done()"), []),
    s2f3_escaping_write_after_join_edge => (
        body!("if c {", "s.cb = func() { use(p) }", "} else {", "done()", "}", "p.f = x"),
        [(7, 3, "cfg")]),
    s2f3_escaping_write_after_creation_same_path_edge => (
        body!("if c {", "s.cb = func() { use(p) }", "p.f = x", "}"), [(4, 3, "cfg")]),
    // Loop-carried writes still reach a literal created in the loop.
    s2f3_escaping_loop_carried_edge => (
        body!("for i := 0; i < n; i++ {", "s.cb = func() { use(p) }", "p.f = x", "}"),
        [(4, 3, "cfg")]),
}

cases! {
    // Probe P665r2-later-exclusive (same-arm control; the exclusive-arm form
    // already admits a collapsed-node self-edge, see the report).
    s2f3_escaping_literal_same_arm_later_write_edge => (
        body!("if c { s.cb = func() { use(p) }; p.f = x }", "done()"), [(2, 2, "cfg")]),
    // Probe P665r2-for-body-post: the post reads what the body wrote.
    s2f3_for_body_write_post_read_same_line_edge => (
        body!("for ; c; use(p) { p.f = x }"), [(2, 2, "same")]),
}

/// A switch with `total` cases whose last two sit on one line, `p.f = x` in one
/// and `use(p)` in the other: exclusive arms, whatever the case count.
fn wide_switch(total: usize) -> String {
    let mut body = String::from("func f(p *T, c bool) {\nswitch {\n");
    for _ in 0..total - 2 {
        body.push_str("case c:\ndone()\n");
    }
    body.push_str("case c: p.f = 1; case !c: use(p)\n}\n}\n");
    body
}

#[test]
fn s2f3_wide_switch_late_exclusive_arms_no_edge() {
    for total in [2, 253, 254, 255, 256, 257, 300] {
        assert_eq!(rows(&wide_switch(total)), BTreeSet::new(), "{total} cases");
    }
}

#[test]
fn s2f3_wide_switch_sequential_late_arm_keeps_edge() {
    // Write then read in the same late arm: still a same-line edge.
    let mut body = String::from("func f(p *T, c bool) {\nswitch {\n");
    for _ in 0..299 {
        body.push_str("case c:\ndone()\n");
    }
    body.push_str("case c: p.f = 1; use(p)\n}\n}\n");
    let got = rows(&body);
    let line = base() + 2 + 299 * 2;
    let want = BTreeSet::from([(line - base() + 1, line - base() + 1, "same")]);
    assert_eq!(got, want);
}
