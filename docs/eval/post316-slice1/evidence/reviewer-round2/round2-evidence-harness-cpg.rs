
#[test]
fn reviewer_scheduling_and_cpg_cache_endpoint_parity() {
    for (language,ext) in [(Language::JavaScript,"js"),(Language::TypeScript,"ts"),(Language::Tsx,"tsx")] {
        let app=format!("app.{ext}");let origin=format!("origin.{ext}");
        let source="function outer(token){register(eager(),function inner(){return item(token);});direct();}";
        let files=BTreeMap::from([(app.clone(),ParsedFile::parse(&app,source,language).unwrap()),(origin.clone(),ParsedFile::parse(&origin,"export function item(input){return input;}",language).unwrap())]);
        let one=rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
        let four=rayon::ThreadPoolBuilder::new().num_threads(4).build().unwrap();
        let a=one.install(||crate::call_graph::CallGraph::build(&files));
        let b=four.install(||crate::call_graph::CallGraph::build(&files));
        let subset=crate::call_graph::CallGraph::build_direct_subset(&files,&files.keys().cloned().collect());
        assert_eq!(complete_site_rows(&a,&app),complete_site_rows(&b,&app));
        assert_eq!(complete_site_rows(&a,&app),complete_site_rows(&subset,&app));
        assert_eq!(complete_site_rows(&a,&app).len(),4);
        let cpg=CodePropertyGraph::build(&files);
        let sources=files.iter().map(|(k,v)|(k.clone(),v.source.clone())).collect();
        let hashes=crate::cpg_cache::compute_file_hashes(&sources);
        let dir=tempfile::tempdir().unwrap();
        crate::cpg_cache::save_cache(&cpg,&hashes,false,dir.path()).unwrap();
        let crate::cpg_cache::CacheResult::Hit(restored)=crate::cpg_cache::load_cache(&hashes,false,dir.path()) else {panic!("expected cache hit")};
        assert_eq!(complete_site_rows(&cpg.call_graph,&app),complete_site_rows(&restored.call_graph,&app));
        assert_eq!(ownership_edge_rows(&cpg),ownership_edge_rows(&restored));
        assert_eq!(resolution_rows(&cpg,&app),resolution_rows(&restored,&app));
        assert_eq!(semantic_dfg_rows(&cpg,&app),semantic_dfg_rows(&restored,&app));
        println!("REVIEW_PARITY {ext} scheduling=1vs4 full_vs_serial_subset=equal sites={:?} edges={:?}",complete_site_rows(&restored.call_graph,&app),ownership_edge_rows(&restored));
    }
}
