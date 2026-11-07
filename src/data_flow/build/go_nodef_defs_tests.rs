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

#[test]
fn header_lines_that_only_declare_are_write_only() {
    // Rule U: a header that merely declares the name is not a use of an earlier clause's def.
    let sel = "package p\nfunc f(ch chan int) {\n\tselect {\n\tcase _, ok := <-ch:\n\t\tsink(ok)\n\tcase _, ok := <-ch:\n\t\tsink(ok)\n\t}\n}\n";
    check(
        "select-two-clauses",
        sel,
        &[
            ("ok", 4, 5, true),
            ("ok", 4, 6, false),
            ("ok", 6, 4, false),
            ("ok", 6, 7, true),
        ],
    );
    let two = "package p\nfunc f(a, b []int) {\n\tfor _, v := range a {\n\t\tuse(v)\n\t}\n\tfor _, v := range b {\n\t\tuse(v)\n\t}\n}\n";
    check(
        "range-twice",
        two,
        &[
            ("v", 3, 4, true),
            ("v", 3, 6, false),
            ("v", 3, 7, false),
            ("v", 6, 7, true),
            ("v", 6, 4, false),
        ],
    );
    let ts = "package p\nfunc f(c, d any) {\n\tswitch t := c.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n\tswitch t := d.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n}\n";
    check(
        "typeswitch-twice",
        ts,
        &[
            ("t", 3, 5, true),
            ("t", 3, 7, false),
            ("t", 3, 9, false),
            ("t", 7, 9, true),
            ("t", 7, 5, false),
        ],
    );
}

#[test]
fn an_arm_write_does_not_reach_a_sibling_clause_alias() {
    // The alias group belongs to the header def only: `v = 1` writes the default clause's
    // own declaration, which no other clause reads.
    let src = "package p\nfunc f(n int, y any) {\n\tfor n > 0 {\n\t\tswitch v := y.(type) {\n\t\tcase int:\n\t\t\tuse(v)\n\t\tdefault:\n\t\t\tv = 1\n\t\t}\n\t}\n}\n";
    check("arm-write", src, &[("v", 4, 6, true), ("v", 8, 6, false)]);
}

#[test]
fn header_targets_mint_no_alias_twin() {
    // `p := q` aliases `p` flow-insensitively; a later range target named `p` is a new variable,
    // not a write to `q`.
    let src = "package p\nfunc f(q int, xs []int) {\n\tp := q\n\tuse(p)\n\tfor _, p := range xs {\n\t\tuse(p)\n\t}\n\tuse(q)\n}\n";
    check("no-twin", src, &[("p", 5, 6, true), ("q", 5, 8, false)]);
}

#[test]
fn header_defs_are_scoped_and_never_dominate_code_after_them() {
    // Rule D: a range/receive target write does not dominate code after the loop or select, and
    // the outer variable keeps reaching it; the target's def does not reach past its clause.
    let rg = "package p\nfunc f(xs []int) {\n\tv := 1\n\tfor _, v := range xs {\n\t\tuse(v)\n\t}\n\tuse(v)\n}\n";
    check(
        "range-after",
        rg,
        &[
            ("v", 3, 7, true),
            ("v", 4, 5, true),
            ("v", 4, 7, false),
            ("v", 3, 5, false),
        ],
    );
    let rc = "package p\nfunc f(ch chan int) {\n\tv := 1\n\tselect {\n\tcase v := <-ch:\n\t\tuse(v)\n\t}\n\tuse(v)\n}\n";
    check(
        "receive-after",
        rc,
        &[
            ("v", 3, 8, true),
            ("v", 5, 6, true),
            ("v", 5, 8, false),
            ("v", 3, 6, false),
        ],
    );
    let ts = "package p\nfunc f(c any) {\n\tt := 1\n\tswitch t := c.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n\tuse(t)\n}\n";
    check(
        "typeswitch-after",
        ts,
        &[
            ("t", 3, 8, true),
            ("t", 4, 6, true),
            ("t", 4, 8, false),
            ("t", 3, 6, false),
        ],
    );
    // Rule U: a header that also reads the name is not write-only.
    let rd = "package p\nfunc f(v []int) {\n\tfor _, v := range v {\n\t\tuse(v)\n\t}\n}\n";
    check(
        "range-self-read",
        rd,
        &[("v", 2, 3, true), ("v", 3, 4, true)],
    );
}

#[test]
fn later_var_names_mint_no_alias_twin() {
    // `b = a` aliases `b` to `a` flow-insensitively; `var a, b` declares a fresh `b`, which is not
    // a write to `a`, so no twin def of `a` appears on the declaration line.
    let src = "package p\nfunc f(x bool) {\n\tvar a, b, c []int\n\tif x {\n\t\tb = a\n\t}\n\tuse(b, c)\n}\n";
    let r = refs(src);
    assert!(has(&r, "a", 3, 5));
    assert!(r.labels.iter().any(|((f, t), l)| f.path.to_string() == "a"
        && f.line == 3
        && t.line == 5
        && *l == crate::cpg::FlowConfidence::Exact));
}

#[test]
fn declare_only_headers_in_closures_are_write_only_too() {
    // Rule U keeps its nested-callable caution for ordinary writes, but a header that only
    // declares the name holds no read wherever it sits.
    let src = "package p\nfunc f(xs []int) {\n\tg := func() {\n\t\tfor _, ct := range xs {\n\t\t\tuse(ct)\n\t\t}\n\t}\n\th := func() {\n\t\tfor _, ct := range xs {\n\t\t\tuse(ct)\n\t\t}\n\t}\n\tg()\n\th()\n}\n";
    check(
        "closures",
        src,
        &[
            ("ct", 4, 5, true),
            ("ct", 4, 9, false),
            ("ct", 9, 10, true),
            ("ct", 9, 4, false),
            ("ct", 9, 5, false),
        ],
    );
}

// Fold 1 (#694a review): a header or spec split across lines has no connected replacement
// path, so rule S keeps the outer def's pair with the body use.
#[test]
fn split_headers_keep_the_outer_pair() {
    let range = "package p\nfunc f(xs []int) {\n\tv := xs\n\tfor _, v := range\n\t\tv {\n\t\tuse(v)\n\t}\n}\n";
    check("split-range", range, &[("v", 3, 6, true)]);
}

#[test]
fn split_receive_keeps_the_outer_pair() {
    let recv = "package p\nfunc f(ch chan int) {\n\tv := 1\n\tselect {\n\tcase v :=\n\t\t<-ch:\n\t\tuse(v)\n\t}\n}\n";
    check("split-recv", recv, &[("v", 3, 7, true)]);
}

#[test]
fn split_type_switch_keeps_the_outer_pair() {
    let tsw = "package p\nfunc f(x any) {\n\tt := 1\n\tswitch t :=\n\t\tx.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n}\n";
    check("split-tswitch", tsw, &[("t", 3, 7, true)]);
}

#[test]
fn single_line_headers_still_drop_the_outer_pair() {
    let range =
        "package p\nfunc f(xs []int) {\n\tv := xs\n\tfor _, v := range xs {\n\t\tuse(v)\n\t}\n}\n";
    check("one-range", range, &[("v", 3, 5, false)]);
    let recv = "package p\nfunc f(ch chan int) {\n\tv := 1\n\tselect {\n\tcase v := <-ch:\n\t\tuse(v)\n\t}\n}\n";
    check("one-recv", recv, &[("v", 3, 6, false)]);
    let tsw = "package p\nfunc f(x any) {\n\tt := 1\n\tswitch t := x.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n}\n";
    check("one-tswitch", tsw, &[("t", 3, 6, false)]);
}
