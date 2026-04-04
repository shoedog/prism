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

// === Tier 2: Absence — JS timers (item 7) ===

#[test]
fn test_absence_js_setinterval_without_clear() {
    let source = r#"
function startPolling(url) {
    const intervalId = setInterval(function() {
        fetch(url);
    }, 5000);
    return intervalId;
}
"#;
    let path = "src/poller.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "Absence should detect setInterval without clearInterval"
    );
}

// === Tier 2: Absence — DB transactions (item 8) ===

#[test]
fn test_absence_python_transaction_without_commit() {
    let source = r#"
def update_records(db, items):
    db.beginTransaction()
    for item in items:
        db.execute("UPDATE t SET v=? WHERE id=?", item)
    return True
"#;
    let path = "db/update.py";
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "Absence should detect beginTransaction without commit/rollback"
    );
}

// === Tier 2: Absence — Go context.WithTimeout (item 9) ===

#[test]
fn test_absence_go_context_with_timeout_without_cancel() {
    let source = r#"
package main

import (
    "context"
    "time"
)

func fetchData(parent context.Context) string {
    ctx, cancel := context.WithTimeout(parent, 5*time.Second)
    result := query(ctx)
    return result
}
"#;
    let path = "cmd/fetch.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10]),
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
        "Absence should detect context.WithTimeout without cancel()"
    );
}

// === Tier 2: Absence — Event subscribe/unsubscribe (item 15) ===

#[test]
fn test_absence_js_addeventlistener_without_remove() {
    let source = r#"
function setup(element) {
    const handler = function(e) { process(e); };
    element.addEventListener('click', handler);
    return handler;
}
"#;
    let path = "src/events.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.findings.is_empty(),
        "Absence should detect addEventListener without removeEventListener"
    );
}
