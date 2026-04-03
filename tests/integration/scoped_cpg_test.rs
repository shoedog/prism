#[path = "../common/mod.rs"]
mod common;
use common::*;

use prism::cpg::CpgContext;

/// Create a multi-file Python fixture where:
/// - `src/api.py` calls `src/db.py:fetch()` and `src/utils.py:validate()`
/// - `src/db.py` calls `src/cache.py:get_cache()`
/// - `src/utils.py` is standalone
/// - `src/cache.py` is standalone
/// - `src/unrelated.py` has no connections to the others
///
/// Diff touches only `src/api.py`.
fn make_multi_file_fixture() -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let api_source = r#"
def handle_request(req):
    data = validate(req.body)
    result = fetch(data)
    return result

def other_endpoint():
    return "ok"
"#;

    let db_source = r#"
def fetch(query):
    cached = get_cache(query)
    if cached:
        return cached
    return run_query(query)

def run_query(q):
    return {"result": q}
"#;

    let utils_source = r#"
def validate(data):
    if not data:
        raise ValueError("empty")
    return data

def format_output(val):
    return str(val)
"#;

    let cache_source = r#"
def get_cache(key):
    return None

def clear_cache():
    pass
"#;

    let unrelated_source = r#"
def completely_unrelated():
    x = 1
    y = 2
    return x + y

def also_unrelated():
    return 42
"#;

    let logging_source = r#"
def setup_logging():
    return None

def log_message(msg):
    pass
"#;

    let config_source = r#"
def load_config():
    return {}

def save_config(data):
    pass
"#;

    let metrics_source = r#"
def record_metric(name, value):
    pass

def get_metrics():
    return []
"#;

    let auth_source = r#"
def authenticate(token):
    return True

def authorize(user, role):
    return True
"#;

    let mut files = BTreeMap::new();
    for (path, source, lang) in [
        ("src/api.py", api_source, Language::Python),
        ("src/db.py", db_source, Language::Python),
        ("src/utils.py", utils_source, Language::Python),
        ("src/cache.py", cache_source, Language::Python),
        ("src/unrelated.py", unrelated_source, Language::Python),
        ("src/logging.py", logging_source, Language::Python),
        ("src/config.py", config_source, Language::Python),
        ("src/metrics.py", metrics_source, Language::Python),
        ("src/auth.py", auth_source, Language::Python),
    ] {
        let parsed = ParsedFile::parse(path, source, lang).unwrap();
        files.insert(path.to_string(), parsed);
    }

    // Diff: only src/api.py is changed (lines 2-5: handle_request body)
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/api.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3, 4, 5]),
        }],
    };

    (files, diff)
}

#[test]
fn test_scoped_cpg_includes_changed_files() {
    let (files, diff) = make_multi_file_fixture();
    let ctx = CpgContext::build_scoped(&files, &diff, None);

    // With 5 files and the scope likely being <= 50%, scoping should activate.
    // The scope should include src/api.py (changed).
    let scope = ctx.scope.as_ref().expect("should be scoped");
    assert!(
        scope.changed_files.contains("src/api.py"),
        "changed files should include src/api.py"
    );
    assert!(
        scope.scoped_files.contains("src/api.py"),
        "scoped files should include src/api.py"
    );
}

#[test]
fn test_scoped_cpg_includes_callees() {
    let (files, diff) = make_multi_file_fixture();
    let ctx = CpgContext::build_scoped(&files, &diff, None);

    // api.py calls validate() (in utils.py) and fetch() (in db.py).
    // Both should be in scope as Tier 2 (direct callees).
    let scope = ctx.scope.as_ref().expect("should be scoped");
    assert!(
        scope.scoped_files.contains("src/db.py"),
        "scoped files should include src/db.py (callee of api.py)"
    );
    assert!(
        scope.scoped_files.contains("src/utils.py"),
        "scoped files should include src/utils.py (callee of api.py)"
    );
}

#[test]
fn test_scoped_cpg_excludes_unrelated() {
    let (files, diff) = make_multi_file_fixture();
    let ctx = CpgContext::build_scoped(&files, &diff, None);

    let scope = ctx.scope.as_ref().expect("should be scoped");
    // unrelated.py has no calls to/from api.py
    assert!(
        !scope.scoped_files.contains("src/unrelated.py"),
        "scoped files should NOT include src/unrelated.py"
    );
}

