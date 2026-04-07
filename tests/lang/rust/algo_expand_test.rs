//! Expanded algorithm coverage tests for Rust language.
//!
//! Covers 8 algorithms not yet tested with Rust fixtures:
//! DeltaSlice, SpiralSlice, CircularSlice, VerticalSlice,
//! ThreeDSlice, ResonanceSlice, GradientSlice, PhantomSlice.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// DeltaSlice — changed function between versions
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_rust() {
    let tmp = TempDir::new().unwrap();

    let old_source = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n";
    std::fs::write(tmp.path().join("calc.rs"), old_source).unwrap();

    let new_source = "fn add(a: i32, b: i32) -> i32 {\n    let result = a + b;\n    result\n}\n";
    let path = "calc.rs";
    let parsed = ParsedFile::parse(path, new_source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

// ---------------------------------------------------------------------------
// SpiralSlice — adaptive depth around changed function
// ---------------------------------------------------------------------------

#[test]
fn test_spiral_slice_rust() {
    let source = r#"
fn process(data: &str) -> i32 {
    let val = data.len();
    let result = val * 2;
    result
}

fn caller() {
    let out = process("hello");
    println!("{}", out);
}
"#;
    let path = "src/lib.rs";
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

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SpiralSlice);
    let spiral_config = prism::algorithms::spiral_slice::SpiralConfig {
        max_ring: 4,
        auto_stop_threshold: 0.0,
    };
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::spiral_slice::slice(&ctx, &diff, &config, &spiral_config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SpiralSlice);
}

// ---------------------------------------------------------------------------
// CircularSlice — mutual recursion
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_rust() {
    let source = r#"
fn is_even(n: u32) -> bool {
    if n == 0 { return true; }
    is_odd(n - 1)
}

fn is_odd(n: u32) -> bool {
    if n == 0 { return false; }
    is_even(n - 1)
}
"#;
    let path = "src/mutual.rs";
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

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::circular_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — end-to-end feature path
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_rust() {
    let source_handler = r#"
fn handle_request(id: &str) -> String {
    let data = fetch_data(id);
    format!("Result: {}", data)
}
"#;
    let source_service = r#"
fn fetch_data(id: &str) -> String {
    format!("data-{}", id)
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/handler.rs".to_string(),
        ParsedFile::parse("src/handler.rs", source_handler, Language::Rust).unwrap(),
    );
    files.insert(
        "src/service.rs".to_string(),
        ParsedFile::parse("src/service.rs", source_service, Language::Rust).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/handler.rs".to_string(),
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

// ---------------------------------------------------------------------------
// ThreeDSlice — temporal-structural risk (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_threed_slice_rust() {
    let source_v1 = "fn calc(x: i32) -> i32 {\n    x\n}\n";
    let source_v2 = "fn calc(x: i32) -> i32 {\n    x + 1\n}\n";
    let filename = "calc.rs";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let ctx = CpgContext::build(&files, None);
    let config = prism::algorithms::threed_slice::ThreeDConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        temporal_days: 365,
    };
    let result = prism::algorithms::threed_slice::slice(&ctx, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ThreeDSlice);
}

// ---------------------------------------------------------------------------
// ResonanceSlice — git co-change coupling
// ---------------------------------------------------------------------------

#[test]
fn test_resonance_slice_rust() {
    let source_v1 = "fn init() {}\n";
    let source_v2 = "fn init() {\n    setup();\n}\n";
    let filename = "init.rs";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let config = prism::algorithms::resonance_slice::ResonanceConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        days: 365,
        min_co_changes: 1,
        min_ratio: 0.0,
    };
    let result = prism::algorithms::resonance_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ResonanceSlice);
}

// ---------------------------------------------------------------------------
// GradientSlice — continuous relevance scoring
// ---------------------------------------------------------------------------

#[test]
fn test_gradient_slice_rust() {
    let source = r#"
fn process(data: &str) -> i32 {
    let val = data.len();
    let result = val * 2;
    result
}
"#;
    let path = "src/lib.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

// ---------------------------------------------------------------------------
// PhantomSlice — recently deleted code (requires git)
// ---------------------------------------------------------------------------

#[test]
fn test_phantom_slice_rust() {
    let source_v1 = "fn cleanup() {\n    drop_all();\n}\nfn work() {\n    cleanup();\n}\n";
    let source_v2 = "fn work() {\n    // simplified\n}\n";
    let filename = "phantom.rs";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: filename.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let config = prism::algorithms::phantom_slice::PhantomConfig {
        git_dir: tmp.path().to_string_lossy().to_string(),
        max_commits: 50,
    };
    let result = prism::algorithms::phantom_slice::slice(&files, &diff, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::PhantomSlice);
}
