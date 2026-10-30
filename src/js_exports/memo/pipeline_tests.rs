use super::*;
use crate::{
    call_graph::CallGraph, cpg::CodePropertyGraph, navigation::queries, repo_loader::load_repo,
};
use std::path::Path;

fn write(root: &Path, name: &str, text: &str) {
    let p = root.join(name);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, text).unwrap();
}
fn assert_graphs(a: &CodePropertyGraph, b: &CodePropertyGraph) {
    let a_cg = &a.call_graph;
    let b_cg = &b.call_graph;
    assert_eq!(a_cg.js_ts_path_modules, b_cg.js_ts_path_modules);
    assert_eq!(a_cg.js_ts_path_exports, b_cg.js_ts_path_exports);
    assert_eq!(a_cg.js_ts_resolved_exports, b_cg.js_ts_resolved_exports);
    assert_eq!(a_cg.js_ts_namespace_exports, b_cg.js_ts_namespace_exports);
    assert_eq!(
        a_cg.js_export_chain_unresolved,
        b_cg.js_export_chain_unresolved
    );
    assert_eq!(
        a_cg.js_export_barrel_conflicts,
        b_cg.js_export_barrel_conflicts
    );
    assert_eq!(
        serde_json::to_vec(&queries::call_stats(a_cg)).unwrap(),
        serde_json::to_vec(&queries::call_stats(b_cg)).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&queries::call_site_dump(a_cg)).unwrap(),
        serde_json::to_vec(&queries::call_site_dump(b_cg)).unwrap()
    );
    assert_eq!(
        serde_json::to_vec(&queries::dfg_edge_dump(a)).unwrap(),
        serde_json::to_vec(&queries::dfg_edge_dump(b)).unwrap()
    );
    assert_eq!(
        crate::data_flow::test_support::current_storage(&a.dfg),
        crate::data_flow::test_support::current_storage(&b.dfg)
    );
}
#[test]
fn multiple_projects_and_cached_graph_match_uncached_oracle_bytes() {
    let d = tempfile::TempDir::new().unwrap();
    for (project, target) in [("one", "a"), ("two", "b"), ("three", "c")] {
        write(d.path(),&format!("{project}/tsconfig.json"),&serde_json::json!({"compilerOptions":{"moduleResolution":"node","baseUrl":"..","paths":{"@barrel":["barrel"],"@leaf":[target]}},"include":["**/*"]}).to_string());
        write(d.path(),&format!("{project}/app.ts"),"import { real } from '@barrel';\nexport function run(value: number) { return real(value); }\n");
    }
    write(d.path(), "barrel.ts", "export { real } from '@leaf';\n");
    for leaf in ["a", "b", "c"] {
        write(
            d.path(),
            &format!("{leaf}.ts"),
            "export function real(value: number) { return value + 1; }\n",
        );
    }
    let old = load_repo(d.path()).unwrap();
    let build = || {
        CodePropertyGraph::build_enriched_with_scope_graph_inputs(
            &old.files,
            None,
            old.scope_graph_inputs.as_ref(),
        )
    };
    let candidate = crate::build_pool::install(build);
    let oracle = crate::build_pool::install(|| oracle::with_uncached(build));
    assert_eq!(candidate.call_graph.js_ts_path_exports.len(), 3);
    for (project, target) in [("one", "a"), ("two", "b"), ("three", "c")] {
        assert_eq!(
            candidate.call_graph.js_ts_path_exports[&format!("{project}/tsconfig.json")]
                ["barrel.ts"]["real"]
                .file,
            format!("{target}.ts")
        );
    }
    assert_graphs(&oracle, &candidate);
    // Config-only edit: no changed source files; cached-graph recomputation
    // must use the new snapshot and cannot retain the old project's memo.
    write(d.path(),"one/tsconfig.json",&serde_json::json!({"compilerOptions":{"moduleResolution":"node","baseUrl":"..","paths":{"@barrel":["barrel"],"@leaf":["c"]}},"include":["**/*"]}).to_string());
    let fresh = load_repo(d.path()).unwrap();
    let rebuild = |old: CodePropertyGraph| {
        CodePropertyGraph::build_incremental_with_scope_graph_inputs(
            old.call_graph,
            old.dfg,
            &BTreeSet::new(),
            &fresh.files,
            None,
            fresh.scope_graph_inputs.as_ref(),
        )
    };
    let candidate = crate::build_pool::install(|| rebuild(candidate));
    let oracle = crate::build_pool::install(|| oracle::with_uncached(|| rebuild(oracle)));
    assert_eq!(
        candidate.call_graph.js_ts_path_exports["one/tsconfig.json"]["barrel.ts"]["real"].file,
        "c.ts"
    );
    assert_graphs(&oracle, &candidate);
    let full = CodePropertyGraph::build_enriched_with_scope_graph_inputs(
        &fresh.files,
        None,
        fresh.scope_graph_inputs.as_ref(),
    );
    assert_graphs(&full, &candidate);
}

