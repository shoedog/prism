use crate::common::*;

use prism::cpg::CodePropertyGraph;
use prism::cpg_cache::{self, CacheResult};
use prism::repo_loader;

/// Helper: extract a CPG from a CacheResult::Hit, panicking on miss.
fn expect_hit(result: CacheResult) -> CodePropertyGraph {
    match result {
        CacheResult::Hit(cpg) => cpg,
        CacheResult::PartialHit { .. } => panic!("expected Hit, got PartialHit"),
        CacheResult::Miss => panic!("expected Hit, got Miss"),
    }
}

fn write_repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, src) in files {
        let abs = dir.path().join(path);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(abs, src).unwrap();
    }
    dir
}

fn parsed_files(fixtures: &[(&str, &str, Language)]) -> BTreeMap<String, ParsedFile> {
    fixtures
        .iter()
        .map(|(path, src, lang)| {
            (
                (*path).to_string(),
                ParsedFile::parse(path, src, *lang).unwrap(),
            )
        })
        .collect()
}

fn fid_dump(fid: &prism::call_graph::FunctionId) -> String {
    format!(
        "{}:{}:{}-{}",
        fid.file, fid.name, fid.start_line, fid.end_line
    )
}

fn callsite_dump(site: &prism::call_graph::CallSite) -> String {
    format!(
        "{} -> {} line={} kind={:?} span={}-{} qual={:?} recv={:?} recovery={:?} materialized={} argc={:?} spread={} outcome={:?} origin={:?}",
        fid_dump(&site.caller),
        site.callee_name,
        site.line,
        site.kind,
        site.start_byte,
        site.end_byte,
        site.qualifier,
        site.receiver_type,
        site.receiver_recovery,
        site.receiver_materialized,
        site.arg_count,
        site.arg_spread,
        site.receiver_outcome,
        site.origin
    )
}

fn var_location_dump(var: &prism::data_flow::VarLocation) -> String {
    format!(
        "{}:{}:{}:{}:{:?}:{:?}:{}-{}",
        var.file,
        var.function,
        var.function_start_line,
        var.line,
        var.path,
        var.kind,
        var.start_byte,
        var.end_byte
    )
}

fn cpg_node_dump(node: &prism::cpg::CpgNode) -> String {
    match node {
        prism::cpg::CpgNode::Function {
            file,
            name,
            start_line,
            end_line,
            start_byte,
            end_byte,
        } => format!("fn {file}:{name}:{start_line}-{end_line}:{start_byte}-{end_byte}"),
        prism::cpg::CpgNode::Statement {
            file,
            line,
            kind,
            start_byte,
            end_byte,
        } => format!("stmt {file}:{line}:{kind:?}:{start_byte}-{end_byte}"),
        prism::cpg::CpgNode::Variable {
            path,
            file,
            function,
            function_start_line,
            line,
            access,
            start_byte,
            end_byte,
        } => format!(
            "var {file}:{function}:{function_start_line}:{line}:{path:?}:{access:?}:{start_byte}-{end_byte}"
        ),
    }
}

fn normalized_cpg_behavior(cpg: &CodePropertyGraph) -> Vec<String> {
    use petgraph::visit::EdgeRef;

    let mut out = Vec::new();
    for (name, fids) in &cpg.call_graph.functions {
        for fid in fids {
            out.push(format!("cg-fn {name} {}", fid_dump(fid)));
        }
    }
    for (caller, sites) in &cpg.call_graph.calls {
        for site in sites {
            out.push(format!(
                "cg-call {} {}",
                fid_dump(caller),
                callsite_dump(site)
            ));
        }
    }
    for (callee, sites) in &cpg.call_graph.callers {
        let mut normalized: Vec<String> = sites.iter().map(callsite_dump).collect();
        normalized.sort();
        for site in normalized {
            out.push(format!("cg-caller {callee} {site}"));
        }
    }
    out.push(format!(
        "cg-index param_slots_unknown {:?}",
        cpg.call_graph.param_slots_unknown
    ));
    out.push(format!(
        "cg-index level3_indirect_resolved {}",
        cpg.call_graph.level3_indirect_resolved
    ));
    out.push(format!(
        "cg-index static_functions {:?}",
        cpg.call_graph.static_functions
    ));
    out.push(format!("cg-index imports {:?}", cpg.call_graph.imports));
    out.push(format!("cg-index methods {:?}", cpg.call_graph.methods));
    out.push(format!(
        "cg-index method_owners {:?}",
        cpg.call_graph.method_owners
    ));
    out.push(format!(
        "cg-index method_class_span {:?}",
        cpg.call_graph.method_class_span
    ));
    out.push(format!(
        "cg-index method_class_span_ambiguous {:?}",
        cpg.call_graph.method_class_span_ambiguous
    ));
    out.push(format!(
        "cg-index class_bases {:?}",
        cpg.call_graph.class_bases
    ));
    out.push(format!(
        "cg-index clean_class_spans {:?}",
        cpg.call_graph.clean_class_spans
    ));
    out.push(format!(
        "cg-index methods_by_scope {:?}",
        cpg.call_graph.methods_by_scope
    ));
    out.push(format!(
        "cg-index extension_methods {:?}",
        cpg.call_graph.extension_methods
    ));
    out.push(format!(
        "cg-index identity_complete {:?}",
        cpg.call_graph.identity_complete
    ));
    out.push(format!(
        "cg-index field_types {:?}",
        cpg.call_graph.field_types
    ));
    out.push(format!(
        "cg-index return_types {:?}",
        cpg.call_graph.return_types
    ));
    out.push(format!(
        "cg-index receiver_vars {:?}",
        cpg.call_graph.receiver_vars
    ));
    out.push(format!(
        "cg-index promoted_aliases {:?}",
        cpg.call_graph.promoted_aliases
    ));
    out.push(format!(
        "cg-index embedding_gaps {:?}",
        cpg.call_graph.embedding_gaps
    ));
    out.push(format!(
        "cg-index interface_impls {:?}",
        cpg.call_graph.interface_impls
    ));
    out.push(format!(
        "cg-index interface_gaps {:?}",
        cpg.call_graph.interface_gaps
    ));
    out.push(format!(
        "cg-index interface_overapprox {:?}",
        cpg.call_graph.interface_overapprox
    ));
    out.push(format!(
        "cg-index interface_method_names {:?}",
        cpg.call_graph.interface_method_names
    ));
    out.push(format!(
        "cg-index interface_dispatch_computed {}",
        cpg.call_graph.interface_dispatch_computed
    ));
    out.push(format!(
        "cg-index method_arity {:?}",
        cpg.call_graph.method_arity
    ));
    out.push(format!(
        "cg-index method_facts {:?}",
        cpg.call_graph.method_facts
    ));
    out.push(format!(
        "cg-index scope_graph {:?}",
        cpg.call_graph.scope_graph
    ));
    out.push(format!(
        "cg-index import_bindings {:?}",
        cpg.call_graph.import_bindings
    ));
    out.push(format!(
        "cg-index module_bindings {:?}",
        cpg.call_graph.module_bindings
    ));
    out.push(format!(
        "cg-index indexed_files {:?}",
        cpg.call_graph.indexed_files
    ));
    for edge in cpg.graph.edge_references() {
        if let prism::cpg::CpgEdge::Call(confidence) = edge.weight() {
            out.push(format!(
                "cpg-call-edge {} -> {} {:?}",
                cpg_node_dump(cpg.node(edge.source())),
                cpg_node_dump(cpg.node(edge.target())),
                confidence
            ));
        }
    }
    for edge in &cpg.dfg.edges {
        out.push(format!(
            "dfg-edge {} -> {}",
            var_location_dump(&edge.from),
            var_location_dump(&edge.to)
        ));
    }
    for (key, vars) in &cpg.dfg.defs {
        let mut normalized: Vec<String> = vars.iter().map(var_location_dump).collect();
        normalized.sort();
        for var in normalized {
            out.push(format!("dfg-def {key:?} {var}"));
        }
    }
    for (key, vars) in &cpg.dfg.uses {
        let mut normalized: Vec<String> = vars.iter().map(var_location_dump).collect();
        normalized.sort();
        for var in normalized {
            out.push(format!("dfg-use {key:?} {var}"));
        }
    }
    out.sort();
    out
}

fn assert_incremental_matches_full(
    v1: BTreeMap<String, ParsedFile>,
    v2: BTreeMap<String, ParsedFile>,
    changed_files: BTreeSet<String>,
) -> CodePropertyGraph {
    let full_v1 = CodePropertyGraph::build(&v1);
    let full_v2 = CodePropertyGraph::build(&v2);
    let scope_inputs = prism::call_graph::ScopeGraphBuildInputs::from_files_convention(&v2);
    let incremental = CodePropertyGraph::build_incremental_with_scope_graph_inputs(
        full_v1.call_graph.clone(),
        full_v1.dfg.clone(),
        &changed_files,
        &v2,
        None,
        Some(&scope_inputs),
    );

    assert_eq!(
        normalized_cpg_behavior(&full_v2),
        normalized_cpg_behavior(&incremental),
        "incremental CPG should match full build for changed files {changed_files:?}"
    );
    incremental
}

