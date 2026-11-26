use super::*;
use crate::call_graph::CallSiteOrigin;
use serde_json::{json, Value};

fn matrix_source<'a>(rows: &'a [Value], id: &str) -> (&'a [Value], &'a str) {
    let row = rows.iter().find(|row| row["id"] == id).unwrap();
    let files = row["files"].as_object().unwrap();
    assert_eq!(files.len(), 1, "{id}");
    (
        row["langs"].as_array().unwrap(),
        files.values().next().unwrap().as_str().unwrap(),
    )
}

fn matrix_rows() -> Vec<Value> {
    include_str!("../../tests/fixtures/param_binding_matrix/rows.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn matrix_input(ext: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let language = match ext {
        "js" => Language::JavaScript,
        "ts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        _ => unreachable!("{ext}"),
    };
    parsed_files(&[(&format!("m.{ext}"), source)], language)
}

fn owner_at(parsed: &ParsedFile, name: &str, start_line: Option<usize>) -> FunctionId {
    let mut owners = parsed.all_functions().into_iter().filter(|node| {
        parsed.dfg_owner_name(node).as_deref() == Some(name)
            && start_line.is_none_or(|line| parsed.node_line_range(node).0 == line)
    });
    let owner = owners
        .next()
        .unwrap_or_else(|| panic!("missing {name}@{start_line:?}"));
    assert!(owners.next().is_none(), "{name}@{start_line:?}");
    let (start_line, end_line) = parsed.node_line_range(&owner);
    FunctionId {
        file: parsed.path.clone(),
        name: name.into(),
        start_line,
        end_line,
    }
}

fn supply_constructor_uses(
    cpg: &mut CodePropertyGraph,
    site: &CallSite,
    parsed: &ParsedFile,
    arguments: &[spread_args::Argument],
) {
    let (file, owner, owner_start) = (&site.caller.file, &site.caller.name, site.caller.start_line);
    for (text, span) in arguments {
        let path = AccessPath::simple(text);
        let Some(formal) = cpg.var_index.get_ref(
            &cpg.graph,
            VarKeyRef {
                file,
                function: owner,
                function_start_line: owner_start,
                line: owner_start,
                path: &path,
                access: VarAccess::Def,
            },
        ) else {
            continue;
        };
        let line = parsed.line_for_byte(span.start);
        let already_present = cpg.nodes_at(file, line).into_iter().any(|index| {
            cpg.to_var_location(index).is_some_and(|loc| {
                loc.kind == VarAccessKind::Use
                    && loc.function == *owner
                    && loc.function_start_line == owner_start
                    && loc.path == path
                    && (loc.start_byte..loc.end_byte) == *span
            })
        });
        if already_present {
            continue;
        }
        // These fixtures pass simple reads of main's formals. Supply the
        // byte-distinct Uses when the ordinary DFG has no new-expression read.
        let use_node = cpg.graph.add_node(CpgNode::Variable {
            file: file.clone(),
            function: owner.clone(),
            function_start_line: owner_start,
            line,
            path,
            access: VarAccess::Use,
            start_byte: span.start,
            end_byte: span.end,
        });
        cpg.location_index
            .entry((file.clone(), line))
            .or_default()
            .push(use_node);
        cpg.graph
            .add_edge(formal, use_node, CpgEdge::DataFlow(FlowConfidence::Exact));
    }
    cpg.var_index = VarIndex::from_graph(&cpg.graph);
}

