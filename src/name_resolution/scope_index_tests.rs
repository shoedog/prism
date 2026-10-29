use super::*;
use crate::name_resolution::rust_policy::{EK_GLOB, NS_MACRO, NS_TYPE, NS_VALUE, VIS_PUB};
use crate::name_resolution::types::*;

pub(super) fn add_entries(graph: &mut ScopeGraph) {
    // Interleaved scopes, kinds/namespaces and files; exact duplicates retain
    // distinct global indices. Edge.order deliberately differs from Vec order.
    for scope in [2, 0, 1, 0, 2, 1] {
        for ns in [NS_MACRO, NS_VALUE, NS_TYPE] {
            for file in [1, 0] {
                let range = Span {
                    lo: SourceLoc {
                        file: FileId(file),
                        byte: 10,
                    },
                    hi: SourceLoc {
                        file: FileId(file),
                        byte: 50,
                    },
                };
                let wildcard = MacroWildcard {
                    scope: ScopeId(scope),
                    ns,
                    range: range.clone(),
                };
                graph.add_macro_wildcard(wildcard.clone());
                graph.add_macro_wildcard(wildcard);
                for kind in [99, EK_GLOB] {
                    let edge = Edge {
                        from: ScopeId(scope),
                        kind,
                        to: if ns == NS_TYPE {
                            BindTarget::Pending(RawPath(vec!["other".into()]), Anchor::default())
                        } else {
                            BindTarget::Resolved(Target::Scope(ScopeId(2)))
                        },
                        vis: Vis {
                            kind: VIS_PUB,
                            restrict: None,
                            payload: Default::default(),
                        },
                        cond: Some(CfgCond::Atom("feature".into(), None)),
                        order: 100 - scope,
                        vis_range: (file == 0).then_some(range.clone()),
                    };
                    graph.add_edge(edge.clone());
                    graph.add_edge(edge);
                }
            }
        }
    }
}

fn fixture() -> ScopeGraph {
    let mut graph = super::tests::fixture();
    add_entries(&mut graph);
    graph
}

pub(super) fn check_equivalence(graph: &ScopeGraph, edges_indexed: bool, wildcards_indexed: bool) {
    for scope in graph.scopes.keys().copied().chain([ScopeId(999)]) {
        let edges = graph.edge_indices(scope);
        assert_eq!(matches!(&edges, ScopeIndices::Indexed(_)), edges_indexed);
        let expected: Vec<_> = graph
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.from == scope)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(edges.collect::<Vec<_>>(), expected);
        for ns in [NS_TYPE, NS_VALUE, NS_MACRO, 99] {
            let wildcards = graph.macro_wildcard_indices(scope, ns);
            assert_eq!(
                matches!(&wildcards, ScopeIndices::Indexed(_)),
                wildcards_indexed
            );
            let expected: Vec<_> = graph
                .macro_wildcards
                .iter()
                .enumerate()
                .filter(|(_, m)| m.scope == scope && m.ns == ns)
                .map(|(i, _)| i)
                .collect();
            assert_eq!(wildcards.collect::<Vec<_>>(), expected);
            // File/range and kind filters must see exactly the original policy
            // inputs, including half-open boundaries, absent ranges and misses.
            for file in [FileId(0), FileId(1), FileId(999)] {
                for byte in [0, 9, 10, 49, 50, 99, 100] {
                    let covers =
                        |s: &Span| s.lo.file == file && byte >= s.lo.byte && byte < s.hi.byte;
                    let expected = graph
                        .macro_wildcards
                        .iter()
                        .any(|m| m.scope == scope && m.ns == ns && covers(&m.range));
                    assert_eq!(
                        graph
                            .macro_wildcard_indices(scope, ns)
                            .any(|i| covers(&graph.macro_wildcards[i].range)),
                        expected
                    );
                    for kind in [EK_GLOB, 99, 100] {
                        let filter =
                            |e: &Edge| e.kind == kind && e.vis_range.as_ref().is_none_or(covers);
                        let expected: Vec<_> = graph
                            .edges
                            .iter()
                            .enumerate()
                            .filter(|(_, e)| e.from == scope && filter(e))
                            .map(|(i, _)| i)
                            .collect();
                        assert_eq!(
                            graph
                                .edge_indices(scope)
                                .filter(|&i| filter(&graph.edges[i]))
                                .collect::<Vec<_>>(),
                            expected
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn indexed_edges_preserve_global_indices_and_policy_inputs() {
    let graph = fixture();
    check_equivalence(&graph, true, true);
    assert_eq!(
        graph.binding_index.1.get().unwrap().len(),
        graph.edges.len()
    );
    let found: Vec<_> = graph.edge_indices(ScopeId(0)).collect();
    assert_eq!(found[0], 24);
    assert!(found.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn indexed_wildcards_preserve_duplicates_namespaces_and_policy_inputs() {
    let graph = fixture();
    check_equivalence(&graph, true, true);
    assert_eq!(
        graph.binding_index.2.get().unwrap().len(),
        graph.macro_wildcards.len()
    );
    assert_eq!(
        graph
            .macro_wildcard_indices(ScopeId(0), NS_MACRO)
            .take(2)
            .collect::<Vec<_>>(),
        [12, 13]
    );
}

#[test]
fn edge_append_invalidates_and_direct_push_falls_back() {
    let mut graph = fixture();
    check_equivalence(&graph, true, true);
    let edge = graph.edges[0].clone();
    graph.add_edge(edge.clone());
    assert!(graph.binding_index.1.get().is_none());
    assert!(graph.binding_index.2.get().is_some());
    check_equivalence(&graph, true, true);
    assert_eq!(
        graph.edge_indices(edge.from).last(),
        Some(graph.edges.len() - 1)
    );
    graph.edges.push(edge.clone());
    check_equivalence(&graph, false, true);
    assert_eq!(
        graph.edge_indices(edge.from).last(),
        Some(graph.edges.len() - 1)
    );
    graph.add_edge(edge);
    check_equivalence(&graph, true, true);
}

#[test]
fn wildcard_append_invalidates_and_direct_push_falls_back() {
    let mut graph = fixture();
    check_equivalence(&graph, true, true);
    let wildcard = graph.macro_wildcards[0].clone();
    graph.add_macro_wildcard(wildcard.clone());
    assert!(graph.binding_index.1.get().is_some());
    assert!(graph.binding_index.2.get().is_none());
    check_equivalence(&graph, true, true);
    assert_eq!(
        graph
            .macro_wildcard_indices(wildcard.scope, wildcard.ns)
            .last(),
        Some(graph.macro_wildcards.len() - 1)
    );
    graph.macro_wildcards.push(wildcard.clone());
    check_equivalence(&graph, true, false);
    assert_eq!(
        graph
            .macro_wildcard_indices(wildcard.scope, wildcard.ns)
            .last(),
        Some(graph.macro_wildcards.len() - 1)
    );
    graph.add_macro_wildcard(wildcard);
    check_equivalence(&graph, true, true);
}

#[test]
fn empty_graph_and_first_appends_after_lookup() {
    let mut graph = ScopeGraph::new();
    check_equivalence(&graph, true, true);
    add_entries(&mut graph);
    graph.add_scope(super::tests::fixture().scopes[&ScopeId(0)].clone());
    check_equivalence(&graph, true, true);
}