fn has_indirect_call(cpg: &CodePropertyGraph, caller: &str, callee: &str) -> bool {
    cpg.call_graph.calls.values().flatten().any(|site| {
        site.caller.name == caller
            && site.callee_name == callee
            && site.origin == prism::call_graph::CallSiteOrigin::IndirectResolution
    })
}

fn call_site(cpg: &CodePropertyGraph, caller: &str, callee: &str) -> prism::call_graph::CallSite {
    cpg.call_graph
        .calls
        .iter()
        .find(|(fid, _)| fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|site| site.callee_name == callee))
        .cloned()
        .unwrap_or_else(|| panic!("missing call site {caller}->{callee}"))
}

fn call_site_in_file(
    cpg: &CodePropertyGraph,
    file: &str,
    caller: &str,
    callee: &str,
) -> prism::call_graph::CallSite {
    cpg.call_graph
        .calls
        .iter()
        .find(|(fid, _)| fid.file == file && fid.name == caller)
        .and_then(|(_, sites)| sites.iter().find(|site| site.callee_name == callee))
        .cloned()
        .unwrap_or_else(|| panic!("missing call site {file}:{caller}->{callee}"))
}

fn has_indirect_call_in_file(
    cpg: &CodePropertyGraph,
    file: &str,
    caller: &str,
    callee: &str,
) -> bool {
    cpg.call_graph.calls.values().flatten().any(|site| {
        site.caller.file == file
            && site.caller.name == caller
            && site.callee_name == callee
            && site.origin == prism::call_graph::CallSiteOrigin::IndirectResolution
    })
}

// ---------------------------------------------------------------------------
// Round-trip tests: build CPG → save → load → verify identical results
// ---------------------------------------------------------------------------

#[test]
fn cache_v6_round_trips_edge_confidence() {
    use prism::cpg::CpgEdge;
    use prism::resolution::ResolutionConfidence;

    let mut sources = std::collections::BTreeMap::new();
    sources.insert(
        "src/lib.rs".to_string(),
        "struct A;\nimpl A { fn run(&self) {} }\nfn c(a: A) { a.run(); }\n".to_string(),
    );
    let mut files = std::collections::BTreeMap::new();
    for (path, src) in &sources {
        files.insert(
            path.clone(),
            ParsedFile::parse(path, src, Language::Rust).unwrap(),
        );
    }
    let ctx = CpgContext::build(&files, None);
    let count_exact = |cpg: &CodePropertyGraph| {
        cpg.graph
            .edge_weights()
            .filter(|w| matches!(w, CpgEdge::Call(ResolutionConfidence::Exact)))
            .count()
    };
    let before = count_exact(&ctx.cpg);
    assert!(
        before >= 1,
        "typed-receiver call should yield a Call(Exact) edge"
    );

    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();
    let loaded = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    assert_eq!(
        before,
        count_exact(&loaded),
        "Call(Exact) confidence must survive the v6 cache round-trip"
    );
}

#[test]
fn cache_v9_round_trips_interface_impls() {
    let mut sources = BTreeMap::new();
    sources.insert(
        "main.go".to_string(),
        "package main\n\
         type Runner interface { Go() }\n\
         type Fast struct{}\n\
         func (f Fast) Go() {}\n\
         func use() { _ = Fast{} }\n\
         func run(r Runner) { r.Go() }\n"
            .to_string(),
    );
    let mut files = BTreeMap::new();
    for (path, src) in &sources {
        files.insert(
            path.clone(),
            ParsedFile::parse(path, src, Language::Go).unwrap(),
        );
    }
    let key = ("Runner".to_string(), "Go".to_string());
    let ctx = CpgContext::build(&files, None);
    assert!(
        ctx.cpg.call_graph.interface_impls.contains_key(&key),
        "constructed Fast should populate Runner.Go before cache save"
    );

    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();
    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    assert!(
        loaded_cpg.call_graph.interface_impls.contains_key(&key),
        "Runner.Go interface impls must survive the v9 cache round-trip"
    );
}

#[test]
fn cache_v12_round_trips_scope_graph_and_rejects_v11() {
    let repo_dir = write_repo(&[
        (
            "Cargo.toml",
            "[package]\nname = \"root\"\nedition = \"2021\"\n",
        ),
        (
            "src/lib.rs",
            "mod util;\nuse crate::util::target;\nfn caller(){ target(); }\n",
        ),
        ("src/util.rs", "pub fn target(){}\n"),
    ]);
    let repo = repo_loader::load_repo(repo_dir.path()).unwrap();
    let ctx = CpgContext::build_with_scope_graph_inputs(
        &repo.files,
        None,
        repo.scope_graph_inputs.as_ref(),
    );
    assert!(
        ctx.cpg.call_graph.scope_graph.is_some(),
        "full build should populate scope_graph before cache save"
    );

    let cache_dir = TempDir::new().unwrap();
    let topology = cpg_cache::compute_topology_key(&repo.file_hashes, &repo.manifest_hashes);
    cpg_cache::save_cache_with_topology(
        &ctx.cpg,
        &repo.file_hashes,
        &topology,
        false,
        cache_dir.path(),
    )
    .unwrap();
    let loaded = expect_hit(cpg_cache::load_cache_with_topology(
        &repo.file_hashes,
        &topology,
        false,
        cache_dir.path(),
    ));
    assert!(
        loaded.call_graph.scope_graph.is_some(),
        "scope_graph must survive v12 cache round trip"
    );

    let bin = cache_dir.path().join("cpg-cache.bin");
    let mut bytes = std::fs::read(&bin).unwrap();
    bytes[0..4].copy_from_slice(&11u32.to_le_bytes());
    std::fs::write(&bin, bytes).unwrap();
    assert!(matches!(
        cpg_cache::load_cache_with_topology(&repo.file_hashes, &topology, false, cache_dir.path()),
        CacheResult::Miss
    ));
}

#[test]
fn cache_topology_key_misses_on_manifest_or_file_existence_changes() {
    let cache_dir = TempDir::new().unwrap();
    let cpg = CodePropertyGraph::empty();
    let source_hashes = BTreeMap::from([
        ("src/lib.rs".to_string(), "h-lib".to_string()),
        ("src/old.rs".to_string(), "h-old".to_string()),
    ]);
    let manifest_hashes = BTreeMap::from([("Cargo.toml".to_string(), "m1".to_string())]);
    let topology = cpg_cache::compute_topology_key(&source_hashes, &manifest_hashes);
    cpg_cache::save_cache_with_topology(&cpg, &source_hashes, &topology, false, cache_dir.path())
        .unwrap();

    let manifest_edit = cpg_cache::compute_topology_key(
        &source_hashes,
        &BTreeMap::from([("Cargo.toml".to_string(), "m2".to_string())]),
    );
    assert!(matches!(
        cpg_cache::load_cache_with_topology(
            &source_hashes,
            &manifest_edit,
            false,
            cache_dir.path()
        ),
        CacheResult::Miss
    ));

    let mut added_file_hashes = source_hashes.clone();
    added_file_hashes.insert("src/new.rs".to_string(), "h-new".to_string());
    let added_file_topology = cpg_cache::compute_topology_key(&added_file_hashes, &manifest_hashes);
    assert!(matches!(
        cpg_cache::load_cache_with_topology(
            &added_file_hashes,
            &added_file_topology,
            false,
            cache_dir.path()
        ),
        CacheResult::Miss
    ));

    let removed_file_hashes = BTreeMap::from([("src/lib.rs".to_string(), "h-lib".to_string())]);
    let removed_file_topology =
        cpg_cache::compute_topology_key(&removed_file_hashes, &manifest_hashes);
    assert!(matches!(
        cpg_cache::load_cache_with_topology(
            &removed_file_hashes,
            &removed_file_topology,
            false,
            cache_dir.path()
        ),
        CacheResult::Miss
    ));
}

#[test]
fn test_cache_round_trip_python() {
    let (files, sources, diff) = make_python_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    assert!(cache_dir.path().join("cpg-cache.bin").exists());
    assert!(cache_dir.path().join("cache-meta.json").exists());

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));

    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count(),
        "node count mismatch after round-trip"
    );
    assert_eq!(
        ctx_original.cpg.graph.edge_count(),
        loaded_cpg.graph.edge_count(),
        "edge count mismatch after round-trip"
    );

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();

    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);
    let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();

    assert_eq!(
        result_original.blocks.len(),
        result_cached.blocks.len(),
        "LeftFlow block count should be identical from cache"
    );
    for (orig, cached) in result_original
        .blocks
        .iter()
        .zip(result_cached.blocks.iter())
    {
        assert_eq!(orig.file_line_map, cached.file_line_map);
    }
}

