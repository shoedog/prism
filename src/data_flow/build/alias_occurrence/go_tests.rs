//! Go rows `G-01`, `G-02`: oracle parity on the pinned fixture, scoping negatives, the policy and
//! the producer effect (a binding write mints no target def or edge; storage writes and
//! unsupported forms keep theirs), plus the extraction-role refusal.
use super::test_support::*;
use super::{go::classify, Disposition};
use crate::languages::Language;

const F: &str = "a.go";
fn d(src: &str, line: usize) -> std::collections::BTreeSet<String> {
    dispositions(src, F, Language::Go, classify, line)
}
fn lines(src: &str) -> Vec<usize> {
    suppressed_lines(F, src, Language::Go)
}

#[test]
fn oracle_pinned_cases_g() {
    check_fixture(
        "g.go",
        include_str!("../../../../tests/fixtures/alias_twin_go/g.go"),
        Language::Go,
        classify,
    );
}

#[test]
fn reads_in_expression_lists_are_not_binding_writes() {
    // `return a, b` and a right-hand expression list hold names the extraction can record as
    // occurrences; only left-hand statement slots are binding writes.
    let src = "package p\nfunc t(o *S) (*S, *S) {\n u := o\n return u, o\n}\n";
    assert!(d(src, 3).contains("u:binding-write:G-01"));
    assert!(
        !d(src, 4).iter().any(|t| t.contains("binding-write")),
        "{:?}",
        d(src, 4)
    );
}

#[test]
fn const_and_type_switch_forms_are_unsupported() {
    let src = "package p\nfunc t(o *S, x any) {\n const c = 1\n switch v := x.(type) {\n case int:\n  _ = v\n }\n}\n";
    for line in [3, 4] {
        assert!(
            !d(src, line).iter().any(|t| t.contains("binding-write")),
            "line {line}: {:?}",
            d(src, line)
        );
    }
    // Control: a plain `var` of the same shape is a binding write.
    assert_eq!(
        lines("package p\nfunc t(o *S) {\n var u = o\n u = o\n}\n"),
        vec![3, 4]
    );
    let _ = Disposition::Unsupported;
}

#[test]
fn the_policy_suppresses_exactly_the_binding_write_occurrences() {
    let src = "package p\nfunc t(o *S, q *S) {\n u := o\n u = q\n u.f = 1\n *u = *q\n}\n";
    assert_eq!(
        lines(src),
        vec![3, 4],
        "declaration and reassignment of u only"
    );
}

#[test]
fn a_binding_write_mints_no_target_def_or_edge_but_keeps_the_alias_defs() {
    let src = "package p\nfunc t(o *S, q *S) {\n u := o\n u = q\n use(o)\n use(u)\n}\n";
    let r = refs(F, src, Language::Go);
    assert!(twins(&r, "u").is_empty(), "{:?}", twins(&r, "u"));
    assert!(r
        .edges
        .iter()
        .any(|e| e.from.path.base == "u" && e.from.line == 4 && e.to.line == 6));
    let write_byte = src.find("u = q").unwrap();
    assert!(
        !r.edges
            .iter()
            .any(|e| e.from.path.base != "u" && e.from.start_byte == write_byte),
        "{:?}",
        r.edges
    );
}

#[test]
fn a_storage_write_through_the_alias_keeps_its_target_twin() {
    let src = "package p\nfunc t(o *S) {\n u := o\n u.f = 1\n use(o.f)\n}\n";
    let r = refs(F, src, Language::Go);
    assert_eq!(twins(&r, "u"), vec![("o.f".to_string(), 4)], "{:?}", r.defs);
}

#[test]
fn a_dereference_write_keeps_its_twin() {
    let src = "package p\nfunc t(o *S, q S) {\n u := o\n *u = q\n use(o)\n}\n";
    let r = refs(F, src, Language::Go);
    assert_eq!(twins(&r, "u"), vec![("o".to_string(), 4)], "{:?}", r.defs);
}

#[test]
fn an_unsupported_occurrence_keeps_its_twin() {
    // A `const` name is not a `var_spec`: the producer retains its twin of `o` exactly as before.
    let src = "package p\nfunc t(o int) {\n const u = o\n use(o)\n}\n";
    let r = refs(F, src, Language::Go);
    assert_eq!(twins(&r, "u"), vec![("o".to_string(), 3)], "{:?}", r.defs);
    assert_eq!(lines(src), Vec::<usize>::new());
    // Positive control: the same shape as a `var` is a binding write and mints no twin.
    let var = "package p\nfunc t(o int) {\n var u = o\n use(o)\n}\n";
    assert!(twins(&refs(F, var, Language::Go), "u").is_empty());
    assert_eq!(lines(var), vec![3]);
}

#[test]
fn a_selector_field_token_of_the_target_spelling_refuses_the_removal() {
    let with = "package p\nfunc t(o *S, x X) {\n u := o\n u = x.Get()\n use(a.o)\n}\n";
    assert!(lines(with).is_empty(), "{:?}", lines(with));
    let method = "package p\nfunc t(o *S, x X) {\n u := o\n u = x.o()\n}\n";
    assert!(lines(method).is_empty());
    let without = "package p\nfunc t(o *S, x X) {\n u := o\n u = x.Get()\n use(a.b)\n}\n";
    assert_eq!(lines(without), vec![3, 4]);
}

#[test]
fn keyed_literal_keys_and_labels_refuse_the_removal() {
    let keyed = "package p\nfunc t(o string) {\n u := o\n u = mk()\n _ = S{o: 1}\n}\n";
    assert!(lines(keyed).is_empty());
    let keyed_control = "package p\nfunc t(o string) {\n u := o\n u = mk()\n _ = S{p: 1}\n}\n";
    assert_eq!(lines(keyed_control), vec![3, 4]);
    let label = "package p\nfunc t(o string) {\n u := o\n u = mk()\no:\n for {\n  break o\n }\n}\n";
    assert!(lines(label).is_empty());
    let label_control =
        "package p\nfunc t(o string) {\n u := o\n u = mk()\nq:\n for {\n  break q\n }\n}\n";
    assert_eq!(lines(label_control), vec![3, 4]);
}

#[test]
fn taking_the_address_of_the_target_does_not_refuse() {
    // Go twins never killed an edge here (base already emits `exact`), so `&o` needs no refusal;
    // pinned so a later widening is a conscious change.
    let src = "package p\nfunc t(o int) {\n u := o\n u = 1\n p := &o\n *p = 3\n use(o)\n}\n";
    assert_eq!(lines(src), vec![3, 4]);
}

#[test]
fn the_refusal_is_per_target_spelling() {
    let src = "package p\nfunc t(size S, other S, r R) {\n a := size\n a = mk()\n b := other\n b = mk()\n g(r.size)\n}\n";
    assert_eq!(lines(src), vec![5, 6]);
}
