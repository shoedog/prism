//! Collector tests supplying occurrences the DFG builder can collapse.

use super::builtin_callbacks::{collect, diagnostic::capture, LABEL};
use super::*;
use crate::call_graph::{CallKind, CallSiteOrigin};
use crate::cpg::param_binding_builtin_tests::{builtin_edges, DIRECT};
use crate::languages::Language;

fn fixture(path: &str, source: &str, lang: Language) -> (ParsedFile, CodePropertyGraph) {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    assert_eq!(parsed.parse_error_count, 0, "{source}");
    let mut files = BTreeMap::from([(path.to_owned(), parsed)]);
    let cpg = CodePropertyGraph::build(&files);
    (files.remove(path).unwrap(), cpg)
}

fn occurrence(
    graph: &DiGraph<CpgNode, CpgEdge>,
    function: &str,
    path: &str,
    span: (usize, usize),
    access: VarAccess,
) -> NodeIndex {
    let nodes: Vec<_> = graph
        .node_indices()
        .filter(|&idx| {
            matches!(&graph[idx], CpgNode::Variable {
            function: owner, path: actual, access: kind, start_byte, end_byte, ..
        } if owner == function && actual.to_string() == path && *kind == access
            && (*start_byte, *end_byte) == span)
        })
        .collect();
    assert_eq!(nodes.len(), 1, "{function}:{path} {span:?} {access:?}");
    nodes[0]
}

fn site(cpg: &CodePropertyGraph, at: usize) -> CallSite {
    cpg.call_graph
        .calls
        .values()
        .flatten()
        .find(|site| site.start_byte == at)
        .unwrap_or_else(|| panic!("no recorded site at {at}"))
        .clone()
}

fn transfer(
    site: &CallSite,
    caller: &FunctionId,
    parsed: &ParsedFile,
    graph: &DiGraph<CpgNode, CpgEdge>,
) -> Vec<(NodeIndex, NodeIndex)> {
    let mut transfers = Vec::new();
    collect(
        &mut transfers,
        site,
        caller,
        parsed,
        &VarIndex::from_graph(graph),
        &CodePropertyGraph::exact_var_index_from_graph(graph),
        graph,
        super::builtin_callbacks::diagnostic::enabled(),
    );
    transfers
        .into_iter()
        .map(|flow| {
            assert_eq!(flow.edge.2, CpgEdge::DataFlow(LABEL));
            assert!(!flow.cross_file_free_single_exact);
            assert!(!flow.bare_literal);
            (flow.edge.0, flow.edge.1)
        })
        .collect()
}

fn use_at(owner: &FunctionId, line: usize, path: AccessPath, span: (usize, usize)) -> CpgNode {
    CpgNode::Variable {
        file: owner.file.clone(),
        function: owner.name.clone(),
        function_start_line: owner.start_line,
        line,
        path,
        access: VarAccess::Use,
        start_byte: span.0,
        end_byte: span.1,
    }
}

#[test]
fn receiver_precedence_rejects_wrong_and_ambiguous_occurrences() {
    let (parsed, mut cpg) = fixture("representation.js", DIRECT, Language::JavaScript);
    let mut call = site(&cpg, 97);
    let caller = call.caller.clone();
    let base = occurrence(&cpg.graph, "main", "items", (97, 97), VarAccess::Use);
    let formal = occurrence(&cpg.graph, "visit", "item", (15, 19), VarAccess::Def);
    assert_eq!(
        transfer(&call, &caller, &parsed, &cpg.graph),
        [(base, formal)]
    );
    let projection = cpg.graph.add_node(use_at(
        &caller,
        call.line,
        AccessPath::with_fields("items", vec!["forEach".into()]),
        (97, 110),
    ));
    assert_eq!(
        transfer(&call, &caller, &parsed, &cpg.graph),
        [(projection, formal)]
    );
    let complete = use_at(&caller, call.line, AccessPath::simple("items"), (97, 102));
    let full = cpg.graph.add_node(complete.clone());
    assert_eq!(
        transfer(&call, &caller, &parsed, &cpg.graph),
        [(full, formal)]
    );
    let duplicate = cpg.graph.add_node(complete);
    assert!(transfer(&call, &caller, &parsed, &cpg.graph).is_empty());
    cpg.graph.remove_node(duplicate);

    for wrong in [
        CallSite {
            end_byte: call.end_byte - 1,
            ..call.clone()
        },
        CallSite {
            callee_name: "map".into(),
            ..call.clone()
        },
        CallSite {
            origin: CallSiteOrigin::IndirectResolution,
            ..call.clone()
        },
        CallSite {
            kind: CallKind::MacroInvocation,
            ..call.clone()
        },
        CallSite {
            jsx_element: true,
            ..call.clone()
        },
    ] {
        assert!(
            transfer(&wrong, &caller, &parsed, &cpg.graph).is_empty(),
            "{wrong:?}"
        );
    }
    let wrong_caller = FunctionId {
        name: "visit".into(),
        ..caller.clone()
    };
    assert!(transfer(&call, &wrong_caller, &parsed, &cpg.graph).is_empty());
    call.caller = wrong_caller.clone();
    assert!(transfer(&call, &wrong_caller, &parsed, &cpg.graph).is_empty());
}

