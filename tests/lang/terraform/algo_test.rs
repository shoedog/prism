#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ── Paper algorithms ──

#[test]
fn test_left_flow_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for Terraform code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

#[test]
fn test_full_flow_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce blocks for Terraform code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

// ── Taxonomy algorithms ──

#[test]
fn test_thin_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for Terraform code"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

// ── Novel algorithms ──

#[test]
fn test_symmetry_slice_terraform() {
    let source = r#"
resource "aws_security_group" "web" {
  name = "web-sg"

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
"#;
    let path = "sg.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff on the ingress block
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6, 7, 8, 9]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
    // SymmetrySlice matches function-name symmetric pairs (serialize/deserialize, etc.).
    // Terraform ingress/egress are not in the default pair list, but the algorithm
    // should still run cleanly without errors on Terraform input.
}

// ── Theoretical algorithms ──

#[test]
fn test_horizontal_slice_terraform() {
    let source = r#"
resource "aws_s3_bucket" "logs" {
  bucket = "my-logs"
}

resource "aws_s3_bucket" "data" {
  bucket = "my-data"
}

resource "aws_s3_bucket" "backups" {
  bucket = "my-backups"
}
"#;
    let path = "buckets.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff on the first bucket
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
    // HorizontalSlice should find peer resource blocks with similar structure
    let has_peers = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_peers,
        "HorizontalSlice should detect peer S3 bucket resources"
    );
}

#[test]
fn test_membrane_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::MembraneSlice);
    // Membrane should run without error; it may or may not produce findings
    // for a single-file fixture (module boundary impact is cross-file)
}

#[test]
fn test_angle_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
    // AngleSlice uses ErrorHandling concern by default. The Terraform fixture
    // may not contain error-handling keywords, but the algorithm should run
    // cleanly on Terraform input without errors.
}

#[test]
fn test_quantum_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
    // Terraform has no async patterns, but QuantumSlice should still
    // run cleanly and return a valid result
}
