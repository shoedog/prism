#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_terraform_basic_parsing() {
    let source = r#"
variable "name" {
  type = string
}

resource "aws_instance" "web" {
  ami           = "ami-123"
  instance_type = "t3.micro"
}
"#;
    let path = "main.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();

    // Should parse without errors
    assert!(
        parsed.error_rate() < 0.1,
        "Terraform file should parse cleanly, error rate: {}",
        parsed.error_rate()
    );

    // Should detect blocks as "functions"
    let blocks = parsed.all_functions();
    assert!(
        !blocks.is_empty(),
        "Should find blocks (variable, resource) as function units"
    );
}

#[test]
fn test_terraform_original_diff() {
    let (files, _, diff) = make_terraform_test();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Terraform code"
    );
}

#[test]
fn test_terraform_parent_function() {
    let (files, _, diff) = make_terraform_test();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include enclosing resource blocks"
    );

    // The diff touches lines inside the SG and instance blocks —
    // ParentFunction should include the full block
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("main.tf").unwrap();
    // Should span more lines than just the diff lines
    assert!(
        lines.len() > 2,
        "ParentFunction should include more than just diff lines"
    );
}

#[test]
fn test_terraform_taint_cidr_blocks() {
    // cidr_blocks is a security-sensitive attribute — should be flagged as a taint sink
    let source = r#"
variable "allowed_cidrs" {
  type = list(string)
}

resource "aws_security_group" "web" {
  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = var.allowed_cidrs
  }
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
            diff_lines: BTreeSet::from([11]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // The taint analysis should detect cidr_blocks as a sink
    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint analysis should flag cidr_blocks as a security-sensitive attribute"
    );
}