#[test]
fn test_cache_round_trip_javascript() {
    let (files, sources, _diff) = make_javascript_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));

    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count()
    );
    assert_eq!(
        ctx_original.cpg.graph.edge_count(),
        loaded_cpg.graph.edge_count()
    );
}

#[test]
fn cache_round_trips_parameter_slot_telemetry() {
    let source = "function safe() {}\nfunction blocked(cb, cb) { cb(); }\nfunction invoke(a, cb) { cb(); }\nfunction outer() { invoke(0, safe); }\n";
    let sources = BTreeMap::from([("callbacks.js".to_string(), source.to_string())]);
    let files = parsed_files(&[("callbacks.js", source, Language::JavaScript)]);
    let ctx = CpgContext::build(&files, None);

    assert_eq!(
        ctx.cpg.call_graph.param_slots_unknown,
        BTreeMap::from([(Language::JavaScript, 1)])
    );
    assert_eq!(ctx.cpg.call_graph.level3_indirect_resolved, 1);

    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();
    let loaded = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));

    assert_eq!(
        loaded.call_graph.param_slots_unknown,
        ctx.cpg.call_graph.param_slots_unknown
    );
    assert_eq!(
        loaded.call_graph.level3_indirect_resolved,
        ctx.cpg.call_graph.level3_indirect_resolved
    );
}

#[test]
fn incremental_parameter_slot_telemetry_matches_full_build() {
    let v1 = parsed_files(&[(
        "callbacks.js",
        "function safe() {}\nfunction invoke(a, cb) { cb(); }\nfunction outer() { invoke(0, safe); }\n",
        Language::JavaScript,
    )]);
    let v2 = parsed_files(&[(
        "callbacks.js",
        "function safe() {}\nfunction blocked(cb, cb) { cb(); }\nfunction invoke(a, cb) { cb(); }\nfunction outer() { invoke(0, safe); }\n",
        Language::JavaScript,
    )]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["callbacks.js".to_string()]));

    assert_eq!(
        incremental.call_graph.param_slots_unknown,
        BTreeMap::from([(Language::JavaScript, 1)])
    );
    assert_eq!(incremental.call_graph.level3_indirect_resolved, 1);
}

#[test]
fn test_cache_round_trip_c() {
    let (files, sources, diff) = make_c_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    assert_eq!(
        ctx_original.cpg.graph.node_count(),
        loaded_cpg.graph.node_count()
    );

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint);
    let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();
    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);
    let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();
    assert_eq!(result_original.blocks.len(), result_cached.blocks.len());
}

// ---------------------------------------------------------------------------
// Cache invalidation tests
// ---------------------------------------------------------------------------

#[test]
fn test_cache_partial_hit_on_file_change() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Modify a source file → same file set, different hash → PartialHit.
    let mut modified_sources = sources.clone();
    if let Some(val) = modified_sources.values_mut().next() {
        val.push_str("\n# new comment\n");
    }
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit { changed_files, .. } => {
            assert!(
                !changed_files.is_empty(),
                "should have at least one changed file"
            );
        }
        other => panic!(
            "expected PartialHit on file content change, got {}",
            match other {
                CacheResult::Hit(_) => "Hit",
                CacheResult::Miss => "Miss",
                CacheResult::PartialHit { .. } => "PartialHit",
            }
        ),
    }
}

#[test]
fn test_cache_miss_when_no_cache_exists() {
    let cache_dir = TempDir::new().unwrap();
    let hashes = BTreeMap::new();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, false, cache_dir.path()),
            CacheResult::Miss
        ),
        "should be Miss when no cache file exists"
    );
}

#[test]
fn test_cache_miss_on_extra_file() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Add a new file → different file set → Miss (not PartialHit).
    let mut extra_sources = sources.clone();
    extra_sources.insert("extra.py".to_string(), "x = 1".to_string());
    let new_hashes = cpg_cache::compute_file_hashes(&extra_sources);

    assert!(
        matches!(
            cpg_cache::load_cache(&new_hashes, false, cache_dir.path()),
            CacheResult::Miss
        ),
        "should be Miss when file set changes (not just content)"
    );
}

// ---------------------------------------------------------------------------
// Multi-algorithm round-trip
// ---------------------------------------------------------------------------

#[test]
fn test_cache_round_trip_multiple_algorithms() {
    let (files, sources, diff) = make_python_test();

    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    let ctx_cached = CpgContext::build_with_cached_cpg(&files, loaded_cpg, None);

    let algos = vec![
        SlicingAlgorithm::LeftFlow,
        SlicingAlgorithm::FullFlow,
        SlicingAlgorithm::ThinSlice,
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::BarrierSlice,
    ];
    for algo in algos {
        let config = SliceConfig::default().with_algorithm(algo);
        let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();
        let result_cached = algorithms::run_slicing(&ctx_cached, &diff, &config).unwrap();
        assert_eq!(
            result_original.blocks.len(),
            result_cached.blocks.len(),
            "{}: block count mismatch",
            algo.name()
        );
    }
}

// ---------------------------------------------------------------------------
// Call graph and DFG preservation
// ---------------------------------------------------------------------------

#[test]
fn test_cache_preserves_call_graph() {
    let (files, sources, _diff) = make_python_test();
    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    assert_eq!(
        ctx_original.cpg.call_graph.functions.len(),
        loaded_cpg.call_graph.functions.len()
    );
    assert_eq!(
        ctx_original.cpg.call_graph.calls.len(),
        loaded_cpg.call_graph.calls.len()
    );
}

#[test]
fn cache_round_trips_python_inherited_receiver_class_spans() {
    let app =
        "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\ndef run(c: Child):\n    c.go()\n";
    let sources = BTreeMap::from([("app.py".to_string(), app.to_string())]);
    let files = parsed_files(&[("app.py", app, Language::Python)]);
    let ctx_original = CpgContext::build(&files, None);
    assert!(ctx_original
        .cpg
        .call_graph
        .clean_class_spans
        .contains_key(&("app.py".to_string(), "Child".to_string())));

    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    assert_eq!(
        ctx_original.cpg.call_graph.clean_class_spans,
        loaded_cpg.call_graph.clean_class_spans
    );
}

#[test]
fn test_cache_preserves_dfg() {
    let (files, sources, _diff) = make_python_test();
    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    let loaded_cpg = expect_hit(cpg_cache::load_cache(&hashes, false, cache_dir.path()));
    assert_eq!(ctx_original.cpg.dfg.edges.len(), loaded_cpg.dfg.edges.len());
    assert_eq!(ctx_original.cpg.dfg.defs.len(), loaded_cpg.dfg.defs.len());
}

// ---------------------------------------------------------------------------
// Cache metadata
// ---------------------------------------------------------------------------

#[test]
fn test_cache_meta_json_is_valid() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    let meta_text = std::fs::read_to_string(cache_dir.path().join("cache-meta.json")).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&meta_text).unwrap();
    assert!(meta["prism_version"].is_string());
    assert!(meta["cache_version"].is_number());
    assert!(meta["file_count"].is_number());
    assert!(meta["node_count"].is_number());
    assert!(meta["edge_count"].is_number());
    assert!(meta["cache_size_bytes"].is_number());
}

// ---------------------------------------------------------------------------
// Cache edge cases: corruption, version, empty sets
// ---------------------------------------------------------------------------

#[test]
fn test_cache_miss_on_corrupt_binary() {
    let cache_dir = TempDir::new().unwrap();
    // Write garbage to the cache file.
    std::fs::create_dir_all(cache_dir.path()).unwrap();
    std::fs::write(cache_dir.path().join("cpg-cache.bin"), b"corrupt data").unwrap();

    let hashes = BTreeMap::new();
    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, false, cache_dir.path()),
            CacheResult::Miss
        ),
        "corrupt binary should produce Miss"
    );
}

#[test]
fn test_cache_miss_on_truncated_binary() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Truncate the binary file to simulate interrupted write.
    let bin_path = cache_dir.path().join("cpg-cache.bin");
    let data = std::fs::read(&bin_path).unwrap();
    std::fs::write(&bin_path, &data[..data.len() / 2]).unwrap();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, false, cache_dir.path()),
            CacheResult::Miss
        ),
        "truncated binary should produce Miss"
    );
}

#[test]
fn test_cache_hit_with_empty_file_set() {
    // Empty hashes → save → load with same empty hashes → Hit.
    let files: BTreeMap<String, ParsedFile> = BTreeMap::new();
    let sources: BTreeMap<String, String> = BTreeMap::new();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, false, cache_dir.path()),
            CacheResult::Hit(_)
        ),
        "empty file set with matching (empty) hashes should Hit"
    );
}

