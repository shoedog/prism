//! Fold-1 contracts for the Go field-write recall lane (#661 review round 1).
//! Programs are parsed, never executed.
use super::Producer;
use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, FlowConfidence};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

type Row = (String, usize, String, usize, FlowConfidence);

const HEAD: &str = "package p\ntype A struct{ b int }\ntype T struct{ f, g int; a A }\nfunc cleanup() {}\nfunc use(x any) {}\nfunc sink(x any) {}\n";

fn src(body: &str) -> String {
    format!("{HEAD}{body}")
}
/// Line of the first line of `body` in the assembled source.
const BASE: usize = 7;

fn recall(source: &str) -> BTreeSet<Row> {
    let parsed = ParsedFile::parse("a.go", source, Language::Go).unwrap();
    let parts = Producer::file_refs(&parsed.path, &parsed);
    parts
        .edges
        .iter()
        .filter(|e| {
            let (d, u) = (&e.from.path, &e.to.path);
            d.base == u.base && d.fields.len() > u.fields.len() && d.fields.starts_with(&u.fields)
        })
        .map(|e| {
            (
                e.from.path.to_string(),
                e.from.line - BASE + 1,
                e.to.path.to_string(),
                e.to.line - BASE + 1,
                *parts.labels.get(&(e.from.clone(), e.to.clone())).unwrap(),
            )
        })
        .collect()
}
fn lines(rows: &BTreeSet<Row>) -> BTreeSet<(String, usize, String, usize)> {
    rows.iter()
        .map(|r| (r.0.clone(), r.1, r.2.clone(), r.3))
        .collect()
}
fn has(rows: &BTreeSet<Row>, from: (&str, usize), to: (&str, usize)) -> bool {
    rows.iter()
        .any(|r| r.0 == from.0 && r.1 == from.1 && r.2 == to.0 && r.3 == to.1)
}
fn tainted_lines(source: &str, seed: usize) -> BTreeSet<usize> {
    let files = BTreeMap::from([(
        "a.go".to_string(),
        ParsedFile::parse("a.go", source, Language::Go).unwrap(),
    )]);
    let graph = CodePropertyGraph::build(&files);
    let trace = graph.taint_trace(&[("a.go".to_string(), seed + BASE - 1)]);
    trace
        .frontier_by_root
        .values()
        .flatten()
        .map(|n| graph.node(*n).line() - BASE + 1)
        .collect()
}

// ---- 1. defer exit dispatch is not a body route ----

const DEFER_EARLY: &str = "func f(p *T, x int, bad bool) *T {
	defer cleanup()
	if bad {
		return p
	}
	p.f = x
	return p
}
";

#[test]
fn fold1_defer_early_return_has_no_backward_edge() {
    let s = src(DEFER_EARLY);
    let rows = recall(&s);
    assert!(has(&rows, ("p.f", 6), ("p", 7)), "{rows:?}");
    assert!(!has(&rows, ("p.f", 6), ("p", 4)), "{rows:?}");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows.iter().next().unwrap().4, FlowConfidence::Exact);
    assert!(!tainted_lines(&s, 6).contains(&4));
}

#[test]
fn fold1_defer_does_not_carry_a_killed_contribution_past_the_kill() {
    let s = src("func f(p *T, x int, bad bool) *T {
	p.f = x
	if bad {
		return nil
	}
	p = &T{}
	defer cleanup()
	return p
}
");
    let rows = recall(&s);
    assert!(rows.is_empty(), "{rows:?}");
    assert!(!tainted_lines(&s, 2).contains(&8));
}

#[test]
fn fold1_pre_loop_use_has_no_backward_edge() {
    let s = src("func f(p *T, x int, c bool) {
	defer cleanup()
	use(p)
	for c {
		p.f = x
	}
}
");
    let rows = recall(&s);
    assert!(rows.is_empty(), "{rows:?}");
    assert!(!tainted_lines(&s, 5).contains(&3));
}

#[test]
fn fold1_real_loop_carried_flow_survives_a_defer() {
    let s = src("func f(p *T, x int, c bool) {
	defer cleanup()
	for c {
		use(p)
		p.f = x
	}
}
");
    let rows = recall(&s);
    assert_eq!(
        lines(&rows),
        BTreeSet::from([("p.f".to_string(), 5, "p".to_string(), 4)])
    );
    assert_eq!(rows.iter().next().unwrap().4, FlowConfidence::Exact);
    assert!(tainted_lines(&s, 5).contains(&4));
}

// ---- 2. range / receive / type-switch rebinding kills ----

#[test]
fn fold1_range_define_rebinds_per_iteration() {
    for header in [
        "for _, it := range items {",
        "for i, it := range items {",
        "for it := range items {",
    ] {
        let s = src(&format!(
            "func f(items []*T, x int) {{\n\t{header}\n\t\tuse(it)\n\t\tit.f = x\n\t}}\n}}\n"
        ));
        let rows = recall(&s);
        assert!(rows.is_empty(), "{header}: {rows:?}");
        assert!(!tainted_lines(&s, 4).contains(&3), "{header}");
    }
    // Value (non-pointer) range variable.
    let s = src("func f(items []T, x int) {
	for _, it := range items {
		use(it)
		it.f = x
	}
}
");
    assert!(recall(&s).is_empty());
}

