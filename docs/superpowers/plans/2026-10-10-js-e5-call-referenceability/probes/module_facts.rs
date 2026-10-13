// Static product-parser facts for exact union-arm attribution and inventories.
// Compiled after copying to examples/e5_step2_modules.rs. Include the canonical
// collector so arm attribution uses exactly the product's exemption evidence.
use prism::{ast, call_graph, languages};
#[path = "../src/call_graph_js_referenceability.rs"]
mod referenceability;
use serde_json::json;
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("root");
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    let evidence = referenceability::module_binding_evidence(repo.files.iter());
    for (file, parsed) in &repo.files {
        let module = evidence.contains_key(file);
        let names = evidence.get(file).cloned().unwrap_or_default();
        println!(
            "{}",
            json!({"file":file,"module":module,"imports":names,"language":format!("{:?}",parsed.language)})
        );
    }
    Ok(())
}
