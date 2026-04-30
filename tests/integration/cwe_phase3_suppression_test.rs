//! Phase 3 JS/TS sanitizer suppression-rate integration test.
//!
//! Sanitized fixtures should suppress taint sinks; unsanitized mirrors should
//! continue to report a `taint_sink`.

#[path = "../common/mod.rs"]
mod common;
use common::*;

use std::fs;
use std::path::{Path, PathBuf};

fn fixture_dir(subdir: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("tests/fixtures/sanitizer-suite-js-ts");
    p.push(subdir);
    p
}

fn language_for(path: &Path) -> Language {
    match path.extension().and_then(|x| x.to_str()) {
        Some("ts") => Language::TypeScript,
        Some("tsx") => Language::Tsx,
        _ => Language::JavaScript,
    }
}

fn run_taint_on_file(path: &PathBuf) -> prism::slice::SliceResult {
    let source = fs::read_to_string(path).expect("read fixture");
    let rel = path.file_name().unwrap().to_str().unwrap().to_string();
    let parsed = ParsedFile::parse(&rel, &source, language_for(path)).unwrap();
    let mut files = BTreeMap::new();
    files.insert(rel.clone(), parsed);

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
fn test_cwe_phase3_js_ts_suppression_rate_meets_80pct() {
    let sanitized_dir = fixture_dir("sanitized");
    let mut sanitized_files: Vec<PathBuf> = fs::read_dir(&sanitized_dir)
        .expect("read sanitized dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    sanitized_files.sort();
    assert_eq!(sanitized_files.len(), 6);

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
        "[cwe_phase3_suppression] sanitizer suppression rate: {}/{} ({}% - pinned floor: 80%). Leaks: {:?}",
        suppressed,
        sanitized_files.len(),
        (suppressed * 100) / sanitized_files.len(),
        leaked
    );
    assert!(
        suppressed >= 5,
        ">=80% suppression rate required. Got {}/{}. Leaks: {:?}",
        suppressed,
        sanitized_files.len(),
        leaked
    );

    let unsanitized_dir = fixture_dir("unsanitized");
    let mut unsanitized_files: Vec<PathBuf> = fs::read_dir(&unsanitized_dir)
        .expect("read unsanitized dir")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    unsanitized_files.sort();
    assert_eq!(unsanitized_files.len(), 6);

    let mut missed: Vec<String> = Vec::new();
    for f in &unsanitized_files {
        let result = run_taint_on_file(f);
        if !has_taint_sink(&result) {
            missed.push(f.file_name().unwrap().to_str().unwrap().to_string());
        }
    }
    let detected = unsanitized_files.len() - missed.len();
    eprintln!(
        "[cwe_phase3_suppression] unsanitized leakage detection: {}/{} ({}% - pinned floor: 100%)",
        detected,
        unsanitized_files.len(),
        (detected * 100) / unsanitized_files.len()
    );
    assert!(
        missed.is_empty(),
        "all unsanitized JS/TS fixtures must fire. Missed: {:?}",
        missed
    );
}
