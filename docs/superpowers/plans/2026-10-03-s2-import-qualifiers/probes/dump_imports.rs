// READ: Measurement-only extraction from the exact linked Prism library.
use prism::{call_graph::CallGraph, repo_loader::load_repo};

fn main() {
    let root = std::env::args().nth(1).expect("corpus root");
    let repo = load_repo(std::path::Path::new(&root)).expect("load corpus");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
    for (file, parsed) in &repo.files {
        if !matches!(parsed.language, prism::languages::Language::JavaScript | prism::languages::Language::TypeScript | prism::languages::Language::Tsx) {
            continue;
        }
        let sites: Vec<_> = cg.calls.iter().filter(|(fid, _)| &fid.file == file)
            .flat_map(|(_, s)| s).map(|s| serde_json::json!({
                "caller":s.caller,"callee_text":s.callee_name,"start_byte":s.start_byte,
                "end_byte":s.end_byte,"qualifier":s.qualifier,"local_binding":s.local_binding,
                "jsx_element":s.jsx_element,"origin":s.origin})).collect();
        let functions: Vec<_> = cg.functions.values().flatten().filter(|f| &f.file == file).collect();
        let proofs: Vec<_> = cg.js_ts_qualifier_modules.iter().filter(|((f,_),_)| f == file)
            .map(|((_,specifier),(module,owner))| serde_json::json!({"specifier":specifier,"module":module,"owner":owner})).collect();
        println!("{}", serde_json::json!({"file":file,"hash":repo.file_hashes.get(file),
            "bindings":cg.import_bindings.get(file),"imports":cg.imports.get(file),
            "functions":functions,"sites":sites,"module_proofs":proofs}));
    }
}
