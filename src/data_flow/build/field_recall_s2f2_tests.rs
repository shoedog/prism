//! S2 fold 2 (#639, PR #665 review round 1): the run-site capture rule,
//! same-line kills and ordering, deferred receivers and closure-level
//! rebinding. The programs are parsed, never executed. Line numbers in the
//! expectations are relative to the first line of each body.
use super::Producer;
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;
use std::collections::BTreeSet;

const HEAD: &str = "package p\ntype A struct{ b int }\ntype T struct{ f, g int; a A }\ntype H struct{ cb func() }\nfunc use(x any) {}\nfunc register(f func()) {}\nfunc each(f func()) {}\nfunc ok(x any) bool { return true }\nfunc done() {}\nfunc tick() {}\nfunc again() bool { return true }\n";

type Row = (usize, usize, &'static str);

fn base() -> usize {
    HEAD.matches('\n').count() + 1
}

fn rows(body: &str) -> BTreeSet<Row> {
    let source = format!("{HEAD}{body}");
    let parsed = ParsedFile::parse("a.go", &source, Language::Go).unwrap();
    let parts = Producer::file_refs(&parsed.path, &parsed);
    let b = base();
    parts
        .edges
        .iter()
        .filter(|e| {
            let (d, u) = (&e.from.path, &e.to.path);
            d.base == u.base && d.fields.len() > u.fields.len() && d.fields.starts_with(&u.fields)
        })
        .map(|e| {
            let label = match parts.labels.get(&(e.from.clone(), e.to.clone())) {
                Some(FlowConfidence::Exact) => "exact",
                Some(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)) => "cfg",
                Some(FlowConfidence::NameOnly(FlowDoubt::SameLine)) => "same",
                other => panic!("{other:?}"),
            };
            (e.from.line - b + 1, e.to.line - b + 1, label)
        })
        .collect()
}

fn check(name: &str, body: &str, expected: &[Row]) {
    let want: BTreeSet<Row> = expected.iter().copied().collect();
    assert_eq!(rows(body), want, "{name}");
}

macro_rules! cases {
    ($($test:ident => ($body:expr, $expected:expr)),* $(,)?) => { $(
        #[test] fn $test() { check(stringify!($test), $body, &$expected); }
    )* };
}

macro_rules! body {
    ($($line:literal),* $(,)?) => { concat!("func f(p *T, q *T, s *H, x int, c bool, n int) {\n", $($line, "\n",)* "}\n") };
}

// ---- 5. run-site capture rule ----------------------------------------------

cases! {
    // IIFE with a write after it: it already ran.
    s2f2_iife_write_after_no_edge => (
        body!("func() { use(p) }()", "p.f = x"), []),
    s2f2_iife_write_before_edge => (
        body!("p.f = x", "func() { use(p) }()"), [(2, 3, "cfg")]),
    // A synchronous callback argument with a write after the call.
    s2f2_callback_arg_write_after_no_edge => (
        body!("each(func() { use(p) })", "p.f = x"), []),
    s2f2_callback_arg_write_before_edge => (
        body!("p.f = x", "each(func() { use(p) })"), [(2, 3, "cfg")]),
    // A local bound literal that is only called.
    s2f2_local_f_write_after_last_call_no_edge => (
        body!("f := func() { use(p) }", "f()", "p.f = x"), []),
    s2f2_local_f_write_before_later_call_edge => (
        body!("f := func() { use(p) }", "f()", "p.f = x", "f()"), [(4, 2, "cfg")]),
    s2f2_local_f_write_before_only_call_edge => (
        body!("f := func() { use(p) }", "p.f = x", "f()"), [(3, 2, "cfg")]),
    // Escaping literals keep the live-at-or-after-creation rule.
    s2f2_stored_literal_write_after_edge => (
        body!("s.cb = func() { use(p) }", "p.f = x"), [(3, 2, "cfg")]),
    s2f2_sent_literal_write_after_edge => (
        body!("ch := make(chan func(), 1)", "ch <- func() { use(p) }", "p.f = x"), [(4, 3, "cfg")]),
    s2f2_local_f_also_passed_is_escaping => (
        body!("f := func() { use(p) }", "f()", "p.f = x", "register(f)"), [(4, 2, "cfg")]),
    s2f2_local_f_assigned_is_escaping => (
        body!("f := func() { use(p) }", "p.f = x", "s.cb = f"), [(3, 2, "cfg")]),
    s2f2_local_f_called_in_literal_is_escaping => (
        body!("f := func() { use(p) }", "f()", "p.f = x", "register(func() { f() })"), [(4, 2, "cfg")]),
    s2f2_var_f_called_only => (
        body!("var f = func() { use(p) }", "f()", "p.f = x"), []),
    s2f2_go_literal_write_after_edge => (
        body!("go func() { use(p) }()", "p.f = x"), [(3, 2, "cfg")]),
    s2f2_defer_literal_write_after_edge => (
        body!("defer func() { use(p) }()", "p.f = x"), [(3, 2, "cfg")]),
    s2f2_go_argument_literal_is_escaping => (
        body!("go each(func() { use(p) })", "p.f = x"), [(3, 2, "cfg")]),
    // Loop-carried: the write of the previous iteration reaches the next call.
    s2f2_loop_carried_iife_edge => (
        body!("for i := 0; i < n; i++ {", "func() { use(p) }()", "p.f = x", "}"),
        [(4, 3, "cfg")]),
}

// ---- 6. kills on the creation line before the literal exists ---------------

