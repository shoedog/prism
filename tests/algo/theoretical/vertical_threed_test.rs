#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_vertical_slice_traces_layers() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::VerticalSlice),
        None,
    )
    .unwrap();

    // Should produce at least one block showing the call chain
    // (calculate is called by process, which calls helper)
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_threed_slice_python() {
    let source =
        "def foo(x):\n    y = x + 1\n    return y\n\ndef bar():\n    r = foo(10)\n    print(r)\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def foo(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
    assert!(
        !result.blocks.is_empty(),
        "ThreeDSlice should produce blocks for functions with churn"
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_threed_slice_go() {
    let source = "package main\n\nfunc compute(n int) int {\n\tresult := n * 2\n\treturn result\n}\n\nfunc caller() {\n\tv := compute(5)\n\t_ = v\n}\n";
    let filename = "main.go";
    let tmp = create_temp_git_repo(
        filename,
        &[
            "package main\n\nfunc compute(n int) int { return n }\n",
            source,
        ],
    );

    let parsed = ParsedFile::parse(filename, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);

    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn test_vertical_slice_python() {
    let source_handler = r#"
def api_handler(request):
    data = request.json()
    result = service_process(data)
    return result
"#;
    let source_service = r#"
def service_process(data):
    validated = validate(data)
    return repo_save(validated)
"#;
    let source_repo = r#"
def repo_save(data):
    db.insert(data)
    return True
"#;
    let handler_path = "handler/api.py";
    let service_path = "service/processor.py";
    let repo_path = "repository/store.py";

    let mut files = BTreeMap::new();
    files.insert(
        handler_path.to_string(),
        ParsedFile::parse(handler_path, source_handler, Language::Python).unwrap(),
    );
    files.insert(
        service_path.to_string(),
        ParsedFile::parse(service_path, source_service, Language::Python).unwrap(),
    );
    files.insert(
        repo_path.to_string(),
        ParsedFile::parse(repo_path, source_repo, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: service_path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_vertical_slice_go() {
    let source = r#"package main

func handler(w http.ResponseWriter, r *http.Request) {
	data := parseRequest(r)
	result := service(data)
	w.Write(result)
}

func service(data string) string {
	return repository(data)
}

func repository(key string) string {
	return db.Get(key)
}
"#;
    let path = "handler/main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_vertical_slice_explicit_layers_python() {
    // Test with explicit layer ordering
    let source = r#"
def api_handler(request):
    return service_call(request.data)

def service_call(data):
    return repo_save(data)

def repo_save(data):
    return True
"#;
    let path = "handler/app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = prism::algorithms::vertical_slice::VerticalConfig {
        layers: vec![
            "Handler".to_string(),
            "Service".to_string(),
            "Repository".to_string(),
        ],
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::VerticalSlice);
}

#[test]
fn test_threed_slice_python_risk_scoring() {
    // ThreeDSlice should produce blocks sorted by risk
    let source =
        "def foo(x):\n    y = x + 1\n    return y\n\ndef bar():\n    r = foo(10)\n    print(r)\n";
    let filename = "app.py";
    let tmp = create_temp_git_repo(filename, &["def foo(x):\n    return x\n", source]);

    let parsed = ParsedFile::parse(filename, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let config = prism::algorithms::threed_slice::ThreeDConfig {
        temporal_days: 365,
        git_dir: tmp.path().to_string_lossy().to_string(),
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThreeDSlice should produce risk-scored blocks"
    );

    // The first block should contain the diff function (highest risk)
    let first_block = &result.blocks[0];
    let lines = first_block.file_line_map.get(filename);
    assert!(
        lines.is_some(),
        "First block should contain lines from the diff file"
    );
}

#[test]
fn test_vertical_slice_python_layer_detection() {
    // Vertical slice should detect layers from file paths
    let source_handler = "def api_handler(request):\n    return service_call(request.data)\n";
    let source_service = "def service_call(data):\n    return repo_save(data)\n";
    let source_repo = "def repo_save(data):\n    return True\n";

    let mut files = BTreeMap::new();
    files.insert(
        "handler/api.py".to_string(),
        ParsedFile::parse("handler/api.py", source_handler, Language::Python).unwrap(),
    );
    files.insert(
        "service/logic.py".to_string(),
        ParsedFile::parse("service/logic.py", source_service, Language::Python).unwrap(),
    );
    files.insert(
        "repository/store.py".to_string(),
        ParsedFile::parse("repository/store.py", source_repo, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "service/logic.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::vertical_slice::slice(
        &ctx,
        &diff,
        &prism::algorithms::vertical_slice::VerticalConfig::default(),
    )
    .unwrap();
    // Should produce blocks — at minimum the diff function
    assert!(
        !result.blocks.is_empty(),
        "VerticalSlice should trace layers for service function"
    );
}

#[test]
fn vertical_diagram_dedupes_overlapping_path_edges() {
    // Two diff lines in the same function should produce the same caller→callee
    // chain in path_node_ids. With dedup, the same edge appears once. Without
    // dedup, it would appear twice.
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::VerticalSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    if result.diagrams.is_empty() {
        return;
    }
    let g = &result.diagrams[0];
    let mut edge_keys: std::collections::BTreeMap<(String, String), usize> =
        std::collections::BTreeMap::new();
    for e in &g.edges {
        *edge_keys.entry((e.from.clone(), e.to.clone())).or_default() += 1;
    }
    for ((from, to), count) in &edge_keys {
        assert_eq!(
            *count, 1,
            "edge {}->{} appears {} times — dedup failed",
            from, to, count
        );
    }
}

#[test]
fn vertical_result_carries_layered_diagram() {
    // Use the same two-file layered fixture as coverage_test::fixture_vertical_layered_python:
    // handler/view.py calls service/compute.py, so VerticalSlice traces a cross-layer path.
    // Raw string literals with a leading newline match tree-sitter line numbers (line 3 = body).
    let service_src = r#"
def compute(value):
    return value * 2
"#;
    let handler_src = r#"
from service import compute

def handle(request):
    result = compute(request)
    return result
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "service/compute.py".to_string(),
        ParsedFile::parse("service/compute.py", service_src, Language::Python).unwrap(),
    );
    files.insert(
        "handler/view.py".to_string(),
        ParsedFile::parse("handler/view.py", handler_src, Language::Python).unwrap(),
    );
    // Diff touches line 3 of service_src: "    return value * 2" (line 1 is the leading blank).
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "service/compute.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::VerticalSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    // The layered two-file fixture must produce blocks — if not, the algorithm
    // or fixture has regressed.
    assert!(
        !result.blocks.is_empty(),
        "VerticalSlice must trace layers for the handler→service fixture and produce blocks"
    );

    assert_eq!(
        result.diagrams.len(),
        1,
        "VerticalSlice should produce exactly one Layered diagram"
    );
    let g = &result.diagrams[0];
    assert!(matches!(g.shape, prism::slice::GraphShape::Layered));
    assert!(!g.nodes.is_empty(), "should have nodes when blocks exist");
    assert!(!g.clusters.is_empty(), "should have at least one cluster");
    assert!(!g.mermaid.is_empty());
    assert!(g.mermaid.starts_with("flowchart TD"));
}