#[test]
fn receiver_byte_and_owner_evidence_cannot_be_borrowed() {
    let source = DIRECT.replace("\nitems.", "\n  items.");
    let (parsed, mut cpg) = fixture("receiver.js", &source, Language::JavaScript);
    let call = site(&cpg, 99);
    let caller = &call.caller;
    occurrence(&cpg.graph, "main", "items", (97, 97), VarAccess::Use);
    let formal = occurrence(&cpg.graph, "visit", "item", (15, 19), VarAccess::Def);
    for decoy in [
        use_at(caller, call.line, AccessPath::simple("items"), (100, 104)),
        use_at(caller, call.line, AccessPath::simple("other"), (99, 104)),
        use_at(
            &FunctionId {
                name: "visit".into(),
                ..caller.clone()
            },
            call.line,
            AccessPath::simple("items"),
            (99, 104),
        ),
        use_at(
            &FunctionId {
                file: "foreign.js".into(),
                ..caller.clone()
            },
            call.line,
            AccessPath::simple("items"),
            (99, 104),
        ),
    ] {
        cpg.graph.add_node(decoy);
    }
    assert!(transfer(&call, caller, &parsed, &cpg.graph).is_empty());
    let full = cpg.graph.add_node(use_at(
        caller,
        call.line,
        AccessPath::simple("items"),
        (99, 104),
    ));
    assert_eq!(
        transfer(&call, caller, &parsed, &cpg.graph),
        [(full, formal)]
    );
}

#[test]
fn represented_formal_without_existing_read_edge_supplies_no_binding() {
    let (parsed, mut cpg) = fixture("formal.js", DIRECT, Language::JavaScript);
    let call = site(&cpg, 97);
    let from = occurrence(&cpg.graph, "main", "items", (97, 97), VarAccess::Use);
    let formal = occurrence(&cpg.graph, "visit", "item", (15, 19), VarAccess::Def);
    let read = occurrence(&cpg.graph, "visit", "item", (29, 33), VarAccess::Use);
    assert_eq!(
        transfer(&call, &call.caller, &parsed, &cpg.graph),
        [(from, formal)]
    );
    cpg.graph.retain_edges(|graph, edge| {
        graph.edge_endpoints(edge) != Some((formal, read)) || !graph[edge].is_data_flow()
    });
    assert!(transfer(&call, &call.caller, &parsed, &cpg.graph).is_empty());
    // A Def carrying the wrong bytes remains indexed in the same slot.
    cpg.graph
        .add_edge(formal, read, CpgEdge::DataFlow(FlowConfidence::Exact));
    let CpgNode::Variable { start_byte, .. } = &mut cpg.graph[formal] else {
        unreachable!()
    };
    *start_byte += 1;
    assert!(transfer(&call, &call.caller, &parsed, &cpg.graph).is_empty());
}

