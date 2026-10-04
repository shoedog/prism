use prism::{call_graph::CallGraph, repo_loader::load_repo};
fn main() {
    let root = std::env::args().nth(1).expect("corpus root");
    let repo = load_repo(std::path::Path::new(&root)).expect("load corpus");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
    for (file, parsed) in &repo.files {
        if !matches!(parsed.language, prism::languages::Language::JavaScript | prism::languages::Language::TypeScript | prism::languages::Language::Tsx) {continue;}
        let modules: std::collections::BTreeMap<_,_> = cg.js_ts_path_modules.iter().filter(|((f,_),_)|f==file).map(|((_,s),v)|(s,v)).collect();
        println!("{}", serde_json::json!({"file":file,"hash":repo.file_hashes.get(file),"bindings":cg.import_bindings.get(file),"modules":modules}));
    }
}
