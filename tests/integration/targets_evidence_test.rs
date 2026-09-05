use prism::api::{load_review_inputs, FindingConfidence, FindingTier, ReviewOptions};
use prism::slice::SliceFinding;
use prism::targets::{project, TargetsMeta};
use std::fs;
use tempfile::TempDir;

#[test]
fn projection_missing_evidence_warns_and_is_never_empty_exact() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.py"), "def read():\n    return 1\n").unwrap();
    let diff = serde_json::json!({
        "files": [{"file_path": "a.py", "modify_type": "Modified", "diff_lines": [2]}]
    });
    let inputs = load_review_inputs(
        &ReviewOptions::new(temp.path()),
        &serde_json::to_string(&diff).unwrap(),
    )
    .unwrap();
    let finding = SliceFinding {
        algorithm: "absence".to_string(),
        file: "a.py".to_string(),
        line: 2,
        severity: "warning".to_string(),
        description: "future pair".to_string(),
        function_name: Some("read".to_string()),
        related_lines: Vec::new(),
        related_files: Vec::new(),
        category: Some("missing_counterpart".to_string()),
        parse_quality: None,
        diagrams: Vec::new(),
    };
    let mut metadata = TargetsMeta::default();
    metadata.repo_root = temp.path().to_path_buf();

    let doc = project(&[finding], &[], &inputs, &metadata);

    assert_eq!(doc.targets[0].confidence, FindingConfidence::Unlabeled);
    assert_eq!(doc.targets[0].tier, FindingTier::Candidate);
    assert!(doc
        .warnings
        .iter()
        .any(|warning| warning.contains("evidence alignment mismatch")));
}