#[test]
fn test_cache_miss_on_removed_file() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Remove a file from hashes → different key set → Miss.
    let fewer_hashes: BTreeMap<String, String> = BTreeMap::new();
    assert!(
        matches!(
            cpg_cache::load_cache(&fewer_hashes, false, cache_dir.path()),
            CacheResult::Miss
        ),
        "removed file (different key set) should produce Miss"
    );
}

#[test]
fn test_compute_file_hashes_deterministic() {
    let mut sources = BTreeMap::new();
    sources.insert("a.py".to_string(), "x = 1\n".to_string());
    sources.insert("b.py".to_string(), "y = 2\n".to_string());

    let h1 = cpg_cache::compute_file_hashes(&sources);
    let h2 = cpg_cache::compute_file_hashes(&sources);
    assert_eq!(h1, h2, "hashes should be deterministic");
}

#[test]
fn test_compute_file_hashes_content_sensitive() {
    let mut s1 = BTreeMap::new();
    s1.insert("a.py".to_string(), "x = 1".to_string());
    let mut s2 = BTreeMap::new();
    s2.insert("a.py".to_string(), "x = 2".to_string());

    let h1 = cpg_cache::compute_file_hashes(&s1);
    let h2 = cpg_cache::compute_file_hashes(&s2);
    assert_ne!(
        h1["a.py"], h2["a.py"],
        "different content should produce different hashes"
    );
}

// ---------------------------------------------------------------------------
// Direct unit tests for CallGraph incremental methods
// ---------------------------------------------------------------------------

#[test]
fn test_callgraph_remove_files() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let cg = CallGraph::build(&files);

    // Baseline: functions from both files exist.
    let all_fids: Vec<_> = cg.functions.values().flatten().collect();
    assert!(
        all_fids.iter().any(|f| f.file == "src/device.c"),
        "should have device.c functions"
    );
    assert!(
        all_fids.iter().any(|f| f.file == "src/handler.c"),
        "should have handler.c functions"
    );

    let mut cg = cg;
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    cg.remove_files(&exclude);

    // After removal: handler.c functions should be gone.
    let remaining_fids: Vec<_> = cg.functions.values().flatten().collect();
    assert!(
        remaining_fids.iter().all(|f| f.file != "src/handler.c"),
        "handler.c functions should be removed"
    );
    assert!(
        remaining_fids.iter().any(|f| f.file == "src/device.c"),
        "device.c functions should be preserved"
    );

    // Call sites from handler.c should be removed.
    for sites in cg.calls.values() {
        for site in sites {
            assert_ne!(
                site.caller.file, "src/handler.c",
                "calls from handler.c should be removed"
            );
        }
    }
    for sites in cg.callers.values() {
        for site in sites {
            assert_ne!(
                site.caller.file, "src/handler.c",
                "callers from handler.c should be removed"
            );
        }
    }

    // imports for handler.c should be removed.
    assert!(
        !cg.imports.contains_key("src/handler.c"),
        "imports for handler.c should be removed"
    );
}

#[test]
fn test_callgraph_merge() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let full_cg = CallGraph::build(&files);

    // Build separate CGs: one with only device.c retained, one with only handler.c.
    let mut retained = full_cg.clone();
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    retained.remove_files(&exclude);

    let handler_only = BTreeSet::from(["src/handler.c".to_string()]);
    let subset = CallGraph::build_direct_subset(&files, &handler_only);

    // Merge and verify both files' functions exist.
    retained.merge(subset);

    let all_fids: Vec<_> = retained.functions.values().flatten().collect();
    assert!(
        all_fids.iter().any(|f| f.file == "src/device.c"),
        "device.c functions should exist after merge"
    );
    assert!(
        all_fids.iter().any(|f| f.file == "src/handler.c"),
        "handler.c functions should exist after merge"
    );
}

#[test]
fn test_callgraph_build_direct_subset() {
    let (files, _sources, _diff) = make_c_multifile_test();

    let only = BTreeSet::from(["src/handler.c".to_string()]);
    let subset = CallGraph::build_direct_subset(&files, &only);

    // Should have functions only from handler.c.
    let all_fids: Vec<_> = subset.functions.values().flatten().collect();
    assert!(
        all_fids.iter().all(|f| f.file == "src/handler.c"),
        "subset should only contain handler.c functions"
    );
    assert!(
        !all_fids.is_empty(),
        "subset should contain at least one function"
    );

    // Call sites should reference handler.c as caller.
    for (caller, _) in &subset.calls {
        assert_eq!(caller.file, "src/handler.c");
    }
}

#[test]
fn test_callgraph_remove_empty_set() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let cg = CallGraph::build(&files);
    let original_count: usize = cg.functions.values().map(|v| v.len()).sum();

    let mut cg = cg;
    let exclude = BTreeSet::new();
    cg.remove_files(&exclude);

    let after_count: usize = cg.functions.values().map(|v| v.len()).sum();
    assert_eq!(
        original_count, after_count,
        "removing empty set should not change anything"
    );
}

// ---------------------------------------------------------------------------
// Direct unit tests for DataFlowGraph incremental methods
// ---------------------------------------------------------------------------

#[test]
fn test_dfg_remove_files() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let dfg = DataFlowGraph::build(&files);

    let has_handler_edges = dfg
        .edges
        .iter()
        .any(|e| e.from.file == "src/handler.c" || e.to.file == "src/handler.c");

    let mut dfg = dfg;
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    dfg.remove_files(&exclude);

    // After removal: no edges involving handler.c.
    let still_has_handler = dfg
        .edges
        .iter()
        .any(|e| e.from.file == "src/handler.c" || e.to.file == "src/handler.c");
    assert!(
        !still_has_handler,
        "handler.c edges should be removed from DFG"
    );

    // Defs and uses for handler.c should be gone.
    for (key, _) in &dfg.defs {
        assert_ne!(
            key.0, "src/handler.c",
            "defs from handler.c should be removed"
        );
    }
    for (key, _) in &dfg.uses {
        assert_ne!(
            key.0, "src/handler.c",
            "uses from handler.c should be removed"
        );
    }

    // Device.c entries should be preserved if they existed.
    let has_device = dfg.edges.iter().any(|e| e.from.file == "src/device.c");
    let device_defs = dfg.defs.keys().any(|k| k.0 == "src/device.c");
    // At least one should be true (device.c has variables).
    assert!(
        has_device || device_defs || dfg.edges.is_empty(),
        "device.c data should be preserved (or DFG was empty)"
    );
}

#[test]
fn test_dfg_merge() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let full_dfg = DataFlowGraph::build(&files);
    let full_edge_count = full_dfg.edges.len();

    // Remove handler.c, then build subset and merge.
    let mut retained = full_dfg;
    let exclude = BTreeSet::from(["src/handler.c".to_string()]);
    retained.remove_files(&exclude);

    let handler_only = BTreeSet::from(["src/handler.c".to_string()]);
    let subset = DataFlowGraph::build_subset(&files, &handler_only);

    retained.merge(subset);

    // Merged DFG should have edges from both files.
    let has_device = retained.edges.iter().any(|e| e.from.file == "src/device.c");
    let has_handler = retained
        .edges
        .iter()
        .any(|e| e.from.file == "src/handler.c");

    // At least handler.c should be present after merge (it has function calls).
    if full_edge_count > 0 {
        assert!(
            has_device || has_handler,
            "merged DFG should have edges from at least one file"
        );
    }

    // Forward/backward adjacency should be rebuilt.
    // Every edge should have a corresponding forward entry.
    for edge in &retained.edges {
        assert!(
            retained.forward.contains_key(&edge.from),
            "forward adjacency should be rebuilt after merge"
        );
    }
}

#[test]
fn test_dfg_build_subset() {
    let (files, _sources, _diff) = make_c_multifile_test();

    let only = BTreeSet::from(["src/device.c".to_string()]);
    let subset = DataFlowGraph::build_subset(&files, &only);

    // All edges should involve device.c only.
    for edge in &subset.edges {
        assert_eq!(
            edge.from.file, "src/device.c",
            "subset edges should only come from device.c"
        );
    }

    // All defs should be from device.c.
    for (key, _) in &subset.defs {
        assert_eq!(key.0, "src/device.c");
    }
}

#[test]
fn test_dfg_remove_empty_set() {
    let (files, _sources, _diff) = make_c_multifile_test();
    let dfg = DataFlowGraph::build(&files);
    let original_edge_count = dfg.edges.len();

    let mut dfg = dfg;
    let exclude = BTreeSet::new();
    dfg.remove_files(&exclude);

    assert_eq!(
        dfg.edges.len(),
        original_edge_count,
        "removing empty set should not change edge count"
    );
}

// ---------------------------------------------------------------------------
// Multi-file incremental cache tests
// ---------------------------------------------------------------------------

/// Build a 3-file Python fixture for multi-file incremental tests.
fn make_python_multifile() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let utils_src = r#"
def add(x, y):
    return x + y

def multiply(x, y):
    return x * y
"#;

    let calc_src = r#"
from utils import add, multiply

