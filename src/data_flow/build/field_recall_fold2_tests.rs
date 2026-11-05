//! Fold-2 contracts for the Go field-write recall lane (#661 review round 2):
//! one virtual node per range header, selector and dereference header targets,
//! and the break-to-exit defer dispatch exclusion. Programs are parsed, never
//! executed.
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;

type Row = (String, usize, String, usize);

const HEAD: &str = "package p\ntype A struct{ b int }\ntype T struct{ f, g int; a A; arr [4]int }\nfunc cleanup() {}\nfunc use(x any) {}\nfunc sink(x any) {}\n";
/// Line of the first line of a body in the assembled source.
const BASE: usize = 7;

fn src(body: &str) -> String {
    format!("{HEAD}{body}")
}

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
            )
        })
        .collect()
}
fn has(rows: &BTreeSet<Row>, from: (&str, usize), to: (&str, usize)) -> bool {
    rows.iter()
        .any(|r| r.0 == from.0 && r.1 == from.1 && r.2 == to.0 && r.3 == to.1)
}

// ---- N1 / sol #1: one virtual node per range header ----

#[test]
fn fold2_two_name_define_range_kills_both_names() {
    for (k, v) in [("*T", "*T"), ("T", "T"), ("*T", "T")] {
        let s = src(&format!(
            "func f(m map[{k}]{v}, x int) {{\n\tfor k, v := range m {{\n\t\tsink(k)\n\t\tsink(v)\n\t\tk.f = x\n\t\tv.f = x\n\t\tsink(k)\n\t\tsink(v)\n\t}}\n}}\n"
        ));
        let rows = recall(&s);
        assert!(!has(&rows, ("k.f", 5), ("k", 3)), "{k},{v}: {rows:?}");
        assert!(!has(&rows, ("v.f", 6), ("v", 4)), "{k},{v}: {rows:?}");
        assert!(has(&rows, ("k.f", 5), ("k", 7)), "{k},{v}: {rows:?}");
        assert!(has(&rows, ("v.f", 6), ("v", 8)), "{k},{v}: {rows:?}");
        assert_eq!(rows.len(), 2, "{k},{v}: {rows:?}");
    }
}

#[test]
fn fold2_two_name_assign_range_kills_both_and_keeps_zero_iteration_exit() {
    for ty in ["*T", "T"] {
        let s = src(&format!(
            "func f(k, v {ty}, m map[{ty}]{ty}, x int) {{\n\tk.f = x\n\tv.f = x\n\tfor k, v = range m {{\n\t\tsink(k)\n\t\tsink(v)\n\t\tk.g = x\n\t\tv.g = x\n\t\tsink(k)\n\t\tsink(v)\n\t}}\n\tsink(k)\n\tsink(v)\n}}\n"
        ));
        let rows = recall(&s);
        // Entry rebinding kills both outer contributions in the body.
        assert!(!has(&rows, ("k.f", 2), ("k", 5)), "{ty}: {rows:?}");
        assert!(!has(&rows, ("v.f", 3), ("v", 6)), "{ty}: {rows:?}");
        // Loop-carried body writes are killed on the next entry.
        assert!(!has(&rows, ("k.g", 7), ("k", 5)), "{ty}: {rows:?}");
        assert!(!has(&rows, ("v.g", 8), ("v", 6)), "{ty}: {rows:?}");
        // Forward flows after the writes inside the body survive.
        assert!(has(&rows, ("k.g", 7), ("k", 9)), "{ty}: {rows:?}");
        assert!(has(&rows, ("v.g", 8), ("v", 10)), "{ty}: {rows:?}");
        // The zero-iteration exit keeps the outer contributions.
        assert!(has(&rows, ("k.f", 2), ("k", 12)), "{ty}: {rows:?}");
        assert!(has(&rows, ("v.f", 3), ("v", 13)), "{ty}: {rows:?}");
    }
}

// ---- N2: break exits the function through a defer ----

#[test]
fn fold2_break_exit_defer_dispatch_is_not_a_body_route() {
    let s = src("func f(p *T, c bool, x int) {
	p.f = x
	for {
		if c {
			break
		}
		p = &T{}
		defer cleanup()
		use(p)
	}
}
");
    let rows = recall(&s);
    assert!(!has(&rows, ("p.f", 2), ("p", 9)), "{rows:?}");
}

// ---- sol #2: selector, dereference and parenthesized header targets ----

fn assert_overwritten(body: &str, label: &str) {
    let rows = recall(&src(body));
    assert!(rows.is_empty(), "{label}: {rows:?}");
}

#[test]
fn fold2_range_assign_to_selector_or_deref_kills_comparable_contributions() {
    assert_overwritten(
        "func f(p *T, values []A, x int) {\n\tp.a.b = x\n\tfor _, p.a = range values {\n\t\tsink(p)\n\t}\n}\n",
        "prefix overwrite",
    );
    assert_overwritten(
        "func f(p *T, values []T, x int) {\n\tp.f = x\n\tfor _, *p = range values {\n\t\tsink(p)\n\t}\n}\n",
        "deref",
    );
    assert_overwritten(
        "func f(p *T, values []T, x int) {\n\tp.f = x\n\tfor _, (*p) = range values {\n\t\tsink(p)\n\t}\n}\n",
        "parenthesized deref",
    );
    assert_overwritten(
        "func f(p *T, values []int, x int) {\n\tp.f = x\n\tfor _, (p.f) = range values {\n\t\tsink(p)\n\t}\n}\n",
        "parenthesized selector",
    );
    assert_overwritten(
        "func f(p *T, values []int, x int) {\n\tp.f = x\n\tfor p.f = range values {\n\t\tsink(p)\n\t}\n}\n",
        "key target",
    );
    assert_overwritten(
        "func f(p *T, values []int, x int) {\n\tp.a.b = x\n\tfor p.a.b, p.g = range values {\n\t\tsink(p)\n\t}\n}\n",
        "two selector targets",
    );
}

