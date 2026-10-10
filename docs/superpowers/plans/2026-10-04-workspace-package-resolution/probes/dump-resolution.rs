use prism::{call_graph::CallGraph, js_packages::resolve_import, repo_loader::load_repo};
use std::collections::BTreeSet;
fn main() {
    let path = std::env::args().nth(1).expect("fixture manifest");
    let cases: Vec<serde_json::Value> =
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    for c in cases {
        let repo = load_repo(std::path::Path::new(c["root"].as_str().unwrap())).expect("fixture");
        let indexed: BTreeSet<_> = repo.files.keys().cloned().collect();
        let writer = c["writer"].as_str().unwrap();
        let spec = c["specifier"].as_str().unwrap();
        let (resolution, owner) = resolve_import(
            &repo.scope_graph_inputs.as_ref().unwrap().js_paths_snapshot,
            writer,
            spec,
            &indexed,
        );
        let cg =
            CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
        let targets: Vec<_> = prism::navigation::queries::call_site_dump(&cg)
            .into_iter()
            .filter(|r| r["caller"]["file"] == writer && r["callee_text"] == "picked")
            .collect();
        let module = cg.js_ts_path_modules.get(&(writer.into(), spec.into()));
        println!(
            "{}",
            serde_json::json!({"id":c["id"],"resolution":resolution,"owner":owner,"production_module":module,"calls":targets})
        );
    }
}
