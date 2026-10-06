// PROBE-ONLY: time phases of a CPG build on one root.
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).unwrap();
    let t = std::time::Instant::now();
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    eprintln!("load {:?}", t.elapsed());
    let t = std::time::Instant::now();
    let dfg = prism::data_flow::DataFlowGraph::build(&repo.files);
    eprintln!("dfg {:?} edges {}", t.elapsed(), dfg.edges.len());
    let t = std::time::Instant::now();
    let cpg = prism::cpg::CodePropertyGraph::build_enriched_with_scope_graph_inputs(
        &repo.files,
        repo.type_db.as_ref(),
        repo.scope_graph_inputs.as_ref(),
    );
    eprintln!("cpg {:?} nodes {}", t.elapsed(), cpg.graph.node_count());
    Ok(())
}
