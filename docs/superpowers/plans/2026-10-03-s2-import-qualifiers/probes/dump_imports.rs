// READ: Measurement-only extraction from the exact linked Prism library.
use prism::{call_graph::CallGraph, repo_loader::load_repo};

fn main() {
    let root = std::env::args().nth(1).expect("corpus root");
    let repo = load_repo(std::path::Path::new(&root)).expect("load corpus");
    let cg = CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
    let indexed = repo.files.keys().cloned().collect();
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
        // Exact B writer population; relative S2 proofs retain their existing
        // scoped refusal controls and are not PKG coverage claims.
        let raw = cg.js_ts_exports.get(file);
        let writer_specs: std::collections::BTreeSet<_> = cg.import_bindings.get(file).into_iter().flatten()
            .filter(|b| raw.is_some_and(|r| !r.qualifiers.complete || r.qualifiers.written.contains(&b.local) || r.qualifiers.called_members.contains_key(&b.local)))
            .map(|b| &b.module_path).filter(|s| !s.starts_with('.')).collect();
        let writer_resolutions: Vec<_> = writer_specs.into_iter().map(|spec| {
            let (resolution, owner) = prism::js_packages::resolve_import(
                &repo.scope_graph_inputs.as_ref().unwrap().js_paths_snapshot, file, spec, &indexed);
            serde_json::json!({"specifier":spec,"resolution":resolution,"owner":owner})
        }).collect();
        println!("{}", serde_json::json!({"file":file,"hash":repo.file_hashes.get(file),
            "bindings":cg.import_bindings.get(file),"imports":cg.imports.get(file),
            "functions":functions,"sites":sites,"module_proofs":proofs,
            "qualifier_facts":cg.js_ts_exports.get(file).map(|f| &f.qualifiers),
            "writer_resolutions":writer_resolutions}));
    }
}
