//! #783: a write through an alias may-kills an earlier direct def of the same path (`alias_kill.rs`). Each row is
//! a source and its `o.*` edges with the label they must carry now and the label 4c8620a2 gave them (`base`, the
//! RED column: a row whose base differs from its label moved from `exact` to `alias_unstable`). Controls are rows
//! whose base equals the label: no write through an alias, a sibling or deeper path, a fall-through route that
//! bypasses the write, a declaration twin (`const { name } = o` is not a write), a write before the alias or the
//! direct def, an alias that is not one (`const u = o.p`), and a one-node owner (already `cfg_incomplete`).
use super::census_defs_tests::trace_lines;
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;

const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";
const KILLED: &str = "nameonly/killed";
const ALIAS: &str = "NameOnly(AliasUnstable)";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

struct Case {
    name: &'static str,
    lang: Language,
    source: &'static str,
    /// `(edge, label now, label on 4c8620a2)`.
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
    Case { name: "issue_form", lang: JS, source: "function g(o) {\n  const u = o;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT), ("o.x@4->o.x@5", ALIAS, ALIAS)] },
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
    Case { name: "chain_alias", lang: JS, source: "function g(o) {\n  const u = o;\n  const w = u;\n  o.x = 0;\n  w.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@4->o.x@6", ALIAS, EXACT), ("o.x@5->o.x@6", ALIAS, ALIAS)] },
    Case { name: "param_alias", lang: JS, source: "function f(o, u = o) {\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@4", ALIAS, EXACT), ("o.x@3->o.x@4", ALIAS, ALIAS)] },
    Case { name: "alias_to_field", lang: JS, source: "function g(o) {\n  const u = o.p;\n  o.p.x = 0;\n  u.x = 1;\n  return sink(o.p.x);\n}",
        rows: &[("o.p.x@3->o.p.x@5", EXACT, EXACT)] },
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
    Case { name: "pattern_alias_field", lang: JS, source: "function g(o) {\n  const { k: a } = o;\n  o.k.x = 0;\n  a.x = 1;\n  return sink(o.k.x);\n}",
        rows: &[("o.k.x@3->o.k.x@5", ALIAS, EXACT), ("o.k.x@4->o.k.x@5", ALIAS, ALIAS), ("o.k@2->o.k@3", ALIAS, ALIAS), ("o.k@2->o.k@5", ALIAS, ALIAS)] },
    Case { name: "pattern_alias_field_incr", lang: JS, source: "function g(o) {\n  const { k: a } = o;\n  o.k.x = 0;\n  a.x++;\n  return sink(o.k.x);\n}",
        rows: &[("o.k.x@3->o.k.x@5", ALIAS, EXACT), ("o.k@2->o.k@3", ALIAS, ALIAS), ("o.k@2->o.k@5", ALIAS, ALIAS)] },
    Case { name: "chain_alias_incr", lang: JS, source: "function g(o) {\n  const u = o;\n  const w = u;\n  o.x = 0;\n  w.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@4->o.x@6", ALIAS, EXACT)] },
    Case { name: "param_alias_incr", lang: JS, source: "function f(o, u = o) {\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@4", ALIAS, EXACT)] },
    Case { name: "param_alias_for_of", lang: JS, source: "function f(o, xs, u = o) {\n  o.x = 0;\n  for (u.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@6", ALIAS, EXACT)] },
    Case { name: "one_node_multiline", lang: JS, source: "function g(o, u = o) {\n  return [\n    (o.x = 0),\n    (u.x = 1),\n    sink(o.x),\n  ];\n}",
        rows: &[("o.x@3->o.x@5", CFG, CFG), ("o.x@4->o.x@5", CFG, CFG)] },
    Case { name: "one_node_multiline_incr", lang: JS, source: "function g(o, u = o) {\n  return [\n    (o.x = 0),\n    u.x++,\n    sink(o.x),\n  ];\n}",
        rows: &[("o.x@3->o.x@5", CFG, CFG)] },
    Case { name: "ts_incr", lang: TS, source: "function g(o: any) {\n  const u: any = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "tsx_for_of", lang: TSX, source: "function g(o: any, xs: any[]) {\n  const u: any = o;\n  o.x = 0;\n  for (u.x of xs) {\n    log();\n  }\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@7", ALIAS, EXACT)] },
    Case { name: "anon_incr", lang: JS, source: "g(function (o) {\n  const u = o;\n  o.x = 0;\n  u.x++;\n  return sink(o.x);\n});",
        rows: &[("o.x@3->o.x@5", ALIAS, EXACT)] },
    Case { name: "ctl_alias_to_field_not_alias", lang: JS, source: "function g(o) {\n  const u = o.p;\n  o.p.x = 0;\n  u.x++;\n  return sink(o.p.x);\n}",
        rows: &[("o.p.x@3->o.p.x@5", EXACT, EXACT)] },
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
    Case { name: "ctl_alias_reassigned", lang: JS, source: "function g(o) {\n  let u = o;\n  o.x = 0;\n  u = {};\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_other_alias", lang: JS, source: "function g(o, p) {\n  const u = p;\n  o.x = 0;\n  u.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@5", EXACT, EXACT)] },
    Case { name: "ctl_no_alias", lang: JS, source: "function g(o) {\n  o.x = 0;\n  o.x = 1;\n  return sink(o.x);\n}",
        rows: &[("o.x@2->o.x@3", EXACT, EXACT), ("o.x@2->o.x@4", KILLED, KILLED), ("o.x@3->o.x@4", EXACT, EXACT)] },
    Case { name: "ctl_alias_write_before_direct", lang: JS, source: "function g(o) {\n  const u = o;\n  u.x = 1;\n  o.x = 0;\n  return sink(o.x);\n}",
        rows: &[("o.x@3->o.x@4", ALIAS, ALIAS), ("o.x@3->o.x@5", ALIAS, ALIAS), ("o.x@4->o.x@5", EXACT, EXACT)] },
];

