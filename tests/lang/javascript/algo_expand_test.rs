//! Expanded algorithm coverage tests for JavaScript language.
//!
//! Covers 2 algorithms not yet tested with JavaScript fixtures:
//! Taint, ContractSlice.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// Taint — user input flowing to dangerous sinks
// ---------------------------------------------------------------------------

#[test]
fn test_taint_javascript() {
    let source = r#"
function processInput(userInput) {
    const query = "SELECT * FROM users WHERE id = " + userInput;
    const result = db.query(query);
    return result;
}
"#;
    let path = "src/handler.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Taint);
}

// ---------------------------------------------------------------------------
// ContractSlice — guard validation
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_javascript() {
    let source = r#"
function processBuffer(buf, len) {
    if (buf === null || buf === undefined) return -1;
    if (len <= 0) return -1;
    if (len > 4096) return -1;
    const result = buf.slice(0, len);
    return result;
}
"#;
    let path = "src/process.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ContractSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ContractSlice);
}