#[test]
fn repository_fixtures_match_uncached_oracle_bytes() {
    for fixture in [
        "sanitizer-suite-js-ts",
        "p8-param-slots",
        "hapi-4552-source",
    ] {
        let loaded = load_repo(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(fixture),
        )
        .unwrap();
        let build = || {
            CodePropertyGraph::build_enriched_with_scope_graph_inputs(
                &loaded.files,
                None,
                loaded.scope_graph_inputs.as_ref(),
            )
        };
        let new = crate::build_pool::install(build);
        let old = crate::build_pool::install(|| oracle::with_uncached(build));
        assert_graphs(&old, &new);
        // Direct library caller on an ordinary thread also hits both functions.
        let raw = &CallGraph::build(&loaded.files).js_ts_exports;
        assert_eq!(
            resolve(
                raw,
                &|f, s| crate::call_graph::resolve_js_ts_relative_module(
                    s,
                    f,
                    &new.call_graph.indexed_files
                ),
                raw.keys(),
                ExportProjection::Legacy
            ),
            oracle::with_uncached(|| resolve(
                raw,
                &|f, s| crate::call_graph::resolve_js_ts_relative_module(
                    s,
                    f,
                    &new.call_graph.indexed_files
                ),
                raw.keys(),
                ExportProjection::Legacy
            ))
        );
    }
}

#[test]
fn snapshot_dependency_sets_match_without_shared_state() {
    let d = tempfile::TempDir::new().unwrap();
    write(d.path(), "tsconfig.json", &serde_json::json!({"compilerOptions":{"moduleResolution":"node","allowJs":true,"baseUrl":".","paths":{"@leaf":["leaf"]}},"include":["**/*"]}).to_string());
    write(d.path(), "leaf.js", "export function real() {}\n");
    let old_snapshot = crate::js_paths_snapshot::JsPathsSnapshot::capture(d.path());
    let new_snapshot = crate::js_paths_snapshot::JsPathsSnapshot::capture(d.path());
    let mut old_resolver = crate::js_paths::Resolver::new(&old_snapshot);
    let mut new_resolver = crate::js_paths::Resolver::new(&new_snapshot);
    assert_eq!(
        old_resolver.project("app.ts").as_deref(),
        Some("tsconfig.json")
    );
    assert_eq!(
        new_resolver.project("app.ts").as_deref(),
        Some("tsconfig.json")
    );
    let leaf = JsExportFacts {
        named: (0..12)
            .map(|i| (format!("name{i}"), JsExportTarget::Local("real".into())))
            .collect(),
        ..Default::default()
    };
    let mut raw = BTreeMap::from([("leaf.js".into(), leaf)]);
    for i in 0..16 {
        raw.insert(
            format!("root{i}.ts"),
            JsExportFacts {
                star_reexports: BTreeSet::from(["@leaf".into()]),
                ..Default::default()
            },
        );
    }
    let indexed = raw.keys().cloned().collect();
    let old_hop = |from: &str, spec: &str| old_resolver.hop("tsconfig.json", from, spec, &indexed);
    let new_hop = |from: &str, spec: &str| new_resolver.hop("tsconfig.json", from, spec, &indexed);
    let before = new_snapshot.topology();
    let old = oracle::with_uncached(|| {
        resolve(&raw, &old_hop, raw.keys(), ExportProjection::CallerPaths)
    });
    assert_eq!(
        resolve(&raw, &new_hop, raw.keys(), ExportProjection::CallerPaths),
        old
    );
    assert_eq!(old.resolved["root0.ts"].len(), 12);
    assert!(new_snapshot.was_probed("leaf.d.ts"));
    assert_ne!(
        before,
        new_snapshot.topology(),
        "must exercise new dependency probes"
    );
    assert_eq!(old_snapshot.topology(), new_snapshot.topology());
}
