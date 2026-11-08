//! #694 slice B fold 1: rows for the round-1 review findings (same-line coalescing, callbacks
//! that are not direct arguments, in-place literals, values returned out of in-place literals).
//! Each row asserts reachability over the emitted edges, as in `go_literal_defs_tests`.
//! Fixtures are static Go text; nothing is executed. Lines are 1-indexed in the fixture.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::{BTreeSet, VecDeque};

fn refs(src: &str) -> FileRefs {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "fixture must parse");
    Producer::file_refs(&"t.go".to_string(), &parsed)
}

fn reach(r: &FileRefs, from: (&str, usize), to: (&str, usize)) -> bool {
    let start = (from.0.to_owned(), from.1);
    let goal = (to.0.to_owned(), to.1);
    let mut seen = BTreeSet::from([start.clone()]);
    let mut queue = VecDeque::from([start]);
    while let Some(node) = queue.pop_front() {
        for e in &r.edges {
            if (e.from.path.to_string(), e.from.line) == node {
                let next = (e.to.path.to_string(), e.to.line);
                if next == goal {
                    return true;
                }
                if seen.insert(next.clone()) {
                    queue.push_back(next);
                }
            }
        }
    }
    false
}

fn check(label: &str, src: &str, rows: &[(&str, usize, usize, bool)]) {
    let r = refs(src);
    let bad: Vec<String> = rows
        .iter()
        .filter(|(n, d, u, want)| reach(&r, (n, *d), (n, *u)) != *want)
        .map(|(n, d, u, want)| format!("{label}: {n} {d}->{u} expected reach={want}"))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// ---- item 1: same-line coalescing ------------------------------------------------------

#[test]
fn formal_then_ordinary_write_on_one_line_keeps_the_ordinary_edge() {
    let src = "package p\nfunc f() {\n\tfunc(v int) { println(v) }(0); v := 7\n\tprintln(v)\n}\n";
    check("coalesce-formal-first", src, &[("v", 3, 4, true)]);
}

#[test]
fn ordinary_then_formal_write_on_one_line_keeps_the_ordinary_edge() {
    let src = "package p\nfunc f() {\n\tv := 7; func(v int) { println(v) }(0)\n\tprintln(v)\n}\n";
    check("coalesce-ordinary-first", src, &[("v", 3, 4, true)]);
}

#[test]
fn named_result_then_ordinary_write_on_one_line_keeps_the_ordinary_edge() {
    let src = "package p\nfunc f() {\n\tfunc() (v int) { return 1 }(); v := 7\n\tprintln(v)\n}\n";
    check("coalesce-result", src, &[("v", 3, 4, true)]);
}

// ---- item 2: literals that are not direct call arguments -------------------------------

#[test]
fn assigned_callback_called_from_if_inits_keeps_both_paths() {
    let src = "package p\nfunc f(a, b func() error) {\n\thandle := func(err error) {\n\t\tlog(err)\n\t}\n\tif err := a(); err != nil {\n\t\thandle(err)\n\t}\n\tif err := b(); err != nil {\n\t\thandle(err)\n\t}\n}\n";
    check("handle", src, &[("err", 6, 4, true), ("err", 9, 4, true)]);
}

#[test]
fn assigned_callback_called_from_a_range_loop_keeps_the_path() {
    let src = "package p\nfunc f(items []int) {\n\tprocess := func(item int) {\n\t\tuse(item)\n\t}\n\tfor _, item := range items {\n\t\tprocess(item)\n\t}\n}\n";
    check("process", src, &[("item", 6, 4, true)]);
}

#[test]
fn assigned_callback_called_with_a_block_local_keeps_the_path() {
    let src = "package p\nfunc f() {\n\tg := func(v int) {\n\t\tprintln(v)\n\t}\n\t{\n\t\tv := 7\n\t\tg(v)\n\t}\n}\n";
    check("block-local", src, &[("v", 7, 4, true)]);
}

#[test]
fn field_stored_callback_keeps_the_path() {
    let src = "package p\nfunc f(o *T) {\n\to.cb = func(v int) {\n\t\tuse(v)\n\t}\n\t{\n\t\tv := 1\n\t\to.cb(v)\n\t}\n}\n";
    check("field-cb", src, &[("v", 7, 4, true)]);
}

// ---- item 3: in-place literals ---------------------------------------------------------

#[test]
fn in_place_literal_called_with_a_derived_argument_keeps_the_outer_pair() {
    let src = "package p\nfunc f() {\n\tv := 7\n\targ := v + 1\n\tfunc(v int) {\n\t\tprintln(v)\n\t}(arg)\n}\n";
    check("iife-derived", src, &[("v", 3, 6, true)]);
}

// ---- item 4: a value returned out of an in-place literal -------------------------------

#[test]
fn if_init_literal_returning_a_local_into_the_same_name_keeps_the_carrier() {
    let src = "package p\nfunc f() (err error) {\n\tif err := func() error {\n\t\terr := do()\n\t\treturn err\n\t}(); err != nil {\n\t\treturn err\n\t}\n\treturn nil\n}\n";
    check("ret-if", src, &[("err", 4, 6, true), ("err", 4, 7, true)]);
}

#[test]
fn literal_named_result_assigned_into_the_same_name_keeps_the_carrier() {
    let src = "package p\nfunc f() error {\n\tvar err error\n\terr = func() (err error) {\n\t\terr = do()\n\t\treturn\n\t}()\n\tuse(err)\n\treturn err\n}\n";
    check("ret-result", src, &[("err", 5, 8, true)]);
}

#[test]
fn literal_formal_updated_and_returned_into_the_same_name_keeps_the_carrier() {
    let src = "package p\nfunc f(v int) {\n\tv = func(v int) int {\n\t\tv++\n\t\treturn v\n\t}(v)\n\tuse(v)\n}\n";
    check("ret-formal", src, &[("v", 4, 7, true)]);
}

#[test]
fn a_literal_whose_value_goes_to_another_name_still_drops() {
    let src = "package p\nfunc f(x int) {\n\tv := x\n\tw := func() int {\n\t\tv := 2\n\t\treturn v\n\t}()\n\tuse(v, w)\n}\n";
    check(
        "ret-other-name",
        src,
        &[("v", 5, 8, false), ("v", 3, 8, true)],
    );
}
