//! #660 S1d-2: the one-node `first_hazard` guard is gone. Each of its six hazards (a)-(f) must be
//! handled in a one-node owner by a named mechanism, so an owner it refused is now either refused by
//! that mechanism (same label as before, new reason) or, where the hazard is harmless, `exact`.
//! Every row asserts the label AND the mechanism (trace lines plus S1a capture lines), so a row
//! that is right for the wrong reason fails. `guard-deleted:<hazard>` names the rows whose label
//! left `NameOnly(CfgIncomplete)` only because the guard no longer refuses the owner.
use super::super::capture::{capture_facts_with, is_capture_to};
use super::super::census_defs_tests::{availability, parse, recorded, trace_lines};
use crate::cpg::census_label_tests::labels_of;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::test_support::label_snapshot;
use crate::data_flow::FlowEdge;
use crate::js_write_census::matrix_tests::{fixtures, lang_of};
use crate::languages::Language;
use std::collections::BTreeMap;

const EXACT: &str = "exact";
const CFG: &str = "nameonly/cfg_incomplete";
const KILLED: &str = "nameonly/killed";
const SAME: &str = "nameonly/sameline";
const JS: Language = Language::JavaScript;
const TS: Language = Language::TypeScript;
const TSX: Language = Language::Tsx;

/// Mechanism names the trace gives the tested edge (`path@line->path@line`): census lines (`traced`)
/// and S1a capture lines, only those whose endpoints are that edge.
fn mechanisms(language: Language, src: &str, edge: &str) -> Vec<String> {
    let (from, to) = edge.split_once("->").unwrap();
    let (from, to) = (format!("{from}:"), format!("{to}:"));
    let on_edge = |l: &str| {
        let f: Vec<&str> = l.split('\t').collect();
        f.len() == 6 && f[0] == "rd-census" && f[4].starts_with(&from) && f[5].starts_with(&to)
    };
    let mut lines = trace_lines(language, src);
    let file = parse(language, src);
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(file.path.clone(), file.clone())]));
    let mut out = Vec::new();
    for owner in file.all_functions() {
        let facts = capture_facts_with(&file, owner, true);
        for (from, to) in label_snapshot(&cpg.dfg).into_keys() {
            is_capture_to(&FlowEdge { from, to }, &facts, &mut out);
        }
    }
    lines.extend(String::from_utf8(out).unwrap().lines().map(String::from));
    let mut all: Vec<String> = lines
        .iter()
        .filter(|l| on_edge(l))
        .map(|l| l.split('\t').nth(1).unwrap().to_string())
        .collect();
    all.sort();
    all.dedup();
    all
}

struct Case {
    name: &'static str,
    lang: Language,
    edge: &'static str,
    source: &'static str,
    label: &'static str,
    /// Exactly these mechanisms are named (empty: only main's own rules decide the label).
    mechs: &'static [&'static str],
}

const ONE: &str = "owner-refuse:one-node";

