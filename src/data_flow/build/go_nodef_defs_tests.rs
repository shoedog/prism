//! #694 slice A: the Go producer emits defs for range/receive `:=` targets,
//! type-switch aliases and the `var`/`const` names it used to skip.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;

fn refs(src: &str) -> FileRefs {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    Producer::file_refs(&"t.go".to_string(), &parsed)
}

fn has(r: &FileRefs, path: &str, def_line: usize, use_line: usize) -> bool {
    r.edges.iter().any(|e| {
        e.from.path.to_string() == path && e.from.line == def_line && e.to.line == use_line
    })
}

/// (path, def line, use line, expected present). Lines are 1-indexed in `src`.
fn check(name: &str, src: &str, rows: &[(&str, usize, usize, bool)]) {
    let r = refs(src);
    let bad: Vec<String> = rows
        .iter()
        .filter(|(p, d, u, want)| has(&r, p, *d, *u) != *want)
        .map(|(p, d, u, want)| format!("{name}: {p} {d}->{u} expected present={want}"))
        .collect();
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn range_targets_get_defs() {
    let src = "package p\nfunc f(xs []int) {\n\tfor k, v := range xs {\n\t\tuse(k, v)\n\t}\n}\n";
    check(
        "range2",
        src,
        &[("xs", 2, 3, true), ("k", 3, 4, true), ("v", 3, 4, true)],
    );
    let n = "package p\nfunc f(n int) {\n\tfor i := range n {\n\t\tuse(i)\n\t}\n}\n";
    check("range-int", n, &[("n", 2, 3, true), ("i", 3, 4, true)]);
    let none = "package p\nfunc f(xs []int) {\n\tfor range xs {\n\t\tuse(1)\n\t}\n}\n";
    check("range-none", none, &[("xs", 2, 3, true)]);
}

#[test]
fn receive_targets_get_defs() {
    let one =
        "package p\nfunc f(ch chan int) {\n\tselect {\n\tcase v := <-ch:\n\t\tuse(v)\n\t}\n}\n";
    check("recv", one, &[("ch", 2, 4, true), ("v", 4, 5, true)]);
    let two =
        "package p\nfunc f(ch chan int) {\n\tselect {\n\tcase v, ok := <-ch:\n\t\tuse(v, ok)\n\t}\n}\n";
    check("recv2", two, &[("v", 4, 5, true), ("ok", 4, 5, true)]);
}

#[test]
fn type_switch_alias_reaches_every_clause() {
    let src = "package p\nfunc f(c any) {\n\tswitch t := c.(type) {\n\tcase int:\n\t\tuse(t)\n\tcase string:\n\t\tuse(t)\n\tdefault:\n\t\tuse(t)\n\t}\n}\n";
    check(
        "typeswitch",
        src,
        &[
            ("c", 2, 3, true),
            ("t", 3, 5, true),
            ("t", 3, 7, true),
            ("t", 3, 9, true),
        ],
    );
}

#[test]
fn var_const_later_names_get_defs() {
    let multi = "package p\nfunc f(x int) {\n\tvar a, b = 1, x\n\tuse(a, b)\n}\n";
    check(
        "var-two",
        multi,
        &[("a", 3, 4, true), ("b", 3, 4, true), ("x", 2, 3, true)],
    );
    let grouped =
        "package p\nfunc f(x int) {\n\tvar (\n\t\ta = 1\n\t\tb = x\n\t)\n\tuse(a, b)\n}\n";
    check(
        "var-group",
        grouped,
        &[("a", 4, 7, true), ("b", 5, 7, true), ("x", 2, 5, true)],
    );
    let later = "package p\nfunc f(x int) {\n\tvar a = 1\n\tvar b = x\n\tuse(a, b)\n}\n";
    check(
        "var-later-decl",
        later,
        &[("a", 3, 5, true), ("b", 4, 5, true)],
    );
    let konst = "package p\nfunc f() {\n\tconst (\n\t\tA = iota\n\t\tB\n\t)\n\tuse(A, B)\n}\n";
    check(
        "const-implicit",
        konst,
        &[("A", 4, 7, true), ("B", 5, 7, true)],
    );
    let call = "package p\nfunc f() {\n\tvar a, b = g()\n\tuse(a, b)\n}\n";
    check("var-call", call, &[("a", 3, 4, true), ("b", 3, 4, true)]);
}

#[test]
fn self_reading_and_independent_guards_drop_outer_pairs() {
    // The value now reaches the body through the new def, so S may drop the
    // outer def -> body use pair.
    let ts = "package p\nfunc f(c any) {\n\tv := c\n\tswitch v := v.(type) {\n\tcase int:\n\t\tuse(v)\n\t}\n}\n";
    check(
        "typeswitch-self",
        ts,
        &[("v", 3, 4, true), ("v", 3, 6, false), ("v", 4, 6, true)],
    );
    let rg =
        "package p\nfunc f(c []int) {\n\tv := c\n\tfor _, v := range v {\n\t\tuse(v)\n\t}\n}\n";
    check(
        "range-self",
        rg,
        &[("v", 3, 4, true), ("v", 3, 5, false), ("v", 4, 5, true)],
    );
    let paren = "package p\nfunc f() {\n\tv := any(7)\n\tc := (v)\n\tswitch v := c.(type) {\n\tcase int:\n\t\tprintln(v)\n\t}\n}\n";
    check(
        "typeswitch-paren-copy",
        paren,
        &[("v", 3, 7, false), ("v", 5, 7, true)],
    );
    let rc = "package p\nfunc f(c chan int) {\n\tv := c\n\tselect {\n\tcase v := <-v:\n\t\tuse(v)\n\t}\n}\n";
    check(
        "receive-self",
        rc,
        &[("v", 3, 5, true), ("v", 3, 6, false), ("v", 5, 6, true)],
    );
    let indep = "package p\nfunc f(c any) {\n\tv := 1\n\tswitch v := c.(type) {\n\tcase int:\n\t\tuse(v)\n\t}\n}\n";
    check(
        "typeswitch-indep",
        indep,
        &[("v", 3, 6, false), ("v", 4, 6, true)],
    );
    let var2 =
        "package p\nfunc f() {\n\tv := 7\n\t{\n\t\tvar _, v = 0, v\n\t\tprintln(v)\n\t}\n}\n";
    check(
        "var-second-name",
        var2,
        &[("v", 3, 5, true), ("v", 3, 6, false), ("v", 5, 6, true)],
    );
}

#[test]
fn function_literal_formals_and_named_results_stay_exempt() {
    let lit = "package p\nfunc f() {\n\tv := 7\n\tfunc(v int) {\n\t\tprintln(v)\n\t}(v)\n}\n";
    check("literal-formal", lit, &[("v", 3, 5, true)]);
    let res = "package p\nfunc f() (v int) {\n\tdefer func() { println(v) }()\n\t{\n\t\tv := 7\n\t\tc := (v)\n\t\treturn c\n\t}\n}\n";
    check("named-result", res, &[("v", 5, 3, true)]);
}
