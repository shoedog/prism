//! JavaScript, TypeScript and TSX rows `S-01..S-05`, `A-01`: oracle parity on the pinned fixture,
//! negatives, the policy, the parameter-list refusal (#761), the member-name refusal (R1), the
//! producer effect and the fallback anchors.
use super::js::classify;
use super::test_support::*;
use crate::languages::Language;

const F: &str = "a.js";
fn d(src: &str, line: usize) -> std::collections::BTreeSet<String> {
    dispositions(src, F, Language::JavaScript, classify, line)
}
fn lines(src: &str) -> Vec<usize> {
    suppressed_lines(F, src, Language::JavaScript)
}

#[test]
fn oracle_pinned_cases_j() {
    check_fixture(
        "j.js",
        include_str!("../../../../tests/fixtures/alias_twin_js/j.js"),
        Language::JavaScript,
        classify,
    );
}

#[test]
fn member_and_default_forms_are_not_binding_writes() {
    let src = "function t(o, xs) {\n  let u = o;\n  u.x++;\n  for (u.x of xs) {}\n  [u = 1] = xs;\n  ({ u = 1 } = xs);\n}\n";
    for line in [3, 4, 5, 6] {
        assert!(
            !d(src, line).iter().any(|t| t.contains("binding-write")),
            "line {line}: {:?}",
            d(src, line)
        );
    }
    assert!(d(src, 2).contains("u:binding-write:S-01"));
}

#[test]
fn the_policy_suppresses_exactly_the_binding_write_occurrences() {
    let src = "function t(o, q) {\n  let u = o;\n  u = q;\n  u += 1;\n  use(u.f);\n}\n";
    assert_eq!(lines(src), vec![2, 3, 4]);
}

#[test]
fn an_alias_the_owner_writes_through_keeps_its_binding_twins() {
    // R2: the twin's kill is what keeps an uncertified write through the alias off `exact`.
    for write in [
        "u.f = 1;",
        "u[0] = 2;",
        "u.f += 1;",
        "u.f++;",
        "--u.f;",
        "delete u.f;",
        "u.a.b = 1;",
        "for (u.f of xs) {}",
        "for (u.f in xs) {}",
        "({ k: u.f } = z);",
        "[u.f] = z;",
        "[, ...u.f] = z;",
        "const h = () => { u.f = 1; };",
        "const h = function () { delete u[k]; };",
    ] {
        let src = format!("function t(o, q, z, xs, k) {{\n  let u = o;\n  u = q;\n  {write}\n}}\n");
        assert!(lines(&src).is_empty(), "{write}: {:?}", lines(&src));
    }
    // Controls: reads and a call on the alias, a write through another name, and another alias.
    for read in ["use(u.f);", "use(u);", "u.m();", "w.f = 1;"] {
        let src = format!("function t(o, q, w) {{\n  let u = o;\n  u = q;\n  {read}\n}}\n");
        assert_eq!(lines(&src), vec![2, 3], "{read}");
    }
    // A write through the target itself is the direct write: the gain survives (the closure excludes `r == o`).
    assert_eq!(
        lines("function t(o, q) {\n  let u = o;\n  u = o;\n  o.f = 1;\n}\n"),
        vec![2, 3]
    );
    let two =
        "function t(o, p, q, r) {\n  let u = o;\n  let w = p;\n  u = q;\n  w = r;\n  w.f = 1;\n}\n";
    assert_eq!(lines(two), vec![2, 4]);
}

#[test]
fn a_binding_write_mints_no_target_def_or_edge_but_keeps_the_alias_defs() {
    let src = "function t(o, q) {\n  let u = o;\n  u = q;\n  use(o);\n  use(u);\n}\n";
    let r = refs(F, src, Language::JavaScript);
    assert!(twins(&r, "u").is_empty(), "{:?}", twins(&r, "u"));
    assert!(r
        .edges
        .iter()
        .any(|e| e.from.path.base == "u" && e.from.line == 3 && e.to.line == 5));
}

#[test]
fn a_storage_write_through_the_alias_keeps_its_target_twin() {
    let src = "function t(o) {\n  let u = o;\n  u.f = 1;\n  use(o.f);\n}\n";
    let r = refs(F, src, Language::JavaScript);
    assert_eq!(
        twins(&r, "u"),
        vec![("o".to_string(), 2), ("o.f".to_string(), 3)],
        "{:?}",
        r.defs
    );
}

#[test]
fn an_unsupported_occurrence_keeps_its_twin() {
    // A parameter-list alias is `X-PARAM`: its binding twin stays.
    let src = "function t(o, u = o) {\n  use(o);\n}\n";
    let r = refs(F, src, Language::JavaScript);
    assert_eq!(twins(&r, "u"), vec![("o".to_string(), 1)], "{:?}", r.defs);
}

