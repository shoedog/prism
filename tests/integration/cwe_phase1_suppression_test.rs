//! Phase 1 sanitizer suppression-rate integration test.
//!
//! Pins acceptance criterion #2 from ACK §1: ≥80% suppression rate on the
//! in-tree 10+10 sanitizer fixture suite. Concretely:
//!   - Sanitized fixtures (`tests/fixtures/sanitizer-suite-go/sanitized/`):
//!     ≥8 of 10 produce zero `taint_sink` findings (the cleansers fire).
//!   - Unsanitized fixtures (`.../unsanitized/`): ALL 10 produce at least one
//!     `taint_sink` finding (taint flows are detected).
//!
//! See `docs/superpowers/specs/2026-04-25-phase1-cwe-go-design.md` §3.10.

#[path = "../common/mod.rs"]
mod common;
use common::*;

use std::fs;
use std::path::PathBuf;

fn fixture_dir(subdir: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/sanitizer-suite-go");
    p.push(subdir);
    p
}

fn run_taint_on_file(path: &PathBuf) -> prism::slice::SliceResult {
    let source = fs::read_to_string(path).expect("read fixture");
    let rel = path.file_name().unwrap().to_str().unwrap().to_string();
    let parsed = ParsedFile::parse(&rel, &source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(rel.clone(), parsed);

    // Diff covers all lines — any taint flow in the file should fire if the
    // cleanser doesn't suppress it.
    let line_count = source.lines().count();
    let diff_lines: BTreeSet<usize> = (1..=line_count).collect();
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: rel,
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn has_taint_sink(result: &prism::slice::SliceResult) -> bool {
    result
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink"))
}

#[test]
fn test_cwe_phase1_suppression_rate_meets_80pct() {
    // Sanitized fixtures: ≥80% should produce zero taint_sink findings.
    let sanitized_dir = fixture_dir("sanitized");
    let mut sanitized_files: Vec<PathBuf> = fs::read_dir(&sanitized_dir)
        .expect("read sanitized dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "go").unwrap_or(false))
        .collect();
    sanitized_files.sort();
    assert_eq!(
        sanitized_files.len(),
        10,
        "expected 10 sanitized fixtures, found {}",
        sanitized_files.len()
    );

    let sanitized_total = sanitized_files.len();
    let mut suppressed = 0;
    let mut leaked: Vec<String> = Vec::new();
    for f in &sanitized_files {
        let result = run_taint_on_file(f);
        if !has_taint_sink(&result) {
            suppressed += 1;
        } else {
            leaked.push(f.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    eprintln!(
        "[cwe_phase1_suppression] sanitizer suppression rate: {}/{} ({}% — pinned floor: 80%)",
        suppressed,
        sanitized_total,
        (suppressed * 100) / sanitized_total
    );
    assert!(
        suppressed >= 8,
        "≥80% suppression rate required. Got {}/10. Leaks: {:?}",
        suppressed,
        leaked
    );

    // Unsanitized fixtures: every one should produce at least one finding.
    let unsanitized_dir = fixture_dir("unsanitized");
    let mut unsanitized_files: Vec<PathBuf> = fs::read_dir(&unsanitized_dir)
        .expect("read unsanitized dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "go").unwrap_or(false))
        .collect();
    unsanitized_files.sort();
    assert_eq!(
        unsanitized_files.len(),
        10,
        "expected 10 unsanitized fixtures, found {}",
        unsanitized_files.len()
    );

    let unsanitized_total = unsanitized_files.len();
    let mut missed: Vec<String> = Vec::new();
    for f in &unsanitized_files {
        let result = run_taint_on_file(f);
        if !has_taint_sink(&result) {
            missed.push(f.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    let leaked_count = unsanitized_total - missed.len();
    eprintln!(
        "[cwe_phase1_suppression] unsanitized leakage detection: {}/{} ({}% — pinned floor: 100%)",
        leaked_count,
        unsanitized_total,
        (leaked_count * 100) / unsanitized_total
    );
    assert!(
        missed.is_empty(),
        "all unsanitized fixtures must fire. Missed: {:?}",
        missed
    );
}
