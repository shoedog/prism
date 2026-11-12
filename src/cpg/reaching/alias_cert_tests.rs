//! #783 option A, slice 4a: the certificate admission matrix (`DESIGN-astra.md` 3). Every row is a source and the
//! `rd-alias-cert` trace lines it must produce, as `verdict:reason:alias` per candidate declarator, in source
//! order. No row moves a label: the certificates are visible only through the trace. The module needs
//! nothing but `trace_lines`, so these rows compile and fail by assertion on a tree without the certificates.
use super::census_defs_tests::trace_lines;
use crate::languages::Language;

const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

/// `verdict:reason:alias` of every certificate line the trace of `src` carries.
fn certs(language: Language, src: &str) -> Vec<String> {
    trace_lines(language, src)
        .iter()
        .filter(|l| l.starts_with("rd-alias-cert\t"))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let alias = f[5].split('@').next().unwrap();
            format!("{}:{}:{}", f[1], f[2], alias)
        })
        .collect()
}

const OK: &str = "admitted:-:u";

fn row(language: Language, src: &str, want: &[&str]) {
    assert_eq!(certs(language, src), want, "{src}");
}

/// The body every positive row shares: a def of `o.x`, a write through `u`, a read.
const BODY: &str = "  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";

fn with_alias(head: &str, decl: &str, body: &str) -> String {
    format!("{head}\n  {decl}\n{body}")
}

// ---- Admitted: one positive root-copy case per declaration kind of `u` and of `o` ----------------------

#[test]
fn a_root_copy_of_a_formal_is_certified_for_every_alias_declaration_kind() {
    for kw in ["const", "let", "var"] {
        let src = with_alias("function g(o) {", &format!("{kw} u = o;"), BODY);
        row(JS, &src, &[OK]);
    }
}

#[test]
fn a_root_copy_of_a_local_is_certified_for_every_target_declaration_kind() {
    for kw in ["const", "let", "var"] {
        let head = format!("function g() {{\n  {kw} o = make();");
        row(JS, &with_alias(&head, "const u = o;", BODY), &[OK]);
    }
}

#[test]
fn a_root_copy_is_certified_in_every_dialect_and_callable_form() {
    row(
        TS,
        &with_alias("function g(o: any) {", "const u: any = o;", BODY),
        &[OK],
    );
    row(
        TS,
        &with_alias("function g(o?: any) {", "const u = o;", BODY),
        &[OK],
    );
    row(
        TSX,
        &with_alias("function g(o: any) {", "const u = o;", BODY),
        &[OK],
    );
    row(
        JS,
        "const g = (o) => {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n};",
        &[OK],
    );
    row(
        JS,
        "const g = o => {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n};",
        &[OK],
    );
    row(JS, "class C {\n  m(o) {\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n    return sink(o.x);\n  }\n}", &[OK]);
    row(
        JS,
        "g(function (o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n});",
        &[OK],
    );
}

#[test]
fn an_alias_in_a_block_is_certified() {
    let src = "function g(o, c) {\n  if (c) {\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n    return sink(o.x);\n  }\n}";
    row(JS, src, &[OK]);
}

#[test]
fn several_aliases_of_one_root_are_each_certified() {
    let src = "function g(o) {\n  const u = o, v = o;\n  o.x = 0;\n  u.x = 1;\n  v.x = 2;\n  return sink(o.x);\n}";
    assert_eq!(certs(JS, src), ["admitted:-:u", "admitted:-:v"]);
}

#[test]
fn an_alias_of_an_alias_certifies_the_first_step_and_refuses_the_chain() {
    let src = "function g(o) {\n  const u = o;\n  const w = u;\n  o.x = 0;\n  w.x = 1;\n  return sink(o.x);\n}";
    assert_eq!(certs(JS, src), ["admitted:-:u", "refused:chain:w"]);
    let this =
        "function g(o) {\n  const t = this;\n  const u = t;\n  o.x = 0;\n  return sink(o.x);\n}";
    assert_eq!(certs(JS, this), ["refused:this-alias:u"]);
}

// ---- Refused: every row names the reason ---------------------------------------------------------------

fn refused(language: Language, src: &str, reason: &str) {
    assert_eq!(
        certs(language, src),
        [format!("refused:{reason}:u")],
        "{src}"
    );
}