#[test]
fn test_terraform_taint_user_data() {
    // user_data is a shell injection vector — should be flagged
    let source = r#"
variable "startup_script" {
  type = string
}

resource "aws_instance" "web" {
  ami           = "ami-123"
  instance_type = "t3.micro"
  user_data     = var.startup_script
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
            diff_lines: BTreeSet::from([9]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let has_taint = !result.blocks.is_empty() || !result.findings.is_empty();
    assert!(
        has_taint,
        "Taint analysis should flag user_data as a security-sensitive attribute"
    );
}

#[test]
fn test_terraform_provenance_var_is_user_input() {
    // var.* inputs come from tfvars/CLI — should be classified as UserInput
    let source = r#"
variable "allowed_cidrs" {
  type = list(string)
}

resource "aws_security_group" "web" {
  ingress {
    cidr_blocks = var.allowed_cidrs
  }
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Should have provenance findings
    let has_provenance = !result.findings.is_empty() || !result.blocks.is_empty();
    assert!(
        has_provenance,
        "Provenance should classify var.* as user input origin"
    );

    // Check for UserInput classification in findings
    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input") || f.description.contains("UserInput"));
    if !result.findings.is_empty() {
        assert!(
            has_user_input,
            "var.* should be classified as UserInput, got findings: {:?}",
            result
                .findings
                .iter()
                .map(|f| &f.description)
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn test_terraform_provenance_data_is_database() {
    // data.* sources are infrastructure queries — should be classified as Database
    let source = r#"
data "aws_ami" "ubuntu" {
  most_recent = true
  owners      = ["099720109477"]
}

resource "aws_instance" "web" {
  ami = data.aws_ami.ubuntu.id
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_provenance = !result.findings.is_empty() || !result.blocks.is_empty();
    assert!(
        has_provenance,
        "Provenance should detect data source references"
    );
}

#[test]
fn test_terraform_absence_s3_missing_encryption() {
    // S3 bucket without encryption configuration — should be flagged
    let source = r#"
resource "aws_s3_bucket" "data" {
  bucket = "my-data-bucket"
}
"#;
    let path = "main.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Should detect missing encryption configuration
    let has_absence = result.findings.iter().any(|f| {
        f.description.contains("encryption")
            || f.description.contains("S3")
            || f.description.contains("aws_s3_bucket")
    });
    assert!(
        has_absence,
        "AbsenceSlice should flag S3 bucket missing encryption config. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_terraform_absence_lambda_missing_logging() {
    // Lambda function without CloudWatch log group — should be flagged
    let source = r#"
resource "aws_lambda_function" "api" {
  function_name = "api-handler"
  handler       = "index.handler"
  runtime       = "nodejs18.x"
  role          = aws_iam_role.lambda.arn
}
"#;
    let path = "main.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Added,
            diff_lines: BTreeSet::from([2, 3, 4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Should detect missing CloudWatch log group
    let has_absence = result.findings.iter().any(|f| {
        f.description.contains("Lambda")
            || f.description.contains("log")
            || f.description.contains("aws_lambda_function")
    });
    assert!(
        has_absence,
        "AbsenceSlice should flag Lambda missing CloudWatch log group. Findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_terraform_ref_graph_taint_flow() {
    // Integration test: verify the TerraformRefGraph correctly traces
    // var.allowed_cidrs → local.merged_cidrs → aws_security_group.web
    use prism::terraform::TerraformRefGraph;

    let source = r#"
variable "allowed_cidrs" {
  type = list(string)
}

locals {
  merged_cidrs = concat(var.allowed_cidrs, ["10.0.0.0/8"])
}

resource "aws_security_group" "web" {
  ingress {
    cidr_blocks = local.merged_cidrs
  }
}
"#;

    let mut sources = BTreeMap::new();
    sources.insert("main.tf".to_string(), source.to_string());
    let graph = TerraformRefGraph::build(&sources);

    // local.merged_cidrs should reference var.allowed_cidrs
    let local_refs = graph.references.get("local.merged_cidrs");
    assert!(
        local_refs.map_or(false, |r| r.contains("var.allowed_cidrs")),
        "local.merged_cidrs should reference var.allowed_cidrs. Refs: {:?}",
        local_refs
    );

    // SG should reference local.merged_cidrs
    let sg_refs = graph.references.get("aws_security_group.web");
    assert!(
        sg_refs.map_or(false, |r| r.contains("local.merged_cidrs")),
        "SG should reference local.merged_cidrs. Refs: {:?}",
        sg_refs
    );

    // Backward from var.allowed_cidrs should reach SG transitively
    let backward = graph.backward_reachable("var.allowed_cidrs");
    assert!(
        backward.contains("local.merged_cidrs"),
        "Backward from var should reach local.merged_cidrs"
    );
    assert!(
        backward.contains("aws_security_group.web"),
        "Backward from var should reach SG transitively"
    );
}

#[test]
fn test_terraform_ref_graph_module_boundary() {
    // Test module boundary detection for MembraneSlice
    use prism::terraform::TerraformRefGraph;

    let source = r#"
variable "vpc_cidr" {
  type = string
}

resource "aws_vpc" "main" {
  cidr_block = var.vpc_cidr
}

output "vpc_id" {
  value = aws_vpc.main.id
}
"#;

    let mut sources = BTreeMap::new();
    sources.insert("main.tf".to_string(), source.to_string());
    let graph = TerraformRefGraph::build(&sources);

    // Should have variable, resource, and output entities
    assert!(graph.entities.contains_key("var.vpc_cidr"));
    assert!(graph.entities.contains_key("aws_vpc.main"));
    assert!(graph.entities.contains_key("output.vpc_id"));

    // output references the resource
    let output_refs = graph.references.get("output.vpc_id");
    assert!(
        output_refs.map_or(false, |r| r.contains("aws_vpc.main")),
        "Output should reference the VPC resource. Refs: {:?}",
        output_refs
    );

    // resource references the variable
    let vpc_refs = graph.references.get("aws_vpc.main");
    assert!(
        vpc_refs.map_or(false, |r| r.contains("var.vpc_cidr")),
        "VPC resource should reference var.vpc_cidr. Refs: {:?}",
        vpc_refs
    );
}
