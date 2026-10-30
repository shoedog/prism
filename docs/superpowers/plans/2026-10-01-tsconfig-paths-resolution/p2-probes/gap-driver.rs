// Diagnostic sidecar. These modules are copied byte-for-byte from the bound SUT.
mod js_paths;
mod js_paths_first_pass;
mod js_paths_snapshot;
mod js_paths_syntax;
use prism::{call_graph::CallGraph, repo_loader::load_repo, js_exports::JsExportTarget};
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root=Path::new(&args[1]);
    let requests: Vec<Value>=serde_json::from_slice(&std::fs::read(&args[2]).unwrap()).unwrap();
    let repo=load_repo(root).expect("load diagnostic source");
    let cg=CallGraph::build_with_scope_graph_inputs(&repo.files,repo.scope_graph_inputs.as_ref());
    let snap=js_paths_snapshot::JsPathsSnapshot::capture(root);
    let mut resolver=js_paths::Resolver::new(&snap);
    let mut entries=Vec::new();
    for r in requests {
        let file=r["key"][0].as_str().unwrap(); let spec=r["specifier"].as_str().unwrap();
        let explanation=resolver.explain(file,spec,&cg.indexed_files,false,true);
        let actual=cg.js_ts_path_modules.get(&(file.into(),spec.into()));
        let binding=cg.import_bindings.get(file).into_iter().flatten()
            .find(|b|Some(b.local.as_str())==r["local"].as_str() && b.module_path==spec);
        if binding.is_some_and(|b|b.eligible && cg.js_ts_exports.get(file)
            .is_some_and(|e|e.esm_named_imports.contains(&b.local))) {
            assert_eq!(explanation["target"].as_str(),actual.map(|a|a.0.as_str()),"entry replay disagreement");
        }
        entries.push(json!({"key":r["key"],"explanation":explanation,
            "binding":binding,"module":actual.map(|(p,a)|json!({"target":p,"allow_js":a})),
            "allow_js":resolver.allow_js(file)}));
    }
    let mut hops=BTreeSet::new();
    for (file,e) in &cg.js_ts_exports {
        for t in e.named.values() {
            if let JsExportTarget::ReExport{module_path,..}|JsExportTarget::ImportForward{module_path,..}=t {
                hops.insert((file.clone(),module_path.clone()));
            }
        }
        for spec in &e.star_reexports {hops.insert((file.clone(),spec.clone()));}
    }
    let hops:Vec<_>=hops.into_iter().flat_map(|(file,spec)| {
        [false,true].map(|allow|json!({"from":file,"specifier":spec,"allow_js":allow,
            "explanation":resolver.explain(&file,&spec,&cg.indexed_files,allow,false)}))
    }).collect();
    let functions:Vec<_>=cg.functions.values().flatten()
        .filter(|f|!cg.method_owners.contains_key(*f)).collect();
    println!("{}",json!({"entries":entries,"hops":hops,"exports":cg.js_ts_exports,
        "path_exports":cg.js_ts_path_exports,"functions":functions,
        "file_hashes":repo.file_hashes,"max_depth":prism::js_exports::MAX_REEXPORT_DEPTH}));
}
