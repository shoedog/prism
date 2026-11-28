//! Refusal-branch checks for the S2b literal-props pass (lane 3, codecov patch). Each refusal is
//! paired with a control that differs only in the guarded fact and flows.

use super::object_literal_arg_tests::{build, parse_files};
use super::CodePropertyGraph;
use crate::call_graph::{CallSite, FunctionId};
use crate::languages::Language;
use std::collections::BTreeMap;

fn counters(graph: &CodePropertyGraph) -> BTreeMap<String, usize> {
    graph.dfg_label_stats.dfg_props.clone()
}

fn count(graph: &CodePropertyGraph, key: &str) -> usize {
    counters(graph).get(key).copied().unwrap_or(0)
}

fn js(source: &str) -> CodePropertyGraph {
    build(Language::JavaScript, &[("m", source)], "js")
}

const D: &str = "function D({a}) {\n  return use(a);\n}\n";

/// `App(x)` passes the literal `argument` to `D`; the control passes `{a: x}`.
fn app(call: &str) -> String {
    format!("{D}function App(x) {{\n  {call}\n}}\n")
}

#[test]
fn literal_leaf_member_chains_need_a_name_root() {
    let control = js(&app("D({a: x.b});"));
    assert!(count(&control, "edges_to_def") > 0);
    assert_eq!(count(&control, "refused_value_not_data"), 0);
    for leaf in ["f(x).b", "(x ? y : z).b", "x[0].b.c", "'s'.length"] {
        let refused = js(&app(&format!("D({{a: {leaf}}});")));
        assert_eq!(count(&refused, "edges_to_def"), 0, "{leaf}");
        assert_eq!(count(&refused, "refused_value_not_data"), 1, "{leaf}");
    }
}

#[test]
fn literal_sites_are_calls_only() {
    let construct = "class K {\n  constructor({a}) {\n    use(a);\n  }\n}\nfunction App(x) {\n  return new K({a: x});\n}\n";
    let refused = js(construct);
    assert_eq!(count(&refused, "sites_literal"), 0);
    assert_eq!(count(&refused, "edges_to_def"), 0);
    let control = js(&app("return D({a: x});"));
    assert_eq!(count(&control, "sites_literal"), 1);
    assert_eq!(count(&control, "edges_to_def"), 1);
}

#[test]
fn an_errorful_callee_pattern_body_is_refused_by_identity() {
    let control = js(&app("D({a: x});"));
    assert_eq!(count(&control, "edges_to_def"), 1);
    // The same callee with a syntax error in its body: its callable is not one clean node.
    let source = "function D({a}) {\n  return use(a) +;\n}\nfunction App(x) {\n  D({a: x});\n}\n";
    let mut files = parse_files(Language::JavaScript, &[("m", &app("D({a: x});"))], "js");
    files.insert(
        "m.js".to_owned(),
        crate::ast::ParsedFile::parse("m.js", source, Language::JavaScript).unwrap(),
    );
    assert!(files["m.js"].tree.root_node().has_error());
    let refused = CodePropertyGraph::build(&files);
    assert_eq!(count(&refused, "edges_to_def"), 0);
    assert_eq!(count(&refused, "refused_target_identity"), 1);
}

#[test]
fn an_escaped_identifier_anywhere_in_the_file_makes_the_literal_site_not_exact() {
    // This is why `only_member_writes_on_line`'s escaped-identifier exit cannot be reached: the
    // site is refused before any read of the callee is classified.
    let source = |extra: &str| {
        format!(
            "function f(o) {{\n  sink(o);\n  o.q = b;\n  sink(o.a);\n}}\nfunction h(x) {{\n  f({{ a: x }});\n}}\n{extra}"
        )
    };
    let control = js(&source(""));
    assert_eq!(count(&control, "refused_not_exact_singleton"), 0);
    assert!(count(&control, "edges_to_use") > 0);
    let escaped = js(&source("function z() { return \\u0062; }\n"));
    assert_eq!(count(&escaped, "refused_not_exact_singleton"), 1);
    assert_eq!(count(&escaped, "edges_to_use"), 0);
}

fn site<'a>(graph: &'a CodePropertyGraph, member: &str) -> &'a CallSite {
    graph
        .call_graph
        .calls
        .iter()
        .flat_map(|(_, sites)| sites)
        .find(|site| site.callee_name == member)
        .expect("recorded site")
}

#[test]
fn props_export_match_is_judged_against_the_target_file_and_module() {
    let files = [
        (
            "m",
            "import * as api from \"./ctx\";\nfunction A(x) {\n  api.send({a: x});\n}\n",
        ),
        ("ctx", "export function send({a}) {\n  return use(a);\n}\n"),
        // A twin of `ctx` that also claims exactly one `send` value export.
        ("twin", "export function send({a}) {\n  return use(a);\n}\n"),
    ];
    let parsed = parse_files(Language::JavaScript, &files, "js");
    let graph = CodePropertyGraph::build(&parsed);
    let site = site(&graph, "send");
    let target = FunctionId {
        file: "ctx.js".into(),
        name: "send".into(),
        start_line: 1,
        end_line: 3,
    };
    let matches = |file: &str, target: &FunctionId| {
        graph
            .call_graph
            .js_ts_props_export_matches(site, target, &parsed[file])
    };
    assert!(matches("ctx.js", &target));
    // The pattern callee's parsed file must be the target's own, even when another file would
    // satisfy the export count.
    assert!(!matches("twin.js", &target));
    assert!(!matches("m.js", &target));
    // A namespace whose module is not in the repository proves no export.
    let lost = parse_files(
        Language::JavaScript,
        &[
            (
                "m",
                "import * as api from \"./nowhere\";\nfunction A(x) {\n  api.send({a: x});\n}\n",
            ),
            files[1],
        ],
        "js",
    );
    let lost_graph = CodePropertyGraph::build(&lost);
    assert!(!lost_graph.call_graph.js_ts_props_export_matches(
        self::site(&lost_graph, "send"),
        &target,
        &lost["ctx.js"]
    ));
}

#[test]
fn alias_of_a_namespace_member_needs_the_spelled_value_export() {
    let ctx = "export function other({a}) {\n  return use(a);\n}\n";
    let caller = |alias: &str| {
        format!("import * as api from \"./ctx\";\nconst send = {alias};\nfunction A(x) {{\n  send({{a: x}});\n}}\n")
    };
    let run = |alias: &str, ctx: &str| {
        build(
            Language::JavaScript,
            &[("m", &caller(alias)), ("ctx", ctx)],
            "js",
        )
    };
    // Control: the alias names the member whose value export is the selected callee.
    let control = run("api.other", ctx);
    assert_eq!(count(&control, "edges_to_def"), 1);
    // A member of the namespace that no value export carries proves nothing.
    let missing = run("api.absent", ctx);
    assert_eq!(count(&missing, "edges_to_def"), 0);
    // A wrapped export is not a literal route.
    let wrapped = run(
        "api.other",
        "import {memo} from 'react';\nexport const other = memo(({a}) => a);\n",
    );
    assert_eq!(count(&wrapped, "edges_to_def"), 0);
}
