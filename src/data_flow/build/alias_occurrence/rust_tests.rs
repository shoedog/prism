//! Rust rows `R-01..R-05`: oracle parity on the pinned fixture, the conservative `R-02`, negatives,
//! the policy, the producer effect and the extraction-role refusal.
use super::rust::classify;
use super::test_support::*;
use crate::languages::Language;

const F: &str = "a.rs";
fn d(src: &str, line: usize) -> std::collections::BTreeSet<String> {
    dispositions(src, F, Language::Rust, classify, line)
}
fn lines(src: &str) -> Vec<usize> {
    suppressed_lines(F, src, Language::Rust)
}

#[test]
fn oracle_pinned_cases_r() {
    check_fixture(
        "r.rs",
        include_str!("../../../../tests/fixtures/alias_twin_rust/r.rs"),
        Language::Rust,
        classify,
    );
}

#[test]
fn compound_assignment_is_never_a_binding_write_even_on_a_numeric() {
    // The oracle says binding-write for `b: i32`; slice 2a keeps the twin (R-02 always Unsupported).
    let src = "fn t(k: i32) {\n let mut b: i32 = k;\n b += 1;\n}\n";
    assert!(d(src, 3).contains("b:unsupported:R-02"), "{:?}", d(src, 3));
    assert_eq!(lines(src), vec![2]);
}

#[test]
fn a_rebound_numeric_spelling_is_still_unsupported() {
    let src = "struct S { f: i32 }\ntype i32 = &'static mut S;\nfn t(s: i32) {\n let mut a: i32 = s;\n a += 1;\n}\n";
    assert!(d(src, 5).contains("a:unsupported:R-02"));
}

#[test]
fn the_policy_suppresses_exactly_the_binding_write_occurrences() {
    let src =
        "fn t(o: &mut S, q: &mut S) {\n let mut u = o;\n u = q;\n u.f = 1;\n *u = S { f: 2 };\n}\n";
    assert_eq!(lines(src), vec![2, 3]);
}

#[test]
fn a_binding_write_mints_no_target_def_or_edge_but_keeps_the_alias_defs() {
    let src = "fn t(o: &mut S, q: &mut S) {\n let mut u = o;\n u = q;\n use_(o);\n use_(u);\n}\n";
    let r = refs(F, src, Language::Rust);
    assert!(twins(&r, "u").is_empty(), "{:?}", twins(&r, "u"));
    assert!(r
        .edges
        .iter()
        .any(|e| e.from.path.base == "u" && e.from.line == 3 && e.to.line == 5));
}

#[test]
fn a_storage_write_through_the_alias_keeps_its_target_twin() {
    let src = "fn t(o: &mut S) {\n let u = o;\n u.f = 1;\n use_(o.f);\n}\n";
    let r = refs(F, src, Language::Rust);
    assert_eq!(twins(&r, "u"), vec![("o.f".to_string(), 3)], "{:?}", r.defs);
}

#[test]
fn a_dereference_write_keeps_its_twin() {
    let src = "fn t(o: &mut S, q: S) {\n let u = o;\n *u = q;\n use_(o);\n}\n";
    let r = refs(F, src, Language::Rust);
    assert_eq!(twins(&r, "u"), vec![("o".to_string(), 3)], "{:?}", r.defs);
}

#[test]
fn an_unsupported_occurrence_keeps_its_twin() {
    let src = "fn t(o: usize) {\n let mut u = o;\n u += 1;\n use_(o);\n}\n";
    let r = refs(F, src, Language::Rust);
    assert_eq!(twins(&r, "u"), vec![("o".to_string(), 3)], "{:?}", r.defs);
}

#[test]
fn pattern_forms_outside_the_table_are_unsupported() {
    let src = "fn t(o: Option<i32>) {\n let u = 1;\n if let Some(u) = o {\n  use_(u);\n }\n for u in 0..3 {\n  use_(u);\n }\n (u, x) = (1, 2);\n}\n";
    for line in [3, 6, 9] {
        assert!(
            !d(src, line).iter().any(|t| t.contains("binding-write")),
            "line {line}: {:?}",
            d(src, line)
        );
    }
}

#[test]
fn a_field_or_method_name_token_of_the_target_spelling_refuses_the_removal() {
    let field = "fn t(o: usize, x: X) {\n let mut u = o;\n u = mk();\n use_(x.o);\n}\n";
    assert!(lines(field).is_empty(), "{:?}", lines(field));
    let method = "fn t(o: usize, x: X) {\n let mut u = o;\n u = mk();\n x.o();\n}\n";
    assert!(lines(method).is_empty());
    let without = "fn t(o: usize, x: X) {\n let mut u = o;\n u = mk();\n x.p();\n}\n";
    assert_eq!(lines(without), vec![2, 3]);
}

#[test]
fn scoped_path_segments_and_macro_tokens_refuse_the_removal() {
    let scoped = "fn t(o: usize) {\n let mut u = o;\n u = mk();\n m::o();\n}\n";
    assert!(lines(scoped).is_empty());
    let macro_dot =
        "fn t(o: usize, x: X) {\n let mut u = o;\n u = mk();\n println!(\"{}\", x.o);\n}\n";
    assert!(lines(macro_dot).is_empty(), "{:?}", lines(macro_dot));
    let macro_label = "fn t(o: usize) {\n let mut u = o;\n u = mk();\n m!(o: 1);\n}\n";
    assert!(lines(macro_label).is_empty());
    // A plain macro argument is a read of the variable, not a non-variable role.
    let macro_arg = "fn t(o: usize) {\n let mut u = o;\n u = mk();\n println!(\"{}\", o);\n}\n";
    assert_eq!(lines(macro_arg), vec![2, 3]);
}

#[test]
fn the_refusal_is_per_target_spelling() {
    let src = "fn t(size: S, other: S, r: R) {\n let mut a = size;\n a = mk();\n let mut b = other;\n b = mk();\n g(r.size);\n}\n";
    assert_eq!(lines(src), vec![4, 5]);
}

#[test]
fn unit_cases_of_the_oracle_for_r02_are_unsupported_or_stricter() {
    // `&mut` receiver with a numeric RHS, an unrelated `i32` in the declaration text and a
    // reference-annotated source: the oracle says unsupported; slice 2a agrees.
    let a = "fn t(s: &mut S) {\n let mut a = s;\n a += 1;\n}\n";
    assert!(d(a, 3).contains("a:unsupported:R-02"));
    let b = "fn t(s: &mut S, k: i32) {\n let mut a = s; /* k: i32 */\n a += 1;\n}\n";
    assert!(d(b, 3).contains("a:unsupported:R-02"));
    let c = "fn t(k: &mut i32) {\n let mut b = k;\n b += 1;\n}\n";
    assert!(d(c, 3).contains("b:unsupported:R-02"));
    // The oracle's two numeric binding-write rows are the documented conservative divergence.
    let e = "fn t(k: i32) {\n let mut b = k;\n b += 1;\n}\n";
    assert!(d(e, 3).contains("b:unsupported:R-02"));
}

#[test]
fn a_destructuring_assignment_leaf_is_not_a_binding_write() {
    let src =
        "fn t(o: usize, q: usize) {\n let mut u = o;\n let mut w = q;\n (u, w) = (q, o);\n}\n";
    assert!(
        !d(src, 4).iter().any(|t| t.contains("binding-write")),
        "{:?}",
        d(src, 4)
    );
}
