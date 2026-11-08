//! #694 slice B fold 3: ReturnSink is cut and the pairs it protected take main's behaviour
//! (a def nested in a callable inside the use's callable, for a name main blanket-kept), and the
//! creation-scope drop for direct callbacks applies only to owners with no other literal able to
//! receive a function value. Each row asserts that one def -> use edge is emitted (or not): a
//! direct edge. Fixtures are static Go text; nothing is executed. Lines are 1-indexed.
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

// ---- parity: a main-blanket name keeps a def nested inside the use's callable ----------------

#[test]
fn a_deferred_recover_setting_the_carriers_named_result_keeps_main_edge() {
    let src = "package p\nfunc f() {\n\tvar err error\n\terr = func() (err error) {\n\t\tdefer func() {\n\t\t\tif r := recover(); r != nil {\n\t\t\t\terr = wrap(r)\n\t\t\t}\n\t\t}()\n\t\twork()\n\t\treturn nil\n\t}()\n\tprintln(err)\n}\n";
    check("recover", src, &[("err", 7, 13, true)]);
}

#[test]
fn an_in_place_statement_literal_setting_the_carriers_result_keeps_main_edge() {
    let src = "package p\nfunc f() {\n\tvar err error\n\terr = func() (err error) {\n\t\tfunc() {\n\t\t\terr = do()\n\t\t}()\n\t\treturn\n\t}()\n\tprintln(err)\n}\n";
    check("stmt-literal", src, &[("err", 6, 10, true)]);
}

#[test]
fn a_literal_inside_a_returned_call_keeps_main_edge() {
    let src = "package p\nfunc f() {\n\tvar err error\n\terr = func() (err error) {\n\t\treturn wrap(func() error {\n\t\t\terr := do()\n\t\t\treturn err\n\t\t}())\n\t}()\n\tprintln(err)\n}\n";
    check("wrap-return", src, &[("err", 6, 10, true)]);
}

#[test]
fn a_bound_literal_that_is_also_passed_keeps_main_edges() {
    let src = "package p\nfunc f() (errs []error) {\n\tfn := func(x int) (errs []error) {\n\t\terrs = append(errs, check(x))\n\t\treturn\n\t}\n\treg(fn)\n\terrs = append(errs, fn(1)...)\n\treturn errs\n}\n";
    check(
        "bound-escapes",
        src,
        &[("errs", 4, 8, true), ("errs", 4, 9, true)],
    );
}

#[test]
fn a_literal_bound_inside_another_literal_keeps_main_edges() {
    let src = "package p\nfunc f() {\n\trun := func() (errs []error) {\n\t\tfn := func(x int) (errs []error) {\n\t\t\terrs = append(errs, check(x))\n\t\t\treturn\n\t\t}\n\t\terrs = append(errs, fn(1)...)\n\t\treturn errs\n\t}\n\tprintln(len(run()))\n}\n";
    check(
        "bound-nested",
        src,
        &[("errs", 5, 8, true), ("errs", 5, 9, true)],
    );
}

// ---- parity: no addition for a name main dropped ----------------------------------------------

#[test]
fn a_block_shadowed_local_the_literal_does_not_return_adds_no_edge() {
    let src = "package p\nfunc f() {\n\tv := func() int {\n\t\tv := 0\n\t\t{\n\t\t\tv := 9\n\t\t\tprintln(v)\n\t\t}\n\t\treturn v\n\t}()\n\tprintln(v)\n}\n";
    check("sol1-shadow", src, &[("v", 6, 11, false)]);
}

#[test]
fn an_always_killed_local_adds_no_edge() {
    let src = "package p\nfunc f() {\n\tv := func() int {\n\t\tv := 9\n\t\tv = 0\n\t\treturn v\n\t}()\n\tprintln(v)\n}\n";
    check("sol1-killed", src, &[("v", 4, 8, false)]);
}

#[test]
fn an_if_init_shadow_in_a_literal_returning_another_err_adds_no_edge() {
    let src = "package p\nfunc f(xs []int) {\n\terr := func() error {\n\t\terr := prepare()\n\t\tfor _, x := range xs {\n\t\t\tif err := check(x); err != nil {\n\t\t\t\tlog(err)\n\t\t\t}\n\t\t}\n\t\treturn err\n\t}()\n\tprintln(err)\n}\n";
    check("opus1-shadow", src, &[("err", 6, 12, false)]);
}

#[test]
fn a_bound_call_resolving_to_a_shadowing_parameter_adds_no_edge() {
    let src = "package p\nfunc f(fn func() int) {\n\t{\n\t\tfn := func() int {\n\t\t\tv := 9\n\t\t\treturn v\n\t\t}\n\t\tfn()\n\t}\n\tv := fn()\n\tprintln(v)\n}\n";
    check("sol2-param", src, &[("v", 5, 11, false)]);
}

// ---- follow-up: ordinary-name return flow is not modelled (as on main) -----------------------

// follow-up: ordinary-name return flow
#[test]
fn ordinary_name_return_flow_out_of_an_iife_is_not_modelled() {
    let src = "package p\nfunc f() {\n\terr := func() error {\n\t\terr := do()\n\t\treturn err\n\t}()\n\tuse(err)\n}\n";
    check("ordinary-iife", src, &[("err", 4, 7, false)]);
}

// follow-up: ordinary-name return flow
#[test]
fn ordinary_name_return_flow_through_a_bound_literal_is_not_modelled() {
    let src = "package p\nfunc f() {\n\tpreparation := func() error {\n\t\terr := do()\n\t\treturn err\n\t}\n\terr := preparation()\n\tuse(err)\n}\n";
    check("ordinary-bound", src, &[("err", 4, 8, false)]);
}

