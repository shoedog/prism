//! #660 S1d-1: the refusal classes' trace mechanisms, trace neutrality, and label-only neutrality
//! (classes on vs off) over every matrix fixture, enumeration case and S1d row.
use super::census_defs_tests::{parse, recorded, trace_lines};
use super::RdOutcome::{Available, Unavailable};
use super::{cfg, reaching_definitions_with_exact, FileCfgEdgeIndex, FlowConfidence, FlowDoubt};
use crate::js_write_census::matrix_tests::{fixtures, lang_of};
use crate::languages::Language;

const JS: Language = Language::JavaScript;

fn mechanisms_of(language: Language, src: &str) -> Vec<String> {
    trace_lines(language, src)
        .iter()
        .map(|l| l.split('\t').nth(1).unwrap().to_string())
        .collect()
}

fn named(language: Language, src: &str, mechanism: &str) -> bool {
    mechanisms_of(language, src).iter().any(|m| m == mechanism)
}

/// One source per mechanism that moves an edge, and one that must not be named for it.
#[test]
fn every_mechanism_is_named_for_the_edges_it_moves() {
    #[rustfmt::skip]
    let moved: &[(&str, &str)] = &[
        ("arguments", "function f(a) {\n  log();\n  arguments[0] = src();\n  sink(a);\n}\n"),
        ("eval", "function f(a) {\n  log();\n  eval('a = 1');\n  sink(a);\n}\n"),
        ("with", "function f(a, o) {\n  log();\n  with (o) {\n    sink(a);\n  }\n}\n"),
        ("dead-formal", "function f(a) {\n  log();\n  sink(a);\n  function a() {}\n}\n"),
        ("annexb", "function f() {\n  var a = src();\n  { function a() {} }\n  sink(a);\n}\n"),
        ("catch-var", "function f() {\n  try { g(); } catch (a) {\n    var a = 2;\n    sink(a);\n  }\n}\n"),
        ("immutable-name", "(function g(a) {\n  log();\n  g = 2;\n  sink(g);\n})();\n"),
    ];
    for (mechanism, src) in moved {
        assert!(
            named(JS, src, mechanism),
            "{mechanism}: {:?}",
            mechanisms_of(JS, src)
        );
    }
    #[rustfmt::skip]
    let unmoved: &[(&str, &str)] = &[
        ("arguments", "'use strict';\nfunction f(a) {\n  log();\n  arguments[0] = src();\n  sink(a);\n}\n"),
        ("eval", "function f(a) {\n  log();\n  o.eval('a = 1');\n  sink(a);\n}\n"),
        ("with", "function f(a) {\n  log();\n  with ({}) { log(); }\n  sink(a);\n}\n"),
        ("dead-formal", "function f(a) {\n  log();\n  sink(a);\n  function b() {}\n}\n"),
        ("annexb", "function f() {\n  var a = src();\n  { function b() {} }\n  sink(a);\n}\n"),
        ("catch-var", "function f() {\n  try { g(); } catch (e) {\n    var a = 2;\n    sink(a);\n  }\n}\n"),
        ("immutable-name", "(function g(a) {\n  var g;\n  g = 2;\n  sink(g);\n})();\n"),
    ];
    for (mechanism, src) in unmoved {
        assert!(
            !named(JS, src, mechanism),
            "{mechanism}: {:?}",
            mechanisms_of(JS, src)
        );
    }
}

/// An enclosing function's mapped `arguments` is named for a nested owner's edge; a nested non-arrow's own
/// `arguments` aliases its own formals only, so it is not named for the enclosing function's formal.
#[test]
fn enclosing_mapped_arguments_are_named_only_when_they_alias_the_formal() {
    let nested = |inner: &str| {
        format!("function f(a) {{\n  {inner}\n    a = 1;\n    arguments[0] = 2;\n    sink(a);\n  }}\n  g();\n}}\n")
    };
    assert!(named(JS, &nested("const g = () => {"), "arguments"));
    assert!(!named(JS, &nested("function g() {"), "arguments"));
}