/// One-node shapes per hazard. `edge` is the formal's def line to its use line.
#[rustfmt::skip]
const CASES: &[Case] = &[
    // (a) update: a formal updated in the node adds a kill-only def of the same base, so the belt
    // refuses the owner by label; an update of another name harms nothing.
    Case { name: "a_update", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a) {\n  return (a++, sink(a));\n}" },
    Case { name: "a_prefix", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a) {\n  return (--a, sink(a));\n}" },
    Case { name: "a_read_first", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a) {\n  return (sink(a), a++);\n}" },
    Case { name: "a_expression_arrow", lang: JS, edge: "x@1->x@2", label: CFG, mechs: &[ONE],
        source: "const f = (x) =>\n  x++;" },
    Case { name: "a_one_line_block", lang: JS, edge: "a@1->a@1", label: CFG, mechs: &[ONE],
        source: "function f(a) { a++; return sink(a); }" },
    Case { name: "a_method", lang: JS, edge: "a@2->a@2", label: CFG, mechs: &[ONE],
        source: "const o = {\n  m(a) { return (a++, sink(a)); }\n};" },
    Case { name: "a_other_name_is_harmless", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &["guard-deleted:a"],
        source: "function f(a) {\n  return (n++, sink(a));\n}" },
    Case { name: "a_nested_callable_local_is_harmless", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &["guard-deleted:a"],
        source: "function f(a) {\n  return (g(() => { let i = 0; i++; }), sink(a));\n}" },
    Case { name: "a_ts", lang: TS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a: number) {\n  return (a++, sink(a));\n}" },
    Case { name: "a_tsx_arrow", lang: TSX, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "const f = (a: number) =>\n  (a++, sink(a));" },
    // (b) a default inside an assignment pattern: the same belt for the formal's own name.
    Case { name: "b_array_pattern", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a) {\n  return ([a = 1] = src(), sink(a));\n}" },
    Case { name: "b_object_pattern", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a) {\n  return ({ a = 1 } = src(), sink(a));\n}" },
    Case { name: "b_nested_pattern", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[ONE],
        source: "function f(a) {\n  return ({ x: [a = 1] } = src(), sink(a));\n}" },
    Case { name: "b_other_name_is_harmless", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &["guard-deleted:b"],
        source: "function f(a) {\n  return ([n = 1] = src(), sink(a));\n}" },
    // (c) a hoisted declaration replaces the formal at entry: the formal's edge is killed.
    Case { name: "c_hoisted", lang: JS, edge: "a@1->a@2", label: KILLED, mechs: &["dead-formal", "guard-deleted:c"],
        source: "function f(a) {\n  return sink(a); function a() {}\n}" },
    Case { name: "c_hoisted_generator", lang: JS, edge: "a@1->a@2", label: KILLED, mechs: &["dead-formal", "guard-deleted:c"],
        source: "function f(a) {\n  return sink(a); function* a() {}\n}" },
    Case { name: "c_one_line", lang: JS, edge: "a@1->a@1", label: KILLED, mechs: &["dead-formal", "guard-deleted:c"],
        source: "function f(a) { return sink(a); function a() {} }" },
    Case { name: "c_other_name_is_harmless", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &[],
        source: "function f(a) {\n  return sink(a); function b() {}\n}" },
    Case { name: "c_nested_declaration_is_harmless", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[],
        source: "function f(a) {\n  return sink(a, function () { function a() {} });\n}" },
    // (d) `arguments`: sloppy mapped arguments alias the formals; strict, non-simple and nested-function
    // `arguments` are not the owner's mapping.
    Case { name: "d_sloppy_write", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["arguments"],
        source: "function f(a) {\n  return (arguments[0] = src(), sink(a));\n}" },
    Case { name: "d_sloppy_read", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["arguments"],
        source: "function f(a) {\n  return (use(arguments.length), sink(a));\n}" },
    Case { name: "d_sloppy_second_formal", lang: JS, edge: "b@1->b@1", label: CFG, mechs: &["arguments"],
        source: "function f(a, b) { arguments[0] = src(); return sink(b); }" },
    Case { name: "d_sloppy_alias", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["arguments"],
        source: "function f(a) {\n  var args = arguments; return (args[0] = src(), sink(a));\n}" },
    Case { name: "d_sloppy_in_nested_arrow", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["arguments"],
        source: "function f(a) {\n  return (g(() => { arguments[0] = src(); }), sink(a));\n}" },
    Case { name: "d_sloppy_method", lang: JS, edge: "a@2->a@2", label: CFG, mechs: &["arguments"],
        source: "const o = {\n  m(a) { return (arguments[0] = 1, sink(a)); }\n};" },
    Case { name: "d_strict_is_unmapped", lang: JS, edge: "a@2->a@3", label: EXACT, mechs: &["guard-deleted:d"],
        source: "'use strict';\nfunction f(a) {\n  return (arguments[0] = src(), sink(a));\n}" },
    Case { name: "d_strict_ts", lang: TS, edge: "a@2->a@3", label: EXACT, mechs: &["guard-deleted:d"],
        source: "'use strict';\nfunction f(a: number) {\n  return (arguments[0] = src(), sink(a));\n}" },
    Case { name: "d_non_simple_parameters_are_unmapped", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &["guard-deleted:d"],
        source: "function f(a, b = 1) {\n  return (arguments[0] = src(), sink(a));\n}" },
    Case { name: "d_arrow_owner_has_no_arguments", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &["guard-deleted:d"],
        source: "const f = (a) =>\n  (use(arguments), sink(a));" },
    Case { name: "d_nested_function_has_its_own_arguments", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &[],
        source: "function f(a) {\n  return (g(function () { arguments[0] = src(); }), sink(a));\n}" },
    // (e) direct eval: owner-wide refusal, nested callables and defaults included.
    Case { name: "e_direct", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["eval"],
        source: "function f(a) {\n  return (eval(\"a = 1\"), sink(a));\n}" },
    Case { name: "e_parenthesized", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["eval"],
        source: "function f(a) {\n  return ((eval)(\"a = 1\"), sink(a));\n}" },
    Case { name: "e_in_nested_callable", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["eval"],
        source: "function f(a) {\n  return (g(() => eval(\"a = 1\")), sink(a));\n}" },
    Case { name: "e_in_own_default", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["eval"],
        source: "function f(a, b = eval(\"a = 1\")) {\n  return sink(a);\n}" },
    Case { name: "e_ts_assertion", lang: TS, edge: "a@1->a@2", label: CFG, mechs: &["eval"],
        source: "function f(a: number) {\n  return ((eval as any)(\"a = 1\"), sink(a));\n}" },
    Case { name: "e_indirect_is_global", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &[],
        source: "function f(a) {\n  return ((0, eval)(\"a = 1\"), sink(a));\n}" },
    Case { name: "e_member_callee", lang: JS, edge: "a@1->a@2", label: EXACT, mechs: &[],
        source: "function f(a) {\n  return (o.eval(\"a = 1\"), sink(a));\n}" },
    // (f) a nested callable's parameter default: a deferred read (S1a) or write (D2-c).
    Case { name: "f_default_read", lang: JS, edge: "y@1->y@2", label: CFG, mechs: &["capture-param"],
        source: "function h(y) {\n  return g(function (z = y) { return z; });\n}" },
    Case { name: "f_arrow_default_read", lang: JS, edge: "y@1->y@2", label: CFG, mechs: &["capture-param"],
        source: "function h(y) {\n  return g((z = y) => z);\n}" },
    Case { name: "f_default_read_ts", lang: TS, edge: "y@1->y@2", label: CFG, mechs: &["capture-param"],
        source: "function h(y: number) {\n  return g(function (z = y) { return z; });\n}" },
    Case { name: "f_default_read_tsx", lang: TSX, edge: "y@1->y@2", label: CFG, mechs: &["capture-param"],
        source: "function h(y: number) {\n  return g(function (z = y) { return z; });\n}" },
    Case { name: "f_default_write", lang: JS, edge: "a@1->a@1", label: CFG, mechs: &["d2c"],
        source: "function f(a) { g((z = (a = 2)) => z); return sink(a); }" },
    Case { name: "f_body_write", lang: JS, edge: "a@1->a@1", label: CFG, mechs: &[],
        source: "function f(a) { g(() => { a = 2; }); return sink(a); }" },
    Case { name: "f_unrelated_default_is_harmless", lang: JS, edge: "y@1->y@2", label: EXACT, mechs: &["guard-deleted:f"],
        source: "function h(y) {\n  return (g((z = 1) => z), sink(y));\n}" },
    // Own parameter defaults (the guard walked them too).
    Case { name: "own_default_update_kills_the_formal", lang: JS, edge: "a@1->a@2", label: KILLED, mechs: &["dead-formal", "guard-deleted:a"],
        source: "function f(a, b = a++) {\n  return sink(a);\n}" },
    Case { name: "own_default_assignment_is_dead_formal", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["dead-formal"],
        source: "function f(a, b = (a = 2)) {\n  return sink(a);\n}" },
    Case { name: "own_default_pattern_kills_the_formal", lang: JS, edge: "a@1->a@2", label: KILLED, mechs: &["dead-formal", "guard-deleted:b"],
        source: "function f(a, b = ([a = 1] = x)) {\n  return sink(a);\n}" },
    Case { name: "own_default_closure_write", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["d2c"],
        source: "function f(a, b = () => { a = 1; }) {\n  return sink(a);\n}" },
    // #761: `b = a` is the alias `const b = a`; its twin def of `a` shares the one node (a name defined twice).
    Case { name: "own_default_alias_one_node", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[],
        source: "function f(a, b = a) {\n  return sink(a, b);\n}" },
    // Other refusal classes (S1d-1) reached for the first time by a one-node owner.
    Case { name: "with_body_use", lang: JS, edge: "a@1->a@1", label: CFG, mechs: &["with"],
        source: "function f(a, o) { with (o) { sink(a); } }" },
    Case { name: "with_multi_line_use", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["with"],
        source: "function f(a, o) {\n  with (o) { sink(a); }\n}" },
    Case { name: "enclosing_arguments_reaches_an_arrow_owner", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[],
        source: "function f(a) {\n  return () => (arguments[0] = 2, sink(a));\n}" },
    Case { name: "eager_class_update", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &["capture-field"],
        source: "function f(a) {\n  return (class { x = a++; }, sink(a));\n}" },
    Case { name: "static_block_site_is_unmapped", lang: JS, edge: "a@1->a@3", label: CFG, mechs: &["site-unmapped"],
        source: "function f(a) {\n  class C { static { a++; } }\n  return sink(a);\n}" },
    Case { name: "two_recorded_defs_keep_main_belt", lang: JS, edge: "a@1->a@2", label: CFG, mechs: &[],
        source: "function f(a) {\n  a = 1; a = 2; return sink(a);\n}" },
    Case { name: "for_head_one_line_is_d3_loop", lang: JS, edge: "a@1->a@2", label: SAME, mechs: &["d3-loop"],
        // (#693 slice 2: `debugger;` last keeps the loop unrewired, else the plan refuses the one-line tree owner-wide.)
        source: "function f(a, xs) {\n  for (a of xs) { sink(a); debugger; }\n}" },
    // SOL-757-01: a write in the owner's own parameter list (defaults, patterns) that the producer does not
    // record refuses the field projections it affects (`own-params`); a member write does not rebind the
    // bare formal. Each FIX row has a multi-node twin (main has the same gap there).
    Case { name: "own_member_pattern_default", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &["own-params"],
        source: "function f(o, unused = ([o.x = 1] = [])) {\n  return sink(o.x);\n}" },
    Case { name: "own_member_pattern_default_multi", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function f(o, unused = ([o.x = 1] = [])) {\n  log();\n  return sink(o.x);\n}" },
    Case { name: "own_member_update", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &["own-params"],
        source: "function h(o, u = o.x++) {\n  return sink(o.x);\n}" },
    Case { name: "own_member_update_multi", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function h(o, u = o.x++) {\n  log();\n  return sink(o.x);\n}" },
    // Recorded by the producer (shares the formal's base): the one-node belt refuses it, not `own-params`.
    Case { name: "own_member_compound", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &[],
        source: "function h(o, u = (o.x += 1)) {\n  return sink(o.x);\n}" },
    Case { name: "own_member_compound_multi", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function h(o, u = (o.x += 1)) {\n  log();\n  return sink(o.x);\n}" },
    Case { name: "own_member_delete", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &["own-params"],
        source: "function h(o, u = delete o.x) {\n  return sink(o.x);\n}" },
    Case { name: "own_member_delete_multi", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function h(o, u = delete o.x) {\n  log();\n  return sink(o.x);\n}" },
    Case { name: "own_computed_member", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &["own-params"],
        source: "function h(o, k, u = (o[k] = 1)) {\n  return sink(o.x);\n}" },
    Case { name: "own_computed_member_multi", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function h(o, k, u = (o[k] = 1)) {\n  log();\n  return sink(o.x);\n}" },
    // Recorded by the producer (shares the formal's base): the one-node belt refuses it, not `own-params`.
    Case { name: "own_object_pattern_target", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &[],
        source: "function h(o, u = ({ x: o.x } = src())) {\n  return sink(o.x);\n}" },
    Case { name: "own_object_pattern_target_multi", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function h(o, u = ({ x: o.x } = src())) {\n  log();\n  return sink(o.x);\n}" },
    Case { name: "own_member_pattern_default_ts", lang: TS, edge: "o@1->o.x@2", label: CFG, mechs: &["own-params"],
        source: "function f(o: any, unused = ([o.x = 1] = [])) {\n  return sink(o.x);\n}" },
    Case { name: "own_member_update_tsx", lang: TSX, edge: "o@1->o.x@2", label: CFG, mechs: &["own-params"],
        source: "function h(o: any, u = o.x++) {\n  return sink(o.x);\n}" },
    Case { name: "own_member_write_other_field_control", lang: JS, edge: "o@1->o.y@2", label: EXACT, mechs: &["guard-deleted:a"],
        source: "function h(o, u = o.x++) {\n  return sink(o.y);\n}" },
    Case { name: "own_member_write_other_field_control_multi", lang: JS, edge: "o@1->o.y@3", label: EXACT, mechs: &[],
        source: "function h(o, u = o.x++) {\n  log();\n  return sink(o.y);\n}" },
    Case { name: "own_member_write_bare_formal_control", lang: JS, edge: "o@1->o@2", label: EXACT, mechs: &["guard-deleted:a"],
        source: "function h(o, u = o.x++) {\n  return sink(o);\n}" },
    Case { name: "own_member_write_bare_formal_control_multi", lang: JS, edge: "o@1->o@3", label: EXACT, mechs: &[],
        source: "function h(o, u = o.x++) {\n  log();\n  return sink(o);\n}" },
    // A recorded write: the formal `o` and the recorded `o.x` share the base, so the one-node belt (main's,
    // on recorded bases) leaves the owner unavailable; multi-node, the recorded def kills the projection.
    Case { name: "own_recorded_member_write", lang: JS, edge: "o@1->o.x@2", label: CFG, mechs: &[],
        source: "function h(o, u = (o.x = 1)) {\n  return sink(o.x);\n}" },
    // SOL-757-01 residual (round 2): the region rule. A def in the own parameter list is refused whatever its
    // byte order (the RHS def `o.x = 99` runs before the pattern default `[o.x = 1]` that textually precedes it).
    Case { name: "own_rhs_def_before_pattern_default", lang: JS, edge: "o.x@2->o.x@3", label: CFG, mechs: &["own-params"],
        source: "const o = { x: 0 };\nfunction f(unused = ([o.x = 1] = [(o.x = 99, void 0)])) {\n  return sink(o.x);\n}" },
    Case { name: "own_rhs_def_before_pattern_default_multi", lang: JS, edge: "o.x@2->o.x@4", label: CFG, mechs: &["own-params"],
        source: "const o = { x: 0 };\nfunction f(unused = ([o.x = 1] = [(o.x = 99, void 0)])) {\n  log();\n  return sink(o.x);\n}" },
    // Accepted cost: a list def that runs after the write is refused too (here the formal `o` is bound after
    // the default that writes `o.x`; the region rule does not order the list).
    Case { name: "own_later_list_def_is_over_refused", lang: JS, edge: "o@1->o.x@3", label: CFG, mechs: &["own-params"],
        source: "function f(u = ([o.x = 1] = []), o) {\n  log();\n  return sink(o.x);\n}" },
    // A body def runs after every list write: not refused.
    Case { name: "own_body_def_after_list_write_stays_exact", lang: JS, edge: "o.x@2->o.x@3", label: EXACT, mechs: &[],
        source: "function f(o, u = o.x++) {\n  o.x = 2;\n  return sink(o.x);\n}" },
    // #761: a parameter-list alias `u = o` is an alias like the body's `const u = o`: the formal is mixed (no
    // projection edge `o@1->o.x`), and the alias twin def of `o` on the signature line refuses the edge.
    Case { name: "own_alias_write_is_761_fixed", lang: JS, edge: "o@1->o@2", label: CFG, mechs: &["guard-deleted:a"],
        source: "function f(o, u = o, v = u.x++) {\n  return sink(o.x);\n}" },
    Case { name: "own_alias_write_is_761_fixed_multi", lang: JS, edge: "o@1->o@3", label: SAME, mechs: &[],
        source: "function f(o, u = o, v = u.x++) {\n  log();\n  return sink(o.x);\n}" },
];

#[test]
fn one_node_rows_have_the_label_and_the_mechanism() {
    let mut failures = Vec::new();
    for c in CASES {
        let got = labels_of(c.lang, c.source, c.edge);
        let mut mechs = mechanisms(c.lang, c.source, c.edge);
        // The sameline refinement and nothing else is traced for the control `for` row.
        mechs.retain(|m| c.name == "for_head_one_line_is_d3_loop" || !m.starts_with("d3-"));
        let mut want: Vec<&str> = c.mechs.to_vec();
        want.sort();
        if got.is_empty() || got.iter().any(|g| g != c.label) || mechs != want {
            failures.push(format!(
                "{} {}: want {} {want:?}, got {got:?} {mechs:?}",
                c.name, c.edge, c.label
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Matrix rows named for S1d-2 (`S1d (guard deleted)`, `S1a + S1d`, and the one-node rows of W1 and D3):
/// the committed `expected` column is the label; the mechanism is pinned beside it.
const MATRIX_MECHS: &[(&str, &[&str])] = &[
    ("s01_update", &[ONE]),
    ("s02_arr_default", &[ONE]),
    ("s03_obj_default", &[ONE]),
    ("s04_hoisted", &["dead-formal", "guard-deleted:c"]),
    ("s05_arguments_sloppy", &["arguments"]),
    ("s06_arguments_strict", &["guard-deleted:d"]),
    ("s07_eval_paren", &["eval"]),
    (
        "s09_own_default_update",
        &["dead-formal", "guard-deleted:a"],
    ),
    ("s10_own_default_eval", &["eval"]),
    ("s11_nested_default_read", &["capture-param"]),
    ("s12_arrow_expr_update", &[ONE]),
    ("s13_unrelated_update_control", &["guard-deleted:a"]),
    ("s14_other_name_fn_control", &[]),
    ("w03_escaped_update_one_node", &["owner-refuse:escape"]),
    ("w04_escaped_braced_one_node", &["owner-refuse:escape"]),
    ("d06_read_before_write_one_node", &[ONE]),
    ("d07_x_eq_x_plus_one_node", &[]),
];

#[test]
fn matrix_rows_have_the_expected_label_and_mechanism() {
    let text = include_str!("../../../tests/fixtures/js_write_census/variant-matrix.tsv");
    let mut failures = Vec::new();
    for (fixture, mechs) in MATRIX_MECHS {
        let row = text
            .lines()
            .map(|l| l.split('\t').collect::<Vec<_>>())
            .find(|c| c[0] == *fixture)
            .unwrap_or_else(|| panic!("no matrix row {fixture}"));
        let source = row[7].replace("\\n", "\n");
        let got = labels_of(lang_of(fixture), &source, row[1]);
        let mut found = mechanisms(lang_of(fixture), &source, row[1]);
        found.retain(|m| !m.starts_with("d3-"));
        let mut want: Vec<&str> = mechs.to_vec();
        want.sort();
        if got.is_empty() || got.iter().any(|g| g != row[4]) || found != want {
            failures.push(format!(
                "{fixture} {}: want {} {want:?}, got {got:?} {found:?}",
                row[1], row[4]
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Edges never move: every row's edge exists (the guard only ever changed a label).
#[test]
fn rows_keep_their_edges() {
    for c in CASES {
        assert!(
            !labels_of(c.lang, c.source, c.edge).is_empty(),
            "{}: edge disappeared",
            c.name
        );
    }
}

/// The solver and the cap preflight call the one admission function, so they agree on every
/// one-node row, and no hazard row is refused by availability any more except by the belt.
#[test]
fn preflight_and_solver_agree_on_every_one_node_row() {
    let mut belt = 0;
    let mut available = 0;
    for c in CASES {
        let parsed = parse(c.lang, c.source);
        for owner in parsed.all_functions() {
            let (defs, _) = recorded(&parsed, &owner);
            let (pre, solver) = availability(&parsed, &owner, &defs);
            assert_eq!(pre, solver, "{}", c.name);
            if solver.is_some() {
                belt += 1;
            } else {
                available += 1;
            }
        }
    }
    assert!(available > 40 && belt > 0, "{available} {belt}");
    for (fixture, _, src) in fixtures() {
        let parsed = parse(lang_of(&fixture), &src.replace("\\n", "\n"));
        for owner in parsed.all_functions() {
            let (defs, _) = recorded(&parsed, &owner);
            let (pre, solver) = availability(&parsed, &owner, &defs);
            assert_eq!(pre, solver, "{fixture}");
        }
    }
}

/// The trace control: `guard-deleted:<hazard>` appears only for an owner the old guard refused, once per
/// moved edge, plus one owner line per flipped owner (moved edges or not); silent for guard-free owners.
#[test]
fn guard_deleted_names_the_old_hazard_once_per_moved_edge_and_once_per_owner() {
    let lines = trace_lines(JS, "function f(a) {\n  return (n++, sink(a));\n}\n");
    let edges: Vec<_> = lines
        .iter()
        .filter(|l| l.starts_with("rd-census\t"))
        .collect();
    assert_eq!(edges.len(), 1, "{lines:?}");
    let f: Vec<&str> = edges[0].split('\t').collect();
    assert_eq!(&f[..4], ["rd-census", "guard-deleted:a", "a.js", "f"]);
    assert!(f[4].starts_with("a@1:"), "{lines:?}");
    assert!(f[5].starts_with("a@2:"), "{lines:?}");
    let owners: Vec<_> = lines
        .iter()
        .filter(|l| l.starts_with("rd-census-owner\t"))
        .collect();
    assert_eq!(owners, ["rd-census-owner\tguard-deleted:a\ta.js\tf\t1"]);
    // A flipped owner whose edges all keep `cfg_incomplete` (the belt refuses it by label): an owner line, no edge line.
    let belt = trace_lines(JS, "function f(a) {\n  return (a++, sink(a));\n}\n");
    assert!(
        belt.iter()
            .any(|l| l == "rd-census-owner\tguard-deleted:a\ta.js\tf\t1"),
        "{belt:?}"
    );
    assert!(
        !belt
            .iter()
            .any(|l| l.starts_with("rd-census\tguard-deleted")),
        "{belt:?}"
    );
    for clean in [
        "function f(a) {\n  return sink(a);\n}\n",
        "function f(a) {\n  log();\n  n++;\n  sink(a);\n}\n",
        "function f(a) {\n  a = 1; a = 2; return (n++, sink(a));\n}\n",
    ] {
        assert!(
            !trace_lines(JS, clean)
                .iter()
                .any(|l| l.contains("guard-deleted")),
            "{clean}"
        );
    }
    // Other languages are never traced.
    let py = trace_lines(Language::Python, "def f(a):\n    return sink(a)\n");
    assert!(py.iter().all(|l| !l.contains("guard-deleted")), "{py:?}");
}
