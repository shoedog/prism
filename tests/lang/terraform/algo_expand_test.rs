//! Expanded algorithm coverage tests for Terraform language.
//!
//! Covers 14 algorithms not yet tested with Terraform fixtures:
//! BarrierSlice, RelevantSlice, ConditionedSlice, DeltaSlice, SpiralSlice,
//! CircularSlice, VerticalSlice, ThreeDSlice, ResonanceSlice, GradientSlice,
//! PhantomSlice, EchoSlice, ContractSlice, Chop.

#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ---------------------------------------------------------------------------
// BarrierSlice — call depth in module references
// ---------------------------------------------------------------------------

#[test]
fn test_barrier_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

// ---------------------------------------------------------------------------
// RelevantSlice — alternate branches in conditionals
// ---------------------------------------------------------------------------

#[test]
fn test_relevant_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

// ---------------------------------------------------------------------------
// ConditionedSlice — conditional resource creation
// ---------------------------------------------------------------------------

#[test]
fn test_conditioned_slice_terraform() {
    let source = r#"
variable "enable_logging" {
  type    = bool
  default = true
}

resource "aws_s3_bucket" "logs" {
  count  = var.enable_logging ? 1 : 0
  bucket = "my-logs"
}

resource "aws_s3_bucket" "data" {
  bucket = "my-data"
}
"#;
    let path = "main.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

// ---------------------------------------------------------------------------
// DeltaSlice — changed resource between versions
// ---------------------------------------------------------------------------

#[test]
fn test_delta_slice_terraform() {
    let tmp = TempDir::new().unwrap();

    let old_source = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-old\"\n  instance_type = \"t2.micro\"\n}\n";
    std::fs::write(tmp.path().join("main.tf"), old_source).unwrap();

    let new_source = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-new\"\n  instance_type = \"t3.micro\"\n}\n";
    let path = "main.tf";
    let parsed = ParsedFile::parse(path, new_source, Language::Terraform).unwrap();
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
// SpiralSlice — adaptive depth
// ---------------------------------------------------------------------------

#[test]
fn test_spiral_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
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
// CircularSlice — circular module references
// ---------------------------------------------------------------------------

#[test]
fn test_circular_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::circular_slice::slice(&ctx, &diff).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::CircularSlice);
}

// ---------------------------------------------------------------------------
// VerticalSlice — end-to-end resource dependency path
// ---------------------------------------------------------------------------

#[test]
fn test_vertical_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
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
fn test_threed_slice_terraform() {
    let source_v1 = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-old\"\n}\n";
    let source_v2 = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-new\"\n  instance_type = \"t3.micro\"\n}\n";
    let filename = "main.tf";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Terraform).unwrap();
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
fn test_resonance_slice_terraform() {
    let source_v1 = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-old\"\n}\n";
    let source_v2 = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-new\"\n}\n";
    let filename = "main.tf";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Terraform).unwrap();
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
fn test_gradient_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
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
fn test_phantom_slice_terraform() {
    let source_v1 = "resource \"aws_instance\" \"old\" {\n  ami = \"ami-123\"\n}\nresource \"aws_instance\" \"web\" {\n  ami = \"ami-456\"\n}\n";
    let source_v2 = "resource \"aws_instance\" \"web\" {\n  ami = \"ami-456\"\n}\n";
    let filename = "main.tf";
    let tmp = create_temp_git_repo(filename, &[source_v1, source_v2]);

    let parsed = ParsedFile::parse(filename, source_v2, Language::Terraform).unwrap();
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

// ---------------------------------------------------------------------------
// EchoSlice — ripple effect
// ---------------------------------------------------------------------------

#[test]
fn test_echo_slice_terraform() {
    let (files, _, diff) = make_terraform_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

// ---------------------------------------------------------------------------
// ContractSlice — guard validation
// ---------------------------------------------------------------------------

#[test]
fn test_contract_slice_terraform() {
    let source = r#"
variable "instance_type" {
  type = string

  validation {
    condition     = contains(["t3.micro", "t3.small"], var.instance_type)
    error_message = "Must be t3.micro or t3.small."
  }
}

resource "aws_instance" "web" {
  ami           = "ami-123"
  instance_type = var.instance_type
}
"#;
    let path = "validated.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
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

// ---------------------------------------------------------------------------
// Chop — data flow between source and sink
// ---------------------------------------------------------------------------

#[test]
fn test_chop_terraform() {
    let source = r#"
variable "bucket_name" {
  type = string
}

resource "aws_s3_bucket" "data" {
  bucket = var.bucket_name
}

resource "aws_s3_bucket_policy" "data_policy" {
  bucket = aws_s3_bucket.data.id
  policy = jsonencode({
    Version = "2012-10-17"
  })
}
"#;
    let path = "bucket.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 7,
        sink_file: path.to_string(),
        sink_line: 11,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}