#[test]
fn diagnostic_records_join_role_outcomes_to_edges() {
    const FILE: &str = "d-capture-only.js";
    let source = "function main(opaque) {\nconst items = [1];\nitems.reduce((...parts) => use(parts));\nitems.reduce((acc, elem) => use(acc, elem), ...opaque);\nitems.reduce((a, b) => use(a, b), undefined);\nitems.map(unread => 0);\nitems.map(...opaque, unused => use(unused));\nitems.map(0, later => use(later));\nitems.map();\nitems.map(stable);\nitems[opaque](stable);\n  items.map(stable);\n}\nfunction stable(value) { use(value); }\nexport {};";
    let (_, control) = fixture(FILE, source, Language::JavaScript);
    let ((_, captured), lines) = capture::run(|| fixture(FILE, source, Language::JavaScript));
    assert_eq!(builtin_edges(&control), builtin_edges(&captured));
    let records: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| {
            serde_json::from_str(line.strip_prefix("PRISM_D_BUILTIN_TRACE ").unwrap()).unwrap()
        })
        .filter(|record: &serde_json::Value| record["caller_file"] == FILE)
        .collect();
    let mut dispositions = BTreeMap::new();
    for record in &records {
        let range = record["call"].as_array().unwrap();
        let start = range[0].as_u64().unwrap() as usize;
        let end = range[1].as_u64().unwrap() as usize;
        assert!(start < end && end <= source.len());
        assert_eq!(record["route"], "builtin_callback");
        assert!(record["caller"].is_string());
        assert!(record["callee"].is_string());
        for key in [
            "receiver",
            "receiver_use",
            "callback",
            "declaration",
            "formal",
            "role",
            "slot",
        ] {
            assert!(record.get(key).is_some(), "{key}: {record}");
        }
        *dispositions
            .entry(record["reason"].as_str().unwrap())
            .or_insert(0) += 1;
    }
    assert_eq!(dispositions["table_role"], builtin_edges(&captured).len());
    for reason in [
        "same_rest_def",
        "seed_presence_unknown",
        "seed_supplied",
        "missing_formal_def",
        "unknown_slot_zero",
        "non_callable_slot_zero",
        "missing_slot_zero",
        "non_public_static_key",
        "missing_receiver_use",
    ] {
        assert_eq!(
            dispositions.get(reason),
            Some(&1),
            "{reason}: {dispositions:?}"
        );
    }
    let named: Vec<_> = records
        .iter()
        .filter(|record| record["target_owner"] == "stable")
        .collect();
    assert_eq!(named.len(), 1);
    let named = &named[0];
    assert_eq!(named["receiver_representation"], "zero_width_base");
    assert_eq!(named["formal"]["path"]["base"], "value");
    assert_eq!(named["receiver_use"]["path"]["base"], "items");
    assert_eq!(named["role"], "element");
    assert_eq!(named["slot"], 0);
    assert_ne!(named["callback"], named["declaration"]);
}

#[test]
fn attached_references_bind_and_detached_invocations_leave_the_formal_unfed() {
    for (expression, callee, accepted) in [
        ("(items.map)(visit)", "(items.map)", true),
        (
            "(items.map as Function)(visit)",
            "(items.map as Function)",
            true,
        ),
        ("(0, items.map)(visit)", "(0, items.map)", false),
        ("items[key](visit)", "items[key]", false),
        ("items.map`${visit}`", "map", false),
        ("new items.map(visit)", "map", false),
    ] {
        let source = format!("function visit(item) {{\nuse(item);\n}}\nfunction main(key) {{\nconst items = [1];\nitems.map(visit);\n{expression};\n}}\nexport {{}};");
        let (parsed, mut cpg) = fixture("syntax.ts", &source, Language::TypeScript);
        let good = source.find("items.map").unwrap();
        let control = site(&cpg, good);
        let original = occurrence(&cpg.graph, "main", "items", (good, good), VarAccess::Use);
        let formal = occurrence(&cpg.graph, "visit", "item", (15, 19), VarAccess::Def);
        assert_eq!(
            transfer(&control, &control.caller, &parsed, &cpg.graph),
            [(original, formal)]
        );
        let start = source.find(&format!("\n{expression}")).unwrap() + 1;
        let receiver = start + expression.find("items").unwrap();
        let alternate = CallSite {
            start_byte: start,
            end_byte: start + expression.len(),
            callee_name: callee.to_owned(),
            line: control.line + 1,
            ..control.clone()
        };
        let from = cpg.graph.add_node(use_at(
            &alternate.caller,
            alternate.line,
            AccessPath::simple("items"),
            (receiver, receiver + "items".len()),
        ));
        assert_eq!(
            transfer(&alternate, &alternate.caller, &parsed, &cpg.graph),
            if accepted {
                vec![(from, formal)]
            } else {
                vec![]
            },
            "{expression}"
        );
    }
}

#[test]
fn non_js_same_spelling_keeps_its_ordinary_binding() {
    let source = "class List:\n def map(self, f):\n  use(f)\n\ndef main():\n xs = List()\n actual = 1\n xs.map(actual)\n";
    let (parsed, cpg) = fixture("control.py", source, Language::Python);
    let call = site(&cpg, source.find("xs.map").unwrap());
    let mut parser_evidence = Vec::new();
    collect(
        &mut parser_evidence,
        &call,
        &call.caller,
        &parsed,
        &cpg.var_index,
        &CodePropertyGraph::exact_var_index_from_graph(&cpg.graph),
        &cpg.graph,
        super::builtin_callbacks::diagnostic::enabled(),
    );
    assert!(parser_evidence.is_empty());
    let edges: Vec<_> = cpg
        .graph
        .edge_references()
        .filter_map(|edge| {
            let CpgEdge::DataFlow(label) = *edge.weight() else {
                return None;
            };
            let from = cpg.to_var_location(edge.source())?;
            let to = cpg.to_var_location(edge.target())?;
            (from.function == "main" && to.function == "map").then_some((
                from.path.base,
                to.path.base,
                label,
            ))
        })
        .collect();
    assert_eq!(edges.len(), 1, "{edges:?}");
    assert_eq!(edges[0].0, "actual");
    assert_eq!(edges[0].1, "f");
    assert_ne!(edges[0].2, LABEL);
}