#[test]
fn test_scoped_cpg_excludes_transitive_callees() {
    let (files, diff) = make_multi_file_fixture();
    let ctx = CpgContext::build_scoped(&files, &diff, None);

    // cache.py is called by db.py, which is called by api.py.
    // cache.py is a *transitive* callee (depth 2), not a direct callee of api.py.
    // It should NOT be in scope.
    let scope = ctx.scope.as_ref().expect("should be scoped");
    assert!(
        !scope.scoped_files.contains("src/cache.py"),
        "scoped files should NOT include src/cache.py (transitive, not direct)"
    );
}

#[test]
fn test_scoped_cpg_files_map_still_has_all_files() {
    let (files, diff) = make_multi_file_fixture();
    let ctx = CpgContext::build_scoped(&files, &diff, None);

    // ctx.files should still reference ALL parsed files, not just scoped ones.
    // AST-only algorithms need access to all files.
    assert_eq!(ctx.files.len(), 9, "ctx.files should have all 9 files");
}

#[test]
fn test_scoped_cpg_falls_back_for_large_scope() {
    // With only 2 files where 1 is changed, scope > 50% → falls back to full build.
    let (files, _, diff) = make_python_test();
    let ctx = CpgContext::build_scoped(&files, &diff, None);

    // Should fall back to full CPG (scope is None).
    assert!(
        ctx.scope.is_none(),
        "should fall back to full CPG when scope > 50% of files"
    );
}

#[test]
fn test_scoped_cpg_results_subset_of_full() {
    let (files, diff) = make_multi_file_fixture();

    // Run barrier_slice with both full and scoped CPG.
    let full_config = SliceConfig {
        algorithm: SlicingAlgorithm::BarrierSlice,
        scoped_cpg: false,
        ..Default::default()
    };
    let scoped_config = SliceConfig {
        algorithm: SlicingAlgorithm::BarrierSlice,
        scoped_cpg: true,
        ..Default::default()
    };

    let full_result = algorithms::run_slicing_compat(&files, &diff, &full_config, None).unwrap();
    let scoped_result =
        algorithms::run_slicing_compat(&files, &diff, &scoped_config, None).unwrap();

    // Scoped results should be a subset of full results (or equal).
    // Collect all (file, line) pairs from each.
    let full_lines: BTreeSet<(String, usize)> = full_result
        .blocks
        .iter()
        .flat_map(|b| {
            b.file_line_map
                .iter()
                .flat_map(|(f, lines)| lines.keys().map(move |&l| (f.clone(), l)))
        })
        .collect();
    let scoped_lines: BTreeSet<(String, usize)> = scoped_result
        .blocks
        .iter()
        .flat_map(|b| {
            b.file_line_map
                .iter()
                .flat_map(|(f, lines)| lines.keys().map(move |&l| (f.clone(), l)))
        })
        .collect();

    // Every line in scoped should also be in full.
    for line in &scoped_lines {
        assert!(
            full_lines.contains(line),
            "scoped result {:?} should be in full results",
            line
        );
    }
}

#[test]
fn test_skeleton_call_graph_has_direct_calls() {
    let (files, _diff) = make_multi_file_fixture();
    let skeleton = CallGraph::build_skeleton(&files);

    // api.py:handle_request should call validate and fetch
    assert!(
        skeleton.functions.contains_key("handle_request"),
        "skeleton should have handle_request"
    );
    assert!(
        skeleton.functions.contains_key("fetch"),
        "skeleton should have fetch"
    );
    assert!(
        skeleton.functions.contains_key("validate"),
        "skeleton should have validate"
    );

    // Check callers of fetch includes handle_request
    let fetch_callers = skeleton.callers.get("fetch");
    assert!(fetch_callers.is_some(), "fetch should have callers");
    let has_api_caller = fetch_callers
        .unwrap()
        .iter()
        .any(|site| site.caller.file == "src/api.py");
    assert!(has_api_caller, "fetch should be called from src/api.py");
}

#[test]
fn test_scoped_cpg_with_taint() {
    let (files, diff) = make_multi_file_fixture();

    // Taint should work with scoped CPG.
    let config = SliceConfig {
        algorithm: SlicingAlgorithm::Taint,
        scoped_cpg: true,
        ..Default::default()
    };

    let result = algorithms::run_slicing_compat(&files, &diff, &config, None);
    assert!(result.is_ok(), "taint should succeed with scoped CPG");
}