def compute(a, b):
    total = add(a, b)
    scaled = multiply(total, 2)
    return scaled
"#;

    let main_src = r#"
from calc import compute

def main():
    result = compute(3, 4)
    print(result)
    return result
"#;

    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();

    for (path, src) in [
        ("src/utils.py", utils_src),
        ("src/calc.py", calc_src),
        ("src/main.py", main_src),
    ] {
        let parsed = ParsedFile::parse(path, src, Language::Python).unwrap();
        files.insert(path.to_string(), parsed);
        sources.insert(path.to_string(), src.to_string());
    }

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/calc.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    (files, sources, diff)
}

#[test]
fn test_incremental_multifile_partial_hit() {
    let (files, sources, _diff) = make_python_multifile();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Modify TWO files to test multi-file partial hit.
    let mut modified_sources = sources.clone();
    modified_sources
        .get_mut("src/utils.py")
        .unwrap()
        .push_str("\n# changed\n");
    modified_sources
        .get_mut("src/main.py")
        .unwrap()
        .push_str("\n# also changed\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit { changed_files, .. } => {
            assert_eq!(changed_files.len(), 2, "should detect 2 changed files");
            assert!(changed_files.contains("src/utils.py"));
            assert!(changed_files.contains("src/main.py"));
        }
        other => panic!(
            "expected PartialHit, got {}",
            match other {
                CacheResult::Hit(_) => "Hit",
                CacheResult::Miss => "Miss",
                _ => "PartialHit",
            }
        ),
    }
}

#[test]
fn test_incremental_multifile_rebuild_matches_full() {
    let (files, sources, diff) = make_python_multifile();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Change two files.
    let mut modified_sources = sources.clone();
    modified_sources
        .get_mut("src/utils.py")
        .unwrap()
        .push_str("\n# v2\n");
    modified_sources
        .get_mut("src/calc.py")
        .unwrap()
        .push_str("\n# v2\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );

            // Node/edge count should match full build (same parsed files).
            assert_eq!(
                ctx_full.cpg.graph.node_count(),
                cpg.graph.node_count(),
                "multi-file incremental node count should match full build"
            );
            assert_eq!(
                ctx_full.cpg.graph.edge_count(),
                cpg.graph.edge_count(),
                "multi-file incremental edge count should match full build"
            );

            // Algorithm results should match.
            let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
            let ctx_inc = CpgContext::build_with_cached_cpg(&files, cpg, None);
            let res_full = algorithms::run_slicing(&ctx_full, &diff, &config).unwrap();
            let res_inc = algorithms::run_slicing(&ctx_inc, &diff, &config).unwrap();
            assert_eq!(res_full.blocks.len(), res_inc.blocks.len());
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_multifile_c_cross_file_calls() {
    let (files, sources, diff) = make_c_multifile_test();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Change handler.c (the caller) to trigger incremental rebuild.
    let mut modified_sources = sources.clone();
    modified_sources
        .get_mut("src/handler.c")
        .unwrap()
        .push_str("\n// changed\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert!(changed_files.contains("src/handler.c"));
            assert!(!changed_files.contains("src/device.c"));

            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            let ctx_inc = CpgContext::build_with_cached_cpg(&files, cpg, None);

            // Taint and BarrierSlice use the call graph — verify they produce
            // the same results from incremental as from full build.
            for algo in [SlicingAlgorithm::Taint, SlicingAlgorithm::BarrierSlice] {
                let config = SliceConfig::default().with_algorithm(algo);
                let res_full = algorithms::run_slicing(&ctx_full, &diff, &config).unwrap();
                let res_inc = algorithms::run_slicing(&ctx_inc, &diff, &config).unwrap();
                assert_eq!(
                    res_full.blocks.len(),
                    res_inc.blocks.len(),
                    "{}: block count mismatch in cross-file incremental",
                    algo.name()
                );
            }
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn incremental_matches_full_for_c_local_function_pointer_changed_caller() {
    let v1 = parsed_files(&[(
        "run.c",
        "void target(int x) { (void)x; }\nvoid run() {}\n",
        Language::C,
    )]);
    let v2 = parsed_files(&[(
        "run.c",
        r#"void target(int x) { (void)x; }
void run() {
    void (*fp)(int);
    fp = target;
    fp(1);
}
"#,
        Language::C,
    )]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["run.c".to_string()]));
    assert!(has_indirect_call(&incremental, "run", "target"));
}

#[test]
fn incremental_matches_full_for_c_local_function_pointer_changed_target() {
    let run_src = r#"extern void old_handler(int);
void run() {
    void (*fp)(int) = old_handler;
    fp(1);
}
"#;
    let v1 = parsed_files(&[
        ("run.c", run_src, Language::C),
        (
            "targets.c",
            "void old_handler(int x) { (void)x; }\n",
            Language::C,
        ),
    ]);
    let v2 = parsed_files(&[
        ("run.c", run_src, Language::C),
        (
            "targets.c",
            "void new_handler(int x) { (void)x; }\n",
            Language::C,
        ),
    ]);
    let full_v1 = CodePropertyGraph::build(&v1);
    assert!(has_indirect_call(&full_v1, "run", "old_handler"));

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["targets.c".to_string()]));
    assert!(!has_indirect_call(&incremental, "run", "old_handler"));
}

#[test]
fn incremental_matches_full_for_c_array_dispatch_same_file_table_change() {
    let v1 = parsed_files(&[(
        "dispatch.c",
        "void a() {}\nvoid b() {}\nvoid (*handlers[])() = { a };\nvoid run() { handlers[0](); }\n",
        Language::C,
    )]);
    let v2 = parsed_files(&[(
        "dispatch.c",
        "void a() {}\nvoid b() {}\nvoid (*handlers[])() = { b };\nvoid run() { handlers[0](); }\n",
        Language::C,
    )]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["dispatch.c".to_string()]));
    assert!(has_indirect_call(&incremental, "run", "b"));
    assert!(!has_indirect_call(&incremental, "run", "a"));
}

#[test]
fn incremental_matches_full_for_c_struct_field_callback_target_removal() {
    let run_src =
        "struct Device { void (*callback)(); };\nvoid run(struct Device *d) { d->callback(); }\n";
    let v1 = parsed_files(&[
        ("device.c", run_src, Language::C),
        (
            "setup.c",
            "struct Device { void (*callback)(); };\nvoid handler() {}\nvoid setup(struct Device *d) { d->callback = handler; }\n",
            Language::C,
        ),
    ]);
    let v2 = parsed_files(&[
        ("device.c", run_src, Language::C),
        (
            "setup.c",
            "struct Device { void (*callback)(); };\nvoid replacement() {}\nvoid setup(struct Device *d) { d->callback = missing_handler; }\n",
            Language::C,
        ),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["setup.c".to_string()]));
    assert!(!has_indirect_call(&incremental, "run", "handler"));
}

#[test]
fn incremental_matches_full_for_c_parameter_callback_target_removal() {
    let execute_src = "void execute(void (*cb)()) { cb(); }\n";
    let outer_src = "extern void execute(void (*cb)());\nextern void handler();\nvoid outer() { execute(handler); }\n";
    let v1 = parsed_files(&[
        ("execute.c", execute_src, Language::C),
        ("outer.c", outer_src, Language::C),
        ("targets.c", "void handler() {}\n", Language::C),
    ]);
    let v2 = parsed_files(&[
        ("execute.c", execute_src, Language::C),
        ("outer.c", outer_src, Language::C),
        ("targets.c", "void replacement() {}\n", Language::C),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["targets.c".to_string()]));
    assert!(!has_indirect_call(&incremental, "execute", "handler"));
}

#[test]
fn incremental_matches_full_for_c_struct_field_callback_assignment_change() {
    let run_src =
        "struct Device { void (*callback)(); };\nvoid run(struct Device *d) { d->callback(); }\n";
    let v1 = parsed_files(&[
        ("device.c", run_src, Language::C),
        (
            "setup.c",
            "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = old_handler; }\n",
            Language::C,
        ),
    ]);
    let v2 = parsed_files(&[
        ("device.c", run_src, Language::C),
        (
            "setup.c",
            "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = new_handler; }\n",
            Language::C,
        ),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["setup.c".to_string()]));
    assert!(has_indirect_call_in_file(
        &incremental,
        "device.c",
        "run",
        "new_handler"
    ));
    assert!(!has_indirect_call_in_file(
        &incremental,
        "device.c",
        "run",
        "old_handler"
    ));
}

