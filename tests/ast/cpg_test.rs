#[path = "../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_without_cpg_context_runs_ast_only() {
    // CpgContext::without_cpg should work for AST-only algorithms
    let source = r#"
def add(x, y):
    return x + y
"#;
    let path = "src/add.py";
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

    let ctx = CpgContext::without_cpg(&files, None);
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&ctx, &diff, &config).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "AST-only algorithm should work with empty CPG context"
    );
}
