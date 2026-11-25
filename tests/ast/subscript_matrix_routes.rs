use petgraph::graph::{DiGraph, NodeIndex};
use prism::cpg::{CpgEdge, CpgNode, FlowConfidence, VarAccess};
use std::collections::{HashSet, VecDeque};

pub(super) type Bytes = [usize; 2];
type Graph = DiGraph<CpgNode, CpgEdge>;

fn at(node: &CpgNode, file: &str, bytes: Bytes, access: VarAccess) -> bool {
    matches!(
        node,
        CpgNode::Variable { file: node_file, access: node_access, start_byte, end_byte, .. }
        if node_file == file && *node_access == access && [*start_byte, *end_byte] == bytes
            && start_byte < end_byte
    )
}

pub(super) fn uses_at(graph: &Graph, file: &str, bytes: Bytes) -> Result<Vec<NodeIndex>, String> {
    let uses: Vec<_> = graph
        .node_indices()
        .filter(|&node| at(&graph[node], file, bytes, VarAccess::Use))
        .collect();
    if uses.is_empty() {
        return Err(format!("missing sink argument Use at {file}:{bytes:?}"));
    }
    Ok(uses)
}

pub(super) fn marked_def(graph: &Graph, file: &str, bytes: Bytes) -> Result<NodeIndex, String> {
    graph
        .node_indices()
        .find(|&node| {
            at(&graph[node], file, bytes, VarAccess::Def)
                && matches!(&graph[node], CpgNode::Variable { path, .. }
                    if path.base == "t" && path.fields.is_empty())
        })
        .ok_or_else(|| format!("missing marked Def(t) at {file}:{bytes:?}"))
}

pub(super) fn route_through(
    graph: &Graph,
    marked: NodeIndex,
    write: Bytes,
    sink_uses: &[NodeIndex],
) -> bool {
    has_route(graph, marked, sink_uses, Some(write), false)
}

pub(super) fn no_exact_route(graph: &Graph, marked: NodeIndex, sink_uses: &[NodeIndex]) -> bool {
    !has_route(graph, marked, sink_uses, None, true)
}

pub(super) fn no_route(graph: &Graph, marked: NodeIndex, sink_uses: &[NodeIndex]) -> bool {
    !has_route(graph, marked, sink_uses, None, false)
}

fn has_route(
    graph: &Graph,
    marked: NodeIndex,
    sink_uses: &[NodeIndex],
    required_write: Option<Bytes>,
    exact_only: bool,
) -> bool {
    use petgraph::visit::EdgeRef;

    let mut todo = VecDeque::from([(marked, required_write.is_none())]);
    let mut visited = HashSet::new();
    while let Some((node, seen_write)) = todo.pop_front() {
        let seen_write = seen_write
            || required_write
                .is_some_and(|bytes| at(&graph[node], graph[marked].file(), bytes, VarAccess::Def));
        if !visited.insert((node, seen_write)) {
            continue;
        }
        if seen_write && sink_uses.contains(&node) {
            return true;
        }

        // The assembled CPG stores the DFG edges and the Step 5b argument edges
        // as DataFlow. Keep their actual node identities and confidence labels.
        todo.extend(
            graph
                .edges(node)
                .filter(|edge| match edge.weight() {
                    CpgEdge::DataFlow(label) => !exact_only || *label == FlowConfidence::Exact,
                    CpgEdge::ReturnInput | CpgEdge::ReturnFlow { .. } => !exact_only,
                    _ => false,
                })
                .map(|edge| (edge.target(), seen_write)),
        );

        if graph[node].is_use() {
            todo.extend(
                graph
                    .node_indices()
                    .filter(|&def| same_line_transfer(&graph[node], &graph[def]))
                    .map(|def| (def, seen_write)),
            );
        }
    }
    false
}

fn same_line_transfer(from: &CpgNode, to: &CpgNode) -> bool {
    matches!(
        (from, to),
        (
            CpgNode::Variable {
                file, function, function_start_line, line, access: VarAccess::Use, ..
            },
            CpgNode::Variable {
                file: def_file, function: def_function, function_start_line: def_owner_start,
                line: def_line, access: VarAccess::Def, ..
            },
        ) if (file, function, function_start_line, line)
            == (def_file, def_function, def_owner_start, def_line)
    )
}

#[cfg(test)]
mod subscript_matrix {
    use super::*;
    use prism::access_path::AccessPath;
    use prism::cpg::FlowDoubt;

    const FILE: &str = "case.js";
    const EXACT: CpgEdge = CpgEdge::DataFlow(FlowConfidence::Exact);
    const NONEXACT: CpgEdge = CpgEdge::DataFlow(FlowConfidence::NameOnly(FlowDoubt::AliasUnstable));

