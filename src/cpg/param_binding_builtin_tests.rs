//! Slice D end-to-end endpoints, roles, and trace boundaries.

use super::object_literal_arg_tests::{build, LANGS};
use super::trace::{BoundaryEdge, BoundaryKind, Relation};
use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use petgraph::graph::NodeIndex;
use std::collections::BTreeMap;

const D: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::BuiltinCallback);
pub(in crate::cpg) const DIRECT: &str = "\
function visit(item) {
  use(item);
}
function main() {
  const items = [1];
  const direct = 2;
items.forEach(visit); visit(direct);
}
";

fn endpoint(
    cpg: &CodePropertyGraph,
    owner: &str,
    path: &str,
    span: (usize, usize),
    kind: VarAccess,
) -> NodeIndex {
    let nodes: Vec<_> = cpg
        .graph
        .node_indices()
        .filter(|&node| {
            matches!(cpg.node(node), CpgNode::Variable {
                file, function, path: actual, start_byte, end_byte, access, ..
            } if file == "m.js" && function == owner && actual.to_string() == path
                && (*start_byte, *end_byte) == span && *access == kind)
        })
        .collect();
    assert_eq!(nodes.len(), 1, "{owner}:{path} {span:?} {kind:?}");
    nodes[0]
}

pub(in crate::cpg) fn builtin_edges(cpg: &CodePropertyGraph) -> Vec<(NodeIndex, NodeIndex)> {
    cpg.graph
        .edge_indices()
        .filter(|&edge| cpg.graph[edge] == CpgEdge::DataFlow(D))
        .map(|edge| cpg.graph.edge_endpoints(edge).unwrap())
        .collect()
}

#[test]
fn named_callback_and_exact_sibling_have_distinct_endpoints() {
    for indent in ["", "  "] {
        let source = DIRECT.replace("\nitems.forEach", &format!("\n{indent}items.forEach"));
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let items_def = endpoint(&cpg, "main", "items", (64, 69), VarAccess::Def);
        let items = endpoint(&cpg, "main", "items", (97, 97), VarAccess::Use);
        let formal = endpoint(&cpg, "visit", "item", (15, 19), VarAccess::Def);
        let direct_def = endpoint(&cpg, "main", "direct", (85, 91), VarAccess::Def);
        let direct = endpoint(
            &cpg,
            "main",
            "direct",
            (125 + indent.len(), 131 + indent.len()),
            VarAccess::Use,
        );
        let read = endpoint(&cpg, "visit", "item", (29, 33), VarAccess::Use);
        let positive = indent.is_empty();
        assert_eq!(
            builtin_edges(&cpg),
            if positive {
                vec![(items, formal)]
            } else {
                vec![]
            }
        );
        assert_eq!(
            cpg.graph
                .edges_connecting(direct, formal)
                .filter(|edge| *edge.weight() == CpgEdge::DataFlow(FlowConfidence::Exact))
                .count(),
            1
        );
        let trace = cpg.taint_trace_nodes(&[items_def, direct_def], None);
        assert_eq!(
            trace.boundary.contains(&BoundaryEdge {
                root: items_def,
                from: items,
                to: formal,
                kind: BoundaryKind::CrossFunction,
            }),
            positive
        );
        assert!(trace.frontier_by_root[&items_def].contains(&items));
        assert!(!trace.frontier_by_root[&items_def].contains(&formal));
        assert!(!trace.frontier_by_root[&items_def].contains(&read));
        assert_eq!(
            trace.parents_by_root[&(direct_def, formal)],
            (direct, Relation::CallDescent)
        );
        assert!(trace.frontier_by_root[&direct_def].contains(&read));
        let review = cpg.dfg_forward_reachable(&cpg.to_var_location(items_def).unwrap());
        assert_eq!(
            review.contains(&cpg.to_var_location(read).unwrap()),
            positive
        );
        if !positive {
            let local_receivers: Vec<_> = cpg
                .graph
                .node_indices()
                .filter(|&node| {
                    matches!(cpg.node(node), CpgNode::Variable {
                    function, line: 7, path, access: VarAccess::Use, ..
                } if function == "main" && path.base == "items")
                })
                .collect();
            assert_eq!(local_receivers, [items]);
        }

        // Reuse identical endpoints to distinguish early label refusal from
        // site failure. Exact has already cached a successful site proof.
        let mut warm = BTreeMap::new();
        assert!(cpg.descend_target(direct, formal, Some(FlowConfidence::Exact), &mut warm));
        assert_eq!(warm.len(), 1);
        assert!(!cpg.descend_target(direct, formal, Some(D), &mut warm));
        let mut empty = BTreeMap::new();
        assert!(!cpg.descend_target(direct, formal, Some(D), &mut empty));
        assert!(empty.is_empty());
    }
}

