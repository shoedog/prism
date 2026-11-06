//! #662: a use in an `if`/`switch`/`for` header whose initializer declares the
//! name reads the header's own variable, never an outer def of that name.
use super::Producer;
use crate::ast::ParsedFile;
use crate::languages::Language;

fn lines_of(src: &str, marker: &str) -> Vec<usize> {
    src.lines()
        .enumerate()
        .filter(|(_, l)| l.contains(marker))
        .map(|(i, _)| i + 1)
        .collect()
}

/// True when an edge `name` from the line marked `@from` to the line marked `@to` exists.
fn edge(src: &str, name: &str, from: &str, to: &str) -> bool {
    let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
    let refs = Producer::file_refs(&"t.go".to_string(), &parsed);
    let (f, t) = (lines_of(src, from), lines_of(src, to));
    assert!(!f.is_empty() && !t.is_empty(), "markers {from} {to}");
    refs.edges.iter().any(|e| {
        e.from.path.to_string() == name && f.contains(&e.from.line) && t.contains(&e.to.line)
    })
}

#[test]
fn if_init_outer_def_does_not_reach_header() {
    let s = "package p\nfunc f() error {\n\terr := a() // @outer\n\tif err := g(); err != nil { // @hdr\n\t\treturn err // @body\n\t}\n\treturn err // @after\n}\n";
    assert!(!edge(s, "err", "@outer", "@hdr"));
    assert!(edge(s, "err", "@hdr", "@body"), "inner pair unchanged");
    assert!(edge(s, "err", "@outer", "@after"));
    assert!(!edge(s, "err", "@outer", "@body"));
}

#[test]
fn multi_name_init() {
    let s = "package p\nfunc f() error {\n\terr := a() // @outer\n\tif x, err := g(); err != nil { // @hdr\n\t\tuse(x) // @body\n\t}\n\treturn err\n}\n";
    assert!(!edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn blank_in_init() {
    let s = "package p\nfunc f() error {\n\terr := a() // @outer\n\tif _, err := g(); err != nil { // @hdr\n\t\treturn nil\n\t}\n\treturn err\n}\n";
    assert!(!edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn switch_init() {
    let s = "package p\nfunc f() {\n\terr := a() // @outer\n\tswitch err := g(); err { // @hdr\n\tcase nil:\n\t}\n\tuse(err)\n}\n";
    assert!(!edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn for_clause() {
    let s = "package p\nfunc f(n int) {\n\ti := 7 // @outer\n\tfor i := 0; i < n; i++ { // @hdr\n\t\tuse(i) // @body\n\t}\n\tuse(i)\n}\n";
    assert!(!edge(s, "i", "@outer", "@hdr"));
    assert!(edge(s, "i", "@hdr", "@body"), "inner pair unchanged");
    assert!(!edge(s, "i", "@outer", "@body"));
}

#[test]
fn nested_headers() {
    let s = "package p\nfunc f() {\n\terr := z() // @outer\n\tif err := a(); err != nil { // @h1\n\t\tif err := b(); err != nil { // @h2\n\t\t\tuse(err) // @body\n\t\t}\n\t}\n}\n";
    assert!(!edge(s, "err", "@outer", "@h1"));
    assert!(!edge(s, "err", "@outer", "@h2"));
    assert!(!edge(s, "err", "@h1", "@h2"));
    assert!(edge(s, "err", "@h2", "@body"));
}

#[test]
fn header_assignment_is_the_same_binding() {
    let s = "package p\nfunc f() {\n\tvar err error\n\terr = a() // @outer\n\tif err = g(); err != nil { // @hdr\n\t\tuse(err)\n\t}\n}\n";
    assert!(edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn header_declaring_other_name_is_unchanged() {
    let s = "package p\nfunc f() {\n\terr := a() // @outer\n\tif x := g(); err != nil { // @hdr\n\t\tuse(x)\n\t}\n}\n";
    assert!(edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn type_switch_guard_reads_the_operand() {
    let s = "package p\nfunc f() {\n\tx := a() // @outer\n\tswitch v := x.(type) { // @hdr\n\tcase int:\n\t\tuse(v)\n\t}\n}\n";
    assert!(edge(s, "x", "@outer", "@hdr"));
}

#[test]
fn multi_line_header_second_line() {
    let s = "package p\nfunc f() {\n\terr := a() // @outer\n\tif err := g(); err != nil &&\n\t\tis(err) { // @second\n\t\tuse(1)\n\t}\n}\n";
    assert!(!edge(s, "err", "@outer", "@second"));
}

#[test]
fn closure_in_header_reads_header_variable() {
    let s = "package p\nfunc f() {\n\terr := a() // @outer\n\tif err := g(); func() bool { return err != nil }() { // @hdr\n\t\tuse(1)\n\t}\n}\n";
    assert!(!edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn closure_shadowing_the_name_keeps_main_edge() {
    let s = "package p\nfunc f() {\n\terr := a() // @outer\n\tif err := g(); func(err error) bool { return err != nil }(nil) { // @hdr\n\t\tuse(1)\n\t}\n}\n";
    assert!(edge(s, "err", "@outer", "@hdr"));
}

#[test]
fn init_rhs_still_reads_the_outer_name() {
    let s = "package p\nfunc f() {\n\terr := a() // @outer\n\tif err := wrap(err); err != nil { // @hdr\n\t\tuse(1)\n\t}\n}\n";
    // The RHS `err` is an outer read; the line also holds an inner use, which RD cannot
    // separate by line, so main's edge must stay.
    assert!(edge(s, "err", "@outer", "@hdr"));
}
