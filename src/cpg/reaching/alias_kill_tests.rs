//! #783 option A, slice 4b: a write through a certified single-step alias gets the label effect of the same write spelled
//! on the aliased path (`alias_kill.rs`). Each row is a source and its `o.*` edges with the label they must carry
//! now and the label on the 4a tip (`base`, the RED column: a row whose base differs from its label moved from
//! `exact`, the alias-twin def never killing, to `alias_unstable`). Controls are rows whose base equals the label.
//! Outside the certified domain (chains, rebinding, parameter defaults, member-value and pattern aliases, `this`,
//! aliases and writes in nested callables, shadowed spellings) every row is a RESIDUAL: its label is the
//! rule-disabled baseline's, which is still the wrong `exact` of #783 and stays open.
use super::census_defs_tests::trace_lines;
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";
const KILLED: &str = "nameonly/killed";
const SAME: &str = "nameonly/sameline";
const ALIAS: &str = "NameOnly(AliasUnstable)";
/// The direct form has no edge: main's rule D prunes a def an unconditional later write dominates.
const PRUNED: &str = "-";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

struct Case {
    name: &'static str,
    lang: Language,
    source: &'static str,
    /// `(edge, label now, label on the 4a tip)`.
    rows: &'static [(&'static str, &'static str, &'static str)],
}

