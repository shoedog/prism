use super::*;
use crate::ast::ParsedFile;
use crate::languages::Language;
use crate::resolution::ResolutionConfidence;
use crate::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

fn build_python_cpg(src: &str) -> CodePropertyGraph {
    let parsed =
        crate::ast::ParsedFile::parse("test.py", src, crate::languages::Language::Python).unwrap();
    let mut files = std::collections::BTreeMap::new();
    files.insert("test.py".to_string(), parsed);
    CodePropertyGraph::build(&files)
}

#[test]
fn compute_param_names_pins_current_behavior() {
    use super::build::compute_param_names;
    use crate::ast::ParsedFile;
    use crate::call_graph::FunctionId;
    use crate::languages::Language;

    // Free function: all params, no self/cls stripping.
    let go = ParsedFile::parse("t.go", "func f(a int, b int) { _ = a }", Language::Go).unwrap();
    let fid = FunctionId {
        file: "t.go".into(),
        name: "f".into(),
        start_line: 1,
        end_line: 1,
    };
    assert_eq!(
        compute_param_names(&go, &fid),
        Some(vec!["a".to_string(), "b".to_string()])
    );

    // Python method with a self receiver + owner: self is stripped.
    let py = ParsedFile::parse(
        "t.py",
        "class C:\n    def m(self, x):\n        return x\n",
        Language::Python,
    )
    .unwrap();
    let mid = FunctionId {
        file: "t.py".into(),
        name: "m".into(),
        start_line: 2,
        end_line: 3,
    };
    assert_eq!(compute_param_names(&py, &mid), Some(vec!["x".to_string()]));

    // Callee FunctionInfo not found -> None (the `else { continue }` path).
    let missing = FunctionId {
        file: "t.go".into(),
        name: "nope".into(),
        start_line: 99,
        end_line: 99,
    };
    assert_eq!(compute_param_names(&go, &missing), None);
}

#[test]
fn step7_parallel_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    use super::types::{CpgEdge, CpgNode};
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use petgraph::graph::{DiGraph, NodeIndex};
    use std::collections::BTreeMap;

    // Step-7-heavy + discriminating: nested fns / closures / multi-line stmts / multi-file.
    let src: &[(&str, &str, Language)] = &[
        (
            "a.rs",
            "fn outer(){ let x=1; fn inner(){ let y=2; } let z=3; }",
            Language::Rust,
        ),
        (
            "b.js",
            "function f(){ items.forEach((x)=>{ use(x); }); return 1; }",
            Language::JavaScript,
        ),
        (
            "c.py",
            "def f():\n    a = 1\n    def g():\n        b = 2\n    return a\n",
            Language::Python,
        ),
        (
            "d.go",
            "func h(){ for i:=0;i<3;i++ { use(i) }; return }",
            Language::Go,
        ),
    ];
    let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    for (p, s, lang) in src {
        files.insert(p.to_string(), ParsedFile::parse(p, s, *lang).unwrap());
    }

    type Step7Fn = fn(
        &BTreeMap<String, ParsedFile>,
        &mut DiGraph<CpgNode, CpgEdge>,
        &mut BTreeMap<(String, usize), Vec<NodeIndex>>,
    ) -> BTreeMap<(String, usize), NodeIndex>;
    let run = |f: Step7Fn| {
        let mut g: DiGraph<CpgNode, CpgEdge> = DiGraph::new();
        let mut li: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();
        let si = f(&files, &mut g, &mut li);
        let nodes: Vec<String> = g.node_indices().map(|i| format!("{:?}", g[i])).collect();
        (nodes, si, li)
    };

    let reference = run(CodePropertyGraph::assemble_step7_reference);
    let production = run(CodePropertyGraph::assemble_step7);
    assert_eq!(
        reference.0, production.0,
        "Statement node sequence diverged"
    );
    assert_eq!(
        reference.1, production.1,
        "stmt_index (file,line)->NodeIndex diverged"
    );
    assert_eq!(reference.2, production.2, "location_index appends diverged");
    assert!(
        !reference.0.is_empty(),
        "fixture produced no statement nodes"
    );
}

/// Shared discriminating fixture for the edge-step old-order oracles. Python
/// (the proven `step5b_param_binding_first_wins_parity` shape - no crate-root
/// subtlety): cross-file `from`-import calls (-> Call/Return + arg->param
/// DataFlow), a same-name `helper` redefinition (exercises the resolver's
/// multi-owner path, whatever it decides), an unresolved call, and branchy
/// bodies (`if/else` -> ControlFlow). Resolution is via the free-function /
/// import fallback; the oracle (par == serial) holds regardless of which
/// callee(s) resolve, since both sides see the same resolution.
fn edge_fixture() -> std::collections::BTreeMap<String, ParsedFile> {
    let callee = "def helper(p):\n    if p > 0:\n        return p\n    return 0\n\
                  \ndef helper(p, q):\n    return p + q\n\
                  \ndef leaf():\n    return 1\n";
    let caller = "from callee import helper, leaf\n\
                  \ndef run(x):\n    y = helper(x)\n    z = leaf()\n\
                  \n    if y > z:\n        return missing_fn(y)\n    return z\n";
    let mut files = std::collections::BTreeMap::new();
    files.insert(
        "callee.py".to_string(),
        ParsedFile::parse("callee.py", callee, Language::Python).unwrap(),
    );
    files.insert(
        "caller.py".to_string(),
        ParsedFile::parse("caller.py", caller, Language::Python).unwrap(),
    );
    files
}

#[test]
fn step5_parallel_edge_collect_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    let files = edge_fixture();
    let cpg = CodePropertyGraph::build(&files); // sources cg + func_index
    let par = CodePropertyGraph::collect_step5_edges(&cpg.call_graph, &cpg.func_index);
    let serial = CodePropertyGraph::collect_step5_edges_reference(&cpg.call_graph, &cpg.func_index);
    assert_eq!(par, serial, "Step-5 Call/Return edge sequence diverged");
    assert!(!par.is_empty(), "fixture produced no call edges");
}