#[test]
fn fold2_receive_into_selector_or_deref_kills_comparable_contributions() {
    for (case, ch) in [
        ("case p.a = <-ch:", "chan A"),
        ("case *p = <-ch:", "chan T"),
        ("case (*p) = <-ch:", "chan T"),
        ("case p.a, ok = <-ch:", "chan A"),
    ] {
        let body = format!(
            "func f(p *T, ch {ch}, ok bool, x int) {{\n\tp.a.b = x\n\tselect {{\n\t{case}\n\t\tsink(p)\n\t}}\n}}\n"
        );
        assert_overwritten(&body, case);
    }
}

#[test]
fn fold2_sibling_and_deeper_header_writes_do_not_kill() {
    // A sibling field survives a selector header write.
    let s = src("func f(p *T, values []int, x int) {
	p.g = x
	for _, p.f = range values {
		sink(p)
	}
}
");
    let rows = recall(&s);
    assert!(has(&rows, ("p.g", 2), ("p", 4)), "{rows:?}");
    // A header write below the contribution does not overwrite it.
    let s = src("func f(p *T, values []int, x int) {
	p.a = A{}
	for _, p.a.b = range values {
		sink(p)
	}
}
");
    let rows = recall(&s);
    assert!(has(&rows, ("p.a", 2), ("p", 4)), "{rows:?}");
    // Receive: sibling survives.
    let s = src("func f(p *T, ch chan int, x int) {
	p.g = x
	select {
	case p.f = <-ch:
		sink(p)
	}
}
");
    assert!(has(&recall(&s), ("p.g", 2), ("p", 5)));
}

#[test]
fn fold2_range_selector_target_keeps_zero_iteration_exit() {
    let s = src("func f(p *T, values []A, x int) {
	p.a.b = x
	for _, p.a = range values {
		sink(p)
	}
	sink(p)
}
");
    let rows = recall(&s);
    assert!(!has(&rows, ("p.a.b", 2), ("p", 4)), "{rows:?}");
    assert!(has(&rows, ("p.a.b", 2), ("p", 6)), "{rows:?}");
}

#[test]
fn fold2_blank_and_indexed_header_targets_kill_nothing() {
    // `_` writes nothing.
    let s = src("func f(p *T, values []int, x int) {
	p.f = x
	for _, _ = range values {
		sink(p)
	}
}
");
    assert!(has(&recall(&s), ("p.f", 2), ("p", 4)));
    // An indexed target is not a static path: it behaves as an ordinary
    // assignment to that target does (no overwrite kill in this lane).
    let header = src("func f(p *T, values []int, x int) {
	p.arr[0] = x
	for _, p.arr[0] = range values {
		sink(p)
	}
}
");
    let ordinary = src("func f(p *T, values []int, x int) {
	p.arr[0] = x
	p.arr[0] = values[0]
	sink(p)
}
");
    assert_eq!(
        recall(&header).is_empty(),
        recall(&ordinary).iter().all(|r| r.1 != 2),
        "{:?} / {:?}",
        recall(&header),
        recall(&ordinary)
    );
}

// ---- population rows not covered above ----

#[test]
fn fold2_remaining_header_forms_and_target_kinds() {
    // range `=`, parenthesized identifier target: peeled to the root.
    assert_overwritten(
        "func f(p *T, ps []*T, x int) {\n\tp.f = x\n\tfor _, (p) = range ps {\n\t\tsink(p)\n\t}\n}\n",
        "range (p)",
    );
    // receive with an ok target, `:=` and `=`.
    assert_overwritten(
        "func f(ch chan *T, p *T, x int) {\n\tp.f = x\n\tfor {\n\t\tselect {\n\t\tcase p, ok := <-ch:\n\t\t\tuse(ok)\n\t\t\tsink(p)\n\t\t}\n\t}\n}\n",
        "receive p, ok :=",
    );
    assert_overwritten(
        "func f(ch chan *T, p *T, ok bool, x int) {\n\tp.f = x\n\tselect {\n\tcase p, ok = <-ch:\n\t\tsink(p)\n\t\tuse(ok)\n\t}\n}\n",
        "receive p, ok =",
    );
    // `_` receive targets write nothing; the contribution survives.
    for case in ["case _ = <-ch:", "case _, _ = <-ch:"] {
        let s = src(&format!(
            "func f(p *T, ch chan int, x int) {{\n\tp.f = x\n\tselect {{\n\t{case}\n\t\tsink(p)\n\t}}\n}}\n"
        ));
        assert!(has(&recall(&s), ("p.f", 2), ("p", 5)), "{case}");
    }
    // Indexed receive target: no static path, same as an ordinary indexed write.
    let s = src("func f(p *T, ch chan int, x int) {
	p.arr[0] = x
	select {
	case p.arr[1] = <-ch:
		sink(p)
	}
}
");
    let ordinary = src("func f(p *T, ch chan int, x int) {
	p.arr[0] = x
	p.arr[1] = <-ch
	sink(p)
}
");
    assert_eq!(
        recall(&s).iter().map(|r| &r.0).collect::<Vec<_>>(),
        recall(&ordinary).iter().map(|r| &r.0).collect::<Vec<_>>()
    );
}