#[test]
fn fold1_range_cross_arm_inside_one_body_is_killed() {
    let s = src("func f(items []*T, x int) {
	for _, it := range items {
		if it.g > 0 {
			it.f = x
		} else {
			use(it)
		}
	}
}
");
    assert!(recall(&s).is_empty(), "{:?}", recall(&s));
}

#[test]
fn fold1_range_assign_kills_on_entry_but_not_on_zero_iteration_exit() {
    let s = src("func f(p *T, ps []*T, x int) {
	p.f = x
	for _, p = range ps {
		use(p)
	}
	use(p)
}
");
    let rows = recall(&s);
    assert!(!has(&rows, ("p.f", 2), ("p", 4)), "{rows:?}");
    assert!(has(&rows, ("p.f", 2), ("p", 6)), "{rows:?}");
}

#[test]
fn fold1_range_assign_target_inside_loop_is_killed_across_iterations() {
    let s = src("func f(it *T, items []*T, x int) {
	for _, it = range items {
		use(it)
		it.f = x
	}
}
");
    assert!(recall(&s).is_empty(), "{:?}", recall(&s));
}

#[test]
fn fold1_range_over_unrelated_root_keeps_loop_carried_flow() {
    let s = src("func f(p *T, items []int, x int) {
	for _, it := range items {
		use(p)
		p.f = it
	}
}
");
    let rows = recall(&s);
    assert_eq!(
        lines(&rows),
        BTreeSet::from([("p.f".to_string(), 4, "p".to_string(), 3)])
    );
    assert_eq!(rows.iter().next().unwrap().4, FlowConfidence::Exact);
    let s = src("func f(p *T, items []int) {
	for _, it := range items {
		p.f = it
		use(p)
	}
}
");
    assert!(has(&recall(&s), ("p.f", 3), ("p", 4)));
}

#[test]
fn fold1_select_receive_rebinds() {
    for case in ["case p = <-ch:", "case p := <-ch:"] {
        let s = src(&format!(
            "func f(ch chan *T, p *T, x int) {{\n\tfor {{\n\t\tselect {{\n\t\t{case}\n\t\t\tuse(p)\n\t\t\tp.f = x\n\t\t}}\n\t}}\n}}\n"
        ));
        let rows = recall(&s);
        assert!(rows.is_empty(), "{case}: {rows:?}");
        assert!(!tainted_lines(&s, 6).contains(&5), "{case}");
    }
}

#[test]
fn fold1_type_switch_alias_rebinds() {
    let s = src("func f(v any, x int, c bool) {
	for c {
		switch p := v.(type) {
		case *T:
			use(p)
			p.f = x
		}
	}
}
");
    let rows = recall(&s);
    assert!(rows.is_empty(), "{rows:?}");
}

// ---- 3. fallback bindings (package variables) ----

const PKG: &str = "var p T\nvar q *T\n";

#[test]
fn fold1_fallback_overwrites_drop_the_stale_contribution() {
    for (case, body, stale) in [
        ("same field", "p.f = x\n\tp.f = 0\n\tsink(p)", 4),
        ("root", "p.f = x\n\tp = T{}\n\tsink(p)", 4),
        ("deref root", "q.f = x\n\t*q = T{}\n\tsink(q)", 4),
        ("prefix", "p.a.b = x\n\tp.a = A{}\n\tsink(p)", 4),
    ] {
        let s = src(&format!("{PKG}func f(x int) {{\n\t{body}\n}}\n"));
        let rows = recall(&s);
        assert!(
            !rows.iter().any(|r| r.1 == stale && r.3 == 6),
            "{case}: {rows:?}"
        );
    }
}

#[test]
fn fold1_fallback_survivors_stay_uncertain_not_exact() {
    let s = src(&format!(
        "{PKG}func f(x int) {{\n\tp.f = x\n\tp.f = 0\n\tsink(p)\n}}\n"
    ));
    let rows = recall(&s);
    let live: Vec<_> = rows.iter().filter(|r| r.3 == 6).collect();
    assert_eq!(live.len(), 1, "{rows:?}");
    assert_eq!(live[0].1, 5);
    assert_ne!(live[0].4, FlowConfidence::Exact);
    // A sibling field of the overwrite is not killed.
    let s = src(&format!(
        "{PKG}func f(x int) {{\n\tp.f = x\n\tp.g = 0\n\tsink(p)\n}}\n"
    ));
    assert!(has(&recall(&s), ("p.f", 4), ("p", 6)), "{:?}", recall(&s));
}

// ---- 4. elided struct literal inside a map literal ----

#[test]
fn fold1_elided_literal_key_is_not_a_read() {
    let s = src("type S struct{ f string }
type U struct{ Name string }
func f(Name *S, x string) {
	Name.f = x
	m := map[string]U{\"k\": {Name: \"v\"}}
	use(m)
}
");
    assert!(recall(&s).is_empty(), "{:?}", recall(&s));
}