    fn variable(
        graph: &mut Graph,
        name: &str,
        line: usize,
        bytes: Bytes,
        access: VarAccess,
    ) -> NodeIndex {
        graph.add_node(CpgNode::Variable {
            file: FILE.into(),
            function: "caseFn".into(),
            function_start_line: 1,
            line,
            path: AccessPath::from_expr(name),
            access,
            start_byte: bytes[0],
            end_byte: bytes[1],
        })
    }

    #[test]
    fn no_exact_route_rejects_an_indirect_all_exact_route() {
        let mut graph = Graph::new();
        let t = variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        let input = variable(&mut graph, "t", 3, [26, 27], VarAccess::Use);
        let w = variable(&mut graph, "o.x", 3, [20, 23], VarAccess::Def);
        let read = variable(&mut graph, "o.x", 4, [40, 43], VarAccess::Use);
        let local = variable(&mut graph, "tmp", 4, [34, 37], VarAccess::Def);
        let sink = variable(&mut graph, "tmp", 5, [51, 54], VarAccess::Use);
        graph.add_edge(t, input, EXACT);
        graph.add_edge(w, read, EXACT);
        graph.add_edge(local, sink, EXACT);
        assert!(!no_exact_route(&graph, t, &[sink]));
    }

    #[test]
    fn route_through_rejects_a_route_that_skips_w() {
        let mut graph = Graph::new();
        let t = variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        let w = variable(&mut graph, "o.x", 3, [20, 23], VarAccess::Def);
        let sink = variable(&mut graph, "t", 4, [33, 34], VarAccess::Use);
        graph.add_edge(t, sink, EXACT);
        graph.add_edge(t, w, EXACT);
        assert!(!route_through(&graph, t, [20, 23], &[sink]));
        graph.add_edge(w, sink, NONEXACT);
        assert!(route_through(&graph, t, [20, 23], &[sink]));
    }

    #[test]
    fn routes_bind_by_bytes_not_line() {
        let mut graph = Graph::new();
        let t = variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        let other = variable(&mut graph, "o.x", 4, [32, 35], VarAccess::Use);
        let sink = variable(&mut graph, "o.x", 4, [42, 45], VarAccess::Use);
        graph.add_edge(t, other, EXACT);
        let uses = uses_at(&graph, FILE, [42, 45]).unwrap();
        assert_eq!(uses, [sink]);
        assert!(no_route(&graph, t, &uses));
    }

    #[test]
    fn an_anchored_use_does_not_stand_in_for_the_sink_argument() {
        let mut graph = Graph::new();
        let t = variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        let anchor = variable(&mut graph, "o.x", 4, [32, 32], VarAccess::Use);
        graph.add_edge(t, anchor, EXACT);
        assert!(uses_at(&graph, FILE, [36, 39]).is_err());
        let sink = variable(&mut graph, "o.x", 4, [36, 39], VarAccess::Use);
        assert_eq!(uses_at(&graph, FILE, [36, 39]).unwrap(), [sink]);
        assert!(no_route(&graph, t, &[sink]));
    }

    #[test]
    fn no_exact_route_finds_an_alternative_clean_route() {
        let mut graph = Graph::new();
        let t = variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        let detour = variable(&mut graph, "o.x", 3, [20, 23], VarAccess::Def);
        let sink = variable(&mut graph, "tmp", 5, [51, 54], VarAccess::Use);
        graph.add_edge(t, detour, EXACT);
        graph.add_edge(detour, sink, NONEXACT);
        assert!(no_exact_route(&graph, t, &[sink]));
        let clean = variable(&mut graph, "tmp", 4, [35, 38], VarAccess::Def);
        graph.add_edge(t, clean, EXACT);
        graph.add_edge(clean, sink, EXACT);
        assert!(!no_exact_route(&graph, t, &[sink]));
    }

    #[test]
    fn no_route_rejects_a_nonexact_route() {
        let mut graph = Graph::new();
        let t = variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        let sink = variable(&mut graph, "o.x", 4, [32, 35], VarAccess::Use);
        graph.add_edge(t, sink, NONEXACT);
        assert!(no_exact_route(&graph, t, &[sink]));
        assert!(!no_route(&graph, t, &[sink]));
    }

    #[test]
    fn a_missing_sink_use_fails_the_row() {
        let mut graph = Graph::new();
        variable(&mut graph, "t", 2, [10, 11], VarAccess::Def);
        variable(&mut graph, "o.x", 4, [35, 38], VarAccess::Def);
        assert_eq!(
            uses_at(&graph, FILE, [35, 38]).unwrap_err(),
            "missing sink argument Use at case.js:[35, 38]"
        );
    }
}
