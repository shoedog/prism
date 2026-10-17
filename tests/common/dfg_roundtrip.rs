//! Ordinary cache/renderer observations; no migration backend or stored corpus pins.
use super::*;
use petgraph::visit::EdgeRef;
use prism::finding_confidence::{EvidenceHop, EvidencePath};

pub fn cpg_bytes(g: &CodePropertyGraph) -> Vec<u8> {
    let nodes: Vec<_> = g.graph.node_indices().map(|n| &g.graph[n]).collect();
    let edges: Vec<_> = g
        .graph
        .edge_references()
        .map(|e| {
            (
                e.source().index() as u32,
                e.target().index() as u32,
                e.weight(),
            )
        })
        .collect();
    bincode::serialize(&(
        nodes,
        edges,
        &g.call_graph,
        &g.dfg,
        &g.return_flow_stats,
        &g.dfg_label_stats,
    ))
    .unwrap()
}

/// Public queries cover all four lookup routes; the crate-unit codec tests also
/// compare the private maps, including their empty rows and exact key payloads.
pub fn lookup_indexes(g: &CodePropertyGraph) -> Vec<u8> {
    let mut functions = vec![];
    let mut variables = vec![];
    let mut lines = BTreeSet::from([("absent.py".to_string(), 0)]);
    for n in g.node_indices() {
        if let Some(id) = g.to_function_id(n) {
            lines.insert((id.file.clone(), id.start_line));
            functions.push((
                id.clone(),
                g.function_node_for_id(&id).map(|n| n.index()),
                g.function_candidates(&id.file, &id.name)
                    .iter()
                    .map(|n| n.index())
                    .collect::<Vec<_>>(),
            ));
        }
        if let Some(v) = g.to_var_location(n) {
            lines.insert((v.file.clone(), v.line));
            variables.push((v.clone(), g.var_node_for_location(&v).map(|n| n.index())));
        }
    }
    let locations: Vec<_> = lines
        .into_iter()
        .map(|(f, l)| {
            let ns: Vec<_> = g.nodes_at(&f, l).iter().map(|n| n.index()).collect();
            (f, l, ns)
        })
        .collect();
    bincode::serialize(&(functions, variables, locations)).unwrap()
}

pub fn observe_evidence(paths: &[Option<EvidencePath>]) -> Vec<u8> {
    let hop = |h: &EvidenceHop| match h {
        EvidenceHop::DataFlow {
            from,
            to,
            confidence,
        } => bincode::serialize(&(0u8, from, to, confidence)).unwrap(),
        EvidenceHop::Call { edge, confidence } => bincode::serialize(&(
            1u8,
            &edge.caller,
            edge.call_site_line,
            edge.confidence,
            edge.kind,
            confidence,
        ))
        .unwrap(),
        _ => panic!("new evidence hop needs an explicit observation"),
    };
    bincode::serialize(
        &paths
            .iter()
            .map(|p| {
                p.as_ref().map(|p| {
                    (
                        p.hops.iter().map(&hop).collect::<Vec<_>>(),
                        p.crossed_unlabeled,
                    )
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

pub fn assert_cpg_roundtrip(name: &str, files: &BTreeMap<String, ParsedFile>) {
    use prism::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult, SaveOutcome};
    let cold = CodePropertyGraph::build(files);
    let sources = files
        .iter()
        .map(|(p, f)| (p.clone(), f.source.clone()))
        .collect();
    let hashes = compute_file_hashes(&sources);
    let dir = TempDir::new().unwrap();
    assert_eq!(
        save_cache(&cold, &hashes, false, dir.path()).unwrap(),
        SaveOutcome::Saved
    );
    let CacheResult::Hit(hit) = load_cache(&hashes, false, dir.path()) else {
        panic!("expected cache hit: {name}")
    };
    assert_eq!(cpg_bytes(&cold), cpg_bytes(&hit), "cache payload {name}");
    assert_eq!(
        lookup_indexes(&cold),
        lookup_indexes(&hit),
        "cache queries {name}"
    );
}
