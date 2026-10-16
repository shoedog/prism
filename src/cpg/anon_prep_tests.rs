use super::*;
use crate::anon_owner::{identity, is_anonymous_owner_name, owner_key};
use crate::languages::Language;

#[test]
fn injected_identity_drives_step1_bytes_and_step6_contains() {
    let source = "function outer(){ sink((p) => p + 1, (q) => q + 2, (x,x) => x); }";
    let files = BTreeMap::from([(
        "a.js".into(),
        ParsedFile::parse("a.js", source, Language::JavaScript).unwrap(),
    )]);
    let parsed = &files["a.js"];
    let cg = CallGraph::build(&files);
    let mut expected = BTreeMap::new();
    for node in parsed.all_functions() {
        let Some(id) = identity(parsed, &node, "a.js") else {
            continue;
        };
        if !is_anonymous_owner_name(&id.name) {
            continue;
        }
        expected.insert(
            id.name.clone(),
            (
                node.start_byte(),
                node.end_byte(),
                parsed.dfg_owner_name(&node).is_some(),
            ),
        );
    }
    assert_eq!(expected.len(), 3);
    let cpg = CodePropertyGraph::assemble_graph(cg, DataFlowGraph::build(&files), &files, None);
    for (name, (start, end, dfg)) in expected {
        let idx = cpg
            .graph
            .node_indices()
            .find(|&i| matches!(&cpg.graph[i],CpgNode::Function {name:n,..} if *n==name))
            .unwrap();
        assert!(
            matches!(&cpg.graph[idx], CpgNode::Function {start_byte,end_byte,..} if (*start_byte,*end_byte)==(start,end))
        );
        let rows: Vec<_> = cpg
            .graph
            .edges(idx)
            .filter(|e| *e.weight() == CpgEdge::Contains)
            .collect();
        assert_eq!(!rows.is_empty(), dfg, "{name}");
        assert!(rows.iter().all(
            |e| matches!(&cpg.graph[e.target()],CpgNode::Variable {function,..} if *function==name)
        ));
    }
}

#[test]
fn d12_refuses_registered_anonymous_sources_only() {
    for (source, new_source) in [
        ("sink(x => x);", true),
        ("sink((...x) => x);", true),
        ("sink((x) => x);", false),
    ] {
        let files = BTreeMap::from([(
            "a.js".into(),
            ParsedFile::parse("a.js", source, Language::JavaScript).unwrap(),
        )]);
        let p = &files["a.js"];
        let callable = p.all_functions()[0];
        let id = identity(p, &callable, "a.js").unwrap();
        let (_, start, end) = p.function_parameter_occurrences(&callable)[0].clone();
        for registered in [false, true] {
            for reaches_seed in [false, true] {
                let mut graph = DiGraph::new();
                let def = graph.add_node(CpgNode::Variable {
                    path: AccessPath::simple("x"),
                    file: "a.js".into(),
                    function: id.name.clone(),
                    function_start_line: 1,
                    line: 1,
                    access: VarAccess::Def,
                    start_byte: start,
                    end_byte: end,
                });
                let sink = graph.add_node(CpgNode::Statement {
                    file: "b.js".into(),
                    line: 1,
                    kind: StmtKind::Call {
                        callee: "sink".into(),
                    },
                    start_byte: 0,
                    end_byte: 1,
                });
                graph.add_edge(def, sink, CpgEdge::DataFlow(FlowConfidence::Exact));
                let owners = if registered {
                    BTreeMap::from([(owner_key("a.js", &id.name, 1), def)])
                } else {
                    BTreeMap::new()
                };
                let seeds = if reaches_seed { vec![sink] } else { Vec::new() };
                CodePropertyGraph::refuse_new_sources_exposing_free_single_calls(
                    &mut graph,
                    &files,
                    seeds,
                    &mut DfgLabelStats::default(),
                    &owners,
                );
                assert_eq!(
                    graph.edge_count(),
                    usize::from(!(new_source && registered && reaches_seed)),
                    "{source}/{registered}/{reaches_seed}"
                );
            }
        }
    }
}