#[test]
fn the_trace_never_changes_an_outcome_on_any_s1d_source() {
    for (language, src) in crate::cpg::s1d_label_tests::sources() {
        let _ = trace_lines(language, &src);
    }
}

/// The plain outcome of every owner of `src`, the refusal classes on or off.
fn outcomes(language: Language, src: &str, off: bool) -> Vec<super::RdOutcome> {
    super::refusal_classes::OFF.with(|o| o.set(off));
    let parsed = parse(language, src);
    let mut out = Vec::new();
    for owner in parsed.all_functions() {
        let (defs, edges) = recorded(&parsed, &owner);
        let cfg = cfg::build_cfg_edges_with_arms(&parsed);
        let index = FileCfgEdgeIndex::new(&cfg);
        out.push(reaching_definitions_with_exact(
            &parsed, &owner, &defs, &edges,
            // A nonempty supplemental list, so `exact_labels` is compared on real entries.
            &edges, &index,
        ));
    }
    super::refusal_classes::OFF.with(|o| o.set(false));
    out
}

/// The classes move labels only: same edges, availability, reaching edges and recall; an `exact` or
/// `sameline` edge becomes `CfgIncomplete` or `Killed`, and no other label changes.
#[test]
fn the_classes_move_labels_only_never_availability_edges_or_recall() {
    let mut inputs: Vec<(String, Language, String)> = fixtures()
        .into_iter()
        .map(|(name, _, src)| (name.clone(), lang_of(&name), src))
        .collect();
    inputs.extend(
        crate::cpg::s1d_label_tests::sources()
            .into_iter()
            .map(|(l, s)| (s.clone(), l, s)),
    );
    let mut moved = 0;
    let mut exact_seen = 0;
    for (name, language, src) in inputs {
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
                    assert_eq!(a.exact_labels.is_empty(), a.labels.is_empty(), "{name}");
                    exact_seen += a.exact_labels.len();
                    for (k, label) in &a.exact_labels {
                        let before = b.exact_labels[k];
                        assert!(
                            *label == before
                                || (before == FlowConfidence::Exact
                                    || before == FlowConfidence::NameOnly(FlowDoubt::SameLine)),
                            "{name}: supplemental {before:?} -> {label:?}"
                        );
                    }
                    assert_eq!(a.reaching_edges, b.reaching_edges, "{name}");
                    assert_eq!(a.recall, b.recall, "{name}");
                    assert!(
                        a.loop_carried_edges.is_subset(&b.loop_carried_edges),
                        "{name}"
                    );
                    for (k, label) in &a.labels {
                        let before = b.labels[k];
                        if *label != before {
                            assert!(
                                matches!(
                                    label,
                                    FlowConfidence::NameOnly(
                                        FlowDoubt::CfgIncomplete | FlowDoubt::Killed { .. }
                                    )
                                ),
                                "{name}: moved to {label:?}"
                            );
                            assert!(
                                before == FlowConfidence::Exact
                                    || before == FlowConfidence::NameOnly(FlowDoubt::SameLine),
                                "{name}: only exact and sameline move, not {before:?}"
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
    assert!(
        moved >= 40,
        "the classes moved {moved} edges over the matrix"
    );
    assert!(
        exact_seen >= 300,
        "supplemental labels compared: {exact_seen}"
    );
}

/// The trace with a nonempty supplemental list never changes the outcome either.
#[test]
fn the_trace_is_neutral_with_supplemental_edges() {
    use super::census_pass;
    for (language, src) in crate::cpg::s1d_label_tests::sources() {
        let parsed = parse(language, &src);
        for owner in parsed.all_functions() {
            let (defs, edges) = recorded(&parsed, &owner);
            let cfg = cfg::build_cfg_edges_with_arms(&parsed);
            let index = FileCfgEdgeIndex::new(&cfg);
            let mut sink = Vec::new();
            let traced =
                census_pass::traced(&parsed, &owner, &defs, &edges, &edges, &index, &mut sink);
            let plain =
                reaching_definitions_with_exact(&parsed, &owner, &defs, &edges, &edges, &index);
            assert_eq!(traced, plain, "{src}");
        }
    }
}