#[test]
fn closure_read_with_outer_owner_bytes_is_not_an_entry_read_witness() {
    for nested in [false, true] {
        let read = if nested {
            "  function later() { use(item); }\n  later();"
        } else {
            "  use(item);"
        };
        let source = format!("function visit(item) {{\n{read}\n}}\nfunction main() {{\nconst items = [1];\nitems.map(visit);\n}}\nexport {{}};");
        let (parsed, mut cpg) = fixture("closure.js", &source, Language::JavaScript);
        let call = site(&cpg, source.find("items.map").unwrap());
        let formal = occurrence(&cpg.graph, "visit", "item", (15, 19), VarAccess::Def);
        let from = occurrence(
            &cpg.graph,
            "main",
            "items",
            (call.start_byte, call.start_byte),
            VarAccess::Use,
        );
        if nested {
            let anchor = occurrence(&cpg.graph, "visit", "item", (23, 23), VarAccess::Use);
            assert!(cpg
                .graph
                .edges_connecting(formal, anchor)
                .any(|edge| edge.weight().is_data_flow()));
            assert!(builtin_edges(&cpg).is_empty());
            // Supply the same outer-owned read identity used by the DFG for
            // captures. Its nested syntax is the only distinction here.
            let start = source.find("use(item)").unwrap() + 4;
            let mut use_node = cpg.graph[formal].clone();
            let CpgNode::Variable {
                access,
                line,
                start_byte,
                end_byte,
                ..
            } = &mut use_node
            else {
                unreachable!()
            };
            (*access, *line, *start_byte, *end_byte) = (VarAccess::Use, 2, start, start + 4);
            let captured = cpg.graph.add_node(use_node);
            cpg.graph
                .add_edge(formal, captured, CpgEdge::DataFlow(FlowConfidence::Exact));
        }
        assert_eq!(
            transfer(&call, &call.caller, &parsed, &cpg.graph),
            if nested { vec![] } else { vec![(from, formal)] },
            "{source}"
        );
    }
}

#[test]
fn sloppy_annex_b_and_with_do_not_reuse_an_outer_declaration() {
    for (path, prologue, wrapper, expected) in [
        (
            "scope.cjs",
            "",
            "if (flag) { function visit(wrong) { use(wrong); } }",
            false,
        ),
        (
            "scope.cjs",
            "'use strict';",
            "if (flag) { function visit(wrong) { use(wrong); } }",
            true,
        ),
        ("scope.cjs", "", "with (env) {", false),
        ("scope.cjs", "", "{", true),
    ] {
        let closing = if wrapper.ends_with('{') { "}" } else { "" };
        let source = format!("{prologue}\nfunction main(flag, env) {{\nfunction visit(item) {{ use(item); }}\nconst items = [1];\n{wrapper}\nitems.forEach(visit);\n{closing}\n}}");
        let (parsed, mut cpg) = fixture(path, &source, Language::JavaScript);
        let call = site(&cpg, source.find("items.forEach").unwrap());
        let start = source.find("item)").unwrap();
        let formal = occurrence(
            &cpg.graph,
            "visit",
            "item",
            (start, start + 4),
            VarAccess::Def,
        );
        if let Some(start) = source.find("wrong)") {
            occurrence(
                &cpg.graph,
                "visit",
                "wrong",
                (start, start + 5),
                VarAccess::Def,
            );
        }
        let receiver = cpg.graph.add_node(use_at(
            &call.caller,
            call.line,
            AccessPath::simple("items"),
            (call.start_byte, call.start_byte + 5),
        ));
        assert_eq!(
            transfer(&call, &call.caller, &parsed, &cpg.graph),
            if expected {
                vec![(receiver, formal)]
            } else {
                vec![]
            },
            "{source}"
        );
    }
}

