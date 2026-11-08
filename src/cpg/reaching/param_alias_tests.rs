//! #761: a parameter-list alias `u = o` is the alias the body statement `const u = o` is. Each row is a
//! parameter-list shape (P) whose edges are pinned beside the body twin's behaviour (B, the same rows
//! the matrix script `scripts/matrix761.py` compares): the projection edge `o@L -> o.x@N` of a
//! member-only formal is gone (the formal is mixed), and a write through the alias reaches `o.x` as an
//! alias twin, so `o` is never `exact` across it. Controls (a default that reads `o.x`, a formal with no
//! parameter-list read) must not move. Rows marked RESIDUAL are forms the body machinery does not alias
//! either; they pin parity (mixed formal, edge stays `exact`), not correctness.
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";
const SAME: &str = "nameonly/sameline";
const ALIAS: &str = "NameOnly(AliasUnstable)";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

struct Case {
    name: &'static str,
    lang: Language,
    source: &'static str,
    /// Edges that must carry exactly this label.
    present: &'static [(&'static str, &'static str)],
    /// Edges that must not exist (the member-only projection edge a mixed formal does not have).
    absent: &'static [&'static str],
}

#[rustfmt::skip]
const CASES: &[Case] = &[
    // The issue's example, one node (a single body statement): the update through `u` is the census's kill,
    // the twin def of `o` shares the signature node, so the edge is label-refused, never exact.
    Case { name: "update_default", lang: JS,
        source: "function f(o, u = o, v = u.x++) {\n  return sink(o.x);\n}",
        present: &[("o@1->o@2", CFG)], absent: &["o@1->o.x@2"] },
    Case { name: "update_default_multi", lang: JS,
        source: "function f(o, u = o, v = u.x++) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME)], absent: &["o@1->o.x@3"] },
    Case { name: "assign_default_multi", lang: JS,
        source: "function f(o, u = o, v = (u.x = 1)) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME), ("o.x@1->o.x@3", ALIAS)], absent: &["o@1->o.x@3"] },
    Case { name: "assign_default_one_node", lang: JS,
        source: "function f(o, u = o, v = (u.x = 1)) {\n  return sink(o.x);\n}",
        present: &[("o@1->o@2", CFG), ("o.x@1->o.x@2", CFG)], absent: &["o@1->o.x@2"] },
    Case { name: "delete_default", lang: JS,
        source: "function f(o, u = o, v = delete u.x) {\n  return sink(o.x);\n}",
        present: &[("o@1->o@2", CFG)], absent: &["o@1->o.x@2"] },
    // The write in the body.
    Case { name: "body_write", lang: JS,
        source: "function f(o, u = o) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME), ("o.x@2->o.x@3", ALIAS), ("u@1->u@2", EXACT)], absent: &["o@1->o.x@3"] },
    Case { name: "body_write_multi", lang: JS,
        source: "function f(o, u = o) {\n  log();\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@4", SAME), ("o.x@3->o.x@4", ALIAS), ("u@1->u@3", EXACT)], absent: &["o@1->o.x@4"] },
    Case { name: "body_update", lang: JS,
        source: "function f(o, u = o) {\n  u.x++;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME)], absent: &["o@1->o.x@3"] },
    Case { name: "body_delete", lang: JS,
        source: "function f(o, u = o) {\n  delete u.x;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME)], absent: &["o@1->o.x@3"] },
    Case { name: "chain_alias", lang: JS,
        source: "function f(o, u = o, w = u, v = (w.x = 1)) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME), ("o.x@1->o.x@3", ALIAS)], absent: &["o@1->o.x@3"] },
    // Other owner kinds and dialects: a `const f = ..` arrow (named `f`), a method, TS, TSX. Truly anonymous owners are below.
    Case { name: "arrow", lang: JS,
        source: "const f = (o, u = o) => {\n  u.x = 1;\n  return sink(o.x);\n};",
        present: &[("o@1->o@3", SAME), ("o.x@2->o.x@3", ALIAS)], absent: &["o@1->o.x@3"] },
    Case { name: "arrow_default_write", lang: JS,
        source: "const f = (o, u = o, v = (u.x = 1)) => {\n  log();\n  return sink(o.x);\n};",
        present: &[("o@1->o@3", SAME)], absent: &["o@1->o.x@3"] },
    Case { name: "method", lang: JS,
        source: "class C {\n  m(o, u = o) {\n    u.x = 1;\n    return sink(o.x);\n  }\n}",
        present: &[("o@2->o@4", SAME), ("o.x@3->o.x@4", ALIAS)], absent: &["o@2->o.x@4"] },
    Case { name: "ts", lang: TS,
        source: "function f(o: any, u: any = o) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME), ("o.x@2->o.x@3", ALIAS)], absent: &["o@1->o.x@3"] },
    Case { name: "tsx", lang: TSX,
        source: "function f(o: any, u: any = o) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", SAME), ("o.x@2->o.x@3", ALIAS)], absent: &["o@1->o.x@3"] },
    // An outer binding: nothing in the signature binds `o`, the alias twin still writes it.
    Case { name: "outer_binding", lang: JS,
        source: "const o = src();\nfunction f(u = o) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o.x@3->o.x@4", ALIAS), ("o@2->o@4", ALIAS)], absent: &[] },
    // A read-only alias: the formal is mixed (a label cost against the old member-only projection).
    Case { name: "read_only_alias", lang: JS,
        source: "function f(o, u = o) {\n  return sink(o.x, u);\n}",
        present: &[("o@1->o@2", CFG)], absent: &["o@1->o.x@2"] },
    // A bare read that is not an alias makes the formal mixed too (the body's `const u = g(o)`), and an edge
    // that was exact stays exact.
    Case { name: "bare_call_default", lang: JS,
        source: "function f(o, u = g(o)) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", EXACT)], absent: &["o@1->o.x@3"] },
    Case { name: "bare_nested_callable_default", lang: JS,
        source: "function f(o, u = () => o) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", EXACT)], absent: &["o@1->o.x@3"] },
    Case { name: "bare_object_default_shorthand", lang: JS,
        source: "function f(o, { k = o } = {}) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", EXACT)], absent: &["o@1->o.x@3"] },
    // RESIDUAL: the body does not alias a pattern default (`const { a: u } = { a: o }`), so neither does the
    // list; the bare read makes `o` mixed and a write through `u` stays unseen (`exact`).
    Case { name: "residual_object_pattern_default", lang: JS,
        source: "function f(o, { a: u } = { a: o }) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", EXACT)], absent: &["o@1->o.x@3"] },
    Case { name: "residual_array_pattern_default", lang: JS,
        source: "function f(o, [u] = [o]) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@3", EXACT)], absent: &["o@1->o.x@3"] },
    // Controls that must not move: a member read in the default is not an alias, a formal with no
    // parameter-list read keeps its projection edge, another formal's alias does not touch `o`.
    Case { name: "ctl_member_read_default", lang: JS,
        source: "function f(o, u = o.x) {\n  log();\n  return sink(o.x, u);\n}",
        present: &[("o@1->o.x@3", EXACT)], absent: &["o@1->o@3"] },
    Case { name: "ctl_no_param_read", lang: JS,
        source: "function f(o, u = 1) {\n  log();\n  return sink(o.x, u);\n}",
        present: &[("o@1->o.x@3", EXACT)], absent: &["o@1->o@3"] },
    Case { name: "ctl_other_formal_alias", lang: JS,
        source: "function f(o, p, u = p, v = (u.x = 1)) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o.x@3", EXACT)], absent: &["o@1->o@3"] },
    // A subscript write through the formal in the list is `own-params`' domain, not a bare read: the formal stays
    // member-only and keeps its refused projection edge.
    Case { name: "ctl_computed_write_is_not_a_read", lang: JS,
        source: "function h(o, k, u = (o[k] = 1)) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o.x@3", CFG)], absent: &["o@1->o@3"] },
    Case { name: "ctl_ts_type_position_is_not_a_read", lang: TS,
        source: "function f(o: any, u: typeof o = 1) {\n  log();\n  return sink(o.x, u);\n}",
        present: &[("o@1->o.x@3", EXACT)], absent: &["o@1->o@3"] },
    // A multi-line signature (#782 round 1): the alias def and its twin sit on the owner's first line, where main puts
    // every formal's def, whatever row the alias is written on. A twin on its own row maps to no CFG node, neither kills
    // nor collapses with the formal's def, and left `o` exact across the write.
    Case { name: "ml_body_write", lang: JS,
        source: "function f(\n  o,\n  u = o,\n) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@6", SAME), ("o.x@5->o.x@6", ALIAS), ("u@1->u@5", EXACT)], absent: &["o@1->o.x@6"] },
    Case { name: "ml_list_write", lang: JS,
        source: "function f(\n  o,\n  u = o,\n  v = (u.x = 1),\n) {\n  log();\n  return sink(o.x);\n}",
        present: &[("o@1->o@7", SAME)], absent: &["o@1->o.x@7"] },
    Case { name: "ml_o_on_second_line", lang: JS,
        source: "function f(a,\n  o, u = o) {\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@4", SAME), ("o.x@3->o.x@4", ALIAS)], absent: &["o@1->o.x@4"] },
    Case { name: "ml_anonymous_function", lang: JS,
        source: "g(function (\n  o,\n  u = o,\n) {\n  u.x = 1;\n  return sink(o.x);\n});",
        present: &[("o@1->o@6", SAME)], absent: &["o@1->o.x@6"] },
    Case { name: "ml_ts_method", lang: TS,
        source: "class C {\n  m(\n    o: any,\n    u: any = o,\n  ) {\n    u.x = 1;\n    return sink(o.x);\n  }\n}",
        present: &[("o@2->o@7", SAME), ("o.x@6->o.x@7", ALIAS)], absent: &["o@2->o.x@7"] },
    // Truly anonymous owners (`<anon@L:C>`, the synthetic pass: the twin is the alias def's own span).
    Case { name: "anonymous_arrow_call_argument", lang: JS,
        source: "g((o, u = o) => {\n  u.x = 1;\n  return sink(o.x);\n});",
        present: &[("o@1->o@3", SAME), ("u@1->u@2", EXACT)], absent: &["o@1->o.x@3"] },
    Case { name: "anonymous_export_default_arrow", lang: JS,
        source: "export default (o, u = o) => {\n  u.x = 1;\n  return sink(o.x);\n};",
        present: &[("o@1->o@3", SAME), ("u@1->u@2", EXACT)], absent: &["o@1->o.x@3"] },
    Case { name: "anonymous_function_argument", lang: JS,
        source: "g(function (o, u = o) {\n  u.x = 1;\n  return sink(o.x);\n});",
        present: &[("o@1->o@3", SAME), ("u@1->u@2", EXACT)], absent: &["o@1->o.x@3"] },
    // A refused formal (seam `var o`, direct `eval`) in an anonymous callable has no def, and an alias of it mints
    // neither the alias binding's def nor a twin (`file.rs` and the frozen producer exclude it).
    Case { name: "anonymous_seam_formal_alias_is_refused", lang: JS,
        source: "g(function (o, u = o) {\n  var o;\n  u.x = 1;\n  return sink(o.x);\n});",
        present: &[], absent: &["u@1->u@3", "o@1->o@4", "o@1->o.x@4"] },
    Case { name: "anonymous_eval_formal_alias_is_refused", lang: JS,
        source: "g(function (o, u = o) {\n  eval(src);\n  u.x = 1;\n  return sink(o.x);\n});",
        present: &[], absent: &["u@1->u@3", "o@1->o@4", "o@1->o.x@4"] },
    // Destructuring defaults with an identifier right-hand side are the body's `const [u] = o` /
    // `const { a: u } = o` (`PATTERN_BODY_TWINS` pins the body's label beside each). Array patterns alias `o` itself
    // (named owners only, as in the body): the twin collapses with the formal's def. Object patterns alias `o.a`:
    // field twins as in the body. RESIDUAL: the twin is a different path from the formal's def and RD cannot order
    // two defs of one node, so `o@1->o@4` stays `exact` (the body kills it with the twin on a later line).
    Case { name: "array_pattern_identifier_rhs", lang: JS,
        source: "function f(o, [u] = o) {\n  log();\n  u.x = 1;\n  return sink(o[0].x);\n}",
        present: &[("o@1->o@4", SAME), ("u@1->u@3", EXACT)], absent: &["o@1->o.x@4"] },
    Case { name: "object_pattern_identifier_rhs", lang: JS,
        source: "function f(o, { a: u } = o) {\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n}",
        present: &[("o.a@1->o.a@4", ALIAS), ("o.a.x@3->o.a.x@4", ALIAS), ("u@1->u@3", EXACT), ("o@1->o@4", EXACT)], absent: &[] },
    Case { name: "object_pattern_shorthand_identifier_rhs", lang: JS,
        source: "function f(o, { a } = o) {\n  log();\n  a.x = 1;\n  return sink(o.a.x);\n}",
        present: &[("o.a@1->o.a@4", ALIAS), ("a@1->a@3", EXACT)], absent: &[] },
    Case { name: "object_pattern_identifier_rhs_ts", lang: TS,
        source: "function f(o: any, { a: u }: any = o) {\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n}",
        present: &[("o.a@1->o.a@4", ALIAS), ("u@1->u@3", EXACT)], absent: &[] },
    Case { name: "object_pattern_identifier_rhs_anonymous", lang: JS,
        source: "g(function (o, { a: u } = o) {\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n});",
        present: &[("o.a@1->o.a@4", ALIAS), ("u@1->u@3", EXACT)], absent: &[] },
    // Parity with the body's limits: an anonymous owner does not alias an array pattern (`js_ts_own_alias_target`), and
    // a defaulted pattern entry (`{ a: u = 1 }`) is no alias in either owner kind.
    Case { name: "array_pattern_identifier_rhs_anonymous_is_not_aliased", lang: JS,
        source: "g(function (o, [u] = o) {\n  log();\n  u.x = 1;\n  return sink(o[0].x);\n});",
        present: &[("o@1->o@4", EXACT), ("u@1->u@3", EXACT)], absent: &[] },
    Case { name: "object_pattern_value_default_is_not_aliased", lang: JS,
        source: "function f(o, { a: u = 1 } = o) {\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n}",
        present: &[("o@1->o@4", EXACT)], absent: &["o.a@1->o.a@4"] },
    // Negative controls: a literal right-hand side (the documented residual) and a TDZ forward reference mint no alias
    // and no alias binding def.
    Case { name: "ctl_object_pattern_literal_rhs", lang: JS,
        source: "function f(o, { a: u } = { a: o }) {\n  log();\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@4", EXACT)], absent: &["u@1->u@3"] },
    Case { name: "ctl_array_pattern_literal_rhs", lang: JS,
        source: "function f(o, [u] = [o]) {\n  log();\n  u.x = 1;\n  return sink(o.x);\n}",
        present: &[("o@1->o@4", EXACT)], absent: &["u@1->u@3"] },
    Case { name: "ctl_array_pattern_tdz_forward_reference", lang: JS,
        source: "function f([u] = o, o) {\n  log();\n  u.x = 1;\n  return sink(o[0].x);\n}",
        present: &[("o@1->o@4", EXACT)], absent: &["u@1->u@3"] },
];

