//! JS/TS/TSX effect rows of slice 4c on the producer and on the labels, written against the public
//! surface only (`Producer::file_refs`, the label helper) so each one runs on the base tree: a binding write mints
//! no target twin and no spurious kill, a storage write and the refusals keep the twin.
use crate::ast::ParsedFile;
use crate::cpg::census_label_tests::labels_of;
use crate::data_flow::build::{FileRefs, Producer};
use crate::data_flow::VarAccessKind;
use crate::languages::Language;
use std::collections::BTreeSet;

const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

fn refs(file: &str, src: &str, lang: Language) -> FileRefs {
    let parsed = ParsedFile::parse(file, src, lang).unwrap();
    Producer::file_refs(&file.to_string(), &parsed)
}

/// `(target path, line)` of every Def minted at the bytes of a Def of `alias` (the alias twins).
fn twins(r: &FileRefs, alias: &str) -> Vec<(String, usize)> {
    let all: Vec<_> = r
        .defs
        .values()
        .flatten()
        .filter(|d| d.kind == VarAccessKind::Def)
        .collect();
    let starts: BTreeSet<usize> = all
        .iter()
        .filter(|d| d.path.base == alias)
        .map(|d| d.start_byte)
        .collect();
    let mut out: Vec<_> = all
        .iter()
        .filter(|d| d.path.base != alias && starts.contains(&d.start_byte))
        .map(|d| (d.path.to_string(), d.line))
        .collect();
    out.sort();
    out
}

fn js_twins(src: &str, alias: &str) -> Vec<(String, usize)> {
    twins(&refs("a.js", src, JS), alias)
}

#[test]
fn binding_writes_mint_no_twin() {
    for (src, alias) in [
        (
            "function t(o, q) {\n  let u = o;\n  use(o);\n  use(u);\n}\n",
            "u",
        ),
        (
            "function t(o, q) {\n  let u = o;\n  u = q;\n  use(o);\n}\n",
            "u",
        ),
        (
            "function t(o, q) {\n  let u = o;\n  u += 1;\n  use(o);\n}\n",
            "u",
        ),
        (
            "function t(o) {\n  const { k } = o;\n  use(o.k, k);\n}\n",
            "k",
        ),
        (
            "function t(o) {\n  const { k: a, ...r } = o;\n  use(o.k, a);\n}\n",
            "a",
        ),
        (
            "function t(o) {\n  let u;\n  [u] = [o];\n  use(o);\n}\n",
            "u",
        ),
    ] {
        assert!(
            js_twins(src, alias).is_empty(),
            "{src}: {:?}",
            js_twins(src, alias)
        );
    }
    // TS and TSX take the same rows.
    let ts = "function t(o: T, q: T) {\n  let u: T = o;\n  u = q;\n  use(o);\n}\n";
    assert!(twins(&refs("a.ts", ts, TS), "u").is_empty());
    assert!(twins(&refs("a.tsx", ts, TSX), "u").is_empty());
}

#[test]
fn a_storage_write_keeps_its_twin_and_so_do_the_binding_writes_of_that_alias() {
    let src = "function t(o) {\n  let u = o;\n  u.f = 1;\n  use(o.f);\n}\n";
    assert_eq!(
        js_twins(src, "u"),
        vec![("o".to_string(), 2), ("o.f".to_string(), 3)]
    );
    // Control: the same alias with no write through it mints nothing at its binding writes.
    let quiet = "function t(o) {\n  let u = o;\n  use(u.f);\n  use(o.f);\n}\n";
    assert!(
        js_twins(quiet, "u").is_empty(),
        "{:?}",
        js_twins(quiet, "u")
    );
}

#[test]
fn every_write_through_the_alias_keeps_the_binding_twin() {
    for write in [
        "u.f++;",
        "delete u.f;",
        "for (u.f of xs) {}",
        "[u.f] = xs;",
        "const h = () => { u.f = 1; };",
    ] {
        let src = format!("function t(o, xs) {{\n  let u = o;\n  {write}\n  use(o.f);\n}}\n");
        assert!(
            js_twins(&src, "u").contains(&("o".to_string(), 2)),
            "{write}: {:?}",
            js_twins(&src, "u")
        );
        // Control: reading the member instead of writing it leaves nothing to keep.
        let read = format!("function t(o, xs) {{\n  let u = o;\n  use(u.f);\n  use(o.f);\n}}\n");
        assert!(js_twins(&read, "u").is_empty(), "{write}");
    }
    let ts = "function t(o: T) {\n  let u: T = o;\n  (u as any).f = 1;\n  use(o.f);\n}\n";
    assert!(twins(&refs("a.ts", ts, TS), "u").contains(&("o".to_string(), 2)));
}