fn g(body: &str) -> String {
    format!("function g(o, p, z, xs) {{\n{body}\n  return sink(o.x);\n}}")
}

#[test]
fn an_assigned_alias_is_refused() {
    for later in [
        "u = p;",
        "u++;",
        "u += 1;",
        "({ u } = z);",
        "[u] = z;",
        "for (u of xs) {}",
        "for (u in xs) {}",
        "(() => { u = p; })();",
        "const f = () => u++;",
    ] {
        refused(
            JS,
            &g(&format!("  let u = o;\n  o.x = 0;\n  u.x = 1;\n  {later}")),
            "alias-assigned",
        );
    }
}

#[test]
fn a_rebound_target_is_refused_wherever_the_write_is() {
    for later in [
        "o = p;",
        "o++;",
        "({ o } = z);",
        "for (o of xs) {}",
        "(() => { o = p; })();",
    ] {
        refused(
            JS,
            &g(&format!(
                "  const u = o;\n  o.x = 0;\n  u.x = 1;\n  {later}"
            )),
            "target-rebound",
        );
    }
    // A write before the alias counts too: the inventory is the whole owner, not the reaching writes.
    refused(
        JS,
        &g("  o = make();\n  const u = o;\n  u.x = 1;"),
        "target-rebound",
    );
}

#[test]
fn a_member_write_through_either_side_is_not_a_rebinding() {
    row(
        JS,
        &g("  const u = o;\n  o.x = 0;\n  u.x = 1;\n  o.y.z = 2;\n  u.w++;\n  delete u.q;"),
        &[OK],
    );
}

#[test]
fn a_redeclared_or_shadowed_alias_spelling_is_refused() {
    for shadow in [
        "{ const u = make(); }",
        "xs.forEach((u) => { u.x = 1; });",
        "xs.forEach(function (u) { u.x = 1; });",
        "const f = () => { const u = make(); return u; };",
        "try { z(); } catch (u) { u.x = 1; }",
        "function u() {}",
        "var u = make();",
    ] {
        refused(
            JS,
            &g(&format!(
                "  const u = o;\n  o.x = 0;\n  u.x = 1;\n  {shadow}"
            )),
            "alias-redeclared",
        );
    }
    // A shadow declared before the alias is the same spelling in the same owner.
    refused(
        JS,
        &g("  { const u = make(); }\n  const u = o;\n  u.x = 1;"),
        "alias-redeclared",
    );
}

#[test]
fn a_redeclared_or_shadowed_target_spelling_is_refused() {
    for shadow in [
        "{ const o = make(); }",
        "xs.forEach((o) => { o.x = 1; });",
        "const f = () => { const o = make(); return o; };",
        "try { z(); } catch (o) { o.x = 1; }",
    ] {
        let src = g(&format!(
            "  const u = o;\n  o.x = 0;\n  u.x = 1;\n  {shadow}"
        ));
        refused(JS, &src, "target-redeclared");
    }
    refused(
        JS,
        "function g(o) {\n  var o = make();\n  const u = o;\n  u.x = 1;\n  return sink(o.x);\n}",
        "target-redeclared",
    );
}

#[test]
fn a_target_with_no_declaration_in_the_owner_is_refused() {
    refused(JS, "function g() {\n  const u = outer;\n  outer.x = 0;\n  u.x = 1;\n  return sink(outer.x);\n}".replace("outer", "o").as_str(), "target-unresolved");
    // `o` declared in a block that does not contain the alias is another (outer) `o` at the alias.
    refused(JS, "function g(c) {\n  if (c) {\n    const o = make();\n    o.y = 1;\n  }\n  const u = o;\n  u.x = 1;\n  return sink(o.x);\n}", "target-identity");
    // A nested callable's declaration of `o` is not the owner's.
    refused(JS, "function g() {\n  const f = () => {\n    const o = make();\n    return o;\n  };\n  const u = o;\n  u.x = 1;\n  return f();\n}", "target-unresolved");
}

#[test]
fn a_target_declared_by_another_form_is_refused() {
    for head in [
        "function g({ o }) {",
        "function g([o]) {",
        "function g(o = make()) {",
        "function g(...o) {",
    ] {
        refused(
            JS,
            &format!("{head}\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}}"),
            "target-kind",
        );
    }
    for decl in [
        "const { o } = z();",
        "const [o] = z();",
        "function o() {}",
        "var o;",
        "class o {}",
    ] {
        let src = format!("function g(z) {{\n  {decl}\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}}");
        refused(JS, &src, "target-kind");
    }
    refused(JS, "function g() {\n  try { run(); } catch (o) {\n    const u = o;\n    u.x = 1;\n    return sink(o.x);\n  }\n}", "target-kind");
}