#[test]
fn incremental_matches_full_for_c_parameter_callback_outer_caller_change() {
    let execute_src = "void execute(void (*cb)()) { cb(); }\n";
    let v1 = parsed_files(&[
        ("execute.c", execute_src, Language::C),
        (
            "outer.c",
            "extern void execute(void (*cb)());\nextern void old_handler();\nvoid outer() { execute(old_handler); }\n",
            Language::C,
        ),
        (
            "targets.c",
            "void old_handler() {}\nvoid new_handler() {}\n",
            Language::C,
        ),
    ]);
    let v2 = parsed_files(&[
        ("execute.c", execute_src, Language::C),
        (
            "outer.c",
            "extern void execute(void (*cb)());\nextern void new_handler();\nvoid outer() { execute(new_handler); }\n",
            Language::C,
        ),
        (
            "targets.c",
            "void old_handler() {}\nvoid new_handler() {}\n",
            Language::C,
        ),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["outer.c".to_string()]));
    assert!(has_indirect_call_in_file(
        &incremental,
        "execute.c",
        "execute",
        "new_handler"
    ));
    assert!(!has_indirect_call_in_file(
        &incremental,
        "execute.c",
        "execute",
        "old_handler"
    ));
}

#[test]
fn incremental_matches_full_for_cpp_function_pointer_changed_caller() {
    let v1 = parsed_files(&[(
        "run.cpp",
        "void target(int x) { (void)x; }\nvoid run() {}\n",
        Language::Cpp,
    )]);
    let v2 = parsed_files(&[(
        "run.cpp",
        r#"void target(int x) { (void)x; }
void run() {
    void (*fp)(int);
    fp = target;
    fp(1);
}
"#,
        Language::Cpp,
    )]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["run.cpp".to_string()]));
    assert!(has_indirect_call(&incremental, "run", "target"));
}

#[test]
fn incremental_level3_matches_full_rust_block_import_function_identity() {
    let lib_v1 = "pub mod m;\nfn safe() {}\nfn invoke(cb: fn()) { cb(); }\nfn start() {\n    { use crate::m::safe; invoke(safe); }\n}\n";
    let lib_v2 = "pub mod m;\nfn safe() {}\nfn invoke(cb: fn()) { cb(); }\nfn start() {\n    { use crate::m::safe; invoke(safe); }\n}\n// changed\n";
    let module = "pub fn safe() {}\n";
    let v1 = parsed_files(&[
        ("src/lib.rs", lib_v1, Language::Rust),
        ("src/m.rs", module, Language::Rust),
    ]);
    let v2 = parsed_files(&[
        ("src/lib.rs", lib_v2, Language::Rust),
        ("src/m.rs", module, Language::Rust),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["src/lib.rs".to_string()]));
    let site = call_site_in_file(&incremental, "src/lib.rs", "invoke", "safe");
    let resolved = incremental.call_graph.resolve_call_site_full(&site);
    assert!(matches!(
        resolved.resolved.as_slice(),
        [target]
            if target.target.file == "src/m.rs"
                && target.target.name == "safe"
                && target.confidence == prism::resolution::ResolutionConfidence::Exact
                && target.kind == prism::resolution::ResolutionKind::ParameterCallback
    ));
}

#[test]
fn incremental_level3_matches_full_js_import_function_identity() {
    let invoke = "export function invoke(cb) { cb(); }\n";
    let target = "export function safe() {}\n";
    let entry_v1 = "import { invoke } from './invoke';\nimport { safe } from './safe';\nfunction forward() { invoke(safe); }\n";
    let entry_v2 = "import { invoke } from './invoke';\nimport { safe } from './safe';\nfunction forward() { invoke(safe); }\n// changed\n";
    let v1 = parsed_files(&[
        ("invoke.js", invoke, Language::JavaScript),
        ("safe.js", target, Language::JavaScript),
        ("decoy.js", target, Language::JavaScript),
        ("entry.js", entry_v1, Language::JavaScript),
    ]);
    let v2 = parsed_files(&[
        ("invoke.js", invoke, Language::JavaScript),
        ("safe.js", target, Language::JavaScript),
        ("decoy.js", target, Language::JavaScript),
        ("entry.js", entry_v2, Language::JavaScript),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["entry.js".to_string()]));
    let site = call_site_in_file(&incremental, "invoke.js", "invoke", "safe");
    let resolved = incremental.call_graph.resolve_call_site_full(&site);
    assert!(matches!(
        resolved.resolved.as_slice(),
        [target]
            if target.target.file == "safe.js"
                && target.target.name == "safe"
                && target.confidence == prism::resolution::ResolutionConfidence::Exact
                && target.kind == prism::resolution::ResolutionKind::ParameterCallback
    ));
}

#[test]
fn incremental_matches_full_for_rust_receiver_rematerialization() {
    let lib_src = "mod model;\n\
                   use crate::model::Outer;\n\
                   pub fn run(o: Outer) { let x = o.inner; x.poke(); }\n";
    let v1 = parsed_files(&[
        ("src/lib.rs", lib_src, Language::Rust),
        (
            "src/model.rs",
            "pub struct Inner;\n\
             impl Inner { pub fn poke(&self) {} }\n\
             pub struct Inner2;\n\
             impl Inner2 { pub fn poke(&self) {} }\n\
             pub struct Outer { pub inner: Inner }\n",
            Language::Rust,
        ),
    ]);
    let v2 = parsed_files(&[
        ("src/lib.rs", lib_src, Language::Rust),
        (
            "src/model.rs",
            "pub struct Inner;\n\
             impl Inner { pub fn poke(&self) {} }\n\
             pub struct Inner2;\n\
             impl Inner2 { pub fn poke(&self) {} }\n\
             pub struct Outer { pub inner: Inner2 }\n",
            Language::Rust,
        ),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["src/model.rs".to_string()]));
    let site = call_site(&incremental, "run", "poke");
    assert!(
        site.receiver_outcome.is_some(),
        "incremental rebuild should rematerialize Rust receiver outcome"
    );
    let resolved = incremental.call_graph.resolve_call_site(&site);
    assert_eq!(
        resolved.len(),
        1,
        "receiver should disambiguate Inner2::poke"
    );
    assert_eq!(resolved[0].target.file, "src/model.rs");
    assert_eq!(resolved[0].target.start_line, 4);
}

#[test]
fn incremental_matches_full_for_mixed_recompute_ordering() {
    let rust_lib_src = "mod model;\n\
                        use crate::model::Outer;\n\
                        pub fn run_rust(o: Outer) { let x = o.inner; x.poke(); }\n";
    let c_device_src =
        "struct Device { void (*callback)(); };\nvoid run_c(struct Device *d) { d->callback(); }\n";

    let v1 = parsed_files(&[
        ("src/lib.rs", rust_lib_src, Language::Rust),
        (
            "src/model.rs",
            "pub struct Inner;\n\
             impl Inner { pub fn poke(&self) {} }\n\
             pub struct Inner2;\n\
             impl Inner2 { pub fn poke(&self) {} }\n\
             pub struct Outer { pub inner: Inner }\n",
            Language::Rust,
        ),
        (
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
            Language::Go,
        ),
        (
            "wrap.go",
            "package p\ntype Wrap struct {\n\tBase\n}\n",
            Language::Go,
        ),
        (
            "iface.go",
            "package p\ntype Runner interface { Go() }\nfunc runGo(r Runner) { r.Go() }\n",
            Language::Go,
        ),
        (
            "fast.go",
            "package p\ntype Fast struct{}\nfunc (f Fast) Go() {}\n",
            Language::Go,
        ),
        (
            "slow.go",
            "package p\ntype Slow struct{}\nfunc (s Slow) Go() {}\n",
            Language::Go,
        ),
        ("live.go", "package p\nfunc use() {}\n", Language::Go),
        ("device.c", c_device_src, Language::C),
        (
            "setup.c",
            "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = old_handler; }\n",
            Language::C,
        ),
    ]);
    let v2 = parsed_files(&[
        ("src/lib.rs", rust_lib_src, Language::Rust),
        (
            "src/model.rs",
            "pub struct Inner;\n\
             impl Inner { pub fn poke(&self) {} }\n\
             pub struct Inner2;\n\
             impl Inner2 { pub fn poke(&self) {} }\n\
             pub struct Outer { pub inner: Inner2 }\n",
            Language::Rust,
        ),
        (
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
            Language::Go,
        ),
        ("wrap.go", "package p\ntype Wrap struct{}\n", Language::Go),
        (
            "iface.go",
            "package p\ntype Runner interface { Go() }\nfunc runGo(r Runner) { r.Go() }\n",
            Language::Go,
        ),
        (
            "fast.go",
            "package p\ntype Fast struct{}\nfunc (f Fast) Go() {}\n",
            Language::Go,
        ),
        (
            "slow.go",
            "package p\ntype Slow struct{}\nfunc (s Slow) Go() {}\n",
            Language::Go,
        ),
        (
            "live.go",
            "package p\nfunc use() { _ = Fast{} }\n",
            Language::Go,
        ),
        ("device.c", c_device_src, Language::C),
        (
            "setup.c",
            "struct Device { void (*callback)(); };\nvoid old_handler() {}\nvoid new_handler() {}\nvoid setup(struct Device *d) { d->callback = new_handler; }\n",
            Language::C,
        ),
    ]);

    let incremental = assert_incremental_matches_full(
        v1,
        v2,
        BTreeSet::from([
            "src/model.rs".to_string(),
            "wrap.go".to_string(),
            "live.go".to_string(),
            "setup.c".to_string(),
        ]),
    );

    let rust_site = call_site_in_file(&incremental, "src/lib.rs", "run_rust", "poke");
    let rust_resolved = incremental.call_graph.resolve_call_site(&rust_site);
    assert_eq!(rust_resolved.len(), 1);
    assert_eq!(rust_resolved[0].target.start_line, 4);

    assert!(!incremental
        .call_graph
        .promoted_aliases
        .contains_key(&("Wrap".to_string(), "Ping".to_string())));
    let go_site = call_site_in_file(&incremental, "iface.go", "runGo", "Go");
    let go_resolved = incremental.call_graph.resolve_call_site(&go_site);
    assert_eq!(go_resolved.len(), 1);
    assert_eq!(go_resolved[0].target.file, "fast.go");

    assert!(has_indirect_call_in_file(
        &incremental,
        "device.c",
        "run_c",
        "new_handler"
    ));
    assert!(!has_indirect_call_in_file(
        &incremental,
        "device.c",
        "run_c",
        "old_handler"
    ));
}