#[test]
fn rest_constructor_supplied_sites_keep_end_to_end_prerequisites_visible() {
    let rows = matrix_rows();
    for (id, expression, line, expected) in [
        (
            "t4-3",
            "new C(x, y)",
            2,
            vec![("x", "first"), ("y", "rest")],
        ),
        (
            "s2-4",
            "new Packet(label, count, text)",
            2,
            vec![("count", "parts"), ("text", "parts")],
        ),
        (
            "s3-3",
            "new Derived(first, last)",
            7,
            vec![("first", "first"), ("last", "items")],
        ),
        ("s5-6", "new Derived(0, value)", 2, vec![("value", "tail")]),
        ("x3-5", "new Explicit(lost)", 8, vec![("lost", "forward")]),
        ("x3-5", "new Implicit(kept)", 2, vec![("kept", "parts")]),
        ("x5-3", "new Shape(direct)", 7, vec![("direct", "other")]),
    ] {
        let (extensions, source) = matrix_source(&rows, id);
        let extended = (id == "s5-6").then(|| {
            format!("{source}\nclass Decoy {{ constructor(first, ...tail) {{ use(tail); }} }}\n")
        });
        let source = extended.as_deref().unwrap_or(source);
        for ext in extensions.iter().map(|ext| ext.as_str().unwrap()) {
            let files = matrix_input(ext, source);
            let parsed = files.values().next().unwrap();
            let target = owner_at(parsed, "constructor", Some(line));
            let caller = owner_at(parsed, "main", None);
            let start = source.find(expression).unwrap();
            let span = start..start + expression.len();
            let node = parsed
                .tree
                .root_node()
                .descendant_for_byte_range(span.start, span.end)
                .unwrap();
            assert_eq!(node.kind(), "new_expression");
            assert_eq!(node.byte_range(), span);
            let constructor = node.child_by_field_name("constructor").unwrap();
            let site: CallSite = serde_json::from_value(json!({
                "caller": caller,
                "callee_name": parsed.node_text(&constructor),
                "line": parsed.line_for_byte(span.start),
                "start_byte": span.start,
                "end_byte": span.end,
            }))
            .unwrap();
            assert_eq!(site.origin, CallSiteOrigin::Source);
            let arguments = node.child_by_field_name("arguments").unwrap();
            let mut cursor = arguments.walk();
            let arguments: Vec<_> = arguments
                .named_children(&mut cursor)
                .filter(|node| !node.is_extra())
                .map(|node| (parsed.node_text(&node).to_owned(), node.byte_range()))
                .collect();
            assert!(!arguments.is_empty());
            assert!(
                spread_args::for_call(parsed, &site).0.is_empty(),
                "{id}/{ext}"
            );
            let mut cpg = CodePropertyGraph::build(&files);
            let real: Vec<_> = cpg
                .call_graph
                .calls
                .get(&site.caller)
                .into_iter()
                .flatten()
                .filter(|real| {
                    real.origin == CallSiteOrigin::Source
                        && real.start_byte == span.start
                        && real.end_byte == span.end
                })
                .map(|real| (real, cpg.call_graph.resolve_call_site(real)))
                .collect();
            assert!(
                !real
                    .iter()
                    .any(|(_, pool)| object_literal_args::exact_singleton(pool)
                        && pool[0].target == &target),
                "{id}/{ext}: constructor prerequisite now present: {real:?}"
            );
            println!(
                "CONSTRUCTOR {id}/{ext} {expression} {span:?} -> {target:?} real_sites={real:?}"
            );
            supply_constructor_uses(&mut cpg, &site, parsed, &arguments);
            if id == "s5-6" {
                let decoy = owner_at(
                    parsed,
                    "constructor",
                    Some(parsed.line_for_byte(source.find("class Decoy").unwrap())),
                );
                let wrong = vec![ResolvedCallee {
                    target: &decoy,
                    confidence: ResolutionConfidence::Exact,
                    kind: ResolutionKind::ConstructorLocal,
                }];
                let flows = run_site(&cpg, &files, &site, wrong, |parsed, _| {
                    (arguments.clone(), spread_args::offsets(parsed, &arguments))
                });
                assert_eq!(flows.len(), 1, "{id}/{ext}: wrong-target control");
                let (from, to, ref edge) = flows[0].edge;
                assert_eq!(cpg.to_var_location(from).unwrap().path.base, "value");
                let to = cpg.to_var_location(to).unwrap();
                assert_eq!(to.path.base, "tail");
                assert_eq!(to.function_start_line, decoy.start_line);
                assert_ne!(to.function_start_line, target.start_line);
                assert_eq!(edge, &CpgEdge::DataFlow(FlowConfidence::Exact));
            }
            let pool = vec![ResolvedCallee {
                target: &target,
                confidence: ResolutionConfidence::Exact,
                kind: ResolutionKind::ConstructorLocal,
            }];
            let pending = run_site(&cpg, &files, &site, pool, |parsed, actual_site| {
                assert_eq!(*actual_site, site);
                (arguments.clone(), spread_args::offsets(parsed, &arguments))
            });
            let got: Vec<_> = pending
                .iter()
                .map(|flow| {
                    let (use_node, def_node, ref edge) = flow.edge;
                    let source = cpg.to_var_location(use_node).unwrap();
                    let dest = cpg.to_var_location(def_node).unwrap();
                    assert_eq!(source.kind, VarAccessKind::Use);
                    assert_eq!(dest.kind, VarAccessKind::Def);
                    assert_eq!(source.function, site.caller.name);
                    assert_eq!(dest.function, target.name);
                    assert_eq!(dest.file, target.file);
                    assert_eq!(dest.function_start_line, target.start_line);
                    assert_eq!(edge, &CpgEdge::DataFlow(FlowConfidence::Exact));
                    assert!(arguments
                        .iter()
                        .any(|(text, span)| text == &source.path.base
                            && *span == (source.start_byte..source.end_byte)));
                    (source.path.base, dest.path.base)
                })
                .collect();
            assert_eq!(
                got,
                expected
                    .iter()
                    .map(|&(a, p)| (a.to_owned(), p.to_owned()))
                    .collect::<Vec<_>>(),
                "{id}/{ext}"
            );

            if id == "s2-4" {
                let tag = source.find("tag:").unwrap();
                assert!(!cpg.graph.node_indices().any(|index| cpg
                    .to_var_location(index)
                    .is_some_and(|loc| loc.kind == VarAccessKind::Def && loc.start_byte == tag)));
            }
            if id == "s3-3" {
                let base = owner_at(parsed, "constructor", Some(2));
                let super_start = source.find("super(").unwrap();
                let super_end = super_start + source[super_start..].find(';').unwrap();
                assert!(cpg.graph.edge_indices().all(|index| {
                    let (from, to) = cpg.graph.edge_endpoints(index).unwrap();
                    let Some(from) = cpg.to_var_location(from) else {
                        return true;
                    };
                    let Some(to) = cpg.to_var_location(to) else {
                        return true;
                    };
                    !matches!(cpg.graph[index], CpgEdge::DataFlow(_))
                        || !(from.start_byte >= super_start
                            && from.end_byte <= super_end
                            && to.kind == VarAccessKind::Def
                            && to.function == target.name
                            && to.function_start_line == target.start_line
                            && to.function_start_line != base.start_line)
                }));
            }
            if matches!(id, "x3-5" | "x5-3") {
                for flow in pending {
                    let (from, to, label) = flow.edge;
                    cpg.graph.add_edge(from, to, label);
                }
                if expression == "new Implicit(kept)" {
                    crate::cpg::param_binding_matrix_tests::assert_supplied_site_reaches_use(
                        cpg, id, ext, 2,
                    );
                    continue;
                }
                let param = if id == "x5-3" { "kept" } else { expected[0].0 };
                let root = cpg
                    .graph
                    .node_indices()
                    .find_map(|index| {
                        cpg.to_var_location(index).filter(|loc| {
                            loc.function == "main"
                                && loc.kind == VarAccessKind::Def
                                && loc.path.base == param
                        })
                    })
                    .unwrap();
                let reachable = cpg.dfg_forward_reachable(&root);
                assert_eq!(
                    reachable.iter().any(|loc| loc.function == "constructor"
                        && loc.kind == VarAccessKind::Def
                        && loc.function_start_line == line
                        && loc.path.base == expected[0].1),
                    id == "x3-5",
                    "{id}/{ext}"
                );
            }
        }
    }
}