#[rustfmt::skip]
const CASES: &[Case] = &[
    Case { name: "assign", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "assign_multi", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  log();\n  u.x = 1;\n  log();\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, EXACT), ("o.x@5->o.x@7", ALIAS, ALIAS)] },
    Case { name: "direct_after_alias_decl", lang: JS, source: "function g(o) {\n  o.x = 0;\n  const u = o;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@5", KILLED, KILLED), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "update", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x += 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "incr", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "for_of_head", lang: JS, source: "function g(o, xs) {\n  const u = o;\n  o.x = 0;\n  for (u.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, EXACT)] },
    Case { name: "destructure_target", lang: JS, source: "function g(o, z) {\n  const u = o;\n  o.x = 0;\n  ({ k: u.x } = z);\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "array_target", lang: JS, source: "function g(o, z) {\n  const u = o;\n  o.x = 0;\n  [u.x] = z;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "prefix_write", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x.y = 0;\n  u.x = {};\n  return sink(o.x.y);\n}",
        rows: &[("o.x.y@3->o.x.y@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "deeper_write_no_kill", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x.y = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "sibling", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.y = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "branch_alias_write", lang: JS, source: "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    u.x = 1;\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", EXACT, EXACT), ("o.x@5->o.x@7", ALIAS, ALIAS)] },
    Case { name: "branch_both_arms", lang: JS, source: "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    u.x = 1;\n  } else {\n    u.x = 2;\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@9", ALIAS, EXACT), ("o.x@5->o.x@9", ALIAS, ALIAS), ("o.x@7->o.x@9", ALIAS, ALIAS)] },
    Case { name: "use_inside_branch_after_write", lang: JS, source: "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    u.x = 1;\n    return sink(o.x);\n  }\n  return 0;\n}",
        rows: &[("o.x@3->o.x@6", ALIAS, EXACT), ("o.x@5->o.x@6", ALIAS, ALIAS)] },
    Case { name: "loop_write_after_read", lang: JS, source: "function g(o, xs) {\n  const u = o;\n  o.x = 0;\n  for (const a of xs) {\n    sink(o.x);\n    u.x = 1;\n  }\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ts", lang: TS, source: "function g(o: any) {\n  const u: any = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "tsx", lang: TSX, source: "function g(o: any) {\n  const u: any = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "arrow", lang: JS, source: "const g = (o) => {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n};",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
    Case { name: "method", lang: JS, source: "class C {\n  m(o) {\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n    return sink(o.x);\n  }\n}",
        rows: &[("o.x@4->o.x@6", ALIAS, EXACT), ("o.x@5->o.x@6", ALIAS, ALIAS)] },
    Case { name: "for_in_head", lang: JS, source: "function g(o, xs) {\n  const u = o;\n  o.x = 0;\n  for (u.x in xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, EXACT)] },
    Case { name: "delete_through_alias", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  delete u.x;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "incr_branch_fallthrough", lang: JS, source: "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    u.x++;\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", EXACT, EXACT)] },
    Case { name: "incr_prefix", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x.y = 0;\n  u.x++;\n  return sink(o.x.y);\n}",
        rows: &[("o.x.y@3->o.x.y@5", ALIAS, EXACT)] },
    Case { name: "array_target_nested", lang: JS, source: "function g(o, z) {\n  const u = o;\n  o.x = 0;\n  [, u.x] = z;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "anon_incr", lang: JS, source: "g(function (o) {\n  const u = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n});",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "ts_incr", lang: TS, source: "function g(o: any) {\n  const u: any = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "tsx_for_of", lang: TSX, source: "function g(o: any, xs: any[]) {\n  const u: any = o;\n  o.x = 0;\n  for (u.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, EXACT)] },
    Case { name: "ctl_incr_unaliased", lang: JS, source: "function g(o, p) {\n  const u = p;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_incr_sibling", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.y++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_incr_before_direct", lang: JS, source: "function g(o) {\n  const u = o;\n  u.x++;\n  o.x = 0;\n  return sink(o.x);\n}",
        rows: &[("o.x@4->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_bare_incr", lang: JS, source: "function g(o) {\n  let u = o;\n  o.x = 0;\n  u++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_alias_read_only", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  log(u.x);\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_destructure_decl", lang: JS, source: "function g(o) {\n  o.name = 0;\n  const { name } = o;\n  return sink(o.name, name);\n}",
        rows: &[("o.name@2->o.name@4", KILLED, KILLED), ("o.name@3->o.name@4", ALIAS, ALIAS)] },
    Case { name: "ctl_alias_field_decl", lang: JS, source: "function g(o) {\n  o.p = 0;\n  const u = o.p;\n  return sink(o.p, u);\n}",
        rows: &[("o.p@2->o.p@3", EXACT, EXACT), ("o.p@2->o.p@4", EXACT, EXACT)] },
    Case { name: "ctl_other_alias", lang: JS, source: "function g(o, p) {\n  const u = p;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_no_alias", lang: JS, source: "function g(o) {\n  o.x = 0;\n  o.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", EXACT, EXACT)] },
    Case { name: "ctl_alias_write_before_direct", lang: JS, source: "function g(o) {\n  const u = o;\n  u.x = 1;\n  o.x = 0;\n  return sink(o.x);\n}",
        rows: &[("o.x@4->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_alias_to_field_not_alias", lang: JS, source: "function g(o) {\n  const u = o.p;\n  o.p.x = 0;\n  u.x++;\n  return sink(o.p.x);\n}",
        rows: &[("o.p.x@3->o.p.x@5", EXACT, EXACT)] },
    Case { name: "alias_to_field", lang: JS, source: "function g(o) {\n  const u = o.p;\n  o.p.x = 0;\n  u.x = 1;\n  return sink(o.p.x);\n}",
        rows: &[("o.p.x@3->o.p.x@5", EXACT, EXACT)] },
    Case { name: "one_node_multiline", lang: JS, source: "function g(o, u = o) {\n  return [\n    (o.x = 0),\n    (u.x = 1),\n    sink(o.x),\n  ];\n}",
        rows: &[("o.x@3->o.x@5", CFG, CFG), ("o.x@4->o.x@5", CFG, CFG)] },
    Case { name: "one_node_multiline_incr", lang: JS, source: "function g(o, u = o) {\n  return [\n    (o.x = 0),\n    u.x++,\n    sink(o.x),\n  ];\n}",
        rows: &[("o.x@3->o.x@5", CFG, CFG)] },
    Case { name: "ctl_alias_reassigned", lang: JS, source: "function g(o) {\n  let u = o;\n  o.x = 0;\n  u = {};\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
];

#[rustfmt::skip]
const RESIDUAL: &[Case] = &[
    Case { name: "chain_alias", lang: JS, source: "function g(o) {\n  const u = o;\n  const w = u;\n  o.x = 0;\n  w.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@4->o.x@6", EXACT, EXACT), ("o.x@5->o.x@6", ALIAS, ALIAS)] },
    Case { name: "chain_alias_incr", lang: JS, source: "function g(o) {\n  const u = o;\n  const w = u;\n  o.x = 0;\n  w.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@4->o.x@6", EXACT, EXACT)] },
    Case { name: "param_alias", lang: JS, source: "function f(o, u = o) {\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@4", EXACT, EXACT), ("o.x@3->o.x@4", ALIAS, ALIAS)] },
    Case { name: "param_alias_incr", lang: JS, source: "function f(o, u = o) {\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@4", EXACT, EXACT)] },
    Case { name: "param_alias_for_of", lang: JS, source: "function f(o, xs, u = o) {\n  o.x = 0;\n  for (u.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@6", EXACT, EXACT)] },
    Case { name: "pattern_alias_field", lang: JS, source: "function g(o) {\n  const { k: a } = o;\n  o.k.x = 0;\n  a.x = 1;\n  return sink(o.k.x);\n}",
        rows: &[("o.k.x@3->o.k.x@5", EXACT, EXACT), ("o.k.x@4->o.k.x@5", ALIAS, ALIAS), ("o.k@2->o.k@3", ALIAS, ALIAS), ("o.k@2->o.k@5", ALIAS, ALIAS)] },
    Case { name: "pattern_alias_field_incr", lang: JS, source: "function g(o) {\n  const { k: a } = o;\n  o.k.x = 0;\n  a.x++;\n  return sink(o.k.x);\n}",
        rows: &[("o.k.x@3->o.k.x@5", EXACT, EXACT), ("o.k@2->o.k@3", ALIAS, ALIAS), ("o.k@2->o.k@5", ALIAS, ALIAS)] },
];

