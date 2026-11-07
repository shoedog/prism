//! #694 slice A fold 2: a split header or spec keeps main's behaviour (owner decision,
//! 2026-11-07; the real fix is #728). Every row asserts that the source value reaches the
//! final use through the edges, not only that one pair exists. Static sources only.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::languages::Language;
use std::collections::BTreeSet;

fn refs(src: &str) -> FileRefs {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error());
    Producer::file_refs(&"t.go".to_string(), &parsed)
}

fn has(r: &FileRefs, path: &str, d: usize, u: usize) -> bool {
    r.edges
        .iter()
        .any(|e| e.from.path.to_string() == path && e.from.line == d && e.to.line == u)
}

/// `(path, line)` nodes reachable from `from` over the edges.
fn reaches(r: &FileRefs, from: (&str, usize), to: (&str, usize)) -> bool {
    let mut seen = BTreeSet::from([(from.0.to_owned(), from.1)]);
    let mut work = vec![(from.0.to_owned(), from.1)];
    while let Some((p, l)) = work.pop() {
        for e in &r.edges {
            if e.from.path.to_string() == p && e.from.line == l {
                let next = (e.to.path.to_string(), e.to.line);
                if seen.insert(next.clone()) {
                    work.push(next);
                }
            }
        }
    }
    seen.contains(&(to.0.to_owned(), to.1))
}

fn keeps(name: &str, src: &str, path: &str, d: usize, u: usize) {
    let r = refs(src);
    assert!(has(&r, path, d, u), "{name}: pair {path} {d}->{u} missing");
    assert!(
        reaches(&r, (path, d), (path, u)),
        "{name}: {path}@{d} does not reach {path}@{u}"
    );
}

#[test]
fn nested_split_range_headers_keep_the_outer_value_path() {
    // sol r2 #1: v@3 -> println@8.
    let src = "package p\nfunc f(xs [][]int) {\n\tv := xs\n\tfor _, v := range\n\t\tv {\n\t\tfor _, v := range\n\t\t\tv {\n\t\t\tprintln(v)\n\t\t}\n\t}\n}\n";
    keeps("nested", src, "v", 3, 8);
}

#[test]
fn split_var_spec_in_a_block_keeps_the_outer_pair_with_d_active() {
    // sol r2 #2: no label, so rule D is active; the spec's def is no dominating write.
    let src = "package p\nfunc f() {\n\tb := 7\n\t{\n\t\tvar _, b = 0,\n\t\t\tb\n\t\tprintln(b)\n\t}\n}\n";
    keeps("block-var", src, "b", 3, 7);
}

#[test]
fn split_range_in_a_later_type_switch_clause_keeps_the_alias_def() {
    // Opus r2 W1.
    let src = "package p\nfunc f(x any) {\n\tswitch v := x.(type) {\n\tcase int:\n\t\t_ = v\n\tcase []int:\n\t\tfor _, v := range\n\t\t\tv {\n\t\t\tuse(v)\n\t\t}\n\t}\n}\n";
    keeps("clause2", src, "v", 3, 9);
}

#[test]
fn split_range_over_a_copy_keeps_main_pair() {
    // Opus r2 W2: indirect feeding through `w := v` keeps as main did.
    let src = "package p\nfunc f(xs []int) {\n\tv := xs\n\tw := v\n\tfor _, v := range\n\t\tw {\n\t\tuse(v)\n\t}\n}\n";
    keeps("copy", src, "v", 3, 7);
}

#[test]
fn split_receive_and_alias_keep_with_reachability() {
    let recv = "package p\nfunc f(ch chan int) {\n\tv := ch\n\tselect {\n\tcase v :=\n\t\t<-v:\n\t\tuse(v)\n\t}\n}\n";
    keeps("recv", recv, "v", 3, 7);
    let tsw = "package p\nfunc f(x any) {\n\tt := x\n\tswitch t :=\n\t\tt.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n}\n";
    keeps("alias", tsw, "t", 3, 7);
}

#[test]
fn split_by_a_non_identifier_token_keeps_main_pair() {
    // Any right-hand token on another row splits it, a literal's brace included.
    let src = "package p\nfunc f(xs []int) {\n\tv := xs\n\tfor _, v := range []int{\n\t\t1,\n\t\t2,\n\t} {\n\t\tuse(v)\n\t}\n}\n";
    keeps("literal", src, "v", 3, 8);
}

#[test]
fn single_line_controls_still_drop() {
    let range =
        "package p\nfunc f(xs []int) {\n\tv := xs\n\tfor _, v := range xs {\n\t\tuse(v)\n\t}\n}\n";
    assert!(!has(&refs(range), "v", 3, 5));
    let spec =
        "package p\nfunc f() {\n\tb := 7\n\t{\n\t\tvar _, b = 0, b\n\t\tprintln(b)\n\t}\n}\n";
    assert!(!has(&refs(spec), "b", 3, 6));
    let recv = "package p\nfunc f(ch chan int) {\n\tv := ch\n\tselect {\n\tcase v := <-v:\n\t\tuse(v)\n\t}\n}\n";
    assert!(!has(&refs(recv), "v", 3, 6));
    let tsw = "package p\nfunc f(x any) {\n\tt := x\n\tswitch t := t.(type) {\n\tcase int:\n\t\tuse(t)\n\t}\n}\n";
    assert!(!has(&refs(tsw), "t", 3, 6));
}

#[test]
fn sibling_split_loops_do_not_cross_connect() {
    // The corpus rows that a bare name-level keep added over e8d8e2a: a header's own def is new
    // (main had none), so it is exempt only for the split header that shadows its variable.
    let src = "package p\nfunc f() {\n\tfor _, a := range []string{\n\t\t\"x\",\n\t} {\n\t\tuse(a)\n\t}\n\tfor _, a := range []string{\n\t\t\"y\",\n\t} {\n\t\tuse(a)\n\t}\n}\n";
    let r = refs(src);
    assert!(has(&r, "a", 3, 6) && has(&r, "a", 8, 11));
    assert!(!has(&r, "a", 3, 11), "forward cross pair");
    assert!(!has(&r, "a", 8, 6), "backward cross pair");
}

#[test]
fn main_emitted_var_def_keeps_its_split_exemption() {
    // sol r3 #1: `var v = ..` (the first name of a direct spec) had a def on main, so main's
    // categorical keep applies to it although the block-local `v` is not visible at the header.
    let src = "package p\nfunc f(xs []int) {\n\tvar w []int\n\t{\n\t\tvar v = append([]int(nil), xs...)\n\t\tw = append(w, v...)\n\t}\n\tfor _, v := range\n\t\tw {\n\t\tprintln(v)\n\t}\n}\n";
    keeps("legacy-var", src, "v", 5, 10);
}