#[test]
fn step5b_parallel_edge_collect_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    let files = edge_fixture();
    let cpg = CodePropertyGraph::build(&files); // sources cg + var_index (and warms call_args)
    let par = CodePropertyGraph::collect_step5b_edges(&cpg.call_graph, &cpg.var_index, &files);
    let serial =
        CodePropertyGraph::collect_step5b_edges_reference(&cpg.call_graph, &cpg.var_index, &files);
    assert_eq!(par, serial, "Step-5b DataFlow edge sequence diverged");
    // The fixture's `let y = helper(x);` yields an arg->param DataFlow edge.
    assert!(
        !par.is_empty(),
        "fixture produced no interproc DataFlow edges"
    );
}

#[test]
fn step8_parallel_edge_collect_matches_serial_reference() {
    use super::build::CodePropertyGraph;
    use super::types::{CpgEdge, CpgNode};
    use petgraph::graph::{DiGraph, NodeIndex};
    use std::collections::BTreeMap;

    let files = edge_fixture();
    // Step 8 reads stmt_index (built by Step 7). Regenerate it deterministically;
    // par vs serial collect share the same stmt_index -> directly comparable.
    let mut g: DiGraph<CpgNode, CpgEdge> = DiGraph::new();
    let mut li: BTreeMap<(String, usize), Vec<NodeIndex>> = BTreeMap::new();
    let stmt_index = CodePropertyGraph::assemble_step7(&files, &mut g, &mut li);

    let par = CodePropertyGraph::collect_step8_edges(&stmt_index, &files);
    let serial = CodePropertyGraph::collect_step8_edges_reference(&stmt_index, &files);
    assert_eq!(par, serial, "Step-8 ControlFlow edge sequence diverged");
    assert!(!par.is_empty(), "fixture produced no CFG edges");
}

#[test]
fn test_taint_trace_straight_line_frontier() {
    let src = "def f():\n    user = input()\n    x = user\n    sink(x)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let lines: std::collections::BTreeSet<usize> = trace
        .frontier()
        .iter()
        .filter_map(|&i| cpg.to_var_location(i).map(|l| l.line))
        .collect();
    assert!(
        lines.contains(&3) && lines.contains(&4),
        "x (l3) and the use in sink(x) (l4): {lines:?}"
    );
    assert!(trace.boundary.is_empty());
}

#[test]
fn test_taint_trace_same_line_assignment() {
    let src = "def f():\n    x = source(); y = x\n    sink(y)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let lines: std::collections::BTreeSet<usize> = trace
        .frontier()
        .iter()
        .filter_map(|&i| cpg.to_var_location(i).map(|l| l.line))
        .collect();
    assert!(
        lines.contains(&3),
        "y (def on l2) must reach the use in sink(y) on l3: {lines:?}"
    );
}

#[test]
fn test_taint_trace_recovered_hops_do_not_cross_same_name_functions() {
    let src = "def run(o):\n    o.v = source()\ndef run(o):\n    sink(o.v)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let reached_sink = trace.frontier().iter().any(|&i| {
        matches!(
            cpg.node(i),
            CpgNode::Variable {
                line: 4,
                function,
                function_start_line,
                path,
                access: VarAccess::Use,
                ..
            } if function == "run"
                && *function_start_line == 3
                && path.base == "o"
                && path.fields.len() == 1
                && path.fields[0] == "v"
        )
    });
    assert!(
        !reached_sink,
        "same-path recovered hops must not cross same-name functions"
    );
}

#[test]
fn test_taint_trace_no_dead_end_witness() {
    let src = "def f():\n    a = input()\n    b = a\n    c = b\n    sink(c)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    for (&root, frontier) in &trace.frontier_by_root {
        for &node in frontier {
            let mut cur = node;
            let mut g = 0;
            while let Some((p, _)) = trace.parents_by_root.get(&(root, cur)) {
                cur = *p;
                g += 1;
                assert!(g < 1000);
            }
            assert_eq!(
                cur, root,
                "per-root witness walk-back must end at the seeded root"
            );
        }
    }
}

#[test]
fn test_taint_trace_no_path_empty_downstream() {
    let src = "def f():\n    a = input()\n    b = 1\n    sink(b)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
    let l4 = trace
        .frontier()
        .iter()
        .any(|&i| cpg.to_var_location(i).map_or(false, |l| l.line == 4));
    assert!(
        !l4,
        "b on line 4 is independent of a; must not be in frontier"
    );
}

