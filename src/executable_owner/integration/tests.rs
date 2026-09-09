use super::*;
use crate::resolution::ResolutionConfidence;

fn acquire() -> (tempfile::TempDir, AuthenticatedProgramEpoch) {
    let root = tempfile::tempdir().unwrap();
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    for (name, source) in corpus["files"].as_object().unwrap() {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source.as_str().unwrap()).unwrap();
    }
    std::fs::write(root.path().join("package.json"), r#"{"type":"module"}"#).unwrap();
    std::fs::write(
        root.path().join("tsconfig.json"),
        serde_json::json!({"compilerOptions":corpus["compiler_options"],"include":["src"]})
            .to_string(),
    )
    .unwrap();
    let repo = crate::repo_loader::load_repo(root.path()).unwrap();
    let compiler = std::env::var("PRISM_TYPESCRIPT").expect("explicit pinned compiler required");
    let epoch = AuthenticatedProgramEpoch::acquire(
        root.path(),
        "tsconfig.json",
        Path::new(&compiler),
        repo.files,
    )
    .unwrap();
    (root, epoch)
}

#[test]
fn staged_owner_reaches_the_real_shared_resolver() {
    let (_root, epoch) = acquire();
    let graph = staged_graph(&epoch);
    let site = graph
        .calls
        .values()
        .flatten()
        .find(|c| c.caller.file == "src/app.ts" && c.callee_name == "m")
        .unwrap();
    let resolved = graph.resolve_call_site_full(site);
    assert_eq!(
        resolved.resolved.len(),
        1,
        "staged owner must reach shared resolver"
    );
    assert_eq!(resolved.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(resolved.resolved[0].target.file, "src/client.ts");
}

fn call(graph: &CallGraph) -> &CallSite {
    graph
        .calls
        .values()
        .flatten()
        .find(|c| c.caller.file.starts_with("src/app.") && c.callee_name == "m")
        .unwrap()
}
fn exact(graph: &CallGraph) -> bool {
    graph
        .resolve_call_site_full(call(graph))
        .resolved
        .iter()
        .any(|r| r.confidence == ResolutionConfidence::Exact)
}
fn compiler() -> std::path::PathBuf {
    std::env::var("PRISM_TYPESCRIPT")
        .expect("explicit compiler")
        .into()
}
fn reacquire(root: &Path) -> AuthenticatedProgramEpoch {
    AuthenticatedProgramEpoch::acquire(
        root,
        "tsconfig.json",
        &compiler(),
        crate::repo_loader::load_repo(root).unwrap().files,
    )
    .unwrap()
}
fn cpg(epoch: &AuthenticatedProgramEpoch) -> crate::cpg::CodePropertyGraph {
    crate::cpg::CodePropertyGraph::build_with_staged_owner(epoch, &epoch.0._files, None).unwrap()
}
fn exact_edges(cpg: &crate::cpg::CodePropertyGraph) -> usize {
    use crate::cpg::CpgEdge;
    cpg.graph
        .edge_weights()
        .filter(|e| {
            matches!(
                e,
                CpgEdge::Call(ResolutionConfidence::Exact)
                    | CpgEdge::Return(ResolutionConfidence::Exact)
            )
        })
        .count()
}

#[test]
fn full_subset_clone_serde_and_generic_mutation_have_explicit_authority() {
    let (_root, epoch) = acquire();
    let graph = staged_graph(&epoch);
    assert!(exact(&graph.clone()));
    let subset = staged_subset(
        &epoch,
        &BTreeSet::from(["src/app.ts".into(), "src/client.ts".into()]),
    )
    .unwrap();
    assert!(exact(&subset));
    let missing_target = staged_subset(&epoch, &BTreeSet::from(["src/app.ts".into()])).unwrap();
    assert!(missing_target
        .resolve_call_site_full(call(&missing_target))
        .resolved
        .is_empty());
    assert!(staged_subset(&epoch, &BTreeSet::from(["not-indexed.ts".into()])).is_err());
    let ordinary = CallGraph::build(&epoch.0._files);
    let bytes = bincode::serialize(&graph).unwrap();
    assert_eq!(
        bytes,
        bincode::serialize(&ordinary).unwrap(),
        "no authority or derived edges in graph serialization"
    );
    let decoded: CallGraph = bincode::deserialize(&bytes).unwrap();
    assert!(!exact(&decoded));
    let mut removed = graph.clone();
    removed.remove_files(&BTreeSet::new());
    assert!(!exact(&removed));
    let mut merged = graph.clone();
    merged.merge(CallGraph::empty());
    assert!(!exact(&merged));
    let mut incoming = CallGraph::empty();
    incoming.merge(graph);
    assert!(!exact(&incoming));
}

#[test]
fn direct_recomputation_cannot_keep_a_previous_epoch() {
    let (_root, epoch) = acquire();
    let mut graph = staged_graph(&epoch);
    let mut files = epoch.0._files.clone();
    files.insert(
        "src/client.ts".into(),
        ParsedFile::parse(
            "src/client.ts",
            "export class Client { m(): number { return 9; } }",
            Language::TypeScript,
        )
        .unwrap(),
    );
    graph.recompute_indirect_calls(&files);
    assert!(
        !exact(&graph),
        "ordinary recomputation must drop historical owner authority"
    );
    assert!(graph.owner_ephemeral, "cache refusal must survive");
}

#[test]
fn exact_site_fields_and_two_genuine_epoch_proofs_cannot_be_substituted() {
    let (_root, a) = acquire();
    let (root_b, _initial_b) = acquire();
    std::fs::write(
        root_b.path().join("src/client.ts"),
        "export class Client { m(): number { return 2; } }\n",
    )
    .unwrap();
    let b = reacquire(root_b.path());
    let mut graph = staged_graph(&a);
    let site = call(&graph).clone();
    let mutations: [fn(&mut CallSite); 8] = [
        |s| s.callee_name = "other".into(),
        |s| s.line += 1,
        |s| s.caller.name = "other".into(),
        |s| s.qualifier = Some("other".into()),
        |s| s.receiver_materialized = !s.receiver_materialized,
        |s| s.arg_count = Some(99),
        |s| s.receiver_type = Some("Decoy".into()),
        |s| s.pre_resolved_target = Some(s.caller.clone()),
    ];
    for mutate in mutations {
        let mut changed = site.clone();
        mutate(&mut changed);
        assert!(graph.resolve_call_site_full(&changed).resolved.is_empty());
    }
    let key = (site.caller.file.clone(), site.start_byte, site.end_byte);
    let mixed = InstalledOwner {
        epoch: AuthenticatedProgramEpoch(Arc::clone(&a.0)),
        entries: BTreeMap::from([(
            key.clone(),
            (site.clone(), b.prove(&key.0, key.1, key.2).unwrap()),
        )]),
    };
    graph.executable_owner = Some(Arc::new(mixed));
    assert!(graph.resolve_call_site_full(&site).resolved.is_empty());
}

#[test]
fn cpg_uses_owned_inputs_refuses_cross_source_and_does_not_cache_edges() {
    let (_root, epoch) = acquire();
    let graph = cpg(&epoch);
    assert_eq!(exact_edges(&graph), 2, "one real Call and Return pair");
    assert_eq!(
        graph.return_flow_stats.return_flow_edges, 0,
        "fixture does not consume a call result"
    );
    let cache = tempfile::tempdir().unwrap();
    let path = cache.path().join("must-not-create");
    let hashes = BTreeMap::new();
    assert!(crate::cpg_cache::save_cache(&graph, &hashes, false, &path).is_err());
    assert!(!path.exists());
    let mut stripped = graph;
    stripped.call_graph.remove_files(&BTreeSet::new());
    assert_eq!(
        exact_edges(&stripped),
        2,
        "clearing proof metadata is not edge deletion"
    );
    assert!(crate::cpg_cache::save_cache(&stripped, &hashes, false, &path).is_err());
    assert!(!path.exists());
    let mut changed = epoch.0._files.clone();
    changed
        .get_mut("src/client.ts")
        .unwrap()
        .source
        .push_str("// another snapshot\n");
    assert!(
        crate::cpg::CodePropertyGraph::build_with_staged_owner(&epoch, &changed, None).is_err()
    );
    changed = epoch.0._files.clone();
    changed.get_mut("src/app.ts").unwrap().tree =
        ParsedFile::parse("src/app.ts", "", Language::TypeScript)
            .unwrap()
            .tree;
    assert_eq!(
        exact_edges(
            &crate::cpg::CodePropertyGraph::build_with_staged_owner(&epoch, &changed, None)
                .unwrap()
        ),
        2,
        "reparse-owned tree wins over public tree mutation"
    );
    let ordinary = crate::cpg::CodePropertyGraph::build(&epoch.0._files);
    assert_eq!(exact_edges(&ordinary), 0);
    crate::cpg_cache::save_cache(&ordinary, &hashes, false, &path).unwrap();
    assert!(path.is_dir());
}

fn assert_navigation(session: &crate::navigation::NavigationSession, expected: usize) {
    use crate::navigation::queries;
    let out =
        queries::callees_with_confidence(session, Some("run"), Some("src/app.ts"), None, 1, true)
            .unwrap();
    let incoming =
        queries::callers_with_confidence(session, Some("m"), Some("src/client.ts"), None, 1, true)
            .unwrap();
    assert_eq!(out.items.len(), expected);
    assert_eq!(incoming.items.len(), expected);
    assert_eq!(exact_edges(session.index.cpg()), expected * 2);
}

#[test]
fn fresh_sessions_replace_positive_different_unproven_failure_and_restore() {
    let (root, _epoch) = acquire();
    let mut active = None;
    replace_session(&mut active, root.path(), "tsconfig.json", &compiler()).unwrap();
    assert_navigation(active.as_ref().unwrap(), 1);
    let historical = active.take().unwrap();
    std::fs::write(
        root.path().join("src/client.ts"),
        "export class Client { m(): number { return 2; } }\n",
    )
    .unwrap();
    replace_session(&mut active, root.path(), "tsconfig.json", &compiler()).unwrap();
    assert_navigation(active.as_ref().unwrap(), 1);
    assert!(!Arc::ptr_eq(
        &historical.index,
        &active.as_ref().unwrap().index
    ));
    std::fs::write(
        root.path().join("src/client.ts"),
        "export class Client { m = () => 3; }\n",
    )
    .unwrap();
    replace_session(&mut active, root.path(), "tsconfig.json", &compiler()).unwrap();
    assert_navigation(active.as_ref().unwrap(), 0);
    assert!(active.as_ref().unwrap().index.cpg().ephemeral_owner);
    std::fs::write(
        root.path().join("src/gap.ts"),
        "import './absent'; export {};\n",
    )
    .unwrap();
    assert!(replace_session(&mut active, root.path(), "tsconfig.json", &compiler()).is_err());
    assert!(active.is_none());
    std::fs::remove_file(root.path().join("src/gap.ts")).unwrap();
    std::fs::write(
        root.path().join("src/client.ts"),
        "export class Client { m(): number { return 4; } }\n",
    )
    .unwrap();
    replace_session(&mut active, root.path(), "tsconfig.json", &compiler()).unwrap();
    assert_navigation(active.as_ref().unwrap(), 1);
    assert_navigation(&historical, 1); // explicit historical handle, not active state
    let config = root.path().join("tsconfig.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&config).unwrap()).unwrap();
    value["compilerOptions"]["types"] = serde_json::json!(["missing-owner-test-type"]);
    std::fs::write(config, value.to_string()).unwrap();
    assert!(replace_session(&mut active, root.path(), "tsconfig.json", &compiler()).is_err());
    assert!(active.is_none());
}

#[test]
fn navigation_discards_even_a_supplied_persistent_sidecar_store() {
    use crate::navigation::{call_edge_cache::NavigationCallEdgeCacheStore, NavigationIndex};
    let (_root, epoch) = acquire();
    let cache = tempfile::tempdir().unwrap();
    let sentinel = cache.path().join("resolved-call-edge-index.bin");
    std::fs::write(&sentinel, b"must not be read or replaced").unwrap();
    let store = NavigationCallEdgeCacheStore::new(
        cache.path(),
        &BTreeMap::new(),
        &BTreeMap::new(),
        false,
        true,
    );
    let ctx = crate::cpg::CpgContext::build_with_fresh_cpg(&epoch.0._files, cpg(&epoch), None);
    let index = NavigationIndex::from_ctx_with_call_edge_cache(ctx, Some(store));
    let _ = index.resolved_call_edges();
    assert_eq!(
        std::fs::read(&sentinel).unwrap(),
        b"must not be read or replaced"
    );
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 1);
}

#[test]
fn generic_incremental_entry_drops_opt_in_and_rebuilds_without_stale_edges() {
    let (_root, epoch) = acquire();
    let before = cpg(&epoch);
    let after = crate::cpg::CodePropertyGraph::build_incremental(
        before.call_graph,
        before.dfg,
        &BTreeSet::new(),
        &epoch.0._files,
        None,
    );
    assert_eq!(exact_edges(&after), 0);
    assert!(!after.ephemeral_owner);
    assert!(!exact(&after.call_graph));
}

#[test]
fn reconstruction_preserves_sticky_refusal_after_sidecar_clear() {
    let (_root, epoch) = acquire();
    let mut before = cpg(&epoch);
    before.call_graph.remove_files(&BTreeSet::new());
    let mut rebuilt = crate::cpg::CodePropertyGraph::from_parts(
        before.graph,
        before.func_index,
        before.name_index,
        before.var_index,
        before.location_index,
        before.call_graph,
        before.dfg,
    );
    rebuilt.call_graph.remove_files(&BTreeSet::new());
    let root = tempfile::tempdir().unwrap();
    assert!(crate::cpg_cache::save_cache(
        &rebuilt,
        &BTreeMap::new(),
        false,
        &root.path().join("forbidden")
    )
    .is_err());
}

#[test]
fn ordinary_cached_context_does_not_adopt_ephemeral_edges() {
    let (_root, epoch) = acquire();
    let after = crate::cpg::CpgContext::build_with_cached_cpg(&epoch.0._files, cpg(&epoch), None);
    assert_eq!(exact_edges(&after.cpg), 0);
    assert!(!after.cpg.ephemeral_owner);
}

#[test]
fn tsx_positive_and_existing_annotation_routes_remain_distinct() {
    let (root, _) = acquire();
    std::fs::rename(
        root.path().join("src/app.ts"),
        root.path().join("src/app.tsx"),
    )
    .unwrap();
    let graph = staged_graph(&reacquire(root.path()));
    assert!(exact(&graph));
    assert_eq!(
        graph.resolve_call_site_full(call(&graph)).resolved[0].kind,
        crate::resolution::ResolutionKind::ContextualOwner
    );
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/eval/receiver-closure/executable-owner-fixtures.json"
    ))
    .unwrap();
    for id in ["inline_signature_control", "explicit_parameter_control"] {
        let (root, _) = acquire();
        let case = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        for (name, source) in case["files"].as_object().unwrap() {
            std::fs::write(root.path().join(name), source.as_str().unwrap()).unwrap();
        }
        let epoch = reacquire(root.path());
        let staged = staged_graph(&epoch);
        let ordinary = CallGraph::build(&epoch.0._files);
        assert!(exact(&ordinary));
        let actual = staged.resolve_call_site_full(call(&staged));
        let control = ordinary.resolve_call_site_full(call(&ordinary));
        assert_eq!(actual.resolved[0].target, control.resolved[0].target);
        assert_eq!(actual.resolved[0].kind, control.resolved[0].kind);
        assert_ne!(
            actual.resolved[0].kind,
            crate::resolution::ResolutionKind::ContextualOwner
        );
    }
}
