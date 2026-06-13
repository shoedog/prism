//! Output shaping helpers for the Tier-2 reasoning layer.

use std::collections::BTreeMap;

use petgraph::graph::NodeIndex;
use petgraph::Direction;

use crate::cpg::{CodePropertyGraph, CpgEdge, CpgNode, Relation, StmtKind, Trace, VarAccess};
use crate::navigation::types::{
    GraphEdge, GraphNode, GraphPayload, Location, Reachability, SymbolRef,
};

/// Best-effort **line-level** reachability of a `(file,line)` sink. NOTE: the CPG has no edges
/// from a `Call` statement to its argument `Variable` nodes, so a line bearing several variables
/// cannot be bound to a specific sink argument. This wrapper is therefore **conservative**
/// (`Reached` only if *every* candidate use on the line is reached) and may report `NotReached`
/// for a line whose actual sink argument *is* tainted. The sound, node-precise contract is
/// [`reachability_for_node`] — Plan B resolves each sink to specific `VarLocation`s and uses it
/// per argument; `reachability_at` is a convenience only.
pub fn reachability_at(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    file: &str,
    line: usize,
) -> Reachability {
    let nodes = sink_nodes_at(cpg, file, line);
    if !nodes.is_empty() && nodes.iter().all(|&n| trace.in_frontier(n)) {
        return Reachability::Reached;
    }

    if nodes
        .iter()
        .any(|&sink| reachability_for_node(cpg, trace, sink) == Reachability::BoundaryExited)
    {
        return Reachability::BoundaryExited;
    }
    Reachability::NotReached
}

pub fn reachability_for_node(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    sink: NodeIndex,
) -> Reachability {
    if trace.in_frontier(sink) {
        return Reachability::Reached;
    }
    // `NotReached` here means "not reached within v1's traced scope" — the seed function plus the
    // first-hop callee boundaries recorded by `taint_trace` — NOT "proven unreachable." v1 does not
    // chase taint transitively through callee chains (`f → g → h`); the full interprocedural chase is
    // Plan B's `taint_reaches`. Consumers must not read `NotReached` as proof of absence. See
    // docs/prism-query-layer/planA-followups.md (Round 4, one-hop boundary closure).
    //
    // BoundaryExited (indeterminate — taint exits to a callee we do not trace into in v1) iff the
    // sink IS a boundary target, OR is forward-reachable from one within the callee's function.
    // Reachability uses `forward_reachable_in_function` — the SAME function-scoped traversal that
    // produced the boundary, including the parameter-binding bridge — so classification cannot
    // disagree with the BFS. The legacy `dfg_forward_reachable` used here previously was not
    // function-scoped: its `(file,line)` same-line propagation leaks across functions sharing a
    // minified line, over-firing `BoundaryExited` for a sink in a different function.
    for b in &trace.boundary {
        if b.to == sink {
            return Reachability::BoundaryExited;
        }
        if cpg.forward_reachable_in_function(b.to).contains(&sink) {
            return Reachability::BoundaryExited;
        }
    }
    Reachability::NotReached
}

fn sink_nodes_at(cpg: &CodePropertyGraph, file: &str, line: usize) -> Vec<NodeIndex> {
    let nodes = cpg.nodes_at(file, line);
    let callees: Vec<String> = nodes
        .iter()
        .filter_map(|&n| match cpg.node(n) {
            CpgNode::Statement {
                kind: StmtKind::Call { callee },
                ..
            } => Some(callee.clone()),
            _ => None,
        })
        .collect();
    let uses: Vec<NodeIndex> = nodes
        .iter()
        .copied()
        .filter(|&n| {
            let CpgNode::Variable {
                access: VarAccess::Use,
                path,
                ..
            } = cpg.node(n)
            else {
                return false;
            };
            // Excludes the callee identifier itself (the CPG has no Call→arg edge, so the callee name
            // appears as a Use on the call line). KNOWN heuristic limitation: a sink argument that
            // *shadows* the callee name (`sink(sink)`) is excluded too, biasing this already-lossy line
            // wrapper further toward `NotReached`. The node-precise API (`reachability_for_node`) is
            // unaffected; Plan B consumes that per-argument and must not inherit this heuristic.
            !callees.iter().any(|callee| callee == &path.base)
        })
        .collect();
    if !uses.is_empty() {
        let dataflow_uses: Vec<NodeIndex> = uses
            .iter()
            .copied()
            .filter(|&n| has_incoming_dataflow(cpg, n))
            .collect();
        if !dataflow_uses.is_empty() {
            return dataflow_uses;
        }
        return uses;
    }
    nodes
        .into_iter()
        .filter(|&n| matches!(cpg.node(n), CpgNode::Variable { .. }))
        .collect()
}

