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
        let read = "function t(o, xs) {\n  let u = o;\n  use(u.f);\n  use(o.f);\n}\n".to_string();
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

// ---- fold 1: R2 reads the write census's forms and is keyed by the target ----

const CENSUS_WRITE_FORMS: [&str; 17] = [
    "w.f = 1;",
    "w.f += 1;",
    "w.f++;",
    "delete w.f;",
    "for (w.f of xs) {}",
    "for (w.f in xs) {}",
    "[w.f] = z;",
    "({ k: w.f } = z);",
    "[w.f = 1] = z;",
    "({ k: w.f = 5 } = z);",
    "[[w.f = 1]] = z;",
    "(/* c */ w).f = 1;",
    "w['f'] = 1;",
    "w.f ||= 1;",
    "[...w.f] = z;",
    "run(() => { w.f = 1; });",
    "({ k: { m: w.f = 2 } } = z);",
];

#[test]
fn every_census_write_form_through_an_alias_keeps_that_aliases_twin_and_a_siblings() {
    for form in CENSUS_WRITE_FORMS {
        // `w` is written through; `u` is a second alias of the same target and is never written.
        let src = format!(
            "function t(o, xs, z, run) {{\n  let u = o;\n  let w = o;\n  u = o;\n  {form}\n  use(o.f);\n}}\n"
        );
        let t = js_twins(&src, "w");
        assert!(t.contains(&("o".to_string(), 3)), "{form}: w {t:?}");
        let t = js_twins(&src, "u");
        assert!(
            t.contains(&("o".to_string(), 2)) && t.contains(&("o".to_string(), 4)),
            "{form}: u {t:?}"
        );
        // Control: the same owner reading `w.f` mints no twin for either.
        let read = "function t(o, xs, z, run) {\n  let u = o;\n  let w = o;\n  u = o;\n  use(w.f);\n  use(o.f);\n}\n".to_string();
        assert!(
            js_twins(&read, "u").is_empty() && js_twins(&read, "w").is_empty(),
            "{form}"
        );
    }
}

#[test]
fn a_second_alias_of_the_same_target_keeps_the_twin_in_every_alias_kind() {
    for (decl, params) in [
        ("const v = o;\n  const w = v;", "o"),
        ("let w = o;\n  if (c) w = o;", "o, c"),
        ("const w = o;", "o"),
        ("", "o, w = o"),
        ("const { p: w } = o;", "o"),
        ("const {\n    p: w,\n  } = o;", "o"),
    ] {
        let src = format!(
            "function t({params}, c) {{\n  const u = o;\n  {decl}\n  run(() => {{ w.f = 1; }});\n  use(o.f);\n}}\n"
        );
        assert!(
            js_twins(&src, "u").contains(&("o".to_string(), 2)),
            "{decl}: {:?}",
            js_twins(&src, "u")
        );
    }
    // Control: an alias of another target, or none written, leaves `u`'s twin removable.
    let other = "function t(o, p) {\n  const u = o;\n  const w = p;\n  w.f = 1;\n  use(o.f);\n}\n";
    assert!(
        js_twins(other, "u").is_empty(),
        "{:?}",
        js_twins(other, "u")
    );
}

#[test]
fn a_second_alias_written_through_keeps_the_formal_label_off_exact() {
    // round-1 head: `u`'s twin removed, `o.x@2->o.x@7` exact although `w.x = 1` may write it.
    let src = "function g(o) {\n  const v = o;\n  const w = v;\n  o.x = 0;\n  const u = o;\n  w.x = 1;\n  return sink(o.x);\n}";
    assert!(
        !labels_of(JS, src, "o.x@4->o.x@7")
            .iter()
            .any(|l| l == "exact"),
        "{:?}",
        labels_of(JS, src, "o.x@4->o.x@7")
    );
    let quiet = "function g(o) {\n  const v = o;\n  const w = v;\n  o.x = 0;\n  const u = o;\n  log(w.x);\n  return sink(o.x);\n}";
    assert_eq!(labels_of(JS, quiet, "o.x@4->o.x@7"), ["exact"]);
    let default =
        "function g(o) {\n  o.x = 0;\n  const u = o;\n  [u.x = 1] = [2];\n  return sink(o.x);\n}";
    assert!(!labels_of(JS, default, "o.x@2->o.x@5")
        .iter()
        .any(|l| l == "exact"));
}

#[test]
fn a_parameter_list_nested_callable_body_is_below_the_parameter_list() {
    let src = "function f(o, g = () => { let u = o; u = mk(); return u; }) {\n  use(o);\n}\n";
    let r = refs("a.js", src, JS);
    assert!(!twins(&r, "u").is_empty(), "{:?}", r.defs);
}

// ---- fold 2: the may-hold closure is independent of how the written name is formed ----

#[test]
fn a_write_through_any_holder_of_the_target_keeps_the_quiet_aliases_twin() {
    for form in [
        "const w = o;",
        "const w = (o);",
        "const w = c ? o : o;",
        "const w = o || {};",
        "const w = o ?? {};",
        "const [w] = [o];",
        "const { w } = { w: o };",
        "const w = f(o);",
        "const k1 = () => o;\n  const w = k1();",
        "arr.push(o);\n  const w = arr[0];",
        "this.k = o;\n  const w = this.k;",
        "let w = z;\n  w = o;",
        "let w = o;\n  w = z;",
        "let w1 = z;\n  let w = w1;\n  w1 = o;",
        "let w = z;\n  for (const t of xs) { w = t; }\n  w = o;",
    ] {
        for write in ["w.f = 1;", "const hh = w;\n  hh.f = 1;"] {
            let src = format!(
                "function t(o, c, z, xs, f, arr) {{\n  const u = o;\n  {form}\n  {write}\n  use(o.f);\n}}\n"
            );
            assert!(
                js_twins(&src, "u").contains(&("o".to_string(), 2)),
                "{form} / {write}: {:?}",
                js_twins(&src, "u")
            );
        }
    }
    // Callback parameter and member value / non-name roots.
    for src in [
        "function t(o) {\n  const u = o;\n  [o].forEach(w => { w.f = 1; });\n  use(o.f);\n}\n",
        "function t(o, f) {\n  const u = o;\n  f(o).x = 1;\n  use(o.f);\n}\n",
        "function t(o, c, z) {\n  const u = o;\n  (c ? o : z).x = 1;\n  use(o.f);\n}\n",
    ] {
        assert!(
            js_twins(src, "u").contains(&("o".to_string(), 2)),
            "{src}: {:?}",
            js_twins(src, "u")
        );
    }
    let ts = "function t(o: any) {\n  const u: any = o;\n  const w = o as any;\n  w.f = 1;\n  use(o.f);\n}\n";
    assert!(twins(&refs("a.ts", ts, TS), "u").contains(&("o".to_string(), 2)));
    // Controls (the gain survives): the written name is unrelated to `o`, or is `o` itself.
    for src in [
        "function t(o, z) {\n  const u = o;\n  const w = z;\n  w.f = 1;\n  use(o.f);\n}\n",
        "function t(o, z) {\n  const u = o;\n  z.f = 1;\n  use(o.f);\n}\n",
        "function t(o) {\n  const u = o;\n  o.f = 1;\n  use(o.f);\n}\n",
        "function t(o, f, z) {\n  const u = o;\n  f(z).x = 1;\n  use(o.f);\n}\n",
    ] {
        assert!(
            js_twins(src, "u").is_empty(),
            "{src}: {:?}",
            js_twins(src, "u")
        );
    }
}