cases! {
    s2f2_same_line_field_kill_before_creation => (
        body!("p.f = x", "p.f = 0; s.cb = func() { use(p) }"), [(3, 3, "cfg")]),
    s2f2_same_line_field_kill_before_creation_local => (
        body!("p.f = x", "p.f = 0; g := func() { use(p) }", "g()"), [(3, 3, "cfg")]),
    s2f2_same_line_field_kill_after_creation_keeps_both => (
        body!("p.f = x", "s.cb = func() { use(p) }; p.f = 0"), [(2, 3, "cfg"), (3, 3, "cfg")]),
    s2f2_same_line_root_kill_before_creation => (
        body!("p.f = x", "p = q; s.cb = func() { use(p) }"), []),
    s2f2_same_line_prefix_kill_before_creation => (
        body!("p.a.b = x", "p.a = A{}; s.cb = func() { use(p) }"), [(3, 3, "cfg")]),
    s2f2_earlier_line_kill_before_creation => (
        body!("p.f = x", "p.f = 0", "s.cb = func() { use(p) }"), [(3, 4, "cfg")]),
}

// ---- 7. same-line ordering --------------------------------------------------

cases! {
    s2f2_if_else_arms_never_both_run => (
        body!("if c { p.f = x } else { use(p) }", "done()"), []),
    s2f2_if_then_arm_sequence_keeps_edge => (
        body!("if c { p.f = x; use(p) }", "done()"), [(2, 2, "same")]),
    s2f2_if_init_then_condition_keeps_edge => (
        body!("if p.f = x; ok(p) {", "done()", "}"), [(2, 2, "same")]),
    s2f2_switch_case_arms_never_both_run => (
        body!("switch {", "case c: p.f = x; case !c: use(p) }", "done()"), []),
    s2f2_switch_init_then_value_keeps_edge => (
        body!("switch p.f = x; ok(p) {", "case true:", "done()", "}"), [(2, 2, "same")]),
    s2f2_enclosing_loop_does_not_certify_inner_post => (
        body!("for again() {", "for ; ok(p); p.f = x {", "break", "}", "tick()", "}"), []),
    s2f2_reachable_inner_post_keeps_edge => (
        body!("for again() {", "for ; ok(p); p.f = x {", "done()", "}", "tick()", "}"),
        [(3, 3, "same")]),
    s2f2_for_post_never_reaches_unreachable_body_read => (
        body!("for ; ok(p); p.f = x { use(p); return }"), []),
    // A body read on its own line is an ordinary (cross-line) pair.
    s2f2_for_post_to_body_read_on_another_line => (
        body!("for ; c; p.f = x {", "use(p)", "}"), [(2, 3, "exact")]),
    // Same-statement killer: `p.f, p = x, q` overwrites the root with the write.
    s2f2_same_statement_root_kill => (
        body!("p.f, p = x, q; use(p)"), []),
    s2f2_post_with_same_line_kill_is_dropped => (
        body!("for ; ok(p); p.f, p = x, q {", "done()", "}"), []),
    s2f2_for_post_never_reaches_the_init_read => (
        body!("for use(p); c; p.f = x {", "done()", "}"), []),
    s2f2_post_read_in_rhs_is_not_the_condition => (
        body!("for ; ok(q); p.f = g(p) {", "done()", "}"), []),
}

// ---- 9. deferred / go receivers ---------------------------------------------

cases! {
    s2f2_defer_receiver_write_before_is_cfg_incomplete => (
        body!("p.f = x", "defer p.M()"), [(2, 3, "cfg")]),
    s2f2_defer_receiver_write_after_edge => (
        body!("defer p.M()", "p.f = x"), [(3, 2, "cfg")]),
    s2f2_go_receiver_write_after_edge => (
        body!("go p.M()", "p.f = x"), [(3, 2, "cfg")]),
    s2f2_defer_receiver_overwritten_is_not_exact => (
        body!("p.f = x", "defer p.M()", "p.f = 0"), [(2, 3, "cfg"), (4, 3, "cfg")]),
    s2f2_defer_chain_inner_receiver_is_eager => (
        body!("p.f = x", "defer p.M().N()", "p.f = 0"), [(2, 3, "exact")]),
    s2f2_plain_receiver_stays_exact => (
        body!("p.f = x", "p.M()"), [(2, 3, "exact")]),
}

// ---- 10. closure-level rebinding ---------------------------------------------

cases! {
    s2f2_closure_range_rebind_refuses_capture => (
        body!("p.f = x", "s.cb = func() { for p = range []*T{} {}; use(p) }"), []),
    s2f2_closure_receive_rebind_refuses_capture => (
        body!("p.f = x", "ch := make(chan *T)", "s.cb = func() { select { case p = <-ch: }; use(p) }"), []),
    s2f2_closure_without_rebind_keeps_capture => (
        body!("p.f = x", "s.cb = func() { use(p) }"), [(2, 3, "cfg")]),
    s2f2_closure_define_range_is_shadowing => (
        body!("p.f = x", "s.cb = func() { for p := range []*T{} { use(p) } }"), []),
}

// ---- fixture rows (tests/fixtures/go_field_write) ---------------------------------

macro_rules! rows {
    ($($test:ident => $case:literal),* $(,)?) => { $(
        #[test] fn $test() { super::field_recall_tests::matrix($case); }
    )* };
}
rows! {
    s2f2_row_98 => "98_run_iife", s2f2_row_99 => "99_run_callback",
    s2f2_row_100 => "100_run_local_f", s2f2_row_101 => "101_sl_if_else_arms",
    s2f2_row_102 => "102_sl_kill_before_creation", s2f2_row_103 => "103_recv_defer_write_after",
}