fn has_incoming_dataflow(cpg: &CodePropertyGraph, n: NodeIndex) -> bool {
    cpg.graph
        .edges_directed(n, Direction::Incoming)
        .any(|e| matches!(e.weight(), CpgEdge::DataFlow))
}

/// Witness `GraphPayload` for a reached sink: parent walk-back, relation-named edges, full node
/// identity. NodeIndex space becomes `Location` only here, at the shape boundary.
pub fn witness_graph(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    file: &str,
    line: usize,
) -> Option<GraphPayload> {
    let sinks = sink_nodes_at(cpg, file, line);
    if sinks.is_empty() || !sinks.iter().all(|&n| trace.in_frontier(n)) {
        return None;
    }
    witness_graph_for_node(cpg, trace, sinks[0])
}

pub fn witness_graph_for_node(
    cpg: &CodePropertyGraph,
    trace: &Trace,
    sink: NodeIndex,
) -> Option<GraphPayload> {
    if !trace.in_frontier(sink) {
        return None;
    }
    let mut chain = Vec::new();
    let root = trace
        .frontier_by_root
        .iter()
        .find_map(|(&root, frontier)| frontier.contains(&sink).then_some(root));
    let mut cur = sink;
    chain.push(cur);
    if let Some(root) = root {
        // `parents_by_root` is a per-root tree (first-enqueue-wins in `taint_trace`), so this is
        // acyclic; the `seen` guard bounds the walk defensively against any future parent-slot
        // reassignment rather than relying on a non-local invariant.
        let mut seen = std::collections::BTreeSet::from([cur]);
        while let Some((p, _)) = trace.parents_by_root.get(&(root, cur)) {
            if !seen.insert(*p) {
                break;
            }
            cur = *p;
            chain.push(cur);
        }
    }
    chain.reverse();

    let mut idx_of: BTreeMap<NodeIndex, usize> = BTreeMap::new();
    let mut nodes = Vec::new();
    for &n in &chain {
        idx_of.entry(n).or_insert_with(|| {
            nodes.push(node_of(cpg, n));
            nodes.len() - 1
        });
    }

    let mut edges = Vec::new();
    for w in chain.windows(2) {
        let (from, to) = (w[0], w[1]);
        if from == to {
            continue;
        }
        let relation = root
            .and_then(|r| trace.parents_by_root.get(&(r, to)))
            .map(|(_, rel)| *rel);
        // Exhaustive over `Option<Relation>` (no `_` wildcard): a new `Relation` variant must be a
        // compile error here, not a silent `"DataFlow"` mislabel that freezes into Plan B's wire shape.
        let kind = match relation {
            Some(Relation::DataFlow) | None => "DataFlow",
            Some(Relation::AssignmentPropagation) => "AssignmentPropagation",
            Some(Relation::RecoveredDefUse) => "RecoveredDefUse",
        };
        edges.push(GraphEdge {
            from: idx_of[&from],
            to: idx_of[&to],
            kind: kind.to_string(),
        });
    }
    Some(GraphPayload { nodes, edges })
}

