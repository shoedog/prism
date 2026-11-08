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
    // Other owner kinds and dialects: the arrow (a synthetic owner: the twin is the alias def's own), a method, TS, TSX.
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