fn failures_of(cases: &[Case]) -> Vec<String> {
    let mut failures = Vec::new();
    for c in cases {
        for (edge, want, _base) in c.rows {
            let got = labels_of(c.lang, c.source, edge);
            if got.is_empty() || got.iter().any(|g| g != want) {
                failures.push(format!("{} {edge}: want {want}, got {got:?}", c.name));
            }
        }
    }
    failures
}

#[test]
fn a_write_through_a_certified_alias_may_kills_an_earlier_direct_def() {
    let failures = failures_of(CASES);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The certified domain's forms move; its controls and everything outside it do not.
#[test]
fn the_rule_moves_the_certified_forms_and_nothing_else_listed() {
    let moved: Vec<&str> = CASES
        .iter()
        .filter(|c| c.rows.iter().any(|(_, now, base)| now != base))
        .map(|c| c.name)
        .collect();
    for form in [
        "assign",
        "update",
        "incr",
        "for_of_head",
        "for_in_head",
        "destructure_target",
        "array_target",
        "prefix_write",
        "ts",
        "tsx",
        "arrow",
        "method",
        "anon_incr",
        "delete_through_alias",
        "ts_incr",
        "tsx_for_of",
        "branch_both_arms",
        "use_inside_branch_after_write",
        "incr_prefix",
    ] {
        assert!(moved.contains(&form), "{form} must move: {moved:?}");
    }
    for control in [
        "sibling",
        "deeper_write_no_kill",
        "branch_alias_write",
        "loop_write_after_read",
        "ctl_destructure_decl",
        "ctl_other_alias",
        "ctl_no_alias",
        "alias_to_field",
        "one_node_multiline",
        "ctl_alias_reassigned",
        "incr_branch_fallthrough",
        "ctl_bare_incr",
        "ctl_alias_read_only",
    ] {
        assert!(
            !moved.contains(&control),
            "{control} must not move: {moved:?}"
        );
    }
    assert!(RESIDUAL
        .iter()
        .all(|c| c.rows.iter().all(|(_, now, base)| now == base)));
}

/// RESIDUAL: the forms option A does not certify keep the rule-disabled baseline's label (the wrong `exact`).
#[test]
fn the_residual_forms_keep_the_baseline_label() {
    let failures = failures_of(RESIDUAL);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Residual rows whose alias relation is refused by identity or inventory (`(name, source, edge)`): each stays
/// at its baseline label. Written as `(name, source, edge, label)` with the label the 4a tip gives.
#[rustfmt::skip]
const REFUSED: &[(&str, &str, &str, &str)] = &[
    ("reassigned_after", "function g(o, p) {\n  let u = o;\n  o.x = 0;\n  u.x = 1;\n  u = p;\n  return sink(o.x);\n}", "o.x@3->o.x@6", EXACT),
    ("reassigned_before", "function g(o, p) {\n  let u = o;\n  o.x = 0;\n  u = {};\n  u.x = 1;\n  return sink(o.x);\n}", "o.x@3->o.x@6", EXACT),
    ("target_rebound", "function g(o, p) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  o = p;\n  return sink(o.x);\n}", "o.x@3->o.x@6", KILLED),
    ("shadowed_alias_block", "function g(o, p, c) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  {\n    const u = p;\n    log(u);\n  }\n  return sink(o.x);\n}", "o.x@3->o.x@9", EXACT),
    ("shadowed_alias_callable", "function g(o, p) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  const h = () => {\n    const u = p;\n    return u;\n  };\n  return sink(o.x, h);\n}", "o.x@3->o.x@9", EXACT),
    ("shadowed_target", "function g(o, p) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  const h = (o) => o;\n  return sink(o.x, h);\n}", "o.x@3->o.x@6", EXACT),
    ("outer_param_u_nested_alias", "function g(o, u) {\n  const f = () => {\n    const u = o;\n    return u;\n  };\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x, f);\n}", "o.x@6->o.x@8", EXACT),
    ("alias_in_loop", "function g(o, xs) {\n  for (const a of xs) {\n    const u = o;\n    o.x = 0;\n    u.x = 1;\n    log(o.x);\n  }\n}", "o.x@4->o.x@6", EXACT),
    ("write_in_closure", "function g(o) {\n  const u = o;\n  o.x = 0;\n  const f = () => {\n    u.x = 1;\n  };\n  return sink(o.x, f);\n}", "o.x@3->o.x@7", EXACT),
    ("write_in_closure_incr", "function g(o) {\n  const u = o;\n  o.x = 0;\n  const f = () => {\n    u.x++;\n  };\n  return sink(o.x, f);\n}", "o.x@3->o.x@7", EXACT),
    ("alias_in_closure", "function g(o) {\n  o.x = 0;\n  const f = () => {\n    const u = o;\n    u.x = 1;\n  };\n  return sink(o.x, f);\n}", "o.x@2->o.x@7", EXACT),
    ("this_alias", "function G(xs) {\n  var self = this;\n  this.x = 0;\n  self.x = 1;\n  return sink(this.x);\n}", "this.x@3->this.x@5", EXACT),
    ("member_value_alias", "function g(o) {\n  const u = o.p;\n  o.p.x = 0;\n  u.x = 1;\n  return sink(o.p.x);\n}", "o.p.x@3->o.p.x@5", EXACT),
    ("pattern_alias", "function g(o) {\n  const { k: a } = o;\n  o.k.x = 0;\n  a.x = 1;\n  return sink(o.k.x);\n}", "o.k.x@3->o.k.x@5", EXACT),
    ("target_is_alias_chain", "function g(p) {\n  const o = p;\n  const u = o;\n  p.x = 0;\n  u.x = 1;\n  return sink(p.x);\n}", "p.x@4->p.x@6", EXACT),
    ("free_target", "function g() {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}", "o.x@3->o.x@5", EXACT),
    ("eval_owner", "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  eval(o);\n  return sink(o.x);\n}", "o.x@3->o.x@6", CFG),
];

#[test]
fn a_refused_relation_keeps_the_baseline_label() {
    let mut failures = Vec::new();
    for (name, source, edge, want) in REFUSED {
        let got = labels_of(JS, source, edge);
        if got.is_empty() || got.iter().any(|g| g != want) {
            failures.push(format!("{name} {edge}: want {want}, got {got:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A write through an alias and the same write spelled on the aliased path, in one template (`{W}` is `u` or
/// `o`). `rows` are the edges from the first write: `(edge, alias label, direct label)`. The parity rule: the
/// alias edge is `exact` exactly when the direct edge is, and where the direct write is a proven kill the alias
/// label is `AliasUnstable`, because the alias may not hold.
struct Pair {
    name: &'static str,
    lang: Language,
    template: &'static str,
    rows: &'static [(&'static str, &'static str, &'static str)],
}

#[rustfmt::skip]
const PAIRS: &[Pair] = &[
    Pair { name: "use_node_comma", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  return ({W}.x = 1, sink(o.x));\n}",
        rows: &[("o.x@3->o.x@4", SAME, SAME)] },
    Pair { name: "use_node_oneline", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  {W}.x = 1; return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", SAME, SAME)] },
    Pair { name: "use_node_incr_oneline", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  {W}.x++; return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", SAME, SAME)] },
    Pair { name: "use_node_multiline", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  return [\n    {W}.x = 1,\n    sink(o.x),\n  ];\n}",
        rows: &[("o.x@3->o.x@6", SAME, SAME)] },
    Pair { name: "use_before_write", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  return (sink(o.x), {W}.x = 1);\n}",
        rows: &[("o.x@3->o.x@4", EXACT, EXACT)] },
    Pair { name: "use_before_write_multiline", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  return [\n    sink(o.x),\n    {W}.x = 1,\n  ];\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Pair { name: "multiple_reads_one_after", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  return [\n    sink(o.x), {W}.x = 1, sink(o.x),\n  ];\n}",
        rows: &[("o.x@3->o.x@5", SAME, SAME)] },
    Pair { name: "def_node_alias_after", lang: JS, template: "function g(o) {\n  const u = o;\n  (o.x = 0, {W}.x++);\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", SAME, SAME)] },
    Pair { name: "def_node_alias_before", lang: JS, template: "function g(o) {\n  const u = o;\n  ({W}.x++, o.x = 0);\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", EXACT, EXACT)] },
    Pair { name: "def_node_incr_separate_statement", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0; {W}.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", SAME, SAME)] },
    Pair { name: "def_node_assign_separate_statement", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0; {W}.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", SAME, SAME)] },
    Pair { name: "or_assign", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 2;\n  {W}.x ||= 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, KILLED)] },
    Pair { name: "and_assign", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 2;\n  {W}.x &&= 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, KILLED)] },
    Pair { name: "nullish_assign", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 2;\n  {W}.x ??= 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, KILLED)] },
    Pair { name: "for_of", lang: JS, template: "function g(o, xs) {\n  const u = o;\n  o.x = 0;\n  for ({W}.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, KILLED)] },
    Pair { name: "for_await", lang: JS, template: "async function g(o, xs) {\n  const u = o;\n  o.x = 0;\n  for await ({W}.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, KILLED)] },
    Pair { name: "plain_assign", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  {W}.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, PRUNED)] },
    Pair { name: "if_fallthrough", lang: JS, template: "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    {W}.x = 1;\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", EXACT, EXACT)] },
    Pair { name: "if_both_arms", lang: JS, template: "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    {W}.x = 1;\n  } else {\n    {W}.x = 2;\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@9", ALIAS, KILLED)] },
    Pair { name: "rhs_reads_the_path", lang: JS, template: "function g(o) {\n  const u = o;\n  o.x = 0;\n  {W}.x = o.x + 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", EXACT, EXACT)] },
];

