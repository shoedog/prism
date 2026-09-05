use prism::access_path::AccessPath;
use prism::api::{
    load_review_inputs, to_sarif, EvidenceHop, EvidencePath, FindingConfidence, FindingTier,
    ReviewInputs, ReviewOptions, SarifInputs,
};
use prism::cpg::FlowConfidence;
use prism::data_flow::{VarAccessKind, VarLocation};
use prism::slice::SliceFinding;
use prism::targets::{project, TargetsMeta};
use std::fs;
use std::path::PathBuf;
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

const SELECTED_A: &str = include_str!("../fixtures/item2-selected-evidence/a.py");
const SELECTED_B: &str = include_str!("../fixtures/item2-selected-evidence/b.py");

fn selected_location(file: &str, line: usize, kind: VarAccessKind) -> VarLocation {
    VarLocation {
        file: file.to_string(),
        function: if file == "a.py" { "source" } else { "consume" }.to_string(),
        function_start_line: 1,
        line,
        path: AccessPath::simple("query"),
        start_byte: 0,
        end_byte: 1,
        kind,
    }
}

fn selected_path() -> EvidencePath {
    let mut evidence = EvidencePath::default();
    evidence.hops.push(EvidenceHop::DataFlow {
        from: selected_location("a.py", 2, VarAccessKind::Def),
        to: selected_location("b.py", 2, VarAccessKind::Use),
        confidence: FlowConfidence::Exact,
    });
    evidence
}

fn selected_finding() -> SliceFinding {
    SliceFinding {
        algorithm: "taint".to_string(),
        file: "a.py".to_string(),
        line: 2,
        severity: "info".to_string(),
        description: "taint source: origin of tainted data at line 2".to_string(),
        function_name: Some("source".to_string()),
        related_lines: Vec::new(),
        related_files: Vec::new(),
        category: Some("taint_source".to_string()),
        parse_quality: None,
        diagrams: Vec::new(),
    }
}

fn selected_inputs(include_b: bool) -> (TempDir, ReviewInputs) {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("a.py"), SELECTED_A).unwrap();
    let mut diff_files = vec![serde_json::json!({
        "file_path": "a.py",
        "modify_type": "Modified",
        "diff_lines": [2]
    })];
    if include_b {
        fs::write(temp.path().join("b.py"), SELECTED_B).unwrap();
        diff_files.push(serde_json::json!({
            "file_path": "b.py",
            "modify_type": "Modified",
            "diff_lines": []
        }));
    }
    let diff = serde_json::json!({ "files": diff_files });
    let inputs = load_review_inputs(
        &ReviewOptions::new(temp.path()),
        &serde_json::to_string(&diff).unwrap(),
    )
    .unwrap();
    (temp, inputs)
}

fn assert_selected_file_candidate(inputs: &ReviewInputs, expected_quality: &str) {
    let findings = [selected_finding()];
    let evidence = [Some(selected_path())];
    assert!(findings[0].related_files.is_empty());

    let sarif = to_sarif(
        &SarifInputs::new(&findings)
            .evidence(&evidence)
            .parse_quality(&inputs.parse_quality)
            .files(&inputs.files)
            .sources(&inputs.sources),
    );
    let mut metadata = TargetsMeta::default();
    metadata.repo_root = PathBuf::from(".");
    let targets = project(&findings, &evidence, inputs, &metadata);
    assert_eq!(targets.targets.len(), 1, "{targets:#?}");
    let properties = &sarif["runs"][0]["results"][0]["properties"];
    assert_eq!(
        (
            properties["confidence"].as_str(),
            properties["tier"].as_str(),
            properties["parse_quality"].as_str(),
            targets.targets[0].confidence,
            targets.targets[0].tier,
            targets.targets[0].parse_quality.as_deref(),
        ),
        (
            Some("exact"),
            Some("candidate"),
            Some(expected_quality),
            FindingConfidence::Exact,
            FindingTier::Candidate,
            Some(expected_quality),
        ),
        "SARIF={sarif:#} targets={targets:#?}"
    );
    assert!(findings[0].related_files.is_empty());
}

#[test]
fn selected_evidence_degraded_endpoint_is_candidate_in_both_projections() {
    let (_temp, inputs) = selected_inputs(true);
    assert_eq!(
        inputs
            .parse_quality
            .get("b.py")
            .map(|quality| quality.quality.as_str()),
        Some("degraded"),
        "fixture must be degraded without invalidating its consume function"
    );
    assert_selected_file_candidate(&inputs, "degraded");
}

#[test]
fn selected_evidence_unknown_endpoint_is_candidate_in_both_projections() {
    let (_temp, inputs) = selected_inputs(false);
    assert!(!inputs.files.contains_key("b.py"));
    assert_selected_file_candidate(&inputs, "unknown");
}
