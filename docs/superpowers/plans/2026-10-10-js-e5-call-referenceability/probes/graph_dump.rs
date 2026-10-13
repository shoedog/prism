// Static CPG comparison: all call/flow edges, and a digest of other nodes/edges.
use petgraph::visit::EdgeRef;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
use serde_json::json;
use sha2::{Digest, Sha256};
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("repository root");
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    let cpg = CodePropertyGraph::build_enriched_with_scope_graph_inputs(
        &repo.files, repo.type_db.as_ref(), repo.scope_graph_inputs.as_ref());
    let mut rows = Vec::new();
    let mut other = Vec::new();
    // ReturnValue nodes are produced by Exact-gated return modeling.
    for node in cpg.graph.node_weights() {
        if !matches!(node, CpgNode::ReturnValue { .. }) {
            other.push(serde_json::to_string(node)?);
        }
    }
    for e in cpg.graph.edge_references() {
        let kind = match e.weight() {
            CpgEdge::DataFlow(_) => "DataFlow", CpgEdge::Call(_) => "Call",
            CpgEdge::Return(_) => "Return", CpgEdge::ReturnFlow { .. } => "ReturnFlow",
            CpgEdge::ReturnInput => "ReturnInput",
            CpgEdge::Contains if matches!(cpg.node(e.target()), CpgNode::ReturnValue { .. }) => "ReturnContains",
            _ => "other",
        };
        let row = json!({"kind":kind,"edge":e.weight(),
            "from":cpg.node(e.source()),"to":cpg.node(e.target())}).to_string();
        if kind == "other" { other.push(row); } else { rows.push(row); }
    }
    other.sort();
    let mut digest = Sha256::new();
    for row in &other { digest.update(row.as_bytes()); digest.update(b"\n"); }
    rows.push(json!({"kind":"non_call_digest","records":other.len(),
        "sha256":format!("{:x}",digest.finalize())}).to_string());
    rows.sort();
    use std::io::Write;
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());
    for row in rows { writeln!(out,"{row}")?; }
    Ok(())
}
