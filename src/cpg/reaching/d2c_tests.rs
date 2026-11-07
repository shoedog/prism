//! #660 S1c: D2-c's trace mechanism, trace neutrality (`trace_lines` asserts the traced outcome equals
//! the plain one), label-only neutrality (D2-c on vs off) over every matrix fixture and binding-shape
//! case, and the placement rows that need one owner's labels in isolation.
use super::census_defs_tests::{parse, recorded, trace_lines};
use super::RdOutcome::{Available, Unavailable};
use super::{cfg, reaching_definitions_with_exact, FileCfgEdgeIndex, FlowConfidence, FlowDoubt};
use crate::js_write_census::matrix_tests::{fixtures, lang_of};
use crate::languages::Language;

const JS: Language = Language::JavaScript;

fn d2c_lines(src: &str) -> Vec<String> {
    trace_lines(JS, src)
        .into_iter()
        .filter(|l| l.split('\t').nth(1) == Some("d2c"))
        .collect()
}

#[test]
fn d2c_is_named_for_the_edges_it_moves_and_only_those() {
    let moved = d2c_lines("function f(a) {\n  log();\n  g(() => { a++; });\n  sink(a);\n}\n");
    assert!(
        moved
            .iter()
            .any(|l| l.contains("\ta@1:") && l.contains("\ta@4:")),
        "{moved:?}"
    );
    // A scoped write and an unrelated base give no d2c line.
    for src in [
        "function f(a) {\n  log();\n  g(() => { let a = 1; a++; });\n  sink(a);\n}\n",
        "function f(a) {\n  log();\n  g(() => { b++; });\n  sink(a);\n}\n",
    ] {
        assert!(d2c_lines(src).is_empty(), "{src}: {:?}", d2c_lines(src));
    }
    // A killed edge keeps its label and has none; the def after the closure still reaches it (refused).
    let src = "function f(a) {\n  log();\n  g(() => { a++; });\n  a = 3;\n  sink(a);\n}\n";
    let lines = d2c_lines(src);
    assert!(
        !lines
            .iter()
            .any(|l| l.contains("\ta@1:") && l.contains("\ta@5:")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\ta@4:") && l.contains("\ta@5:")),
        "{lines:?}"
    );
}

#[test]
fn the_trace_never_changes_an_outcome_on_any_matrix_fixture_or_case() {
    for (name, _, src) in fixtures() {
        let _ = trace_lines(lang_of(&name), &src);
    }
    for (language, src) in crate::cpg::s1c_label_tests::sources() {
        let _ = trace_lines(language, &src);
    }
}

/// The plain outcome of every owner of `src`, with D2-c on or off.
fn outcomes(language: Language, src: &str, off: bool) -> Vec<super::RdOutcome> {
    super::d2c::OFF.with(|o| o.set(off));
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
    super::d2c::OFF.with(|o| o.set(false));
    out
}

#[test]
fn d2c_moves_labels_only_never_availability_edges_or_recall() {
    let mut inputs: Vec<(String, Language, String)> = fixtures()
        .into_iter()
        .map(|(name, _, src)| (name.clone(), lang_of(&name), src))
        .collect();
    inputs.extend(
        crate::cpg::s1c_label_tests::sources()
            .into_iter()
            .map(|(l, s)| (s.clone(), l, s)),
    );
    let mut moved = 0;
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
                    assert_eq!(a.reaching_edges, b.reaching_edges, "{name}");
                    assert_eq!(a.recall, b.recall, "{name}");
                    assert!(
                        a.loop_carried_edges.is_subset(&b.loop_carried_edges),
                        "{name}"
                    );
                    for (k, label) in &a.labels {
                        let before = b.labels[k];
                        if *label != before {
                            assert_eq!(*label, FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));
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
    assert!(moved >= 40, "D2-c moved {moved} edges over the matrix");
}

/// The label of `from_line -> to_line` of `base` in the first owner of `src`, D2-c on (`false`) or off.
fn first_owner_label(src: &str, base: &str, from: usize, to: usize, off: bool) -> FlowConfidence {
    let Available(r) = &outcomes(JS, src, off)[0] else {
        panic!("RD unavailable for {src}");
    };
    *r.labels
        .iter()
        .find(|((f, t), _)| {
            f.path.to_string() == base
                && f.line == from
                && t.line == to
                && t.path.to_string() == base
        })
        .unwrap_or_else(|| panic!("no edge {base}@{from}->@{to} in {:?}", r.labels))
        .1
}

const CFG: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

/// A use inside the writing region is not outside it; a write in a second region reaches it. (Every
/// edge inside one nested callable's statement is `sameline`; D2-c refines `sameline` too.)
#[test]
fn a_use_inside_the_writing_region_is_not_refused_by_that_write() {
    let one =
        "function f(a) {\n  log();\n  g(() => {\n    a = 1;\n    a++;\n    sink(a);\n  });\n}\n";
    let two = "function f(a) {\n  log();\n  g(() => {\n    a = 1;\n    h(() => { a++; });\n    sink(a);\n  });\n}\n";
    let same = FlowConfidence::NameOnly(FlowDoubt::SameLine);
    assert_eq!(first_owner_label(one, "a", 4, 6, false), same);
    assert_eq!(first_owner_label(two, "a", 4, 6, true), same);
    assert_eq!(first_owner_label(two, "a", 4, 6, false), CFG);
}

/// A declaration of the written name in an enclosing callable is another binding than the owner's, and
/// the same binding as the defs inside that callable.
#[test]
fn a_write_scoped_by_an_enclosing_callable_refuses_only_that_callables_edges() {
    let src = "function f(a) {\n  log();\n  g(() => {\n    var a = 1;\n    h(() => { a++; });\n    sink(a);\n  });\n  sink(a);\n}\n";
    let same = FlowConfidence::NameOnly(FlowDoubt::SameLine);
    // The callable's own `var a` reaches its later read, past the closure: refused.
    assert_eq!(first_owner_label(src, "a", 4, 6, true), same);
    assert_eq!(first_owner_label(src, "a", 4, 6, false), CFG);
    // The owner's parameter is another binding: its edge is untouched (main kills it by name).
    assert_eq!(
        first_owner_label(src, "a", 1, 8, false),
        first_owner_label(src, "a", 1, 8, true)
    );
}

/// A region write whose byte no region holds is refused for the whole base (fail closed).
#[test]
fn a_captured_write_no_region_holds_refuses_its_base() {
    use super::census_defs_tests::parse;
    use crate::access_path::AccessPath;
    use crate::cpg::binding_facts::BindingFacts;
    use crate::data_flow::FlowEdge;
    use crate::js_write_census::captured::Captured;
    let src = "function f(a) {\n  log();\n  g(() => { b(); });\n  sink(a);\n}\n";
    let parsed = parse(JS, src);
    let owner = parsed.all_functions()[0];
    let (defs, edges) = recorded(&parsed, &owner);
    let facts = BindingFacts::new(&parsed, &owner, &defs, edges.iter());
    let js = super::capture_regions::js_regions(&parsed, owner);
    let d2c = super::d2c::D2c::new(
        vec![Captured {
            path: AccessPath::simple("a"),
            start: 0,
            shadows: Vec::new(),
        }],
        false,
    )
    .map(std::rc::Rc::new);
    let edge: &FlowEdge = edges
        .iter()
        .find(|e| e.from.path.base == "a" && e.to.path.base == "a")
        .expect("a@1 -> a@4");
    let got = super::d2c::D2c::refine(
        d2c.as_ref(),
        FlowConfidence::Exact,
        edge,
        &[0],
        Some(&js),
        &facts,
    );
    assert_eq!(got, CFG);
}
