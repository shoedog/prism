use super::*;
use crate::call_graph::CallGraph;
use crate::resolution::{ResolutionConfidence, ResolutionKind, ResolutionOutcome, ResolvedCallee};
use std::collections::BTreeMap;

fn files() -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([(
        "a.js".into(),
        ParsedFile::parse("a.js", "function f() {}\nf();\n", Language::JavaScript).unwrap(),
    )])
}

#[test]
fn module_identity_claims_and_sources() {
    let files = files();
    let p = &files["a.js"];
    let root = p.tree.root_node();
    let id = identity(p, &root, "a.js").unwrap();
    assert_eq!(id.name, MODULE_OWNER_NAME);
    assert!(is_anonymous_owner_name(&id.name));
    assert!(is_module_owner_name(&id.name));
    assert!(!is_module_owner_name(&anonymous_owner_name(1, 1)));
    let other = identity(p, &root, "b.js").unwrap();
    assert_ne!(id, other, "module identity must be file-bound");
    assert_eq!(
        source_index::SourceIndex::new(p).source_bytes(&id),
        Some((0, p.source.len()))
    );
    assert_eq!(node_for_id(p, &id).unwrap().id(), root.id());
    let start = p.source.find("f();").unwrap();
    let end = start + 3;
    assert!(may_mint_call(p, &root, start, end, &BTreeSet::new()));
    assert!(
        !may_mint_call(p, &root, start, end, &BTreeSet::from([(start, end)])),
        "named byte claims always win"
    );
    assert!(
        may_mint_call(p, &root, start, end, &BTreeSet::from([(start + 1, end)])),
        "claims use bytes, not lines"
    );
    let declaration = p.all_functions()[0];
    assert_eq!(identity(p, &declaration, "a.js").unwrap().name, "f");
}

#[test]
fn module_exact_needs_unchanged_barrier_certificate() {
    let files = files();
    let graph = CallGraph::build(&files);
    let site = graph.calls[&graph.functions[MODULE_OWNER_NAME][0]]
        .iter()
        .next()
        .unwrap();
    let target = &graph.functions["f"][0];
    let candidate = ResolutionOutcome::hit(vec![ResolvedCallee {
        target,
        confidence: ResolutionConfidence::Exact,
        kind: ResolutionKind::LocalDef,
    }]);
    let denied = graph.admit_anonymous_exact(site, candidate);
    assert_eq!(denied.resolved.len(), 1);
    assert_eq!(
        denied.resolved[0].confidence,
        ResolutionConfidence::NameOnly,
        "B1: an Exact candidate without a certificate must demote"
    );
    let proven = graph.resolve_call_site_full(site);
    assert_eq!(proven.resolved[0].confidence, ResolutionConfidence::Exact);
    assert_eq!(
        graph.admit_anonymous_exact(site, proven.clone()).resolved[0].confidence,
        ResolutionConfidence::Exact
    );
    let mut wrong_site = site.clone();
    wrong_site.start_byte += 1;
    assert_eq!(
        graph.admit_anonymous_exact(&wrong_site, proven).resolved[0].confidence,
        ResolutionConfidence::NameOnly
    );
}

#[test]
fn module_navigation_cache_parity() {
    use crate::navigation::{queries, NavigationSession};
    for threads in [1, 4] {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap()
            .install(|| {
                let root = tempfile::tempdir().unwrap();
                let cache = tempfile::tempdir().unwrap();
                std::fs::write(root.path().join("a.ts"), "import {f} from './lib';\nf();").unwrap();
                let lib = root.path().join("lib.ts");
                std::fs::write(&lib, "export function f() {}").unwrap();
                let mut options = crate::api::NavOptions {
                    cache_dir: Some(cache.path().into()),
                    ..Default::default()
                };
                let query = |s: &NavigationSession| {
                    serde_json::to_vec(&queries::callers(s, Some("f"), None, None, 1).unwrap())
                        .unwrap()
                };
                let cold = crate::api::nav_session(root.path(), &options).unwrap();
                let expected = query(&cold);
                assert!(String::from_utf8_lossy(&expected).contains("module-level"));
                assert_eq!(cold.index.cpg_loaded_from_cache(), Some(false));
                assert_eq!(cold.index.call_edges_loaded_from_cache(), Some(false));
                let warm = crate::api::nav_session(root.path(), &options).unwrap();
                assert_eq!(query(&warm), expected);
                assert_eq!(warm.index.cpg_loaded_from_cache(), Some(true));
                // Dirty binaries intentionally recompute the sidecar. The CLI
                // parity test uses an isolated per-child override to exercise loading.
                assert!(warm.index.call_edges_loaded_from_cache().is_some());
                std::fs::write(&lib, "export function f() {} eval('x');").unwrap();
                let partial = crate::api::nav_session(root.path(), &options).unwrap();
                let changed = query(&partial);
                assert_ne!(
                    changed, expected,
                    "dependency B0 veto changes the module edge grade"
                );
                assert_eq!(partial.index.cpg_loaded_from_cache(), Some(false));
                assert_eq!(partial.index.call_edges_loaded_from_cache(), Some(false));
                options.no_cache = true;
                options.cache_dir = None;
                let fresh = crate::api::nav_session(root.path(), &options).unwrap();
                assert_eq!(query(&fresh), changed);
            });
    }
}
