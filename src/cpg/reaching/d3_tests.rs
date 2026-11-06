//! #660 S1b-2: D3's trace mechanisms, trace neutrality (`trace_lines` asserts the traced outcome equals
//! the plain one) and availability neutrality (D3 on vs off), over every matrix fixture.
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

#[test]
fn the_trace_never_changes_an_outcome_on_any_matrix_fixture() {
    use crate::js_write_census::matrix_tests::{fixtures, lang_of};
    for (name, _, src) in fixtures() {
        // `trace_lines` asserts, per owner, that the traced outcome equals the plain one.
        let _ = trace_lines(lang_of(&name), &src);
    }
}

/// The plain outcome of every owner of `src`, with D3 on or off.
fn outcomes(language: Language, src: &str, d3_off: bool) -> Vec<super::RdOutcome> {
    use super::census_defs_tests::{parse, recorded};
    use super::{cfg, reaching_definitions_with_exact, FileCfgEdgeIndex};
    super::d3::OFF.with(|off| off.set(d3_off));
    let parsed = parse(language, src);
    let mut out = Vec::new();
    for owner in parsed.all_functions() {
        let (defs, edges) = recorded(&parsed, &owner);
        let cfg = cfg::build_cfg_edges_with_arms(&parsed);
        let index = FileCfgEdgeIndex::new(&cfg);
        out.push(reaching_definitions_with_exact(
            &parsed,
            &owner,
            &defs,
            &edges,
            &[],
            &index,
        ));
    }
    super::d3::OFF.with(|off| off.set(false));
    out
}

#[test]
fn d3_moves_labels_only_never_availability_edges_or_recall() {
    use super::RdOutcome::{Available, Unavailable};
    use super::{FlowConfidence, FlowConfidence::Exact, FlowDoubt::SameLine};
    use crate::js_write_census::matrix_tests::{fixtures, lang_of};
    let mut moved = 0;
    for (name, _, src) in fixtures() {
        let language = lang_of(&name);
        let (on, off) = (
            outcomes(language, &src, false),
            outcomes(language, &src, true),
        );
        assert_eq!(on.len(), off.len(), "{name}");
        for (a, b) in on.iter().zip(&off) {
            match (a, b) {
                (Available(a), Available(b)) => {
                    assert!(a.labels.keys().eq(b.labels.keys()), "{name}: edge sets");
                    assert!(a.exact_labels.keys().eq(b.exact_labels.keys()), "{name}");
                    assert_eq!(a.reaching_edges, b.reaching_edges, "{name}");
                    assert_eq!(a.recall, b.recall, "{name}");
                    assert!(
                        a.loop_carried_edges.is_subset(&b.loop_carried_edges),
                        "{name}"
                    );
                    for (k, label) in &a.labels {
                        let before = b.labels[k];
                        if *label != before {
                            assert_eq!(
                                (before, *label),
                                (Exact, FlowConfidence::NameOnly(SameLine)),
                                "{name}: only exact moves, only to sameline"
                            );
                            moved += 1;
                        }
                    }
                }
                (Unavailable(a), Unavailable(b)) => assert_eq!(a, b, "{name}"),
                _ => panic!("{name}: availability moved"),
            }
        }
    }
    assert!(moved >= 30, "D3 moved {moved} edges over the matrix");
}

#[test]
fn the_def_side_reason_outranks_a_use_side_loop() {
    // Use side: a loop on U's chain (`d3-loop`); def side: `a = 1` then `a++` in D's node (`d3-def`).
    let src = "function f(b) {\n  let a = b;\n  a = 1, a++;\n  while (c()) { sink(a); a++; }\n}\n";
    let lines = d3_lines(src);
    let hit = lines
        .iter()
        .find(|l| l.contains("\ta@3:") && l.contains("\ta@4:"))
        .unwrap_or_else(|| panic!("no d3 line for a@3->a@4: {lines:?}"));
    assert_eq!(hit.split('\t').nth(1), Some("d3-def"), "{hit}");
}

#[test]
fn an_unplaced_formal_on_the_write_line_is_d3_unknown() {
    let got = mechanisms("function f(a) { a++;\n  sink(a);\n}\n");
    assert!(got.iter().any(|m| m == "d3-unknown"), "{got:?}");
}

#[test]
fn a_read_in_a_later_statement_refuses_as_unknown_not_as_exact() {
    let got = mechanisms("function f(a) {\n  log();\n  a = 2; sink(a);\n}\n");
    assert!(got.iter().any(|m| m == "d3-unknown"), "{got:?}");
}
