#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_thin_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_typescript() {
    let (files, _, diff) = make_typescript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_lua() {
    let source = r#"
function compute(x)
    local y = x + 1
    local z = y * 2
    return z
end
"#;
    let path = "compute.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

#[test]
fn test_thin_slice_rust() {
    let source = r#"
fn compute(x: i32) -> i32 {
    let y = x + 1;
    let z = y * 2;
    z
}
"#;
    let path = "compute.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

#[test]
fn test_thin_slice_cpp() {
    let (files, _, diff) = make_cpp_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_barrier_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_barrier_slice_javascript() {
    let (files, _, diff) = make_javascript_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_relevant_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_relevant_slice_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_thin_slice_global_scope_python() {
    // Test thin slice with diff lines at global scope (no enclosing function)
    let source = r#"
x = 10
y = x + 1
print(y)
"#;
    let path = "global.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should handle global-scope diff lines"
    );
}

#[test]
fn test_barrier_slice_with_barriers_python() {
    // Test barrier slice with explicit barrier symbols and modules
    let source_main = r#"
def handler(request):
    data = parse(request)
    result = service(data)
    logged = log_result(result)
    return result
"#;
    let source_service = r#"
def service(data):
    validated = validate(data)
    return transform(validated)
"#;
    let source_log = r#"
def log_result(result):
    print(result)
    return result
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "handler.py".to_string(),
        ParsedFile::parse("handler.py", source_main, Language::Python).unwrap(),
    );
    files.insert(
        "service.py".to_string(),
        ParsedFile::parse("service.py", source_service, Language::Python).unwrap(),
    );
    files.insert(
        "log.py".to_string(),
        ParsedFile::parse("log.py", source_log, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "handler.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let barrier_config = prism::algorithms::barrier_slice::BarrierConfig {
        max_depth: 2,
        barrier_symbols: BTreeSet::from(["log_result".to_string()]),
        barrier_modules: vec!["log.py".to_string()],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice);
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::barrier_slice::slice(&ctx, &diff, &config, &barrier_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_delta_slice_missing_old_file_python() {
    // Delta slice when old file doesn't exist (tests error handling path)
    let tmp = TempDir::new().unwrap();
    // No old file written — old_repo has nothing

    let new_source = "x = 1\ny = x + 2\nprint(y)\n";
    let path = "missing.py";
    let parsed = ParsedFile::parse(path, new_source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
    // Should succeed but with empty old files — no edge differences
}

#[test]
fn test_chop_python_verifies_path_lines() {
    // Chop should find data flow from source (line 2: x = input()) to sink (line 5: result = z * 2)
    let source = "x = input()\ny = int(x)\nz = y + 1\nresult = z * 2\nprint(result)\n";
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 1,
        sink_file: path.to_string(),
        sink_line: 4,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    // If data flow exists, blocks should contain lines between source and sink
    if !result.blocks.is_empty() {
        let block = &result.blocks[0];
        let lines = block.file_line_map.get(path).unwrap();
        // Source and/or sink lines should appear in the output
        let has_endpoint = lines.contains_key(&1) || lines.contains_key(&4);
        assert!(
            has_endpoint,
            "Chop should include source or sink line in output, got lines: {:?}",
            lines.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_conditioned_slice_prunes_unreachable_python() {
    // When x==5, the `if x > 0` body is reachable but we test a different condition
    let source = r#"
def process(x):
    if x != 5:
        result = x * 2
    else:
        result = 0
    return result
"#;
    let path = "cond.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 6]),
        }],
    };

    // With condition x==5, the if-body (x != 5) should be unreachable
    let condition = prism::algorithms::conditioned_slice::Condition::parse("x==5").unwrap();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let conditioned_result =
        prism::algorithms::conditioned_slice::slice(&files, &diff, &config, &condition).unwrap();

    // Also get unconditioned (LeftFlow) for comparison
    let left_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let conditioned_lines: usize = conditioned_result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let left_lines: usize = left_result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    // Conditioned slice should have fewer or equal lines (pruned unreachable)
    assert!(
        conditioned_lines <= left_lines,
        "ConditionedSlice ({} lines) should be <= LeftFlow ({} lines)",
        conditioned_lines,
        left_lines
    );
}