#[test]
fn a_parameter_property_formal_is_refused_on_identity() {
    // Fail closed: the producer's def for a parameter property is not at the identifier, so no identity.
    let prop = "class C {\n  constructor(private o: any) {\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n    return sink(o.x);\n  }\n}";
    refused(TS, prop, "target-identity");
}

#[test]
fn a_target_that_is_itself_an_alias_is_a_chain() {
    let src = "function g(p) {\n  const o = p;\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
    // `o -> p` is itself a certified single step; `u -> o` is the chain.
    assert_eq!(certs(JS, src), ["admitted:-:o", "refused:chain:u"]);
}

#[test]
fn a_target_declared_after_the_alias_is_refused() {
    refused(
        JS,
        "function g() {\n  const u = o;\n  var o = make();\n  u.x = 1;\n  return sink(o.x);\n}",
        "target-after-alias",
    );
}

#[test]
fn an_alias_formed_in_a_loop_is_refused() {
    for (open, close) in [
        ("for (const a of xs) {", "}"),
        ("for (const a in xs) {", "}"),
        ("for (let i = 0; i < 3; i++) {", "}"),
        ("while (cond()) {", "}"),
        ("do {", "} while (cond());"),
    ] {
        let src = g(&format!(
            "  {open}\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n  {close}"
        ));
        refused(JS, &src, "loop");
    }
    // The loop head's own declarator.
    refused(
        JS,
        &g("  for (let u = o; cond(); ) {\n    u.x = 1;\n  }"),
        "loop",
    );
    // A loop elsewhere in the owner does not matter.
    row(
        JS,
        &g("  const u = o;\n  for (const a of xs) {\n    u.x = 1;\n  }"),
        &[OK],
    );
}

#[test]
fn an_alias_in_a_nested_callable_is_that_callables_own_candidate() {
    let src = "function g(o) {\n  o.y = 1;\n  return () => {\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n    return sink(o.x);\n  };\n}";
    // The outer owner sees it in a deferred region; the arrow's own owner sees `o` as a free name.
    assert_eq!(
        certs(JS, src),
        ["refused:deferred-region:u", "refused:target-unresolved:u"]
    );
    let own = "function g(o) {\n  o.y = 1;\n  return () => {\n    const p = make();\n    const u = p;\n    p.x = 0;\n    u.x = 1;\n    return sink(p.x);\n  };\n}";
    assert_eq!(
        certs(JS, own),
        ["refused:deferred-region:u", "admitted:-:u"]
    );
}

#[test]
fn a_dynamic_binding_hazard_refuses_every_candidate_of_the_owner() {
    refused(
        JS,
        &g("  const u = o;\n  o.x = 0;\n  u.x = 1;\n  eval(p);"),
        "dynamic",
    );
    refused(
        JS,
        &g("  const u = o;\n  with (z) {\n    o.x = 0;\n  }\n  u.x = 1;"),
        "dynamic",
    );
    refused(JS, "function g(o) {\n  const u = o;\n  arguments[0] = {};\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}", "dynamic");
    refused(
        JS,
        "function g(o) {\n  function o() {}\n  const u = o;\n  u.x = 1;\n}",
        "dynamic",
    );
}

#[test]
fn a_self_reference_is_refused() {
    refused(
        JS,
        "function g(o) {\n  const u = u;\n  o.x = 0;\n  return sink(o.x);\n}",
        "self-reference",
    );
}

// ---- Not candidates: forms A does not model produce no certificate and no refusal line --------------------

#[test]
fn forms_that_are_not_a_bare_identifier_copy_have_no_certificate() {
    for decl in [
        "const u = o.p;",
        "const u = o[0];",
        "const u = (o);",
        "const u = o!;",
        "const u = o as any;",
        "const u = make(o);",
        "const u = o || p;",
        "const u = this;",
        "const { k: u } = o;",
        "const { u } = o;",
        "const [u] = o;",
        "const { ...u } = o;",
        "let u;",
    ] {
        let src = format!(
            "function g(o, p) {{\n  {decl}\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}}"
        );
        assert!(certs(TS, &src).is_empty(), "{decl}: {:?}", certs(TS, &src));
    }
}