#[test]
fn same_owner_callback_is_a_known_trace_limit() {
    let source =
        "function visit(item, items) {\nobserve(items);\nitems.map(visit);\nuse(item);\n}\n";
    let cpg = build(LANGS[0].0, &[("m", source)], "js");
    let start = source.find("items.map").unwrap();
    let items = endpoint(&cpg, "visit", "items", (start, start), VarAccess::Use);
    let formal = endpoint(&cpg, "visit", "item", (15, 19), VarAccess::Def);
    assert_eq!(builtin_edges(&cpg), [(items, formal)]);
    let root = endpoint(&cpg, "visit", "items", (21, 26), VarAccess::Def);
    let trace = cpg.taint_trace_nodes(&[root], None);
    assert!(!trace
        .boundary
        .iter()
        .any(|edge| edge.from == items && edge.to == formal));
    assert_eq!(
        trace.parents_by_root[&(root, formal)],
        (items, Relation::DataFlow)
    );
}

#[test]
fn runtime_slot_zero_is_independent_of_later_callable_values() {
    for (argument, expected) in [
        ("x => use(x)", true),
        ("...[], /* trivia */ x => use(x)", true),
        ("...[/* trivia */ x => use(x), 0]", true),
        ("...opaque, x => use(x)", false),
        ("...[, x => use(x)]", false),
        ("0, x => use(x)", false),
        ("opaque.member, x => use(x)", false),
    ] {
        let source =
            format!("function main(opaque) {{\nconst items = [1];\nitems.map({argument});\n}}");
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let start = source.find("items.map").unwrap();
        let from = endpoint(&cpg, "main", "items", (start, start), VarAccess::Use);
        let x = source.find("x =>").unwrap();
        let column = x - start + 1;
        let to = endpoint(
            &cpg,
            &format!("<anon@3:{column}>"),
            "x",
            (x, x + 1),
            VarAccess::Def,
        );
        assert_eq!(
            builtin_edges(&cpg),
            if expected { vec![(from, to)] } else { vec![] },
            "{source}"
        );
    }
}

#[test]
fn reducer_seed_and_rest_roles_have_exact_multiplicity() {
    for (suffix, expected) in [
        ("", 2),
        (", undefined", 1),
        (", ...opaque", 1),
        (", ...[]", 2),
    ] {
        let source = format!("function main(opaque) {{\nconst items = [1];\nitems.reduce((acc, value, index, array) => use(acc, value, index, array){suffix});\n}}");
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let start = source.find("items.reduce").unwrap();
        let from = endpoint(&cpg, "main", "items", (start, start), VarAccess::Use);
        let owner = "<anon@3:14>";
        let mut destinations = BTreeMap::new();
        for formal in ["acc", "value", "index", "array"] {
            let start = source.find(formal).unwrap();
            destinations.insert(
                formal,
                endpoint(
                    &cpg,
                    owner,
                    formal,
                    (start, start + formal.len()),
                    VarAccess::Def,
                ),
            );
        }
        let edges = builtin_edges(&cpg);
        assert_eq!(edges.len(), expected, "{source}");
        assert!(edges.contains(&(from, destinations["value"])), "{source}");
        assert_eq!(
            edges.contains(&(from, destinations["acc"])),
            expected == 2,
            "{source}"
        );
    }

    let source =
        "function main() {\nconst items = [1];\nitems.reduce((...parts) => use(parts));\n}";
    let cpg = build(LANGS[0].0, &[("m", source)], "js");
    let start = source.find("items.reduce").unwrap();
    let from = endpoint(&cpg, "main", "items", (start, start), VarAccess::Use);
    let start = source.find("parts").unwrap();
    let to = endpoint(
        &cpg,
        "<anon@3:14>",
        "parts",
        (start, start + 5),
        VarAccess::Def,
    );
    assert_eq!(builtin_edges(&cpg), [(from, to)]);
}

