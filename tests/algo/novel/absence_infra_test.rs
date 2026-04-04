#[path = "../../common/mod.rs"]
mod common;
use common::*;

// === Tier 1: Absence — Bash paired patterns ===

#[test]
fn test_absence_bash_mktemp_without_rm() {
    let source = r#"#!/bin/bash
deploy() {
    TMPFILE=$(mktemp /tmp/deploy.XXXXXX)
    echo "data" > "$TMPFILE"
    process_file "$TMPFILE"
}
"#;
    let path = "scripts/deploy.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "Absence should detect mktemp without rm cleanup"
    );
}

#[test]
fn test_absence_bash_pushd_without_popd() {
    let source = r#"#!/bin/bash
build() {
    pushd /src/project
    make all
    make install
}
"#;
    let path = "scripts/build.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "Absence should detect pushd without matching popd"
    );
}

#[test]
fn test_absence_bash_exec_fd_without_close() {
    let source = r#"#!/bin/bash
start_logging() {
    exec 3>/tmp/output.log
    echo "Starting" >&3
    do_work
    echo "Done" >&3
}
"#;
    let path = "scripts/log.sh";
    let parsed = ParsedFile::parse(path, source, Language::Bash).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "Absence should detect exec FD open without exec FD>&- close"
    );
}

// === Tier 1: Absence — Terraform S3 security patterns ===

#[test]
fn test_absence_terraform_s3_missing_encryption() {
    let source = r#"
resource "aws_s3_bucket" "data" {
  bucket = "my-data-bucket"
  acl    = "private"
}

resource "aws_s3_bucket_versioning" "data" {
  bucket = aws_s3_bucket.data.id
  versioning_configuration {
    status = "Enabled"
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
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    // S3 bucket without encryption config should be flagged
    let has_encryption_finding = result
        .findings
        .iter()
        .any(|f| f.description.contains("encryption") || f.description.contains("S3"));
    assert!(
        has_encryption_finding,
        "Absence should detect S3 bucket missing encryption configuration"
    );
}

#[test]
fn test_absence_terraform_s3_missing_public_access_block() {
    let source = r#"
resource "aws_s3_bucket" "uploads" {
  bucket = "user-uploads"
}

resource "aws_s3_bucket_server_side_encryption_configuration" "uploads" {
  bucket = aws_s3_bucket.uploads.id
  rule {
    apply_server_side_encryption_by_default {
      sse_algorithm = "aws:kms"
    }
  }
}
"#;
    let path = "storage.tf";
    let parsed = ParsedFile::parse(path, source, Language::Terraform).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    let has_public_access_finding = result
        .findings
        .iter()
        .any(|f| f.description.contains("public access") || f.description.contains("S3"));
    assert!(
        has_public_access_finding,
        "Absence should detect S3 bucket missing public access block"
    );
}
