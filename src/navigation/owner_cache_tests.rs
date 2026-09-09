use super::*;

#[test]
fn ephemeral_cache_guards_work_without_a_compiler() {
    for (cpg_marker, graph_marker) in [(false, false), (true, false), (false, true)] {
        let files = BTreeMap::new();
        let mut cpg = CodePropertyGraph::empty();
        cpg.ephemeral_owner = cpg_marker;
        cpg.call_graph.owner_ephemeral = graph_marker;
        let root = tempfile::tempdir().unwrap();
        let cache_path = root.path().join("cache");
        let save = crate::cpg_cache::save_cache(&cpg, &BTreeMap::new(), false, &cache_path);
        let ephemeral = cpg_marker || graph_marker;
        assert_eq!(save.is_err(), ephemeral);
        assert_eq!(cache_path.exists(), !ephemeral);
        let ctx = CpgContext::build_with_fresh_cpg(&files, cpg, None);
        let store = call_edge_cache::NavigationCallEdgeCacheStore::new(
            root.path(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            false,
            true,
        );
        let index = NavigationIndex::from_ctx_with_call_edge_cache(ctx, Some(store));
        assert_eq!(
            index.call_edge_cache_store.is_none(),
            ephemeral,
            "no store means neither lazy load nor save can execute"
        );
    }
}