#[test]
fn named_value_requires_live_initialization_and_rejects_writes() {
    let cases = [
        ("", "", "function visit(item) { use(item); }", true),
        ("", "const visit = (item) => use(item);", "", true),
        ("", "", "const visit = (item) => use(item);", false),
        ("", "let visit = (item) => use(item);", "", false),
        (
            "function visit(item) { use(item); }",
            "visit = other;",
            "",
            false,
        ),
        (
            "function visit(item) { use(item); }",
            "function later() { visit = other; }",
            "",
            false,
        ),
        (
            "function visit(item) { use(item); }",
            "function later(visit) { visit = other; }",
            "",
            true,
        ),
    ];
    for (before, local, after, expected) in cases {
        let source = format!("{before}\nfunction main() {{\nconst items = [1];\n{local}\nitems.forEach(visit);\n{after}\n}}");
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let from = source.find("items.forEach").unwrap();
        let from = endpoint(&cpg, "main", "items", (from, from), VarAccess::Use);
        let to = source.find("item)").unwrap();
        let to = endpoint(&cpg, "visit", "item", (to, to + 4), VarAccess::Def);
        assert_eq!(
            builtin_edges(&cpg),
            if expected { vec![(from, to)] } else { vec![] },
            "{source}"
        );
    }
}

#[test]
fn deferred_const_read_requires_execution_order_proof() {
    for nested in [false, true] {
        let call = if nested {
            "function consume() {\nconst items = [1];\nitems.map(visit);\n}"
        } else {
            "items.map(visit);"
        };
        let early_call = if nested { "consume();" } else { "" };
        let source = format!("function main() {{\nconst items = [1];\n{early_call}\nconst visit = (item) => use(item);\n{call}\n}}");
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let start = source.find("items.map").unwrap();
        let from = endpoint(
            &cpg,
            if nested { "consume" } else { "main" },
            "items",
            (start, start),
            VarAccess::Use,
        );
        let start = source.find("item)").unwrap();
        let to = endpoint(&cpg, "visit", "item", (start, start + 4), VarAccess::Def);
        assert_eq!(
            builtin_edges(&cpg),
            if nested { vec![] } else { vec![(from, to)] },
            "{source}"
        );
    }
}

#[test]
fn h07e_receiver_inventory_uses_only_represented_occurrences() {
    let source = "class C {\n  main() {\n    this.items = read();\n    this.items.forEach(v => use(v));\n  }\n}\n";
    let start = source.find("this.items.forEach").unwrap();
    assert_eq!(start, 50);
    let cpg = build(LANGS[0].0, &[("m", source)], "js");
    let target = source.find("v =>").unwrap();
    endpoint(
        &cpg,
        "<anon@4:24>",
        "v",
        (target, target + 1),
        VarAccess::Def,
    );
    let uses: Vec<_> = cpg
        .graph
        .node_indices()
        .filter_map(|node| match cpg.node(node) {
            CpgNode::Variable {
                function,
                line: 4,
                path,
                start_byte,
                end_byte,
                access: VarAccess::Use,
                ..
            } if function == "main" && path.base == "this" => {
                Some((path.to_string(), *start_byte, *end_byte))
            }
            _ => None,
        })
        .collect();
    assert!(builtin_edges(&cpg).is_empty(), "{uses:?}");
    assert_eq!(uses, [("this.items".to_owned(), 46, 46)]);
}

