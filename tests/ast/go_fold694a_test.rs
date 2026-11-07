//! #694 slice A review fold 1: alias-group kills and split-header scope pruning.
//! Static Go sources only; nothing is executed.
use crate::common::*;
use std::collections::BTreeMap;

/// (def line, use line, is_exact) of every labelled `name` pair.
fn labels(src: &str, name: &str) -> Vec<(usize, usize, bool)> {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error());
    let graph = CodePropertyGraph::build(&BTreeMap::from([("t.go".to_owned(), parsed)]));
    graph
        .dfg
        .labels()
        .filter(|(e, _)| e.from.path.base == name)
        .map(|(e, l)| (e.from.line, e.to.line, l.is_exact()))
        .collect()
}

fn exact(l: &[(usize, usize, bool)], d: usize, u: usize) -> bool {
    l.contains(&(d, u, true))
}
fn present(l: &[(usize, usize, bool)], d: usize, u: usize) -> bool {
    l.iter().any(|x| x.0 == d && x.1 == u)
}

#[test]
fn default_clause_writes_kill_the_alias_header_def() {
    let src = "package p\nfunc f(x any, c bool) {\n\tswitch v := x.(type) {\n\tcase string:\n\t\tprintln(v)\n\tdefault:\n\t\tif c {\n\t\t\tv = 1\n\t\t} else {\n\t\t\tv = 2\n\t\t}\n\t\tprintln(v)\n\t}\n}\n";
    let l = labels(src, "v");
    assert!(exact(&l, 3, 5), "clause-1 control: {l:?}");
    assert!(!exact(&l, 3, 12), "header def must be killed: {l:?}");
    assert!(exact(&l, 8, 12) && exact(&l, 10, 12), "{l:?}");
}

#[test]
fn non_default_clause_writes_kill_the_alias_header_def() {
    let src = "package p\nfunc f(x any, c bool) {\n\tswitch v := x.(type) {\n\tcase int:\n\t\tprintln(v)\n\tcase string:\n\t\tif c {\n\t\t\tv = \"a\"\n\t\t} else {\n\t\t\tv = \"b\"\n\t\t}\n\t\tprintln(v)\n\tdefault:\n\t\tprintln(v)\n\t}\n}\n";
    let l = labels(src, "v");
    assert!(exact(&l, 3, 5), "{l:?}");
    assert!(exact(&l, 3, 14), "clause 3 is untouched: {l:?}");
    assert!(!exact(&l, 3, 12), "{l:?}");
    assert!(
        !present(&l, 8, 14) && !present(&l, 10, 14),
        "clause-local: {l:?}"
    );
}

#[test]
fn clause_one_write_control() {
    let src = "package p\nfunc f(x any) {\n\tswitch v := x.(type) {\n\tcase string:\n\t\tv = \"a\"\n\t\tprintln(v)\n\tdefault:\n\t\tprintln(v)\n\t}\n}\n";
    let l = labels(src, "v");
    assert!(!exact(&l, 3, 6), "{l:?}");
    assert!(exact(&l, 5, 6) && exact(&l, 3, 8), "{l:?}");
}

#[test]
fn nested_same_name_switch_keeps_each_group_separate() {
    let src = "package p\nfunc f(x any, y any) {\n\tswitch v := x.(type) {\n\tcase string:\n\t\tprintln(v)\n\tdefault:\n\t\tv = 1\n\t\tswitch v := y.(type) {\n\t\tcase int:\n\t\t\tprintln(v)\n\t\t}\n\t\tprintln(v)\n\t}\n}\n";
    let l = labels(src, "v");
    assert!(exact(&l, 8, 10), "inner header reaches inner use: {l:?}");
    assert!(!exact(&l, 3, 12), "outer header killed by the write: {l:?}");
    assert!(!present(&l, 3, 10), "{l:?}");
    assert!(exact(&l, 7, 12), "{l:?}");
}

#[test]
fn loop_reentry_header_kills_carried_clause_write() {
    let src = "package p\nfunc f(x any, n int) {\n\tfor i := 0; i < n; i++ {\n\t\tswitch v := x.(type) {\n\t\tcase string:\n\t\t\tprintln(v)\n\t\tdefault:\n\t\t\tprintln(v)\n\t\t\tv = 2\n\t\t}\n\t}\n}\n";
    let l = labels(src, "v");
    assert!(exact(&l, 4, 6) && exact(&l, 4, 8), "{l:?}");
    assert!(!exact(&l, 9, 8), "carried write must die at H: {l:?}");
}
