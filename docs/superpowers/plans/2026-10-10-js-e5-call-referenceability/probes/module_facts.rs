// Static product-parser facts for exact union-arm attribution and inventories.
use prism::languages::Language;
use serde_json::json;
fn main() -> anyhow::Result<()> {
    let root = std::env::args().nth(1).expect("root");
    let repo = prism::repo_loader::load_repo(std::path::Path::new(&root))?;
    for (file, parsed) in &repo.files {
        let js = matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        );
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        let module = js
            && root
                .named_children(&mut cursor)
                .any(|n| matches!(n.kind(), "import_statement" | "export_statement"));
        println!(
            "{}",
            json!({"file":file,"module":module,"imports":parsed.extract_imports(),"language":format!("{:?}",parsed.language)})
        );
    }
    Ok(())
}