// ---- guards that still hold -------------------------------------------------------------------

#[test]
fn a_goroutine_local_does_not_return_through_the_literal() {
    let src = "package p\nfunc f() {\n\terr := func() error {\n\t\tgo func() {\n\t\t\terr := work()\n\t\t\tlog(err)\n\t\t}()\n\t\treturn err\n\t}()\n\tuse(err)\n}\n";
    check("go-local", src, &[("err", 5, 10, false)]);
}

#[test]
fn an_index_operand_and_a_map_key_on_the_left_add_no_edge() {
    let src = "package p\nfunc f() {\n\tv := 0\n\txs := []int{0}\n\txs[v] = func() int {\n\t\tv := 9\n\t\treturn v\n\t}()\n\tprintln(v)\n}\n";
    check("lhs-index", src, &[("v", 6, 9, false), ("v", 3, 9, true)]);
    let src = "package p\nfunc f() {\n\tk := 1\n\tm := map[int]int{}\n\tm[k] = func() int {\n\t\tk := 2\n\t\treturn k\n\t}()\n\tuse(k)\n}\n";
    check("lhs-map", src, &[("k", 6, 9, false)]);
}

#[test]
fn a_read_before_the_statement_is_not_fed() {
    let src = "package p\nfunc f() {\n\tvar err error\n\tuse(err)\n\terr = func() error {\n\t\terr := do()\n\t\treturn err\n\t}()\n}\n";
    check("read-before", src, &[("err", 6, 4, false)]);
}

// ---- direct callbacks: an owner literal that may hold the callee ------------------------------

#[test]
fn a_package_function_variable_assigned_an_owner_literal_keeps_the_callback_path() {
    let src = "package p\nvar apply func(func(int))\nfunc f() {\n\tapply = func(cb func(int)) {\n\t\tfor i := 0; i < 1; i++ {\n\t\t\tcb(i)\n\t\t}\n\t}\n\tapply(func(i int) {\n\t\tprintln(i)\n\t})\n}\n";
    check("pkg-var", src, &[("i", 5, 10, true)]);
}

#[test]
fn an_indexed_callee_holding_an_owner_literal_keeps_the_callback_path() {
    let src = "package p\nfunc f(xs []int) {\n\tfs := []func(func(int)){\n\t\tfunc(cb func(int)) {\n\t\t\tfor _, v := range xs {\n\t\t\t\tcb(v)\n\t\t\t}\n\t\t},\n\t}\n\tfs[0](func(v int) {\n\t\tprintln(v)\n\t})\n}\n";
    check("index-callee", src, &[("v", 5, 11, true)]);
}

#[test]
fn a_field_callee_assigned_an_owner_literal_keeps_the_callback_path() {
    let src = "package p\nfunc f(xs []int) {\n\tvar s struct{ each func(func(int)) }\n\ts.each = func(cb func(int)) {\n\t\tfor _, v := range xs {\n\t\t\tcb(v)\n\t\t}\n\t}\n\ts.each(func(v int) {\n\t\tprintln(v)\n\t})\n}\n";
    check("field-callee", src, &[("v", 5, 10, true)]);
}

#[test]
fn an_aliased_field_callee_keeps_the_callback_path() {
    let src = "package p\nfunc f(xs []int) {\n\tvar s struct{ each func(func(int)) }\n\tg := s\n\ts.each = func(cb func(int)) {\n\t\tfor _, v := range xs {\n\t\t\tcb(v)\n\t\t}\n\t}\n\tg.each(func(v int) {\n\t\tprintln(v)\n\t})\n}\n";
    check("alias-callee", src, &[("v", 6, 11, true)]);
}

#[test]
fn an_unassigned_package_function_variable_still_drops() {
    let src = "package p\nvar hook func(func(int))\nfunc f(xs []int) {\n\tfor _, i := range xs {\n\t\tuse(i)\n\t}\n\thook(func(i int) {\n\t\tuse(i)\n\t})\n}\n";
    check("pkg-var-unassigned", src, &[("i", 4, 8, false)]);
}

#[test]
fn a_callback_with_its_own_function_formal_still_drops() {
    // The only literal able to receive a function value is the callback itself.
    let src = "package p\nfunc f(xs []int) {\n\tfor _, i := range xs {\n\t\tuse(i)\n\t}\n\tx.Visit(func(i int, done func()) {\n\t\tuse(i)\n\t})\n}\n";
    check("self-formal", src, &[("i", 3, 7, false)]);
}

#[test]
fn another_literal_with_a_basic_or_pointer_formal_does_not_open_the_gate() {
    let src = "package p\nfunc f(xs []int) {\n\tfor _, i := range xs {\n\t\tuse(i)\n\t}\n\tg := func(n int, p *T, m map[string]int, s []int) {}\n\tx.Visit(func(i int) {\n\t\tuse(i)\n\t})\n\tg(1, nil, nil, nil)\n}\n";
    check("gate-closed", src, &[("i", 3, 8, false)]);
}

#[test]
fn another_literal_with_an_interface_or_named_formal_opens_the_gate() {
    for (label, formal) in [
        ("any", "a any"),
        ("iface", "a interface{ M() }"),
        ("named", "a T"),
        ("qualified", "a pkg.T"),
        ("error", "e error"),
        ("variadic", "a ...any"),
        ("func", "a func()"),
    ] {
        let src = format!(
            "package p\nfunc f(xs []int) {{\n\tfor _, i := range xs {{\n\t\tuse(i)\n\t}}\n\tg := func({formal}) {{}}\n\tx.Visit(func(i int) {{\n\t\tuse(i)\n\t}})\n\tg(nil)\n}}\n"
        );
        check(label, &src, &[("i", 3, 8, true)]);
    }
}