fn node_of(cpg: &CodePropertyGraph, n: NodeIndex) -> GraphNode {
    let loc = cpg.to_var_location(n);
    let (file, line, function, path, access, start_byte, end_byte) = match &loc {
        Some(l) => (
            l.file.clone(),
            l.line,
            l.function.clone(),
            l.path.to_string(),
            match l.kind {
                crate::data_flow::VarAccessKind::Def => "def",
                crate::data_flow::VarAccessKind::Use => "use",
            },
            l.start_byte,
            l.end_byte,
        ),
        None => {
            // Unreachable while witness chains are all Variable nodes (frontier/parents are
            // Variables by construction). Fail loudly in debug so a future non-Variable in a chain
            // surfaces here instead of silently fabricating an empty node into serialized Evidence.
            debug_assert!(
                false,
                "node_of: non-Variable chain node {n:?} has no VarLocation"
            );
            (String::new(), 0, String::new(), String::new(), "use", 0, 0)
        }
    };
    GraphNode {
        symbol: Some(SymbolRef::Variable {
            file: file.clone(),
            function,
            line,
            path,
            access: access.into(),
            start_byte,
            end_byte,
            // RESERVED — occurrence discriminator deferred (S2 section 9), NOT byte rank.
            ordinal: 0,
        }),
        location: Location {
            file,
            start_line: line,
            end_line: line,
            start_byte,
            end_byte,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpg::CodePropertyGraph;
    use crate::navigation::types::Reachability;

    fn build_python_cpg(src: &str) -> CodePropertyGraph {
        let parsed =
            crate::ast::ParsedFile::parse("test.py", src, crate::languages::Language::Python)
                .unwrap();
        let mut files = std::collections::BTreeMap::new();
        files.insert("test.py".to_string(), parsed);
        CodePropertyGraph::build(&files)
    }

    fn var_node(cpg: &CodePropertyGraph, file: &str, line: usize, path: &str) -> NodeIndex {
        cpg.nodes_at(file, line)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n)
                    .is_some_and(|l| l.path.to_string() == path)
            })
            .expect("variable node")
    }

    #[test]
    fn test_loop_carried_field_path_reaches_not_false_negative() {
        // BLOCKER regression (round 9): a loop-carried FIELD-path flow. `data_flow.rs` filters a field
        // path's refs to lines after the def, so the loop-carried `o.data def@5 → use@4 (4<5)` edge
        // never exists (simple vars get it). The field-path recovery arm in `taint_neighbors`
        // re-supplies any-line same-path uses; the back-edge-aware `cfg_valid` keeps the loop-carried
        // one. Otherwise `sink(o.data)` is a false negative on iteration 2+.
        let cpg = build_python_cpg(
            "def f(o):\n    o.data = ''\n    while cond():\n        sink(o.data)\n        o.data = input()\n",
        );
        let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]); // o.data = input()
        let sink_field = cpg
            .nodes_at("test.py", 4)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n).is_some_and(|l| {
                    l.path.to_string() == "o.data"
                        && matches!(l.kind, crate::data_flow::VarAccessKind::Use)
                })
            })
            .expect("o.data use in sink");
        assert_eq!(
            reachability_for_node(&cpg, &trace, sink_field),
            Reachability::Reached,
            "loop-carried field-path flow must not be a false negative"
        );
    }

    #[test]
    fn test_nested_callback_continuation_reaches_not_false_negative() {
        // BLOCKER regression (round 9): a tainted argument after an inline callback in a multi-line
        // call. The continuation walk-back from the `user` line hits the callback's body statement
        // first (not in the outer cfg_set); the reasoning fail-open scan keeps walking to the
        // enclosing `sink(` statement (which IS reachable), so `user` is Reached, not pruned.
        let cpg = js_cpg(
            "function g(){\n  var user = input();\n  var k = 1;\n  k = k + 1;\n  sink(\n    () => {\n      const z = 0;\n    },\n    user\n  );\n}\n",
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 2usize)]);
        let user_arg = use_node(&cpg, 9, "g", "user");
        assert_eq!(
            reachability_for_node(&cpg, &trace, user_arg),
            Reachability::Reached,
            "a tainted arg after an inline callback must not be a false negative"
        );
    }

    #[test]
    fn test_long_multiline_call_argument_reaches_not_false_negative() {
        // BLOCKER regression (round 8): a tainted argument on the 21st+ continuation line of a
        // multi-line `sink(...)` call. The 20-line scan-back cap (kept on the production taint path
        // for byte-stability) would drop the def→use as CFG-unreachable; the reasoning path scans
        // unbounded, so the argument is Reached — an ordinary-code unsafe false negative otherwise.
        let mut src = String::from(
            "function h(){\n  var user = input();\n  var z = 0;\n  z = z + 1;\n  sink(\n",
        );
        for i in 0..24 {
            src.push_str(&format!("    a{i},\n"));
        }
        src.push_str("    user\n  );\n  var w = 2;\n}\n");
        let cpg = js_cpg(&src);
        assert!(
            cpg.has_cfg_edges(),
            "fixture must exercise the CFG-pruning path"
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 2usize)]); // var user = input()
        let user_arg = use_node(&cpg, 30, "h", "user"); // 25 continuation lines past `sink(` on line 5
        assert_eq!(
            reachability_for_node(&cpg, &trace, user_arg),
            Reachability::Reached,
            "a tainted arg far down a multi-line call must not be a false negative"
        );
    }

    #[test]
    fn test_multiline_signature_recursion_records_boundary() {
        // MAJOR-2 pin (round 8): the recursion fix must keep working when the callee has a MULTI-LINE
        // signature (the round-7 tests all used single-line signatures). Guards against a future
        // change to where param Defs are pinned silently reviving the recursion false negative.
        let cpg = js_cpg(
            "function f(\n  flag,\n  u\n){\n  if(flag){ sink(u); return; }\n  var x = input();\n  f(true, x);\n}\n",
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 6usize)]); // var x = input()
        let sink_u = use_node(&cpg, 5, "f", "u");
        assert_eq!(
            reachability_for_node(&cpg, &trace, sink_u),
            Reachability::BoundaryExited,
            "recursion into a multi-line-signature param must record a boundary"
        );
    }

    #[test]
    fn test_recursion_records_param_boundary_not_false_negative() {
        // BLOCKER regression (round 7): a recursive self-call `f(...)` passes tainted data to f's own
        // parameter. The arg→param edge has next_fn == src_fn (both "f"), so the name-keyed boundary
        // check used to miss it; the param's signature line is not CFG-reachable from the call site, so
        // the edge was dropped and `sink(u)` classified NotReached for a path taint truly crosses.
        // Treating a parameter binding as a boundary records it instead → BoundaryExited (the
        // indeterminate state; v1 does not trace into the recursive call).
        let cpg = js_cpg(
            "function f(flag, u){\n  if(flag){ sink(u); return; }\n  var x = input();\n  f(true, x);\n}\n",
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 3usize)]); // var x = input()
        let sink_u = use_node(&cpg, 2, "f", "u");
        assert_eq!(
            reachability_for_node(&cpg, &trace, sink_u),
            Reachability::BoundaryExited,
            "taint into a recursive self-call's parameter must record a boundary, not vanish"
        );
    }

    #[test]
    fn test_nested_named_function_param_seed_not_pruned() {
        // BLOCKER regression (round 7): a named nested function's parameter seed must not inherit the
        // enclosing function's CFG scope. Seeding `inner`'s signature line degrades to pure taint
        // (function-signature seed line), so `p → q → sink(q)` inside `inner` is reached, not pruned.
        let cpg = js_cpg(
            "function outer() { return function inner(p) {\n  var q = p;\n  sink(q); }; }\n",
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 1usize)]); // inner(p) signature line
        assert!(
            trace.degraded,
            "a function-signature seed line degrades to pure taint"
        );
        let q_sink = use_node(&cpg, 3, "inner", "q");
        assert_eq!(
            reachability_for_node(&cpg, &trace, q_sink),
            Reachability::Reached,
            "nested function body flow must not be CFG-pruned by the encloser's scope"
        );
    }

    #[test]
    fn test_shared_line_multifunction_degrades_not_false_negative() {
        // BLOCKER regression (round 6): a minified line hosting two functions, only one of which
        // contributes a statement node. `b`'s param root would inherit `a`'s per-line CFG scope (which
        // doesn't cover b's body) and CFG-prune `p → q → sink(q)` into a false NotReached. The
        // multi-function degrade falls back to pure taint (safe), so the intra-`b` flow is reached.
        let src = "int a(){ int z=0; bar(z); foo(z); } int b(int p){\n  int q=p;\n  sink(q); }\n";
        let parsed =
            crate::ast::ParsedFile::parse("m.c", src, crate::languages::Language::C).unwrap();
        let mut files = std::collections::BTreeMap::new();
        files.insert("m.c".to_string(), parsed);
        let cpg = CodePropertyGraph::build(&files);
        assert!(
            cpg.has_cfg_edges(),
            "fixture must exercise the CFG-pruning path"
        );
        let trace = cpg.taint_trace(&[("m.c".to_string(), 1usize)]); // shared line hosts a and b
        let q_sink = cpg
            .nodes_at("m.c", 3)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n).is_some_and(|l| {
                    l.path.to_string() == "q"
                        && l.function == "b"
                        && matches!(l.kind, crate::data_flow::VarAccessKind::Use)
                })
            })
            .expect("q use in sink(q)");
        assert_eq!(
            reachability_for_node(&cpg, &trace, q_sink),
            Reachability::Reached,
            "intra-function flow in b must not be CFG-pruned by a's scope on a shared line"
        );
    }

    #[test]
    fn test_reachability_tri_state() {
        // lines: 1 `def g(p)`, 2 `sink(p)`, 4 `def f()`, 5 `w = 1`, 6 `u = input()`, 7 `v = u`, 8 `g(u)`
        let src =
            "def g(p):\n    sink(p)\n\ndef f():\n    w = 1\n    u = input()\n    v = u\n    g(u)\n";
        let cpg = build_python_cpg(src);
        let trace = cpg.taint_trace(&[("test.py".to_string(), 6usize)]); // u = input()
        assert_eq!(
            reachability_at(&cpg, &trace, "test.py", 7), // v = u — tainted intraprocedurally
            Reachability::Reached
        );
        assert_eq!(
            reachability_at(&cpg, &trace, "test.py", 2), // sink(p) inside g — taint exits to a callee
            Reachability::BoundaryExited
        );
        assert_eq!(
            reachability_at(&cpg, &trace, "test.py", 5), // w = 1 — genuinely unrelated
            Reachability::NotReached
        );
    }

    #[test]
    fn test_reachability_multi_variable_line_is_node_precise() {
        let src =
            "def f():\n    clean = 'x'\n    tainted = input()\n    sink(clean); other = tainted\n";
        let cpg = build_python_cpg(src);
        let trace = cpg.taint_trace(&[("test.py".to_string(), 3usize)]);
        let clean = var_node(&cpg, "test.py", 4, "clean");
        let tainted = var_node(&cpg, "test.py", 4, "tainted");

        assert_eq!(
            reachability_for_node(&cpg, &trace, clean),
            Reachability::NotReached
        );
        assert_eq!(
            reachability_for_node(&cpg, &trace, tainted),
            Reachability::Reached
        );
        assert_eq!(
            reachability_at(&cpg, &trace, "test.py", 4),
            Reachability::NotReached,
            "line-level reachability must not let an unrelated reached variable prove the clean sink"
        );
        assert!(witness_graph_for_node(&cpg, &trace, clean).is_none());
    }

    #[test]
    fn test_boundary_classification_uses_node_identity() {
        // MAJOR 5: boundary classification keys on NODE identity, not line arithmetic. A sink use
        // inside a callee that tainted data flows into is BoundaryExited via the sound node API
        // (the lossy line wrapper is best-effort by design and not asserted here).
        let src = "def g(p):\n    sink(p)\n\ndef f():\n    u = input()\n    g(u)\n";
        let cpg = build_python_cpg(src);
        let trace = cpg.taint_trace(&[("test.py".to_string(), 5usize)]);
        let p_use = cpg
            .nodes_at("test.py", 2)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n).is_some_and(|l| {
                    l.path.to_string() == "p"
                        && l.function == "g"
                        && matches!(l.kind, crate::data_flow::VarAccessKind::Use)
                })
            })
            .expect("p use inside g on the one-liner");
        assert_eq!(
            reachability_for_node(&cpg, &trace, p_use),
            Reachability::BoundaryExited
        );
    }

    fn js_cpg(src: &str) -> CodePropertyGraph {
        let parsed =
            crate::ast::ParsedFile::parse("t.js", src, crate::languages::Language::JavaScript)
                .unwrap();
        let mut files = std::collections::BTreeMap::new();
        files.insert("t.js".to_string(), parsed);
        CodePropertyGraph::build(&files)
    }

    fn use_node(cpg: &CodePropertyGraph, line: usize, func: &str, path: &str) -> NodeIndex {
        cpg.nodes_at("t.js", line)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n).is_some_and(|l| {
                    l.path.to_string() == path
                        && l.function == func
                        && matches!(l.kind, crate::data_flow::VarAccessKind::Use)
                })
            })
            .unwrap_or_else(|| panic!("{path} use in {func} on line {line}"))
    }

    #[test]
    fn test_boundary_classification_same_signature_line_body_use() {
        // BLOCKER regression (round 4): a callee whose FIRST body statement shares its signature line.
        // The function is multi-line (so the param `p` Def IS registered, via the line-2 use) and a
        // boundary to `p Def@1` IS recorded — but `data_flow.rs` drops the def→use edge to the
        // same-line `sink(p)` use (`ref_line == start`). The param-binding bridge must still classify
        // that sink BoundaryExited; otherwise it is an unsafe false negative for a path taint crosses.
        let cpg = js_cpg(
            "function g(p) { sink(p);\n  log(p); }\nfunction f() {\n  var u = input();\n  g(u);\n}\n",
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 4usize)]); // var u = input()
        let p_sink_use = use_node(&cpg, 1, "g", "p"); // the sink(p) argument on the signature line
        assert_eq!(
            reachability_for_node(&cpg, &trace, p_sink_use),
            Reachability::BoundaryExited,
            "taint into a same-signature-line param use must not be a false negative"
        );
    }

    #[test]
    fn test_same_line_def_then_use_reaches_intra_function() {
        // BLOCKER regression (round 5, main BFS): `var y = u; sink(y)` on ONE line. `data_flow.rs`
        // drops the y def→use edge (`ref_line == def_line`), so without the uniform Def→same-line-Use
        // arm in `taint_neighbors` the sink use of y is never in the frontier — an unsafe false
        // negative in the node-precise contract Plan B consumes per-argument.
        let cpg = js_cpg("function h(u) {\n  var y = u; sink(y); }\n");
        let trace = cpg.taint_trace(&[("t.js".to_string(), 1usize)]); // param u
        let y_sink_use = use_node(&cpg, 2, "h", "y");
        assert_eq!(
            reachability_for_node(&cpg, &trace, y_sink_use),
            Reachability::Reached,
            "same-line `var y = u; sink(y)` must reach the sink use of y"
        );
    }

    #[test]
    fn test_boundary_classification_same_line_def_use_one_hop_deeper() {
        // BLOCKER regression (round 5, classification, one hop past the param): the param flows
        // through a same-line local def-then-use (`var q = p; sink(q)` on a body line). The bridge
        // being start-only missed this; the uniform Def→same-line-Use arm recovers q's dropped
        // def→use edge so the sink classifies BoundaryExited.
        let cpg = js_cpg(
            "function g(p) {\n  var q = p; sink(q); }\nfunction f() {\n  var u = input();\n  g(u);\n}\n",
        );
        let trace = cpg.taint_trace(&[("t.js".to_string(), 4usize)]);
        let q_sink_use = use_node(&cpg, 2, "g", "q");
        assert_eq!(
            reachability_for_node(&cpg, &trace, q_sink_use),
            Reachability::BoundaryExited,
            "taint through a same-line local def-then-use inside a callee must not be a false negative"
        );
    }

    #[test]
    fn test_witness_graph_relation_named_edges() {
        let cpg = build_python_cpg("def f():\n    a = input()\n    b = a\n    sink(b)\n");
        let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
        let g = witness_graph(&cpg, &trace, "test.py", 4).expect("reached sink has witness");
        assert!(
            g.edges.iter().all(|e| e.kind == "DataFlow"
                || e.kind == "AssignmentPropagation"
                || e.kind == "RecoveredDefUse"),
            "{:?}",
            g.edges
        );
    }

    #[test]
    fn test_witness_node_carries_occurrence_byte_and_reserved_ordinal() {
        let cpg = build_python_cpg("def f(p):\n    q = p\n    return q\n");
        let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
        let q_return = cpg
            .nodes_at("test.py", 3)
            .into_iter()
            .find(|&n| {
                cpg.to_var_location(n).is_some_and(|l| {
                    l.path.to_string() == "q"
                        && matches!(l.kind, crate::data_flow::VarAccessKind::Use)
                })
            })
            .expect("q use in return");
        let payload = witness_graph_for_node(&cpg, &trace, q_return).expect("witness graph");
        let n = &payload.nodes[0];
        match n.symbol.as_ref().unwrap() {
            SymbolRef::Variable {
                start_byte,
                end_byte,
                ordinal,
                ..
            } => {
                assert!(end_byte >= start_byte);
                assert_eq!(*ordinal, 0, "reserved");
                assert_eq!(
                    (*start_byte, *end_byte),
                    (n.location.start_byte, n.location.end_byte)
                );
            }
            _ => panic!("variable symbol"),
        }
        assert!(n.location.end_byte >= n.location.start_byte);
    }

    #[test]
    fn test_witness_graph_call_line_ignores_callee_identifier() {
        let cpg = build_python_cpg("def f():\n    a = input()\n    sink(a)\n");
        let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
        let g = witness_graph(&cpg, &trace, "test.py", 3)
            .expect("reached argument should produce a witness even if the callee is not tainted");
        assert!(g.nodes.iter().any(|n| {
            matches!(
                &n.symbol,
                Some(SymbolRef::Variable { path, .. }) if path == "a"
            )
        }));
    }

    #[test]
    fn test_witness_graph_unreached_sink_is_none() {
        let cpg = build_python_cpg("def f():\n    a = input()\n    b = 1\n    sink(b)\n");
        let trace = cpg.taint_trace(&[("test.py".to_string(), 2usize)]);
        assert!(witness_graph(&cpg, &trace, "test.py", 4).is_none());
    }
}
