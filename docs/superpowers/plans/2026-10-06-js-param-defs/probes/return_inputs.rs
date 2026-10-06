// PR-B probe: dump every ReturnInput edge (Use node -> synthetic ReturnValue node) with identities.
// Copy to examples/pd_return_inputs.rs; build --release --example pd_return_inputs --offline --locked.
use petgraph::visit::EdgeRef;
use prism::cpg::{CodePropertyGraph, CpgEdge, CpgNode};
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("repository root");
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    let cpg = CodePropertyGraph::build_enriched_with_scope_graph_inputs(
        &repo.files, repo.type_db.as_ref(), repo.scope_graph_inputs.as_ref());
    let mut rows = Vec::new();
    for e in cpg.graph.edge_references() {
        if !matches!(e.weight(), CpgEdge::ReturnInput) { continue; }
        let (CpgNode::Variable { file, function, function_start_line, line, path, start_byte, end_byte, .. },
             CpgNode::ReturnValue { function: rf, line: rl, start_byte: rs, end_byte: re, .. }) =
            (cpg.node(e.source()), cpg.node(e.target())) else { continue; };
        rows.push(serde_json::json!({"file": file, "function": function, "fsl": function_start_line, "line": line,
            "path": path.to_string(), "bytes": [start_byte, end_byte], "ret_fn": rf, "ret_line": rl, "ret_bytes": [rs, re]}).to_string());
    }
    rows.sort();
    for r in rows { println!("{r}"); }
    Ok(())
}