#[test]
fn incremental_matches_full_for_go_embedding_and_interface_dispatch() {
    let v1 = parsed_files(&[
        (
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
            Language::Go,
        ),
        (
            "wrap.go",
            "package p\ntype Wrap struct {\n\tBase\n}\n",
            Language::Go,
        ),
        (
            "iface.go",
            "package p\ntype Runner interface { Go() }\nfunc run(r Runner) { r.Go() }\n",
            Language::Go,
        ),
        (
            "fast.go",
            "package p\ntype Fast struct{}\nfunc (f Fast) Go() {}\n",
            Language::Go,
        ),
        (
            "slow.go",
            "package p\ntype Slow struct{}\nfunc (s Slow) Go() {}\n",
            Language::Go,
        ),
        ("live.go", "package p\nfunc use() {}\n", Language::Go),
    ]);
    let v2 = parsed_files(&[
        (
            "base.go",
            "package p\ntype Base struct{}\nfunc (b Base) Ping() {}\n",
            Language::Go,
        ),
        ("wrap.go", "package p\ntype Wrap struct{}\n", Language::Go),
        (
            "iface.go",
            "package p\ntype Runner interface { Go() }\nfunc run(r Runner) { r.Go() }\n",
            Language::Go,
        ),
        (
            "fast.go",
            "package p\ntype Fast struct{}\nfunc (f Fast) Go() {}\n",
            Language::Go,
        ),
        (
            "slow.go",
            "package p\ntype Slow struct{}\nfunc (s Slow) Go() {}\n",
            Language::Go,
        ),
        (
            "live.go",
            "package p\nfunc use() { _ = Fast{} }\n",
            Language::Go,
        ),
    ]);

    let incremental = assert_incremental_matches_full(
        v1,
        v2,
        BTreeSet::from(["wrap.go".to_string(), "live.go".to_string()]),
    );
    assert!(!incremental
        .call_graph
        .promoted_aliases
        .contains_key(&("Wrap".to_string(), "Ping".to_string())));

    let site = call_site(&incremental, "run", "Go");
    let resolved = incremental.call_graph.resolve_call_site(&site);
    assert_eq!(
        resolved.len(),
        1,
        "RTA should prune Slow after Fast is live"
    );
    assert_eq!(resolved[0].target.file, "fast.go");
}

#[test]
fn incremental_matches_full_for_python_js_ts_import_bindings() {
    let v1 = parsed_files(&[
        (
            "app.py",
            "from utils import process\ndef run_py():\n    process()\n",
            Language::Python,
        ),
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.js",
            "import { process } from './utils';\nfunction runJs() { process(); }\n",
            Language::JavaScript,
        ),
        (
            "utils.js",
            "export function process() { return 1; }\n",
            Language::JavaScript,
        ),
        (
            "app.ts",
            "import { process } from './utils';\nfunction runTs() { process(); }\n",
            Language::TypeScript,
        ),
        (
            "utils.ts",
            "export function process() { return 1; }\n",
            Language::TypeScript,
        ),
    ]);
    let v2 = parsed_files(&[
        (
            "app.py",
            "from utils import process as work\ndef run_py():\n    work()\n",
            Language::Python,
        ),
        (
            "utils.py",
            "def process():\n    return 1\n",
            Language::Python,
        ),
        (
            "app.js",
            "import { process as workJs } from './utils';\nfunction runJs() { workJs(); }\n",
            Language::JavaScript,
        ),
        (
            "utils.js",
            "export function process() { return 1; }\n",
            Language::JavaScript,
        ),
        (
            "app.ts",
            "import { process as workTs } from './utils';\nfunction runTs() { workTs(); }\n",
            Language::TypeScript,
        ),
        (
            "utils.ts",
            "export function process() { return 1; }\n",
            Language::TypeScript,
        ),
    ]);

    let incremental = assert_incremental_matches_full(
        v1,
        v2,
        BTreeSet::from([
            "app.py".to_string(),
            "app.js".to_string(),
            "app.ts".to_string(),
        ]),
    );
    let py_bindings = incremental
        .call_graph
        .import_bindings
        .get("app.py")
        .expect("python import bindings");
    assert!(py_bindings
        .iter()
        .any(|binding| binding.local == "work" && binding.member.as_deref() == Some("process")));
    let js_bindings = incremental
        .call_graph
        .import_bindings
        .get("app.js")
        .expect("js import bindings");
    assert!(js_bindings.iter().any(|binding| binding.local == "workJs"));
    let ts_bindings = incremental
        .call_graph
        .import_bindings
        .get("app.ts")
        .expect("ts import bindings");
    assert!(ts_bindings.iter().any(|binding| binding.local == "workTs"));
}

#[test]
fn incremental_matches_full_for_python_inherited_receiver_changed_file() {
    let app_v1 = "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\nclass Other:\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n";
    let app_v2 = "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\nclass Other:\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n\ndef touched():\n    pass\n";
    let v1 = parsed_files(&[
        ("app.py", app_v1, Language::Python),
        ("util.py", "def marker():\n    return 1\n", Language::Python),
    ]);
    let v2 = parsed_files(&[
        ("app.py", app_v2, Language::Python),
        ("util.py", "def marker():\n    return 1\n", Language::Python),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["app.py".to_string()]));
    let site = call_site_in_file(&incremental, "app.py", "run", "go");
    let resolved = incremental.call_graph.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1, "{resolved:?}");
    assert_eq!(resolved[0].target.file, "app.py");
    assert_eq!(resolved[0].target.start_line, 2);
    assert_eq!(
        resolved[0].kind,
        prism::resolution::ResolutionKind::TypedParam
    );
    assert_eq!(
        resolved[0].confidence,
        prism::resolution::ResolutionConfidence::Exact
    );
    assert!(incremental
        .call_graph
        .clean_class_spans
        .contains_key(&("app.py".to_string(), "Child".to_string())));
}

#[test]
fn incremental_preserves_python_inherited_receiver_on_unrelated_change() {
    let app = "class Base:\n    def go(self):\n        pass\n\nclass Child(Base):\n    pass\n\nclass Other:\n    def go(self):\n        pass\n\ndef run(c: Child):\n    c.go()\n";
    let v1 = parsed_files(&[
        ("app.py", app, Language::Python),
        ("util.py", "def marker():\n    return 1\n", Language::Python),
    ]);
    let v2 = parsed_files(&[
        ("app.py", app, Language::Python),
        ("util.py", "def marker():\n    return 2\n", Language::Python),
    ]);

    let incremental =
        assert_incremental_matches_full(v1, v2, BTreeSet::from(["util.py".to_string()]));
    let site = call_site_in_file(&incremental, "app.py", "run", "go");
    let resolved = incremental.call_graph.resolve_call_site(&site);
    assert_eq!(resolved.len(), 1, "{resolved:?}");
    assert_eq!(resolved[0].target.file, "app.py");
    assert_eq!(resolved[0].target.start_line, 2);
    assert_eq!(
        resolved[0].kind,
        prism::resolution::ResolutionKind::TypedParam
    );
    assert!(incremental
        .call_graph
        .clean_class_spans
        .contains_key(&("app.py".to_string(), "Child".to_string())));
}

