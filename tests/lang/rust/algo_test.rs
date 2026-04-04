#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_left_flow_rust() {
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for Rust code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

#[test]
fn test_full_flow_rust() {
    let source = r#"
fn transform(input: &str) -> String {
    let trimmed = input.trim();
    let upper = trimmed.to_uppercase();
    let result = format!("Result: {}", upper);
    result
}

fn caller() {
    let out = transform("hello");
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce blocks for Rust code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

#[test]
fn test_barrier_slice_rust_trait_dispatch() {
    let source = r#"
fn level0(x: i32) -> i32 {
    level1(x + 1)
}

fn level1(y: i32) -> i32 {
    level2(y * 2)
}

fn level2(z: i32) -> i32 {
    z + 10
}
"#;
    let path = "src/chain.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let barrier_config = prism::algorithms::barrier_slice::BarrierConfig {
        max_depth: 2,
        barrier_symbols: BTreeSet::new(),
        barrier_modules: vec![],
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::barrier_slice::slice(
        &ctx,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        &barrier_config,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_relevant_slice_rust() {
    let source = r#"
fn compute(x: i32, flag: bool) -> i32 {
    let base = x * 2;
    let adjusted = if flag {
        base + 10
    } else {
        base - 5
    };
    let result = adjusted + 1;
    result
}
"#;
    let path = "src/compute.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice should produce blocks for Rust code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_conditioned_slice_rust_match() {
    let source = r#"
fn classify(score: i32) -> &'static str {
    match score {
        90..=100 => "A",
        80..=89 => "B",
        _ => "C",
    }
}
"#;
    let path = "src/classify.rs";
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
    let condition = prism::algorithms::conditioned_slice::Condition {
        var_name: "score".to_string(),
        op: prism::algorithms::conditioned_slice::ConditionOp::Gt,
        value: "89".to_string(),
    };
    let result = prism::algorithms::conditioned_slice::slice(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        &condition,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_echo_slice_rust_result_callers() {
    let source = r#"
fn compute(x: i32) -> Result<i32, String> {
    if x < 0 {
        return Err("negative".to_string());
    }
    Ok(x * 2)
}

fn caller() {
    let val = compute(42).unwrap();
    println!("{}", val);
}

fn safe_caller() {
    match compute(10) {
        Ok(v) => println!("{}", v),
        Err(e) => eprintln!("Error: {}", e),
    }
}
"#;
    let path = "src/echo.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::echo_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

#[test]
fn test_symmetry_slice_rust_serde_pair() {
    let source = r#"
fn serialize(data: &Data) -> String {
    format!("{}", data.value)
}

fn deserialize(s: &str) -> Data {
    Data { value: s.parse().unwrap() }
}
"#;
    let path = "src/serde.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let result = prism::algorithms::symmetry_slice::slice(&files, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_horizontal_slice_rust_parse_peers() {
    let source = r#"
fn parse_header(input: &str) -> Result<String, String> {
    Ok(input.to_string())
}

fn parse_body(input: &str) -> Result<String, String> {
    Ok(input.to_string())
}

fn parse_footer(input: &str) -> Result<String, String> {
    Ok(input.to_string())
}
"#;
    let path = "src/parser.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };
    let pattern =
        prism::algorithms::horizontal_slice::PeerPattern::NamePattern("parse_".to_string());
    let result = prism::algorithms::horizontal_slice::slice(&files, &diff, &pattern).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_angle_slice_rust_error_handling() {
    let source = r#"
use std::fs;
use std::io;

fn read_config(path: &str) -> Result<String, io::Error> {
    let content = fs::read_to_string(path)?;
    Ok(content)
}

fn process_file(path: &str) -> Result<(), io::Error> {
    let data = read_config(path)?;
    let parsed = data.parse::<i32>().map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidData, e)
    })?;
    println!("Value: {}", parsed);
    Ok(())
}
"#;
    let path = "src/config.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
        }],
    };
    let concern = prism::algorithms::angle_slice::Concern::ErrorHandling;
    let result = prism::algorithms::angle_slice::slice(&files, &diff, &concern).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

#[test]
fn test_chop_rust_data_pipeline() {
    let source = r#"
fn pipeline(input: &str) -> String {
    let validated = validate(input);
    let transformed = transform(&validated);
    let result = format!("output: {}", transformed);
    result
}

fn validate(s: &str) -> String {
    s.trim().to_string()
}

fn transform(s: &str) -> String {
    s.to_uppercase()
}
"#;
    let path = "src/pipeline.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}