#[test]
fn named_callback_refusals_keep_the_same_file_control() {
    let control = "function stable(kept) { use(kept); }";
    for (prefix, statements, decoy) in [
        ("", "const visit = stable;", "visit"),
        ("", "const visit = stable.bind(null);", "visit"),
        ("", "const visit = factory(stable);", "visit"),
        ("", "const visit = { fn: stable };", "visit.fn"),
        (
            "",
            "function visit(wrong) { use(wrong); }\nfunction visit(alsoWrong) { use(alsoWrong); }",
            "visit",
        ),
        ("", "const box = { visit: stable };", "box.visit"),
        (
            "",
            "let visit = function wrong(item) { use(item); };",
            "visit",
        ),
        ("import { visit } from './external.js';", "", "visit"),
        (
            "import { visit as foreign } from './external.js';",
            "const visit = foreign;",
            "visit",
        ),
    ] {
        let source = format!("{prefix}\n{control}\nfunction main() {{\nconst items = [1];\n{statements}\nitems.forEach({decoy});\nitems.forEach(stable);\n}}");
        let cpg = build(
            LANGS[0].0,
            &[
                ("m", &source),
                ("external", "export function visit(other) { use(other); }"),
            ],
            "js",
        );
        let bad = source.find("items.forEach").unwrap();
        let good = source.rfind("items.forEach").unwrap();
        endpoint(&cpg, "main", "items", (bad, bad), VarAccess::Use);
        let from = endpoint(&cpg, "main", "items", (good, good), VarAccess::Use);
        let start = source.find("kept)").unwrap();
        let formal = endpoint(&cpg, "stable", "kept", (start, start + 4), VarAccess::Def);
        if let Some(start) = source.find("wrong)") {
            endpoint(&cpg, "visit", "wrong", (start, start + 5), VarAccess::Def);
        }
        assert_eq!(builtin_edges(&cpg), [(from, formal)], "{source}");
    }
}

#[test]
fn attached_syntax_uses_static_names_and_distinct_inline_owners() {
    for (callee, dialect) in [
        ("items.map", "js"),
        ("items?.map", "js"),
        ("items['map']?.", "js"),
        ("(items.map)", "js"),
        (r"items.\u006dap", "js"),
        ("(items.map as Function)", "ts"),
        ("items.map!", "ts"),
    ] {
        let source = format!("function main() {{\nconst items = [1];\n{callee}(/* é */ function* (value) {{ use(value); }});\n}}");
        let lang = LANGS.iter().find(|&&(_, ext)| ext == dialect).unwrap().0;
        let cpg = build(lang, &[("m", &source)], dialect);
        let token = source.find("function*").unwrap();
        let line = source.find(callee).unwrap();
        let formal = source.find("value)").unwrap();
        let owner = format!("<anon@3:{}>", token - line + 1);
        let defs: Vec<_> = cpg
            .graph
            .node_indices()
            .filter(|&node| {
                matches!(cpg.node(node), CpgNode::Variable {
                function, path, start_byte, end_byte, access: VarAccess::Def, ..
            } if *function == owner && path.base == "value"
                && (*start_byte, *end_byte) == (formal, formal + 5))
            })
            .collect();
        assert_eq!(defs.len(), 1, "{source}");
        let edges = builtin_edges(&cpg);
        if callee.starts_with('(') {
            // The grouped expression can have only a line-start fallback.
            // The supplied-occurrence tests exercise grouping admission.
            assert!(edges.is_empty(), "{source}");
        } else {
            assert_eq!(edges.len(), 1, "{source}");
            assert_eq!(edges[0].1, defs[0], "{source}");
            assert!(
                matches!(cpg.node(edges[0].0), CpgNode::Variable {
                function, path, start_byte, access: VarAccess::Use, ..
            } if function == "main" && path.base == "items" && *start_byte == line),
                "{source}"
            );
        }
    }
}