#[test]
fn a_property_name_token_of_the_target_spelling_refuses_the_removal() {
    let with = "function t(o, x) {\n  let u = o;\n  u = mk();\n  use(x.o);\n}\n";
    assert!(lines(with).is_empty(), "{:?}", lines(with));
    let without = "function t(o, x) {\n  let u = o;\n  u = mk();\n  use(x.p);\n}\n";
    assert_eq!(lines(without), vec![2, 3]);
}

#[test]
fn object_keys_shorthand_methods_labels_and_type_names_refuse_the_removal() {
    for tok in [
        "use({ o: 1 });",
        "use({ o });",
        "class K { o() {} }",
        "o: for (;;) { break o; }",
    ] {
        let src = format!("function t(o) {{\n  let u = o;\n  u = mk();\n  {tok}\n}}\n");
        assert!(lines(&src).is_empty(), "{tok}: {:?}", lines(&src));
    }
    let ts = "function t(o: T) {\n  let u = o;\n  u = mk();\n  let x: o;\n}\n";
    assert_eq!(
        suppressed_lines("a.ts", ts, Language::TypeScript),
        Vec::<usize>::new()
    );
}

#[test]
fn the_refusal_is_per_target_spelling() {
    let src = "function t(size, other, r) {\n  let a = size;\n  a = mk();\n  let b = other;\n  b = mk();\n  g(r.size);\n}\n";
    assert_eq!(lines(src), vec![4, 5]);
}

#[test]
fn a_parameter_list_alias_keeps_its_twin() {
    // #761: the binding twin of `o` at `u = o` keeps `o@1 -> o@3` off `exact`.
    let src = "function f(o, u = o, v = u.x++) {\n  log();\n  return sink(o.x);\n}\n";
    assert!(lines(src).is_empty(), "{:?}", lines(src));
    assert!(
        d(src, 1).iter().any(|t| t.contains("X-PARAM")),
        "{:?}",
        d(src, 1)
    );
    // Control: the same alias in the body is a binding write.
    let body = "function f(o) {\n  const u = o;\n  log();\n  return sink(o.x);\n}\n";
    assert_eq!(lines(body), vec![2]);
    let ts = "function f(o: T, u: T = o, v = u.x++) {\n  log();\n  return sink(o.x);\n}\n";
    assert!(suppressed_lines("a.ts", ts, Language::TypeScript).is_empty());
    let arrow = "const f = (o, u = o) => {\n  log();\n  return sink(o.x);\n};\n";
    assert!(lines(arrow).is_empty(), "{:?}", lines(arrow));
}

#[test]
fn typescript_wrappers_do_not_hide_a_write_through_the_alias() {
    for write in [
        "(u as any).f = 1;",
        "u!.f = 1;",
        "(<any>u).f = 1;",
        "(u satisfies T).f = 1;",
    ] {
        let src = format!("function t(o: T, q: T) {{\n  let u: T = o;\n  u = q;\n  {write}\n}}\n");
        assert!(
            suppressed_lines("a.ts", &src, Language::TypeScript).is_empty(),
            "{write}"
        );
    }
}

#[test]
fn typescript_and_tsx_enable_the_same_rows() {
    let src = "function t(o: T, q: T) {\n  let u: T = o;\n  u = q;\n  use(u.f);\n}\n";
    assert_eq!(
        suppressed_lines("a.ts", src, Language::TypeScript),
        vec![2, 3]
    );
    assert_eq!(suppressed_lines("a.tsx", src, Language::Tsx), vec![2, 3]);
}

fn anchor_lines(src: &str) -> Vec<(String, usize)> {
    anchors(F, src, Language::JavaScript)
}

#[test]
fn a_multi_line_destructuring_declaration_anchor_is_a_binding_write() {
    let src = "function t(o) {\n  const {\n    a,\n    b: { c },\n  } = o;\n  use(o.a);\n}\n";
    let got = anchor_lines(src);
    // R2: a destructured alias the owner writes through keeps its anchor.
    let written = "function t(o) {\n  const {\n    a,\n  } = o;\n  a.x = 1;\n}\n";
    assert!(anchor_lines(written).is_empty());
    assert!(got.iter().any(|(a, l)| a == "a" && *l == 2), "{got:?}");
    assert!(got.iter().any(|(a, l)| a == "c" && *l == 2), "{got:?}");
    let r = refs(F, src, Language::JavaScript);
    assert!(
        !r.defs
            .values()
            .flatten()
            .any(|d| d.path.base == "o" && d.start_byte == d.end_byte),
        "{:?}",
        r.defs
    );
}

#[test]
fn an_anchor_on_an_assignment_line_is_unsupported() {
    let src = "function t(o) {\n  let a;\n  ({\n    a,\n  } = o);\n  use(o.a);\n}\n";
    assert!(anchor_lines(src).is_empty(), "{:?}", anchor_lines(src));
}
