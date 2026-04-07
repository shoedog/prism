//! Expanded algorithm coverage tests for TypeScript language.
//!
//! Covers 1 algorithm not yet tested with TypeScript fixtures:
//! ContractSlice.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// ContractSlice — guard validation with type narrowing
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_typescript() {
    let source = r#"
function processUser(user: any): string {
    if (!user) throw new Error("user required");
    if (typeof user.name !== "string") throw new Error("invalid name");
    if (user.age < 0) throw new Error("invalid age");
    return `${user.name} (${user.age})`;
}
"#;
    let path = "src/user.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
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