fn assert_callback_decisions(file: &str, source: &str, language: Language, calls: &[(&str, &str)]) {
    let ((_, cpg), lines) = capture::run(|| fixture(file, source, language));
    let records: Vec<serde_json::Value> = lines
        .iter()
        .map(|line| {
            serde_json::from_str(line.strip_prefix("PRISM_D_BUILTIN_TRACE ").unwrap()).unwrap()
        })
        .filter(|record: &serde_json::Value| record["caller_file"] == file)
        .collect();
    for &(call, reason) in calls {
        let start = source.find(call).unwrap();
        let outcomes: Vec<_> = records
            .iter()
            .filter(|record| record["call"][0] == start && record["disposition"] != "omitted")
            .map(|record| {
                (
                    record["disposition"].as_str().unwrap(),
                    record["reason"].as_str().unwrap(),
                )
            })
            .collect();
        let disposition = if reason == "table_role" {
            "admitted"
        } else {
            "refused"
        };
        assert_eq!(
            outcomes,
            [(disposition, reason)],
            "{file}: {call}\n{source}"
        );
    }
    let admitted = calls
        .iter()
        .filter(|(_, reason)| *reason == "table_role")
        .count();
    assert_eq!(builtin_edges(&cpg).len(), admitted, "{file}");
}

#[test]
fn direct_eval_with_type_arguments_refuses_the_named_callback() {
    for (callee, reason) in [
        ("(eval)", "dynamic_binding"),
        ("(eval<string>)", "dynamic_binding"),
        ("(eval<string>)?.", "table_role"),
        ("(0, eval<string>)", "table_role"),
    ] {
        let source = format!(
            "function main(replacement) {{\nfunction visit(v) {{ use(v); }}\nconst items = [1];\n{callee}(\"visit = replacement\");\nitems.map(visit);\n}}"
        );
        assert_callback_decisions(
            "typed-eval.ts",
            &source,
            Language::TypeScript,
            &[("items.map(visit)", reason)],
        );
    }
}

#[test]
fn global_object_alias_refuses_script_global_callback_but_preserves_local_and_module() {
    let source = "function visit(v) { use(v); }\nfunction main(replacement) {\nfunction local(v) { use(v); }\nconst items = [1];\nglobalThis.visit = replacement;\nitems.map(visit);\nitems.map(local);\n}";
    assert_callback_decisions(
        "classic.js",
        source,
        Language::JavaScript,
        &[
            ("items.map(visit)", "script_global_binding"),
            ("items.map(local)", "table_role"),
        ],
    );
    assert_callback_decisions(
        "module.js",
        &format!("{source}\nexport {{}};"),
        Language::JavaScript,
        &[
            ("items.map(visit)", "table_role"),
            ("items.map(local)", "table_role"),
        ],
    );
}

#[test]
fn refuses_indirect_initializer() {
    let source = "import {useCallback} from 'react';\nfunction main() {\nconst items = [1];\nconst visit = useCallback((item) => use(item), []);\nconst stable = (item) => use(item);\nitems.forEach(visit);\nitems.forEach(stable);\n}";
    assert_callback_decisions(
        "indirect.js",
        source,
        Language::JavaScript,
        &[
            ("items.forEach(visit)", "indirect_initializer"),
            ("items.forEach(stable)", "table_role"),
        ],
    );
}

#[test]
fn refuses_unsupported_declaration() {
    let source = "const outer = function walk(node) {\nnode.kids.forEach(walk);\n};\nfunction main() {\nconst items = [1];\nitems.forEach(stable => use(stable));\n}";
    assert_callback_decisions(
        "self-reference.js",
        source,
        Language::JavaScript,
        &[
            ("node.kids.forEach(walk)", "unsupported_declaration"),
            ("items.forEach(stable", "table_role"),
        ],
    );
}

#[test]
fn refuses_callback_owner() {
    let source = "function main() {\nconst items = [1];\nitems.forEach((a, a) => use(a));\nitems.forEach((a, b) => use(a, b));\n}";
    assert_callback_decisions(
        "owner.js",
        source,
        Language::JavaScript,
        &[
            ("items.forEach((a, a)", "callback_owner"),
            ("items.forEach((a, b)", "table_role"),
        ],
    );
}

#[test]
fn refuses_formal_layout() {
    let source = "function main() {\nconst items = [1];\nitems.reduce(function (a, a) { use(a); }, 0);\nitems.reduce(function (a, b) { use(a, b); }, 0);\n}";
    assert_callback_decisions(
        "layout.cjs",
        source,
        Language::JavaScript,
        &[
            ("items.reduce(function (a, a)", "formal_layout"),
            ("items.reduce(function (a, b)", "table_role"),
        ],
    );
}