#[test]
fn a_write_through_a_certified_alias_has_the_label_effect_of_the_direct_write() {
    let mut failures = Vec::new();
    for p in PAIRS {
        let alias = p.template.replace("{W}", "u");
        let direct = p.template.replace("{W}", "o");
        for (edge, want, want_direct) in p.rows {
            let a = labels_of(p.lang, &alias, edge);
            let d = labels_of(p.lang, &direct, edge);
            let exact = |l: &[String]| l.iter().all(|x| x == EXACT);
            if a.is_empty() || a.iter().any(|g| g != want) {
                failures.push(format!("{} {edge} alias: want {want}, got {a:?}", p.name));
            }
            let pruned = *want_direct == PRUNED;
            if d.is_empty() != pruned || d.iter().any(|g| g != want_direct) {
                failures.push(format!(
                    "{} {edge} direct: want {want_direct}, got {d:?}",
                    p.name
                ));
            }
            // A pruned direct edge is a proven kill: the alias edge must not stay exact.
            if exact(&a) != (exact(&d) && !pruned) {
                failures.push(format!(
                    "{} {edge}: parity broken, alias {a:?} direct {d:?}",
                    p.name
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The trace names `kill:alias` / `alias-d3` on the edges the rule moved, with an `rd-alias-move` record that
/// names the raw write, the alias def and both declarations; and nothing on an edge it did not move.
#[test]
fn the_trace_names_the_moved_edges_and_their_writes() {
    let src = "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}";
    let lines = trace_lines(JS, src);
    let named: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("rd-census\tkill:alias\t"))
        .collect();
    assert!(
        named
            .iter()
            .any(|l| l.contains("o.x@3:") && l.contains("o.x@5:")),
        "{lines:?}"
    );
    let moves: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("rd-alias-move\t"))
        .collect();
    assert!(
        moves.iter().any(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            f[1] == "kill:alias"
                && f[6].starts_with("u.x@4:")
                && f[7].starts_with("u@2:")
                && f[8].starts_with("u@2:")
                && f[9].starts_with("o@1:")
                && f[10] == "o.x"
        }),
        "{moves:?}"
    );
    // A bypassing write moves nothing: no mechanism line, no record.
    let bypass = "function g(o, c) {\n  const u = o;\n  o.x = 0;\n  if (c) {\n    u.x++;\n  }\n  return sink(o.x);\n}";
    let lines = trace_lines(JS, bypass);
    assert!(
        !lines
            .iter()
            .any(|l| l.starts_with("rd-alias-move\t") || l.contains("\tkill:alias\t")),
        "{lines:?}"
    );
}

/// Without a certificate the owner keeps its labels and records nothing.
#[test]
fn a_refused_owner_has_no_alias_mechanism_in_its_trace() {
    for (_, source, _, _) in REFUSED {
        let lines = trace_lines(JS, source);
        assert!(
            !lines.iter().any(|l| l.starts_with("rd-alias-move\t")
                || l.contains("\tkill:alias\t")
                || l.contains("\talias-d3\t")),
            "{source}: {lines:?}"
        );
    }
}