#[test]
fn a_parameter_list_alias_keeps_its_twin() {
    let src = "function f(o, u = o, v = u.x++) {\n  log();\n  return sink(o.x);\n}\n";
    assert!(
        js_twins(src, "u").contains(&("o".to_string(), 1)),
        "{:?}",
        js_twins(src, "u")
    );
    let quiet = "function f(o, u = o) {\n  log();\n  return sink(o.x);\n}\n";
    assert_eq!(js_twins(quiet, "u"), vec![("o".to_string(), 1)]);
    // Control: the same alias in the body is a binding write and mints no twin.
    let body = "function f(o) {\n  const u = o;\n  log();\n  return sink(o.x);\n}\n";
    assert!(js_twins(body, "u").is_empty(), "{:?}", js_twins(body, "u"));
}

#[test]
fn a_member_name_token_of_the_target_spelling_keeps_the_twin() {
    for token in [
        "use(x.o);",
        "use({ o: 1 });",
        "use({ o });",
        "class K { o() {} }",
    ] {
        let src = format!("function t(o, x) {{\n  let u = o;\n  u = mk();\n  {token}\n}}\n");
        assert!(!js_twins(&src, "u").is_empty(), "{token}");
        // Control: with a token of another spelling the binding writes mint nothing.
        let other = src
            .replace("o: 1", "p: 1")
            .replace("{ o }", "{ p }")
            .replace("o()", "p()")
            .replace("x.o", "x.p");
        assert!(
            js_twins(&other, "u").is_empty(),
            "{token}: {:?}",
            js_twins(&other, "u")
        );
    }
}

#[test]
fn a_destructuring_fallback_anchor_is_suppressed_unless_written_through() {
    let src = "function t(o) {\n  const {\n    a,\n    b: { c },\n  } = o;\n  use(o.a, a, c);\n}\n";
    let r = refs("a.js", src, JS);
    assert!(
        !r.defs
            .values()
            .flatten()
            .any(|d| d.path.base == "o" && d.start_byte == d.end_byte),
        "{:?}",
        r.defs
    );
    let written = "function t(o) {\n  const {\n    a,\n  } = o;\n  a.x = 1;\n  use(o.a);\n}\n";
    let r = refs("a.js", written, JS);
    assert!(r
        .defs
        .values()
        .flatten()
        .any(|d| d.path.base == "o" && d.start_byte == d.end_byte));
}

// ---- labels: the declaration no longer kills the target's earlier defs ----

#[test]
fn a_declaration_does_not_kill_the_targets_earlier_defs() {
    let src = "function g(o) {\n  o.x = 0;\n  const u = o;\n  log(u);\n  return sink(o.x);\n}";
    assert_eq!(labels_of(JS, src, "o.x@2->o.x@5"), ["exact"]);
    assert_eq!(labels_of(JS, src, "o@1->o@5"), ["exact"]);
    let ts =
        "function g(o: any) {\n  o.x = 0;\n  const u: any = o;\n  log(u);\n  return sink(o.x);\n}";
    assert_eq!(labels_of(TS, ts, "o.x@2->o.x@5"), ["exact"]);
    assert_eq!(labels_of(TSX, ts, "o.x@2->o.x@5"), ["exact"]);
}

#[test]
fn a_write_through_the_alias_keeps_the_twins_kill() {
    // Control: an alias the owner writes through keeps its binding twin and so its label.
    let src = "function g(o) {\n  o.x = 0;\n  const u = o;\n  u.y = 1;\n  return sink(o.x);\n}";
    assert_ne!(labels_of(JS, src, "o.x@2->o.x@5"), ["exact"]);
    // Control: the same owner with a read of the alias instead of a write is exact.
    let read = "function g(o) {\n  o.x = 0;\n  const u = o;\n  log(u.y);\n  return sink(o.x);\n}";
    assert_eq!(labels_of(JS, read, "o.x@2->o.x@5"), ["exact"]);
}

#[test]
fn a_parameter_list_alias_keeps_the_formal_off_exact() {
    // #761: the twin of `o` at the parameter-list `u = o` is what keeps `o@1->o@3` off `exact`.
    let src = "function f(o, u = o, v = u.x++) {\n  log();\n  return sink(o.x);\n}";
    assert!(!labels_of(JS, src, "o@1->o@3").iter().any(|l| l == "exact"));
    // Control: a body alias with a write through it keeps its twin, and a quiet body alias is exact.
    let quiet = "function g(o) {\n  const u = o;\n  log();\n  return sink(o.x);\n}";
    assert!(
        labels_of(JS, quiet, "o@1->o@4")
            .iter()
            .all(|l| l == "exact"),
        "{:?}",
        labels_of(JS, quiet, "o@1->o@4")
    );
}
