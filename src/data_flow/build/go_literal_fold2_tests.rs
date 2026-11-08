//! #694 slice B fold 2: rows for the round-2 review findings (a coalesced store through the same
//! declaration, direct callbacks that escape, ReturnSink soundness and its bound-name form).
//! Each row asserts that one def -> use edge is emitted (or not): a direct edge, so a chain through
//! a same-line def/use cannot satisfy it. Fixtures are static Go text; nothing is executed.
//! Lines are 1-indexed in the fixture.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

fn refs(src: &str) -> FileRefs {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "fixture must parse");
    Producer::file_refs(&"t.go".to_string(), &parsed)
}

fn edge(r: &FileRefs, name: &str, from: usize, to: usize) -> bool {
    r.edges.iter().any(|e| {
        e.from.path.to_string() == name
            && e.to.path.to_string() == name
            && e.from.line == from
            && e.to.line == to
    })
}

fn check(label: &str, src: &str, rows: &[(&str, usize, usize, bool)]) {
    let r = refs(src);
    let bad: Vec<String> = rows
        .iter()
        .filter(|(n, d, u, want)| edge(&r, n, *d, *u) != *want)
        .map(|(n, d, u, want)| format!("{label}: {n} {d}->{u} expected edge={want}"))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

// ---- item 1: a store through the same declaration, coalesced with a declare-only def --------

#[test]
fn store_through_a_literal_formal_on_the_signature_line_keeps_the_edge() {
    let src = "package p\nfunc f() {\n\tx := 0\n\tv := &x\n\tfunc(v *int) { *v = 1 }(v)\n\tprintln(*v)\n}\n";
    check("store-formal", src, &[("v", 5, 6, true)]);
}

#[test]
fn store_through_a_named_result_on_the_signature_line_keeps_the_edge() {
    let src = "package p\nfunc f() {\n\tx := 0\n\tv := &x\n\tfunc() (v *int) { *v = 1; return }()\n\tprintln(*v)\n}\n";
    check("store-result", src, &[("v", 5, 6, true)]);
}

// ---- item 2: direct callbacks that escape ----------------------------------------------------

#[test]
fn conversion_of_a_literal_keeps_a_later_call_path() {
    let src = "package p\nfunc f(w W, reqs []R) {\n\th := http.HandlerFunc(func(w W, r R) {\n\t\tserve(w, r)\n\t})\n\tfor _, r := range reqs {\n\t\th(w, r)\n\t}\n}\n";
    check("convert", src, &[("r", 6, 4, true)]);
}

#[test]
fn append_of_a_literal_keeps_a_later_call_path() {
    let src = "package p\nfunc f() {\n\tfs := []func(int){}\n\tfs = append(fs, func(v int) {\n\t\tprintln(v)\n\t})\n\t{\n\t\tv := 7\n\t\tfs[0](v)\n\t}\n}\n";
    check("append", src, &[("v", 8, 5, true)]);
}

#[test]
fn wrapper_with_a_used_result_keeps_a_later_call_path() {
    let src = "package p\nfunc f(ks []int) {\n\th := memo(func(k int) int {\n\t\treturn k * 2\n\t})\n\tfor _, k := range ks {\n\t\th(k)\n\t}\n}\n";
    check("memo", src, &[("k", 6, 4, true)]);
}

#[test]
fn literal_callee_keeps_the_callback_path() {
    let src = "package p\nfunc f(xs []int) {\n\tfunc(cb func(int)) {\n\t\tfor _, x := range xs { cb(x) }\n\t}(func(x int) {\n\t\tuse(x)\n\t})\n}\n";
    check("lit-callee", src, &[("x", 4, 6, true)]);
}

#[test]
fn local_literal_callee_keeps_the_callback_path() {
    let src = "package p\nfunc f(n int) {\n\tapply := func(g func(int)) {\n\t\tfor i := 0; i < n; i++ { g(i) }\n\t}\n\tapply(func(i int) {\n\t\tuse(i)\n\t})\n}\n";
    check("local-callee", src, &[("i", 4, 7, true)]);
}

#[test]
fn statement_callbacks_still_drop_a_def_out_of_creation_scope() {
    // The def is a range variable whose scope ends before the literal exists.
    for (label, call, params) in [
        ("sort", "sort.Slice(xs, func(i int) bool {", "i"),
        ("run", "t.Run(\"n\", func(i int) {", "i"),
        ("walk", "filepath.Walk(root, func(i int) error {", "i"),
        ("visit", "x.Visit(func(i int) {", "i"),
    ] {
        let src = format!(
            "package p\nfunc f(xs []int) {{\n\tfor _, i := range xs {{\n\t\tuse(i)\n\t}}\n\t{call}\n\t\tuse({params})\n\t}})\n}}\n"
        );
        check(label, &src, &[("i", 3, 7, false)]);
    }
}

// ---- item 3: ReturnSink soundness ------------------------------------------------------------

#[test]
fn index_operand_on_the_left_is_not_an_assigned_variable() {
    let src = "package p\nfunc f() {\n\tv := 0\n\txs := []int{0}\n\txs[v] = func() int {\n\t\tv := 9\n\t\treturn v\n\t}()\n\tprintln(v)\n}\n";
    check("lhs-index", src, &[("v", 6, 9, false), ("v", 3, 9, true)]);
}

#[test]
fn map_key_on_the_left_is_not_an_assigned_variable() {
    let src = "package p\nfunc f() {\n\tk := 1\n\tm := map[int]int{}\n\tm[k] = func() int {\n\t\tk := 2\n\t\treturn k\n\t}()\n\tuse(k)\n}\n";
    check("lhs-map", src, &[("k", 6, 9, false)]);
}

#[test]
fn a_goroutine_local_does_not_return_through_the_literal() {
    let src = "package p\nfunc f() {\n\terr := func() error {\n\t\tgo func() {\n\t\t\terr := work()\n\t\t\tlog(err)\n\t\t}()\n\t\treturn nil\n\t}()\n\tuse(err)\n}\n";
    check("go-local", src, &[("err", 5, 10, false)]);
}

#[test]
fn a_read_before_the_statement_outside_a_loop_is_not_fed() {
    let src = "package p\nfunc f() {\n\tvar err error\n\tuse(err)\n\terr = func() error {\n\t\terr := do()\n\t\treturn err\n\t}()\n}\n";
    check("read-before", src, &[("err", 6, 4, false)]);
}

#[test]
fn a_read_before_the_statement_inside_a_loop_is_kept() {
    let src = "package p\nfunc f() {\n\tvar err error\n\tfor i := 0; i < 3; i++ {\n\t\tuse(err)\n\t\terr = func() error {\n\t\t\terr := do()\n\t\t\treturn err\n\t\t}()\n\t}\n}\n";
    check("read-in-loop", src, &[("err", 7, 5, true)]);
}

#[test]
fn a_parenthesized_literal_keeps_the_carrier() {
    let src = "package p\nfunc f() (err error) {\n\terr = (func() error {\n\t\terr := do()\n\t\treturn err\n\t})()\n\tuse(err)\n\treturn\n}\n";
    check("ret-paren", src, &[("err", 4, 7, true)]);
}

#[test]
fn a_nested_in_place_literal_returned_by_the_sink_keeps_the_carrier() {
    let src = "package p\nfunc f() error {\n\tvar err error\n\terr = func() error {\n\t\treturn func() error {\n\t\t\terr := do()\n\t\t\treturn err\n\t\t}()\n\t}()\n\tuse(err)\n\treturn err\n}\n";
    check("ret-nested", src, &[("err", 6, 10, true)]);
}

// ---- item 4: a literal bound once and only called --------------------------------------------

#[test]
fn bound_literal_result_appended_into_the_same_name_keeps_the_carrier() {
    let src = "package p\nfunc f() (errs []error) {\n\tfn := func(x int) (errs []error) {\n\t\terrs = append(errs, check(x))\n\t\treturn\n\t}\n\terrs = append(errs, fn(1)...)\n\treturn errs\n}\n";
    check("bound", src, &[("errs", 4, 7, true), ("errs", 4, 8, true)]);
}

#[test]
fn a_bound_literal_that_escapes_gives_no_carrier() {
    let src = "package p\nfunc f() (errs []error) {\n\tfn := func(x int) (errs []error) {\n\t\terrs = append(errs, check(x))\n\t\treturn\n\t}\n\treg(fn)\n\terrs = append(errs, fn(1)...)\n\treturn errs\n}\n";
    check(
        "bound-escapes",
        src,
        &[("errs", 4, 8, false), ("errs", 4, 9, false)],
    );
}
