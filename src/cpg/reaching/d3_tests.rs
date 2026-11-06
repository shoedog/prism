//! #660 S1b-2: D3's trace mechanisms, its trace-off neutrality, and its availability neutrality.
use super::census_defs_tests::trace_lines;
use crate::languages::Language;

const JS: Language = Language::JavaScript;

fn d3_lines(src: &str) -> Vec<String> {
    trace_lines(JS, src)
        .into_iter()
        .filter(|l| l.split('\t').nth(1).is_some_and(|m| m.starts_with("d3-")))
        .collect()
}

fn mechanisms(src: &str) -> Vec<String> {
    d3_lines(src)
        .iter()
        .map(|l| l.split('\t').nth(1).unwrap().to_string())
        .collect()
}

#[test]
fn each_d3_mechanism_is_named() {
    for (src, want) in [
        // The order is plain: a write before the read in one node.
        ("function f(a) {\n  log();\n  a++, sink(a);\n}\n", "d3-use"),
        // A write after the def in its node (def side).
        (
            "function f(b) {\n  let a = b;\n  a = 1, a++;\n  sink(a);\n}\n",
            "d3-use",
        ),
        // A loop on the chain: no proof either way.
        (
            "function f(a) {\n  log();\n  while (c()) { sink(a); a = src(); }\n}\n",
            "d3-loop",
        ),
        // An unknown kind (a switch clause) on the chain.
        (
            "function f(a, k) {\n  log();\n  switch (k) { case 1: sink(a); case a++: break; }\n}\n",
            "d3-unknown",
        ),
    ] {
        let got = mechanisms(src);
        assert!(!got.is_empty(), "{want}: no d3 line for {src}");
        assert!(got.iter().any(|m| m == want), "{want}: {got:?} for {src}");
    }
}

#[test]
fn the_def_side_has_its_own_mechanism() {
    // `a = 1` (D) is followed by `a++` in its node: D does not leave the node (d3-def).
    let src = "function f(b) {\n  let a = b;\n  a = 1, a++;\n  sink(a);\n}\n";
    assert!(mechanisms(src).iter().any(|m| m == "d3-def"), "{src}");
}

#[test]
fn a_proven_order_leaves_no_d3_line() {
    for src in [
        "function f(a) {\n  log();\n  sink(a), a++;\n}\n",
        "function f(a) {\n  log();\n  a = g(a);\n}\n",
        "function f(a) {\n  log();\n  while (c()) a++;\n  sink(a);\n}\n",
    ] {
        assert!(d3_lines(src).is_empty(), "{src}: {:?}", d3_lines(src));
    }
}

#[test]
fn a_d3_line_names_the_refused_edge() {
    let lines = d3_lines("function f(a) {\n  log();\n  a++, sink(a);\n}\n");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\ta@1:") && l.contains("\ta@3:")),
        "{lines:?}"
    );
}

/// The counting writes D3 keeps for the owner of `src`: (node line, path) pairs, and the unplaced bases.
fn counting(language: Language, src: &str) -> (Vec<(usize, String)>, Vec<String>) {
    use super::census_defs_tests::{parse, recorded};
    let parsed = parse(language, src);
    let owner = parsed.all_functions().into_iter().next().unwrap();
    let (defs, _) = recorded(&parsed, &owner);
    let (_, facts) = super::census_defs::complete(&parsed, &owner, defs);
    let Some(d3) = facts.d3 else {
        return (Vec::new(), Vec::new());
    };
    let counts = d3.counts();
    let mut by: Vec<(usize, String)> = counts
        .by_node
        .iter()
        .flat_map(|(n, ws)| ws.iter().map(move |w| (*n, w.path.to_string())))
        .collect();
    by.sort();
    (by, counts.unplaced.iter().cloned().collect())
}

#[test]
fn counting_writes_are_the_eager_writes_that_store_a_value() {
    for (src, want) in [
        // An update and a plain write are counted at their node.
        (
            "function f(a) {\n  log();\n  a++, a = 1;\n  sink(a);\n}\n",
            vec![(3, "a"), (3, "a")],
        ),
        // A write in a nested callable and in an instance field initializer is D2-c's.
        (
            "function f(a) {\n  log();\n  h(() => { a++; });\n  sink(a);\n}\n",
            vec![],
        ),
        (
            "function f(a) {\n  log();\n  class C { x = (a = 1); }\n  sink(a);\n}\n",
            vec![],
        ),
        // A hoisted function declaration and `var a;` store nothing at their position.
        (
            "function f(a) {\n  log();\n  function a() {}\n  var a;\n  sink(a);\n}\n",
            vec![],
        ),
        // `this` inside a static block is another object.
        (
            "function f() {\n  this.x = 1;\n  class C { static { this.x++; } }\n  sink(this.x);\n}\n",
            vec![(2, "this.x")],
        ),
        // A `let` declarator with a value is a write at its statement.
        (
            "function f() {\n  log();\n  let b = 1;\n  sink(b);\n}\n",
            vec![(3, "b")],
        ),
    ] {
        let want: Vec<(usize, String)> = want.into_iter().map(|(n, p)| (n, p.to_string())).collect();
        let (got, unplaced) = counting(JS, src);
        let _ = unplaced;
        assert_eq!(got, want, "{src}");
    }
}

#[test]
fn a_counting_write_with_no_node_marks_its_base_unplaced() {
    // A write in a class body outside any statement of the owner (a static field value) has no node
    // to name: the base's D3 queries are unknown rather than silently unordered.
    let src = "function f(a) {\n  log();\n  class C { static x = (a = 5); }\n  sink(a);\n}\n";
    let (by, unplaced) = counting(JS, src);
    assert!(by.is_empty() && unplaced == ["a"], "{by:?} {unplaced:?}");
}
