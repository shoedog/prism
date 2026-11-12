//! Python rows `Y-01..Y-03`: oracle parity on the pinned fixture, negatives, the policy, the
//! producer effect and the extraction-role refusal.
use super::python::classify;
use super::test_support::*;
use crate::languages::Language;

const F: &str = "a.py";
fn d(src: &str, line: usize) -> std::collections::BTreeSet<String> {
    dispositions(src, F, Language::Python, classify, line)
}
fn lines(src: &str) -> Vec<usize> {
    suppressed_lines(F, src, Language::Python)
}

#[test]
fn oracle_pinned_cases_p() {
    check_fixture(
        "p.py",
        include_str!("../../../../tests/fixtures/alias_twin_python/p.py"),
        Language::Python,
        classify,
    );
}

#[test]
fn augmented_assignment_and_attribute_loop_targets_are_not_binding_writes() {
    let src = "def t(o, xs):\n    u = o\n    u += 1\n    for u.x in xs:\n        pass\n";
    assert!(d(src, 3).contains("u:unsupported:Y-02"));
    assert!(
        !d(src, 4).iter().any(|t| t.contains("binding-write")),
        "{:?}",
        d(src, 4)
    );
}

#[test]
fn comprehension_targets_are_unsupported() {
    let src = "def t(o, xs):\n    u = o\n    ys = [u for u in xs]\n";
    assert!(
        !d(src, 3).iter().any(|t| t.contains("u:binding-write")),
        "{:?}",
        d(src, 3)
    );
}

#[test]
fn the_policy_suppresses_exactly_the_binding_write_occurrences() {
    let src = "def t(o, q):\n    u = o\n    u = q\n    u += 1\n    u.f = 1\n    u[0] = 2\n";
    assert_eq!(lines(src), vec![2, 3]);
}

#[test]
fn a_binding_write_mints_no_target_def_or_edge_but_keeps_the_alias_defs() {
    let src = "def t(o, q):\n    u = o\n    u = q\n    use(o)\n    use(u)\n";
    let r = refs(F, src, Language::Python);
    assert!(twins(&r, "u").is_empty(), "{:?}", twins(&r, "u"));
    assert!(r
        .edges
        .iter()
        .any(|e| e.from.path.base == "u" && e.from.line == 3 && e.to.line == 5));
}

#[test]
fn a_storage_write_through_the_alias_keeps_its_target_twin() {
    let src = "def t(o):\n    u = o\n    u.f = 1\n    use(o.f)\n";
    let r = refs(F, src, Language::Python);
    assert_eq!(twins(&r, "u"), vec![("o.f".to_string(), 3)], "{:?}", r.defs);
}

#[test]
fn an_unsupported_occurrence_keeps_its_twin() {
    let src = "def t(o, q):\n    u = o\n    u += q\n    use(o)\n";
    let r = refs(F, src, Language::Python);
    assert_eq!(twins(&r, "u"), vec![("q".to_string(), 3)], "{:?}", r.defs);
}

#[test]
fn an_attribute_name_token_of_the_target_spelling_refuses_the_removal() {
    let with = "def t(o, x):\n    u = o\n    u = mk()\n    use(x.o)\n";
    assert!(lines(with).is_empty(), "{:?}", lines(with));
    let without = "def t(o, x):\n    u = o\n    u = mk()\n    use(x.p)\n";
    assert_eq!(lines(without), vec![2, 3]);
}

#[test]
fn a_keyword_argument_name_of_the_target_spelling_refuses_the_removal() {
    let with = "def t(o):\n    u = o\n    u = mk()\n    f(o=1)\n";
    assert!(lines(with).is_empty());
    let without = "def t(o):\n    u = o\n    u = mk()\n    f(p=1)\n";
    assert_eq!(lines(without), vec![2, 3]);
}

#[test]
fn the_refusal_is_per_target_spelling() {
    let src = "def t(size, other, r):\n    a = size\n    a = mk()\n    b = other\n    b = mk()\n    g(r.size)\n";
    assert_eq!(lines(src), vec![4, 5]);
}

#[test]
fn a_nonlocal_write_in_a_nested_def_needs_no_refusal() {
    // Python has no address-of; the closest analog (a nested def writing the target through
    // `nonlocal`) never un-masked a label (base already emits `exact`), so no refusal exists.
    let src = "def t(o):\n    u = o\n    u = 1\n    def g():\n        nonlocal o\n        o = 3\n    g()\n    use(o)\n";
    assert_eq!(lines(src), vec![2, 3]);
}