#[test]
fn test_incremental_all_files_changed_matches_full() {
    let (files, sources, diff) = make_python_multifile();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Change ALL files — still a PartialHit (same keys, all different values).
    let mut modified_sources = sources.clone();
    for val in modified_sources.values_mut() {
        val.push_str("\n# changed\n");
    }
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert_eq!(
                changed_files.len(),
                sources.len(),
                "all files should be marked changed"
            );

            // Incremental with all changed should still produce valid CPG.
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            assert_eq!(
                ctx_full.cpg.graph.node_count(),
                cpg.graph.node_count(),
                "all-changed incremental should match full build"
            );

            let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
            let ctx_inc = CpgContext::build_with_cached_cpg(&files, cpg, None);
            let res_full = algorithms::run_slicing(&ctx_full, &diff, &config).unwrap();
            let res_inc = algorithms::run_slicing(&ctx_inc, &diff, &config).unwrap();
            assert_eq!(res_full.blocks.len(), res_inc.blocks.len());
        }
        _ => panic!("expected PartialHit"),
    }
}

// ---------------------------------------------------------------------------
// Cache overwrite / re-save tests
// ---------------------------------------------------------------------------

#[test]
fn test_cache_overwrite_with_different_files() {
    let (files_py, sources_py, _diff) = make_python_test();
    let (files_c, sources_c, _diff) = make_c_multifile_test();

    let cache_dir = TempDir::new().unwrap();

    // Save Python cache.
    let ctx_py = CpgContext::build(&files_py, None);
    let hashes_py = cpg_cache::compute_file_hashes(&sources_py);
    cpg_cache::save_cache(&ctx_py.cpg, &hashes_py, false, cache_dir.path()).unwrap();
    assert!(matches!(
        cpg_cache::load_cache(&hashes_py, false, cache_dir.path()),
        CacheResult::Hit(_)
    ));

    // Overwrite with C cache.
    let ctx_c = CpgContext::build(&files_c, None);
    let hashes_c = cpg_cache::compute_file_hashes(&sources_c);
    cpg_cache::save_cache(&ctx_c.cpg, &hashes_c, false, cache_dir.path()).unwrap();

    // Old Python hashes should Miss (different file set).
    assert!(matches!(
        cpg_cache::load_cache(&hashes_py, false, cache_dir.path()),
        CacheResult::Miss
    ));

    // New C hashes should Hit.
    assert!(matches!(
        cpg_cache::load_cache(&hashes_c, false, cache_dir.path()),
        CacheResult::Hit(_)
    ));
}

// ---------------------------------------------------------------------------
// Phase 2: Incremental cache update tests (original)
// ---------------------------------------------------------------------------

#[test]
fn test_incremental_rebuild_produces_correct_results() {
    let (files, sources, diff) = make_python_test();

    // Build and cache.
    let ctx_original = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_original.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Modify a file to trigger partial hit.
    let mut modified_sources = sources.clone();
    let changed_file = modified_sources.keys().next().unwrap().clone();
    modified_sources
        .get_mut(&changed_file)
        .unwrap()
        .push_str("\n# comment\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    // Load returns PartialHit.
    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
        } => {
            assert!(changed_files.contains(&changed_file));

            // Rebuild incrementally using the original parsed files.
            // (In real usage, the changed file would be re-parsed; here we test the
            // merge logic with the same parsed files.)
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );
            let ctx_incremental = CpgContext::build_with_cached_cpg(&files, cpg, None);

            // The incremental CPG should produce the same algorithm results as a
            // full build (since the actual ParsedFiles haven't changed — only hashes differ).
            let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
            let result_original = algorithms::run_slicing(&ctx_original, &diff, &config).unwrap();
            let result_incremental =
                algorithms::run_slicing(&ctx_incremental, &diff, &config).unwrap();

            assert_eq!(
                result_original.blocks.len(),
                result_incremental.blocks.len(),
                "incremental rebuild should produce same block count"
            );
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_rebuild_node_count_matches_full_build() {
    let (files, sources, _diff) = make_python_test();

    let ctx_full = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx_full.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Trigger partial hit by changing one file hash.
    let mut modified_sources = sources.clone();
    let changed_file = modified_sources.keys().next().unwrap().clone();
    modified_sources
        .get_mut(&changed_file)
        .unwrap()
        .push_str("\nz = 99\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
            ..
        } => {
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );

            // Node and edge counts should match the full build (using same ParsedFiles).
            assert_eq!(
                ctx_full.cpg.graph.node_count(),
                cpg.graph.node_count(),
                "incremental node count should match full build"
            );
            assert_eq!(
                ctx_full.cpg.graph.edge_count(),
                cpg.graph.edge_count(),
                "incremental edge count should match full build"
            );
        }
        _ => panic!("expected PartialHit"),
    }
}

#[test]
fn test_incremental_saves_updated_cache() {
    let (files, sources, _diff) = make_python_test();

    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // After incremental rebuild, saving the new CPG with updated hashes
    // should produce a valid cache that hits on the next load.
    let mut modified_sources = sources.clone();
    let changed_file = modified_sources.keys().next().unwrap().clone();
    modified_sources
        .get_mut(&changed_file)
        .unwrap()
        .push_str("\n# v2\n");
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    match cpg_cache::load_cache(&new_hashes, false, cache_dir.path()) {
        CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            changed_files,
            ..
        } => {
            let cpg = CodePropertyGraph::build_incremental(
                cached_call_graph,
                cached_dfg,
                &changed_files,
                &files,
                None,
            );

            // Save with the new hashes.
            cpg_cache::save_cache(&cpg, &new_hashes, false, cache_dir.path()).unwrap();

            // The next load with the same new hashes should be a full Hit.
            assert!(
                matches!(
                    cpg_cache::load_cache(&new_hashes, false, cache_dir.path()),
                    CacheResult::Hit(_)
                ),
                "saved incremental cache should hit on next load"
            );
        }
        _ => panic!("expected PartialHit"),
    }
}

// ---------------------------------------------------------------------------
// Phase 3: type_db consistency tests
// ---------------------------------------------------------------------------

#[test]
fn test_cache_miss_when_type_db_added() {
    // Cache built without type_db, loaded with type_db → Miss.
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Loading with has_type_db=true should miss.
    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, true, cache_dir.path()),
            CacheResult::Miss
        ),
        "cache built without type_db should miss when type_db is now available"
    );
}

#[test]
fn test_cache_miss_when_type_db_removed() {
    // Cache built with type_db, loaded without type_db → Miss.
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, true, cache_dir.path()).unwrap();

    // Loading with has_type_db=false should miss.
    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, false, cache_dir.path()),
            CacheResult::Miss
        ),
        "cache built with type_db should miss when type_db is no longer available"
    );
}

#[test]
fn test_cache_hit_when_type_db_matches_true() {
    // Cache built with type_db=true, loaded with type_db=true → Hit.
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, true, cache_dir.path()).unwrap();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, true, cache_dir.path()),
            CacheResult::Hit(_)
        ),
        "matching type_db=true should produce Hit"
    );
}

#[test]
fn test_cache_hit_when_type_db_matches_false() {
    // Cache built with type_db=false, loaded with type_db=false → Hit.
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    assert!(
        matches!(
            cpg_cache::load_cache(&hashes, false, cache_dir.path()),
            CacheResult::Hit(_)
        ),
        "matching type_db=false should produce Hit"
    );
}

#[test]
fn test_cache_type_db_mismatch_triggers_rebuild_not_partial() {
    // Even with same file set and same hashes, type_db mismatch → Miss (not PartialHit).
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Same hashes, different type_db → should be Miss, not Hit or PartialHit.
    let result = cpg_cache::load_cache(&hashes, true, cache_dir.path());
    assert!(
        matches!(result, CacheResult::Miss),
        "type_db mismatch should force Miss even with identical file hashes"
    );
}

#[test]
fn test_cache_meta_includes_type_db_field() {
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, true, cache_dir.path()).unwrap();

    let meta_text = std::fs::read_to_string(cache_dir.path().join("cache-meta.json")).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&meta_text).unwrap();
    assert_eq!(
        meta["has_type_db"].as_bool(),
        Some(true),
        "cache-meta.json should record has_type_db"
    );
}

#[test]
fn test_incremental_partial_hit_respects_type_db() {
    // PartialHit should also check type_db consistency.
    let (files, sources, _diff) = make_python_test();
    let ctx = CpgContext::build(&files, None);
    let cache_dir = TempDir::new().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&ctx.cpg, &hashes, false, cache_dir.path()).unwrap();

    // Modify a file to make content hashes differ.
    let mut modified_sources = sources.clone();
    if let Some(val) = modified_sources.values_mut().next() {
        val.push_str("\n# changed\n");
    }
    let new_hashes = cpg_cache::compute_file_hashes(&modified_sources);

    // With same type_db=false, should be PartialHit.
    assert!(
        matches!(
            cpg_cache::load_cache(&new_hashes, false, cache_dir.path()),
            CacheResult::PartialHit { .. }
        ),
        "same type_db with changed file should be PartialHit"
    );

    // With different type_db=true, should be Miss (not PartialHit).
    assert!(
        matches!(
            cpg_cache::load_cache(&new_hashes, true, cache_dir.path()),
            CacheResult::Miss
        ),
        "different type_db with changed file should be Miss, not PartialHit"
    );
}