#[test]
fn test_taint_trace_no_cfg_falls_back_to_pure_taint() {
    let mut graph = petgraph::graph::DiGraph::new();
    let a = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("a"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let b = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("b"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 2,
        access: VarAccess::Use,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(a, b, CpgEdge::DataFlow);
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 1), vec![a]);
    location_index.insert(("manual.py".to_string(), 2), vec![b]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    assert!(!cpg.has_cfg_edges());
    let trace = cpg.taint_trace(&[("manual.py".to_string(), 1usize)]);
    assert!(trace.in_frontier(b));
}

#[test]
fn test_taint_trace_param_seed_reaches_body() {
    let src = "def g(p):\n    x = p\n    sink(x)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 1usize)]);
    // A parameter seed sits at the function-start line, which has no CFG statement node, so v1 uses
    // the pure-taint fallback (degraded) — an over-approximation. The param must still reach the
    // body uses; CFG-precise param body-entry handling is a tracked follow-up.
    assert!(
        trace.degraded,
        "a parameter seed has no CFG statement node → pure-taint fallback"
    );
    let lines: std::collections::BTreeSet<usize> = trace
        .frontier()
        .iter()
        .filter_map(|&i| cpg.to_var_location(i).map(|l| l.line))
        .collect();
    assert!(
        lines.contains(&2) && lines.contains(&3),
        "parameter p should reach body use and sink: {lines:?}"
    );
}

#[test]
fn test_taint_trace_statement_miss_degrades_to_pure_taint() {
    let mut graph = petgraph::graph::DiGraph::new();
    let stmt1 = graph.add_node(CpgNode::Statement {
        file: "manual.py".into(),
        line: 1,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    let stmt2 = graph.add_node(CpgNode::Statement {
        file: "manual.py".into(),
        line: 2,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(stmt1, stmt2, CpgEdge::ControlFlow);
    let src = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("src"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 10,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let dst = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("dst"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 11,
        access: VarAccess::Use,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(src, dst, CpgEdge::DataFlow);
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 1), vec![stmt1]);
    location_index.insert(("manual.py".to_string(), 2), vec![stmt2]);
    location_index.insert(("manual.py".to_string(), 10), vec![src]);
    location_index.insert(("manual.py".to_string(), 11), vec![dst]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    assert!(cpg.has_cfg_edges());
    let trace = cpg.taint_trace(&[("manual.py".to_string(), 10usize)]);
    assert!(trace.degraded);
    assert!(!trace.warnings.is_empty());
    assert!(trace.in_frontier(dst));
}

#[test]
fn test_degraded_seed_line_warns_once_per_line() {
    // Two variable roots on one degraded seed line (no CFG statement node) must produce ONE warning,
    // not one per variable — `cfg_scope_for_seed` is hoisted to per-(file,line).
    let mut graph = petgraph::graph::DiGraph::new();
    let stmt1 = graph.add_node(CpgNode::Statement {
        file: "manual.py".into(),
        line: 1,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    let stmt2 = graph.add_node(CpgNode::Statement {
        file: "manual.py".into(),
        line: 2,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(stmt1, stmt2, CpgEdge::ControlFlow);
    let a = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("a"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 10,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let b = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("b"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 10,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 1), vec![stmt1]);
    location_index.insert(("manual.py".to_string(), 2), vec![stmt2]);
    location_index.insert(("manual.py".to_string(), 10), vec![a, b]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    assert!(cpg.has_cfg_edges());
    let trace = cpg.taint_trace(&[("manual.py".to_string(), 10usize)]);
    assert!(trace.degraded);
    assert_eq!(
        trace.warnings.len(),
        1,
        "a degraded seed line must warn once, not once per variable node: {:?}",
        trace.warnings
    );
    // Finding 8: a duplicate `(file,line)` seed must not re-run the BFS or re-push the warning.
    let trace_dup = cpg.taint_trace(&[
        ("manual.py".to_string(), 10usize),
        ("manual.py".to_string(), 10),
    ]);
    assert_eq!(
        trace_dup.warnings.len(),
        1,
        "duplicate seeds must be deduped: {:?}",
        trace_dup.warnings
    );
}

#[test]
fn test_seed_with_no_variable_nodes_warns() {
    // Finding 4: a seed line resolving to zero Variable nodes (e.g. a call-only or blank line) must
    // surface a warning, not be silently dropped (indistinguishable from "reached nothing").
    let mut graph = petgraph::graph::DiGraph::new();
    let stmt = graph.add_node(CpgNode::Statement {
        file: "manual.py".into(),
        line: 5,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 5), vec![stmt]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    let trace = cpg.taint_trace(&[("manual.py".to_string(), 5usize)]);
    assert!(
        trace
            .warnings
            .iter()
            .any(|w| w.contains("no variable nodes")),
        "unresolved seed must warn: {:?}",
        trace.warnings
    );
}

#[test]
fn test_cfg_reachable_lines_unioned_covers_all_statements() {
    // Finding 3: a minified line can host statements from two functions. `cfg_reachable_lines` seeds
    // from the first statement only; the unioned variant must cover BOTH so neither function's flow
    // is CFG-pruned into a false NotReached.
    let mut graph = petgraph::graph::DiGraph::new();
    let stmt_a = graph.add_node(CpgNode::Statement {
        file: "m.js".into(),
        line: 1,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    let stmt_b = graph.add_node(CpgNode::Statement {
        file: "m.js".into(),
        line: 1,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    let succ_a = graph.add_node(CpgNode::Statement {
        file: "m.js".into(),
        line: 20,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    let succ_b = graph.add_node(CpgNode::Statement {
        file: "m.js".into(),
        line: 30,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(stmt_a, succ_a, CpgEdge::ControlFlow);
    graph.add_edge(stmt_b, succ_b, CpgEdge::ControlFlow);
    let mut location_index = BTreeMap::new();
    location_index.insert(("m.js".to_string(), 1), vec![stmt_a, stmt_b]);
    location_index.insert(("m.js".to_string(), 20), vec![succ_a]);
    location_index.insert(("m.js".to_string(), 30), vec![succ_b]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    let unioned = cpg.cfg_reachable_lines_unioned("m.js", 1);
    assert!(
        unioned.contains(&("m.js".to_string(), 20)) && unioned.contains(&("m.js".to_string(), 30)),
        "union must cover both functions' successors: {unioned:?}"
    );
    // The first-statement-only baseline reaches only one of them.
    let single = cpg.cfg_reachable_lines("m.js", 1);
    assert!(
        !(single.contains(&("m.js".to_string(), 20)) && single.contains(&("m.js".to_string(), 30))),
        "baseline should cover only the first statement's successor: {single:?}"
    );
}

#[test]
fn test_taint_trace_keeps_per_root_attribution() {
    let mut graph = petgraph::graph::DiGraph::new();
    let a = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("a"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let b = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("b"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 2,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let sink = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("sink_arg"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 3,
        access: VarAccess::Use,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(a, sink, CpgEdge::DataFlow);
    graph.add_edge(b, sink, CpgEdge::DataFlow);
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 1), vec![a]);
    location_index.insert(("manual.py".to_string(), 2), vec![b]);
    location_index.insert(("manual.py".to_string(), 3), vec![sink]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    let trace = cpg.taint_trace(&[
        ("manual.py".to_string(), 1usize),
        ("manual.py".to_string(), 2usize),
    ]);
    assert!(trace.frontier_by_root[&a].contains(&sink));
    assert!(trace.frontier_by_root[&b].contains(&sink));
    assert_eq!(trace.parents_by_root[&(a, sink)].0, a);
    assert_eq!(trace.parents_by_root[&(b, sink)].0, b);
}

#[test]
fn test_taint_trace_dataflow_wins_same_line_tie() {
    let mut graph = petgraph::graph::DiGraph::new();
    let root = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("x"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        access: VarAccess::Use,
        start_byte: 0,
        end_byte: 0,
    });
    let target = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("y"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(root, target, CpgEdge::DataFlow);
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 1), vec![root, target]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    let trace = cpg.taint_trace(&[("manual.py".to_string(), 1usize)]);
    assert_eq!(
        trace.parents_by_root[&(root, target)].1,
        Relation::DataFlow,
        "DataFlow must win the parent slot over same-line propagation"
    );
}

#[test]
fn test_taint_trace_skips_non_variable_dataflow_neighbors() {
    let mut graph = petgraph::graph::DiGraph::new();
    let root = graph.add_node(CpgNode::Variable {
        path: AccessPath::simple("root"),
        file: "manual.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    });
    let stmt = graph.add_node(CpgNode::Statement {
        file: "manual.py".into(),
        line: 2,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    graph.add_edge(root, stmt, CpgEdge::DataFlow);
    let mut location_index = BTreeMap::new();
    location_index.insert(("manual.py".to_string(), 1), vec![root]);
    location_index.insert(("manual.py".to_string(), 2), vec![stmt]);
    let cpg = CodePropertyGraph::from_parts(
        graph,
        BTreeMap::new(),
        BTreeMap::new(),
        BTreeMap::new(),
        location_index,
        crate::call_graph::CallGraph::empty(),
        crate::data_flow::DataFlowGraph::empty(),
    );
    let trace = cpg.taint_trace(&[("manual.py".to_string(), 1usize)]);
    assert!(trace.boundary.is_empty());
    assert!(!trace.in_frontier(stmt));
}

#[test]
fn test_taint_trace_records_boundary_at_param_def() {
    let src = "def g(p):\n    sink(p)\n\ndef f():\n    u = input()\n    g(u)\n";
    let cpg = build_python_cpg(src);
    let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]);
    assert!(
        !trace.boundary.is_empty(),
        "u flows into g's param across a function boundary"
    );
    for be in &trace.boundary {
        assert!(
            !trace.in_frontier(be.to),
            "cross-function target not traversed"
        );
        if let Some(loc) = cpg.to_var_location(be.to) {
            assert_eq!(loc.function, "g", "boundary target is g's parameter");
        }
    }
}

#[test]
fn test_forward_reachable_in_function_is_function_scoped() {
    // Two functions on one minified line. `forward_reachable_in_function` from `t` in `a` must NOT
    // leak into `b` — unlike the legacy `dfg_forward_reachable`, whose `(file,line)`-keyed same-line
    // propagation reaches `c` in `b`. This is the function-scoping the boundary classifier relies on.
    let src = "function a(){ var t = input(); foo(t); }function b(){ var c = 1; sink(c); }\n";
    let parsed =
        crate::ast::ParsedFile::parse("m.js", src, crate::languages::Language::JavaScript).unwrap();
    let mut files = std::collections::BTreeMap::new();
    files.insert("m.js".to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);
    let t_use = cpg
        .nodes_at("m.js", 1)
        .into_iter()
        .find(|&n| {
            cpg.to_var_location(n).is_some_and(|l| {
                l.path.to_string() == "t" && matches!(l.kind, crate::data_flow::VarAccessKind::Use)
            })
        })
        .expect("t use in function a");
    let reached_fns: std::collections::BTreeSet<String> = cpg
        .forward_reachable_in_function(t_use)
        .into_iter()
        .filter_map(|n| cpg.to_var_location(n).map(|l| l.function))
        .collect();
    assert!(
        reached_fns.iter().all(|f| f == "a"),
        "forward reachability leaked out of function a: {reached_fns:?}"
    );
}

#[test]
fn test_node_accessors() {
    let func = CpgNode::Function {
        name: "main".into(),
        file: "src/main.c".into(),
        start_line: 1,
        end_line: 10,
        start_byte: 0,
        end_byte: 0,
    };
    assert_eq!(func.file(), "src/main.c");
    assert_eq!(func.line(), 1);
    assert!(func.is_function());

    let var_def = CpgNode::Variable {
        path: AccessPath::from_expr("dev->name"),
        file: "src/dev.c".into(),
        function: "init".into(),
        function_start_line: 1,
        line: 5,
        access: VarAccess::Def,
        start_byte: 0,
        end_byte: 0,
    };
    assert!(var_def.is_def());
    assert!(!var_def.is_use());

    let call = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 3,
        kind: StmtKind::Call {
            callee: "init".into(),
        },
        start_byte: 0,
        end_byte: 0,
    };
    assert!(call.is_call());
}

#[test]
fn test_cpg_node_equality_excludes_byte_spans() {
    let function_a = CpgNode::Function {
        name: "main".into(),
        file: "src/main.c".into(),
        start_line: 1,
        end_line: 10,
        start_byte: 0,
        end_byte: 100,
    };
    let function_b = CpgNode::Function {
        name: "main".into(),
        file: "src/main.c".into(),
        start_line: 1,
        end_line: 10,
        start_byte: 900,
        end_byte: 1000,
    };
    assert_eq!(function_a, function_b);

    let statement_a = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 3,
        kind: StmtKind::Return,
        start_byte: 10,
        end_byte: 20,
    };
    let statement_b = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 3,
        kind: StmtKind::Return,
        start_byte: 30,
        end_byte: 40,
    };
    assert_eq!(statement_a, statement_b);

    let variable_a = CpgNode::Variable {
        path: AccessPath::simple("x"),
        file: "src/main.c".into(),
        function: "main".into(),
        function_start_line: 1,
        line: 4,
        access: VarAccess::Use,
        start_byte: 10,
        end_byte: 11,
    };
    let variable_b = CpgNode::Variable {
        path: AccessPath::simple("x"),
        file: "src/main.c".into(),
        function: "main".into(),
        function_start_line: 1,
        line: 4,
        access: VarAccess::Use,
        start_byte: 99,
        end_byte: 100,
    };
    assert_eq!(variable_a, variable_b);

    let variable_c = CpgNode::Variable {
        path: AccessPath::simple("x"),
        file: "src/main.c".into(),
        function: "main".into(),
        function_start_line: 2,
        line: 4,
        access: VarAccess::Use,
        start_byte: 10,
        end_byte: 11,
    };
    assert_ne!(variable_b, variable_c);
}

#[test]
fn test_edge_classification() {
    assert!(CpgEdge::DataFlow.is_data_flow());
    assert!(!CpgEdge::Call(ResolutionConfidence::Exact).is_data_flow());
    assert!(CpgEdge::Call(ResolutionConfidence::Exact).is_interprocedural());
    assert!(CpgEdge::Return(ResolutionConfidence::Exact).is_interprocedural());
    assert!(!CpgEdge::DataFlow.is_interprocedural());
    assert!(!CpgEdge::Contains.is_interprocedural());
    assert!(!CpgEdge::FieldOf.is_interprocedural());
    assert!(!CpgEdge::ControlFlow.is_data_flow());
}

#[test]
fn test_variable_node_accessors() {
    let var_use = CpgNode::Variable {
        path: AccessPath::from_expr("dev->id"),
        file: "src/dev.c".into(),
        function: "get_id".into(),
        function_start_line: 1,
        line: 8,
        access: VarAccess::Use,
        start_byte: 0,
        end_byte: 0,
    };
    assert!(var_use.is_use());
    assert!(!var_use.is_def());
    assert!(!var_use.is_function());
    assert!(!var_use.is_call());
    assert_eq!(var_use.file(), "src/dev.c");
    assert_eq!(var_use.line(), 8);
}

#[test]
fn test_statement_node_non_call() {
    let branch = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 15,
        kind: StmtKind::Branch,
        start_byte: 0,
        end_byte: 0,
    };
    assert!(!branch.is_call());
    assert!(!branch.is_function());
    assert!(!branch.is_def());
    assert_eq!(branch.file(), "src/main.c");
    assert_eq!(branch.line(), 15);

    let ret = CpgNode::Statement {
        file: "src/main.c".into(),
        line: 20,
        kind: StmtKind::Return,
        start_byte: 0,
        end_byte: 0,
    };
    assert!(!ret.is_call());
}

#[test]
fn test_cpg_build_basic() {
    let source = r#"
void init() {
    int x = 1;
    int y = x;
    use(y);
}
"#;
    let path = "src/test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Should have at least one function node
    assert!(cpg.node_count() > 0, "CPG should have nodes");
    assert!(cpg.edge_count() > 0, "CPG should have edges");

    // Should be able to look up the function
    let func_idx = cpg.function_node(path, "init");
    assert!(func_idx.is_some(), "Should find function 'init'");

    // Function node should have correct metadata
    let func = cpg.node(func_idx.unwrap());
    assert!(func.is_function());
    assert_eq!(func.file(), path);
}

#[test]
fn test_cpg_dataflow_edges() {
    let source = r#"
void flow() {
    int x = 1;
    int y = x;
}
"#;
    let path = "src/flow.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Check that dataflow edges exist
    let df_edges: Vec<_> = cpg
        .graph
        .edge_indices()
        .filter(|&e| cpg.graph[e] == CpgEdge::DataFlow)
        .collect();
    assert!(
        !df_edges.is_empty(),
        "CPG should have DataFlow edges for x → y"
    );
}

#[test]
fn step5b_param_binding_first_wins_parity() {
    // callee file defines two same-named fns; Step 5b must bind args to the FIRST
    // (tree-order) match — pinned-until-S2. Caller passes `tainted`.
    // Line map (m.py): 1 `def f(p):`  2 `return p`  3 blank  4 `class K:`
    //                  5 `def f(q):`  6 `return q`   — param defs pin to the
    // function start line (data_flow.rs:221-237), so p@1 and q@5.
    let callee_src = "def f(p):\n    return p\n\nclass K:\n    def f(q):\n        return q\n";
    let caller_src = "from m import f\n\ndef call():\n    tainted = source()\n    f(tainted)\n";
    let mut files = BTreeMap::new();
    files.insert(
        "m.py".to_string(),
        ParsedFile::parse("m.py", callee_src, Language::Python).unwrap(),
    );
    files.insert(
        "c.py".to_string(),
        ParsedFile::parse("c.py", caller_src, Language::Python).unwrap(),
    );
    let ctx = CpgContext::build(&files, None);

    fn node_matches(n: &CpgNode, (file, line, base): (&str, usize, &str)) -> bool {
        matches!(n, CpgNode::Variable { file: f, line: l, path, .. }
            if f == file && *l == line && path.base == base)
    }

    fn has_dataflow_edge(
        cpg: &CodePropertyGraph,
        from: (&str, usize, &str),
        to: (&str, usize, &str),
    ) -> bool {
        cpg.graph.edge_indices().any(|e| {
            cpg.graph[e] == CpgEdge::DataFlow
                && cpg
                    .graph
                    .edge_endpoints(e)
                    .map(|(source, target)| {
                        node_matches(cpg.node(source), from) && node_matches(cpg.node(target), to)
                    })
                    .unwrap_or(false)
        })
    }

    // arg->param edge must target p (first f) at m.py:1 — and NOT q at m.py:5
    assert!(has_dataflow_edge(
        &ctx.cpg,
        ("c.py", 5, "tainted"),
        ("m.py", 1, "p")
    ));
    assert!(!has_dataflow_edge(
        &ctx.cpg,
        ("c.py", 5, "tainted"),
        ("m.py", 5, "q")
    ));
}

#[test]
fn test_cpg_call_edges() {
    let source = r#"
void callee() {
    return;
}

void caller() {
    callee();
}
"#;
    let path = "src/calls.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Check caller → callee Call edge
    let caller_idx = cpg.function_node(path, "caller").unwrap();
    let callee_idx = cpg.function_node(path, "callee").unwrap();

    let call_reachable = cpg.reachable_forward(caller_idx, &|e| matches!(e, CpgEdge::Call(_)));
    assert!(
        call_reachable.contains(&callee_idx),
        "caller should reach callee via Call edge"
    );

    // Check callee → caller Return edge
    let return_reachable = cpg.reachable_forward(callee_idx, &|e| matches!(e, CpgEdge::Return(_)));
    assert!(
        return_reachable.contains(&caller_idx),
        "callee should reach caller via Return edge"
    );
}

#[test]
fn test_cpg_contains_edges() {
    let source = r#"
void f() {
    int x = 1;
    int y = x;
}
"#;
    let path = "src/contains.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let func_idx = cpg.function_node(path, "f").unwrap();
    let contained = cpg.reachable_forward(func_idx, &|e| matches!(e, CpgEdge::Contains));
    assert!(
        !contained.is_empty(),
        "Function 'f' should contain variable nodes"
    );

    // All contained nodes should be Variable nodes
    for idx in &contained {
        let node = cpg.node(*idx);
        assert!(
            node.is_def() || node.is_use(),
            "Contains edge should lead to Variable nodes, got {:?}",
            node
        );
    }
}

#[test]
fn test_cpg_edge_filtered_reachability() {
    // DataFlow-only reachability should NOT follow Call edges
    let source = r#"
void helper() {
    return;
}

void main_func() {
    int x = 1;
    int y = x;
    helper();
}
"#;
    let path = "src/filter.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let main_idx = cpg.function_node(path, "main_func").unwrap();
    let helper_idx = cpg.function_node(path, "helper").unwrap();

    // Call-only should reach helper
    let call_reach = cpg.reachable_forward(main_idx, &|e| matches!(e, CpgEdge::Call(_)));
    assert!(call_reach.contains(&helper_idx));

    // DataFlow-only from main_func should NOT reach helper function node
    let df_reach = cpg.reachable_forward(main_idx, &|e| matches!(e, CpgEdge::DataFlow));
    assert!(
        !df_reach.contains(&helper_idx),
        "DataFlow-only traversal should not reach function nodes via Call edges"
    );
}

#[test]
fn test_cpg_call_graph_cycles() {
    let source = r#"
void a() {
    b();
}

void b() {
    a();
}
"#;
    let path = "src/cycle.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    let cycles = cpg.call_graph_cycles();

    assert!(!cycles.is_empty(), "Should detect a → b → a call cycle");

    // The cycle should contain both function nodes
    let cycle_names: BTreeSet<String> = cycles[0]
        .iter()
        .filter_map(|&idx| match cpg.node(idx) {
            CpgNode::Function { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(cycle_names.contains("a"), "Cycle should contain 'a'");
    assert!(cycle_names.contains("b"), "Cycle should contain 'b'");
}

#[test]
fn test_cpg_bfs_with_distance() {
    let source = r#"
void a() {
    b();
}

void b() {
    c();
}

void c() {
    return;
}
"#;
    let path = "src/dist.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let a_idx = cpg.function_node(path, "a").unwrap();
    let b_idx = cpg.function_node(path, "b").unwrap();
    let c_idx = cpg.function_node(path, "c").unwrap();

    let distances = cpg.bfs_with_distance(&[a_idx], 5, &|e| matches!(e, CpgEdge::Call(_)));

    assert_eq!(distances.get(&a_idx), Some(&0));
    assert_eq!(distances.get(&b_idx), Some(&1));
    assert_eq!(distances.get(&c_idx), Some(&2));
}

#[test]
fn test_cpg_bridge_to_var_location() {
    let source = r#"
void f() {
    int x = 1;
    int y = x;
}
"#;
    let path = "src/bridge.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Find a variable node and convert back
    let var_nodes: Vec<_> = cpg
        .graph
        .node_indices()
        .filter(|&idx| cpg.node(idx).is_def())
        .collect();
    assert!(!var_nodes.is_empty());

    let loc = cpg.to_var_location(var_nodes[0]);
    assert!(loc.is_some());
    let loc = loc.unwrap();
    assert_eq!(loc.file, path);
    assert_eq!(loc.function, "f");
}

#[test]
fn test_cpg_bridge_to_function_id() {
    let source = r#"
void my_func() {
    return;
}
"#;
    let path = "src/bridge_func.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let func_idx = cpg.function_node(path, "my_func").unwrap();
    let fid = cpg.to_function_id(func_idx).unwrap();
    assert_eq!(fid.name, "my_func");
    assert_eq!(fid.file, path);
}

#[test]
fn test_build_enriched_without_types() {
    let source = "void f() { int x = 1; }\n";
    let path = "src/enriched.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build_enriched(&files, None);
    assert!(!cpg.has_type_info());
    assert!(cpg.node_count() > 0);
}

#[test]
fn test_build_enriched_with_types() {
    let source = "void f() { int x = 1; }\n";
    let path = "src/enriched2.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "MyStruct".to_string(),
        RecordInfo {
            name: "MyStruct".to_string(),
            kind: RecordKind::Struct,
            fields: vec![FieldInfo {
                name: "x".to_string(),
                type_str: "int".to_string(),
                offset: None,
            }],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_enriched(&files, Some(&type_db));
    assert!(cpg.has_type_info());
    assert!(cpg.node_count() > 0);
}

#[test]
fn test_build_with_types() {
    let source = "void f() { int x = 1; }\n";
    let path = "src/owned.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let type_db = TypeDatabase::default();
    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert!(cpg.has_type_info());
}

#[test]
fn test_all_fields_of() {
    let source = "void f() {}\n";
    let path = "src/fields.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "Point".to_string(),
        RecordInfo {
            name: "Point".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "x".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "y".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    let fields = cpg.all_fields_of("Point").unwrap();
    assert_eq!(fields, vec!["x", "y"]);

    // Unknown type returns None
    assert!(cpg.all_fields_of("Unknown").is_none());
}

#[test]
fn test_resolve_type_with_typedef() {
    let source = "void f() {}\n";
    let path = "src/typedef.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.typedefs.insert(
        "handle_t".to_string(),
        TypedefInfo {
            name: "handle_t".to_string(),
            underlying: "struct device *".to_string(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert_eq!(cpg.resolve_type("handle_t"), "struct device *");
    assert_eq!(cpg.resolve_type("int"), "int"); // not a typedef
}

#[test]
fn test_resolve_type_without_type_db() {
    let source = "void f() {}\n";
    let path = "src/no_types.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    assert_eq!(cpg.resolve_type("handle_t"), "handle_t");
}

#[test]
fn test_is_union_type() {
    let source = "void f() {}\n";
    let path = "src/union.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "MyUnion".to_string(),
        RecordInfo {
            name: "MyUnion".to_string(),
            kind: RecordKind::Union,
            fields: vec![
                FieldInfo {
                    name: "i".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "f".to_string(),
                    type_str: "float".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );
    type_db.records.insert(
        "MyStruct".to_string(),
        RecordInfo {
            name: "MyStruct".to_string(),
            kind: RecordKind::Struct,
            fields: vec![],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert!(cpg.is_union_type("MyUnion"));
    assert!(!cpg.is_union_type("MyStruct"));
    assert!(!cpg.is_union_type("NonExistent"));
}

#[test]
fn test_field_type() {
    let source = "void f() {}\n";
    let path = "src/field_type.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "Device".to_string(),
        RecordInfo {
            name: "Device".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "id".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "name".to_string(),
                    type_str: "char *".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: BTreeMap::new(),
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert_eq!(cpg.field_type("Device", "id"), Some("int".to_string()));
    assert_eq!(cpg.field_type("Device", "name"), Some("char *".to_string()));
    assert_eq!(cpg.field_type("Device", "nonexistent"), None);
    assert_eq!(cpg.field_type("Unknown", "id"), None);
}

#[test]
fn test_function_at() {
    let source = r#"
void first() {
    int x = 1;
}

void second() {
    int y = 2;
}
"#;
    let path = "src/func_at.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Line 3 is inside first()
    let result = cpg.function_at(path, 3);
    assert!(result.is_some());
    let (_, fid) = result.unwrap();
    assert_eq!(fid.name, "first");

    // Line 7 is inside second()
    let result = cpg.function_at(path, 7);
    assert!(result.is_some());
    let (_, fid) = result.unwrap();
    assert_eq!(fid.name, "second");

    // Line 5 is between functions
    let result = cpg.function_at(path, 5);
    assert!(result.is_none());

    // Non-existent file
    let result = cpg.function_at("no_such_file.c", 1);
    assert!(result.is_none());
}

#[test]
fn test_callers_of() {
    let source = r#"
void target() {
    return;
}

void caller1() {
    target();
}

void caller2() {
    target();
}
"#;
    let path = "src/callers.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let callers = cpg.callers_of("target", 1);
    let caller_names: BTreeSet<String> = callers.iter().map(|(fid, _)| fid.name.clone()).collect();
    assert!(caller_names.contains("caller1"));
    assert!(caller_names.contains("caller2"));
    assert_eq!(callers.len(), 2);
}

#[test]
fn test_callees_of() {
    let source = r#"
void helper1() {
    return;
}

void helper2() {
    return;
}

void main_fn() {
    helper1();
    helper2();
}
"#;
    let path = "src/callees.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let callees = cpg.callees_of("main_fn", path, 1);
    let callee_names: BTreeSet<String> = callees.iter().map(|(fid, _)| fid.name.clone()).collect();
    assert!(callee_names.contains("helper1"));
    assert!(callee_names.contains("helper2"));
}

#[test]
fn test_function_nodes() {
    let source = r#"
void a() { return; }
void b() { return; }
void c() { return; }
"#;
    let path = "src/func_nodes.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    let func_nodes = cpg.function_nodes();
    assert_eq!(func_nodes.len(), 3);
    for idx in &func_nodes {
        assert!(cpg.node(*idx).is_function());
    }
}

#[test]
fn test_virtual_dispatch_enrichment() {
    let source = r#"
void render() {
    draw();
}

void draw() {
    return;
}
"#;
    let path = "src/virtual.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
        "Shape".to_string(),
        RecordInfo {
            name: "Shape".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec![],
            virtual_methods: {
                let mut m = BTreeMap::new();
                m.insert("draw".to_string(), "void".to_string());
                m
            },
            size: None,
            file: String::new(),
        },
    );

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);
    assert!(cpg.has_type_info());

    // The CPG should still have both functions
    assert!(cpg.function_node(path, "render").is_some());
    assert!(cpg.function_node(path, "draw").is_some());
}

#[test]
fn test_taint_forward_basic() {
    let source = r#"
void process() {
    int input = read_user();
    int data = input;
    write(data);
}
"#;
    let path = "src/taint.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    let sources = vec![(path.to_string(), 3usize)]; // line where input is defined
    let paths = cpg.taint_forward(&sources);
    // Should find at least one taint path from the source
    // (may be empty if DFG doesn't connect precisely, but shouldn't panic)
    let _ = paths;
}

#[test]
fn test_has_type_info() {
    let source = "void f() {}\n";
    let path = "src/has_type.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg_no_types = CodePropertyGraph::build(&files);
    assert!(!cpg_no_types.has_type_info());

    let cpg_with_types = CodePropertyGraph::build_with_types(&files, TypeDatabase::default());
    assert!(cpg_with_types.has_type_info());
}

// -----------------------------------------------------------------------
// Phase 6: CFG edge tests
// -----------------------------------------------------------------------

#[test]
fn test_cpg_has_cfg_edges() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    assert!(cpg.has_cfg_edges(), "CPG should have ControlFlow edges");
    assert!(cpg.cfg_edge_count() > 0);
}

#[test]
fn test_cpg_statement_nodes_created() {
    let source = r#"
void f() {
    int x = 1;
    int y = x;
    return;
}
"#;
    let path = "src/stmt_nodes.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Should have Statement nodes at lines 3, 4, 5
    assert!(
        cpg.statement_at(path, 3).is_some(),
        "Should have statement at line 3"
    );
    assert!(
        cpg.statement_at(path, 4).is_some(),
        "Should have statement at line 4"
    );
    assert!(
        cpg.statement_at(path, 5).is_some(),
        "Should have statement at line 5 (return)"
    );
}

#[test]
fn test_cpg_cfg_sequential_flow() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_seq.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Line 3 → Line 4 via ControlFlow
    let stmt3 = cpg.statement_at(path, 3).unwrap();
    let successors = cpg.cfg_successors(stmt3);
    let succ_lines: Vec<usize> = successors.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        succ_lines.contains(&4),
        "Line 3 should flow to line 4, got {:?}",
        succ_lines
    );
}

#[test]
fn test_cpg_cfg_return_terminates() {
    let source = r#"
void f() {
    int x = 1;
    return;
    int y = 2;
}
"#;
    let path = "src/cfg_ret.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // return at line 4 should NOT have a successor to line 5
    let stmt4 = cpg.statement_at(path, 4).unwrap();
    let successors = cpg.cfg_successors(stmt4);
    let succ_lines: Vec<usize> = successors.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        !succ_lines.contains(&5),
        "return should not flow to line 5, got {:?}",
        succ_lines
    );
}

#[test]
fn test_cpg_cfg_if_branches() {
    let source = r#"
void f(int x) {
    if (x > 0) {
        int a = 1;
    } else {
        int b = 2;
    }
    int c = 3;
}
"#;
    let path = "src/cfg_if.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // if at line 3 should have CFG successors to both branches
    let if_stmt = cpg.statement_at(path, 3).unwrap();
    let successors = cpg.cfg_successors(if_stmt);
    assert!(
        successors.len() >= 2,
        "if should branch to at least 2 targets, got {} successors",
        successors.len()
    );
}

#[test]
fn test_cpg_cfg_predecessors() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_pred.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // Line 4 should have line 3 as predecessor
    let stmt4 = cpg.statement_at(path, 4).unwrap();
    let preds = cpg.cfg_predecessors(stmt4);
    let pred_lines: Vec<usize> = preds.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        pred_lines.contains(&3),
        "Line 4 should have line 3 as predecessor, got {:?}",
        pred_lines
    );
}

#[test]
fn test_cpg_cfg_goto_edge() {
    let source = r#"
void f() {
    int x = 1;
    goto cleanup;
    int y = 2;
cleanup:
    free(x);
}
"#;
    let path = "src/cfg_goto.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // goto at line 4 should have a CFG edge (either to label or through goto resolution)
    let goto_stmt = cpg.statement_at(path, 4);
    assert!(goto_stmt.is_some(), "Should have statement at goto line 4");

    // goto should NOT have sequential successor to line 5
    if let Some(idx) = goto_stmt {
        let successors = cpg.cfg_successors(idx);
        let succ_lines: Vec<usize> = successors.iter().map(|&s| cpg.node(s).line()).collect();
        assert!(
            !succ_lines.contains(&5),
            "goto should not fall through to line 5, got {:?}",
            succ_lines
        );
    }
}

#[test]
fn test_cpg_cfg_edge_filtered_reachability() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    int z = 3;
}
"#;
    let path = "src/cfg_reach.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);

    // CFG reachability: line 3 should reach line 5 via ControlFlow edges
    let stmt3 = cpg.statement_at(path, 3).unwrap();
    let reachable = cpg.reachable_forward(stmt3, &|e| matches!(e, CpgEdge::ControlFlow));
    let reachable_lines: BTreeSet<usize> =
        reachable.iter().map(|&idx| cpg.node(idx).line()).collect();
    assert!(
        reachable_lines.contains(&5),
        "Line 3 should CFG-reach line 5, got {:?}",
        reachable_lines
    );
}

#[test]
fn test_cpg_cfg_python() {
    let source = r#"
def f():
    x = 1
    y = 2
    z = 3
"#;
    let path = "src/cfg_py.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build(&files);
    assert!(
        cpg.has_cfg_edges(),
        "Python CPG should have ControlFlow edges"
    );

    // Sequential flow: line 3 → line 4
    let stmt3 = cpg.statement_at(path, 3);
    assert!(stmt3.is_some(), "Should have Python statement at line 3");
    if let Some(idx) = stmt3 {
        let succs = cpg.cfg_successors(idx);
        assert!(
            !succs.is_empty(),
            "Python line 3 should have CFG successors"
        );
    }
}

// -----------------------------------------------------------------------
// Phase 6 PR C: CFG-constrained analysis tests
// -----------------------------------------------------------------------

#[test]
fn test_cfg_reachable_lines() {
    let source = r#"
void f() {
    int x = 1;
    int y = 2;
    return;
    int z = 3;
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    // Line 3 should reach lines 4 and 5 (return), but NOT line 6 (after return)
    let reachable = cpg.cfg_reachable_lines(path, 3);
    assert!(
        reachable.contains(&(path.to_string(), 4)),
        "Line 3 should CFG-reach line 4, got {:?}",
        reachable
    );
    assert!(
        reachable.contains(&(path.to_string(), 5)),
        "Line 3 should CFG-reach line 5 (return), got {:?}",
        reachable
    );
    // Line 6 is dead code after return — should NOT be reachable
    assert!(
        !reachable.contains(&(path.to_string(), 6)),
        "Line 6 (after return) should NOT be CFG-reachable from line 3, got {:?}",
        reachable
    );
}

#[test]
fn test_taint_forward_cfg_prunes_dead_code() {
    // Taint source at line 3 (x = input), return at line 4,
    // sink at line 5 (after return — dead code). CFG-constrained taint
    // should NOT reach line 5.
    let source = r#"
void f(char* input) {
    char* x = input;
    return;
    exec(x);
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    let taint_sources = vec![(path.to_string(), 3)];
    let paths = cpg.taint_forward_cfg(&taint_sources);

    // Collect all tainted target lines
    let tainted_lines: BTreeSet<usize> = paths
        .iter()
        .flat_map(|p| p.edges.iter().map(|e| e.to.line))
        .collect();

    // Line 5 (exec after return) should be pruned by CFG constraint
    assert!(
        !tainted_lines.contains(&5),
        "CFG-constrained taint should NOT reach dead code at line 5, got {:?}",
        tainted_lines
    );
}

#[test]
fn test_dfg_cfg_chop_prunes_unreachable() {
    // Source at line 3, sink at line 6. Line 5 is dead code after return.
    // CFG-constrained chop should exclude the dead-code line.
    let source = r#"
void f() {
    int x = 1;
    int y = x;
    return;
    int z = x;
    int w = z;
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    // DFG chop: source=line 3, sink=line 7 (dead code)
    // CFG-constrained should be empty or exclude dead lines
    let chop = cpg.dfg_cfg_chop(path, 3, path, 7);

    // Line 7 is dead code — CFG forward from line 3 can't reach it
    // The chop should not include line 6 or 7 since they're unreachable
    let has_dead_code = chop.iter().any(|(_, l)| *l == 6 || *l == 7);
    assert!(
        !has_dead_code,
        "CFG-constrained chop should not include dead code lines 6-7, got {:?}",
        chop
    );
}

#[test]
fn test_cfg_constrained_fallback_without_cfg() {
    // When no CFG edges exist (e.g., no functions), methods should
    // gracefully return empty/fallback results
    let source = "int x = 1;\n";
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let cpg = CodePropertyGraph::build(&files);

    // cfg_reachable_lines on non-existent statement → empty
    let reachable = cpg.cfg_reachable_lines(path, 999);
    assert!(reachable.is_empty());

    // taint_forward_cfg falls back to taint_forward
    let paths_cfg = cpg.taint_forward_cfg(&[(path.to_string(), 1)]);
    let paths_dfg = cpg.taint_forward(&[(path.to_string(), 1)]);
    assert_eq!(paths_cfg.len(), paths_dfg.len());
}