#[test]
fn assigned_and_default_and_parameter_forms_have_no_certificate() {
    for src in [
        "function g(o) {\n  let u;\n  u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        "function g(o, u = o) {\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        "function g(o, u = o, w = u) {\n  o.x = 0;\n  w.x = 1;\n  return sink(o.x);\n}",
        "function g(o, { k: u } = o) {\n  o.k.x = 0;\n  u.x = 1;\n  return sink(o.k.x);\n}",
        "function g() {\n  const self = this;\n  this.x = 0;\n  self.x = 1;\n  return sink(this.x);\n}",
        "function g(o) {\n  o.x = 0;\n  return sink(o.x);\n}",
    ] {
        assert!(certs(JS, src).is_empty(), "{src}: {:?}", certs(JS, src));
    }
}

#[test]
fn a_refused_census_certifies_nothing() {
    // An escaped identifier refuses the whole owner (`owner-refuse:escape`): no inventory, no certificate.
    let src =
        "function g(o) {\n  const \\u0075 = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
    assert!(certs(JS, src).is_empty(), "{:?}", certs(JS, src));
}

// ---- Inert --------------------------------------------------------------------------------------------

#[test]
fn certificates_are_only_trace_lines() {
    use crate::cpg::census_label_tests::labels_of;
    let src = "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
    // The rows' labels are those of the tree without certificates (main: the write through the alias does
    // not stop `o.x@3`).
    assert_eq!(labels_of(JS, src, "o.x@3->o.x@5"), ["exact"]);
    assert_eq!(certs(JS, src), [OK]);
}

// ---- Fold 1 (review round 1 of #817) -----------------------------------------------------------------

#[test]
fn a_target_whose_initializer_is_a_wrapped_identifier_or_this_is_still_a_chain() {
    let body = "  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
    for init in ["(p)", "((p))"] {
        let src = format!("function g(p) {{\n  const o = {init};\n{body}");
        // `o = (p)` is no bare-identifier copy: only `u -> o` is a candidate, and `o` is not a root.
        assert_eq!(certs(JS, &src), ["refused:chain:u"], "{init}");
    }
    for init in [
        "p as any",
        "p!",
        "(p as any)",
        "p satisfies any",
        "<any>p",
        "(p!) as any",
    ] {
        let src = format!("function g(p: any) {{\n  const o = {init};\n{body}");
        let got = certs(TS, &src);
        assert_eq!(
            got.last().map(String::as_str),
            Some("refused:chain:u"),
            "{init}: {got:?}"
        );
    }
    for init in ["(this)", "(this as any)"] {
        let src = format!("function g() {{\n  const o = {init};\n{body}");
        assert_eq!(certs(TS, &src), ["refused:this-alias:u"], "{init}");
    }
    // Control: a wrapped call is still a root.
    let src = format!("function g() {{\n  const o = (make());\n{body}");
    assert_eq!(certs(JS, &src), [OK]);
    let src = format!("function g() {{\n  const o = make() as any;\n{body}");
    assert_eq!(certs(TS, &src), [OK]);
}

#[test]
fn an_alias_wrapped_on_the_alias_side_is_not_a_candidate() {
    for decl in ["const u = (o);", "const u = o!;", "const u = <any>o;"] {
        let src = format!(
            "function g(o: any) {{\n  {decl}\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}}"
        );
        assert!(certs(TS, &src).is_empty(), "{decl}");
    }
}

/// Recall residual, pinned: `o` declared by an earlier declarator of the same declaration has no declaration
/// identity at the alias (`BindingFacts` makes a binding visible from the end of its declaration). The count on
/// excalidraw, express and TypeScript/src is 3 in all (`target-identity` 2 + 1, none proven to be this form).
#[test]
fn an_earlier_declarator_of_the_same_declaration_is_refused_on_identity() {
    for kw in ["const", "let"] {
        let src = format!("function g() {{\n  {kw} o = make(), u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}}");
        assert_eq!(certs(JS, &src), ["refused:target-identity:u"], "{kw}");
    }
    // The two-statement control is admitted; `var` has one binding for the whole function.
    let two = "function g() {\n  const o = make();\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
    assert_eq!(certs(JS, two), [OK]);
    let var =
        "function g() {\n  var o = make(), u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}";
    assert_eq!(certs(JS, var), [OK]);
}

#[test]
fn writes_in_class_regions_and_parameter_defaults_count_as_bindings_of_u() {
    for rebind in [
        "class C { f = (u = p); }",
        "class C { static f = (u = p); }",
        "class C { static { u = p; } }",
        "class C { [(u = p)]() {} }",
        "const k = (a = (u = p)) => a;",
        "const k = ((u as any) = p);",
    ] {
        let src = format!(
            "function g(o, p) {{\n  const u = o;\n  o.x = 0;\n  {rebind}\n  return sink(o.x);\n}}"
        );
        assert_eq!(certs(TS, &src), ["refused:alias-assigned:u"], "{rebind}");
    }
    for shadow in [
        "const k = (u = 1) => u;",
        "const k = function (u) { return u; };",
    ] {
        let src = format!(
            "function g(o) {{\n  const u = o;\n  o.x = 0;\n  {shadow}\n  return sink(o.x);\n}}"
        );
        assert_eq!(certs(TS, &src), ["refused:alias-redeclared:u"], "{shadow}");
    }
    // A member write through `u` in those places is no rebinding.
    let ok = "function g(o) {\n  const u = o;\n  o.x = 0;\n  class C { static { u.x = 1; } }\n  return sink(o.x);\n}";
    assert_eq!(certs(JS, ok), [OK]);
}

#[test]
fn a_labelled_loop_and_annex_b_and_catch_hazards_refuse() {
    refused(JS, &g("  outer: for (const a of xs) {\n    const u = o;\n    u.x = 1;\n    continue outer;\n  }"), "loop");
    refused(
        JS,
        &g("  const u = o;\n  { function o() {} }\n  u.x = 1;"),
        // A formal's own name is outside Annex B: the block function is a second declaration of `o`.
        "target-redeclared",
    );
    refused(
        JS,
        &g("  const u = o;\n  { function u() {} }\n  u.x = 1;"),
        "dynamic",
    );
    refused(
        JS,
        &g("  try { run(); } catch (u) {\n    var u = o;\n    u.x = 1;\n  }"),
        "dynamic",
    );
}

// ---- Fold 2 (allow-list of supported target roots) ---------------------------------------------------------

fn root_row(language: Language, init: &str, want: &[&str]) {
    let async_ = if init.contains("await") { "async " } else { "" };
    let src = format!(
        "{async_}function g(p) {{\n  const o = {init};\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}}"
    );
    let got: Vec<String> = certs(language, &src)
        .into_iter()
        .filter(|c| c.ends_with(":u"))
        .collect();
    assert_eq!(got, want, "{init}");
}

#[test]
fn every_allow_listed_initializer_kind_makes_a_supported_root() {
    for init in [
        "make()",
        "make(p)",
        "(make())",
        "make?.()",
        "new Foo()",
        "new Foo",
        "1",
        "'s'",
        "`plain`",
        "/x/",
        "true",
        "false",
        "null",
        "{}",
        "[]",
        "function () {}",
        "() => 1",
        "class {}",
        "await make()",
        "(make() as any)",
    ] {
        root_row(TS, init, &[OK]);
    }
}

#[test]
fn a_forwarding_or_unknown_initializer_is_not_a_root() {
    for (init, reason) in [
        ("p", "chain"),
        ("(p)", "chain"),
        ("p as any", "chain"),
        // The grammar reads `p<number>` as a comparison, not an instantiation: refused as an unknown kind either way.
        ("p<number>", "target-kind"),
        ("await p", "chain"),
        ("(0, p)", "target-kind"),
        ("p || p", "target-kind"),
        ("p ?? p", "target-kind"),
        ("true ? p : p", "target-kind"),
        ("p.q", "target-kind"),
        ("p[0]", "target-kind"),
        ("-p", "target-kind"),
        ("p + 1", "target-kind"),
        ("`a${p}`", "target-kind"),
        ("(o2 = p)", "target-kind"),
        ("typeof p", "target-kind"),
    ] {
        root_row(TS, init, &[&format!("refused:{reason}:u")]);
    }
    root_row(TS, "this", &["refused:this-alias:u"]);
    root_row(TS, "(this)", &["refused:this-alias:u"]);
}