#[test]
fn destructure_and_typescript_this_keep_runtime_slots() {
    for (lang, ext, formals) in [
        (LANGS[0].0, "js", "{ ignored }, value"),
        (LANGS[1].0, "ts", "this: object, { ignored }, value: number"),
        (
            LANGS[2].0,
            "tsx",
            "this: object, { ignored }, value: number",
        ),
    ] {
        let source = format!("function step({formals}) {{\nuse(ignored, value);\n}}\nfunction main() {{\nconst items = [1];\nitems.reduce(step);\n}}");
        let cpg = build(lang, &[("m", &source)], ext);
        let edges = builtin_edges(&cpg);
        let from = source.find("items.reduce").unwrap();
        let to = source.find("value").unwrap();
        assert_eq!(edges.len(), 1, "{source}");
        assert!(
            matches!(cpg.node(edges[0].0), CpgNode::Variable {
            function, path, start_byte, end_byte, access: VarAccess::Use, ..
        } if function == "main" && path.base == "items"
            && (*start_byte, *end_byte) == (from, from)),
            "{source}"
        );
        assert!(
            matches!(cpg.node(edges[0].1), CpgNode::Variable {
            function, path, start_byte, end_byte, access: VarAccess::Def, ..
        } if function == "step" && path.base == "value"
            && (*start_byte, *end_byte) == (to, to + 5)),
            "{source}"
        );
    }
}

#[test]
fn fulfillment_only_leaves_rejection_unfed() {
    let source = "function ok(value) {\nuse(value);\n}\nfunction fail(reason) {\nuse(reason);\n}\nfunction main() {\nconst promise = source();\npromise.then(ok, fail);\n}";
    let cpg = build(LANGS[0].0, &[("m", source)], "js");
    let from = source.find("promise.then").unwrap();
    let from = endpoint(&cpg, "main", "promise", (from, from), VarAccess::Use);
    let ok = source.find("value)").unwrap();
    let ok = endpoint(&cpg, "ok", "value", (ok, ok + 5), VarAccess::Def);
    let fail = source.find("reason)").unwrap();
    endpoint(&cpg, "fail", "reason", (fail, fail + 6), VarAccess::Def);
    assert_eq!(builtin_edges(&cpg), [(from, ok)]);
}

#[test]
fn producer_arguments_are_not_receivers_of_call_results() {
    for (producer, outer, decoys) in [
        ("Object.keys(o)", "forEach", &["o"][..]),
        ("arr.map(f)", "filter", &["f"][..]),
        ("fetch(u)", "then", &["u"][..]),
    ] {
        let source = format!("function visit(item) {{\nuse(item);\n}}\nfunction main(o, arr, f, u) {{\nconst control = [1];\n{producer}.{outer}(visit);\ncontrol.map(visit);\n}}");
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let formal = endpoint(&cpg, "visit", "item", (15, 19), VarAccess::Def);
        let start = source.find("control.map").unwrap();
        let represented = endpoint(&cpg, "main", "control", (start, start), VarAccess::Use);
        for decoy in decoys {
            let start = source.find(&format!("({decoy})")).unwrap() + 1;
            endpoint(
                &cpg,
                "main",
                decoy,
                (start, start + decoy.len()),
                VarAccess::Use,
            );
        }
        assert_eq!(builtin_edges(&cpg), [(represented, formal)], "{source}");
    }
}

#[test]
fn installed_override_keeps_the_table_candidate_at_nameonly() {
    // The external oracle grades one witnessed contrary delivery and one
    // opaque delivery. Both remain NameOnly candidates in the product.
    for override_value in ["cb => { cb(99, 0, []); }", "opaque()"] {
        let source = format!("function visit(item) {{\nuse(item);\n}}\nfunction main() {{\nconst items = [0];\nitems.forEach = {override_value};\nitems.forEach(visit);\n}}");
        let cpg = build(LANGS[0].0, &[("m", &source)], "js");
        let formal = endpoint(&cpg, "visit", "item", (15, 19), VarAccess::Def);
        let start = source.find("items.forEach(visit)").unwrap();
        let recv = endpoint(&cpg, "main", "items", (start, start), VarAccess::Use);
        assert_eq!(builtin_edges(&cpg), [(recv, formal)], "{source}");
        assert!(!cpg
            .graph
            .edges_connecting(recv, formal)
            .any(|edge| *edge.weight() == CpgEdge::DataFlow(FlowConfidence::Exact)));
    }
}