#[test]
fn parameter_list_alias_rows_match_the_body_alias_rule() {
    let mut failures = Vec::new();
    for c in CASES {
        for (edge, want) in c.present {
            let got = labels_of(c.lang, c.source, edge);
            if got.is_empty() || got.iter().any(|g| g != want) {
                failures.push(format!("{} {edge}: want {want}, got {got:?}", c.name));
            }
        }
        for edge in c.absent {
            let got = labels_of(c.lang, c.source, edge);
            if !got.is_empty() {
                failures.push(format!("{} {edge}: want no edge, got {got:?}", c.name));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The body twins of the pattern rows: `const [u] = o` / `const { a: u } = o` on a line of their own. The twin def kills the
/// formal's edge (`killed`), which a parameter default cannot do (see the RESIDUAL note on the object row).
const PATTERN_BODY_TWINS: &[(&str, &str, &str, &str)] = &[
    ("array", "function f(o) {\n  const [u] = o;\n  log();\n  u.x = 1;\n  return sink(o[0].x);\n}", "o@1->o@5", "nameonly/killed"),
    ("object", "function f(o) {\n  const { a: u } = o;\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n}", "o@1->o@5", "nameonly/killed"),
    ("object field twin", "function f(o) {\n  const { a: u } = o;\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n}", "o.a@2->o.a@5", ALIAS),
    ("object, value default (body does not alias)", "function f(o) {\n  const { a: u = 1 } = o;\n  log();\n  u.x = 1;\n  return sink(o.a.x);\n}", "o@1->o@5", EXACT),
];

#[test]
fn pattern_rows_have_the_body_twins_this_pr_follows() {
    let mut failures = Vec::new();
    for (name, source, edge, want) in PATTERN_BODY_TWINS {
        let got = labels_of(JS, source, edge);
        if got.is_empty() || got.iter().any(|g| g != want) {
            failures.push(format!("body twin {name} {edge}: want {want}, got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The rows named `anonymous_*` are owned by a synthetic `<anon@L:C>` callable, the rows without `const f =` a name.
#[test]
fn anonymous_rows_are_owned_by_an_anonymous_callable() {
    for c in CASES
        .iter()
        .filter(|c| c.name.contains("anonymous") && !c.name.starts_with("ml_"))
    {
        let file = crate::ast::ParsedFile::parse("a.js", c.source, c.lang).unwrap();
        assert!(
            file.all_functions().iter().any(|f| file
                .dfg_owner_name(f)
                .is_some_and(|n| crate::ast::is_synthetic_owner(&n))),
            "{}: no anonymous owner in {}",
            c.name,
            c.source
        );
    }
}