#[test]
fn a_write_through_an_alias_may_kills_an_earlier_direct_def() {
    let mut failures = Vec::new();
    for c in CASES {
        for (edge, want, _base) in c.rows {
            let got = labels_of(c.lang, c.source, edge);
            if got.is_empty() || got.iter().any(|g| g != want) {
                failures.push(format!("{} {edge}: want {want}, got {got:?}", c.name));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The rows the rule moves are the issue's forms, in both named and anonymous owners, three dialects.
#[test]
fn the_rule_moves_the_issue_forms_and_nothing_else_listed() {
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
        "param_alias",
        "ts",
        "tsx",
        "anon_incr",
        "chain_alias",
        "delete_through_alias",
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
    ] {
        assert!(
            !moved.contains(&control),
            "{control} must not move: {moved:?}"
        );
    }
}

/// Rows whose write has no alias twin (the census kill-only source). The solver tests below feed the trace the
/// defs `recorded` reads back from edge sources, which never carry `alias_derived`, so twin rows cannot be traced
/// here; the corpus measurement (`kill:alias` lines against base labels) covers them.
const CENSUS_ROWS: &[&str] = &[
    "incr",
    "for_of_head",
    "for_in_head",
    "array_target",
    "array_target_nested",
    "delete_through_alias",
    "incr_prefix",
    "pattern_alias_field_incr",
    "chain_alias_incr",
    "param_alias_incr",
    "param_alias_for_of",
    "ts_incr",
    "tsx_for_of",
    "anon_incr",
];

/// The trace names `kill:alias` on exactly the edges the rule moved (the control pass has no may-kill), and on no
/// edge of a row that does not move.
#[test]
fn the_trace_names_the_moved_census_edges_kill_alias() {
    let mut failures = Vec::new();
    for c in CASES {
        let lines = trace_lines(c.lang, c.source);
        let named = |edge: &str| {
            let (from, to) = edge.split_once("->").unwrap();
            let (from, to) = (format!("{from}:"), format!("{to}:"));
            lines.iter().any(|l| {
                let f: Vec<&str> = l.split('\t').collect();
                f.len() == 6
                    && f[1] == super::alias_kill::KILL_ALIAS
                    && f[4].starts_with(&from)
                    && f[5].starts_with(&to)
            })
        };
        let census = CENSUS_ROWS.contains(&c.name);
        for (edge, now, base) in c.rows {
            let moved = now != base;
            // A twin row may or may not be traced here (the helper's defs lack the raw write); it must
            // never be traced when it does not move.
            if (census || !moved) && named(edge) != (moved && census) {
                failures.push(format!(
                    "{} {edge}: moved={moved} named={}",
                    c.name,
                    named(edge)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
