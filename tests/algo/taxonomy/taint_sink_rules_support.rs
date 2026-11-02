//! Shared harness for the #622 S1 sink-rule matrices: one single-file fixture per case.
//!
//! A fixture marks its seed line with `SEED` and its sink line with `SINK` (in a
//! comment). The helper seeds taint from the seed line and reports whether a
//! `taint_sink` finding lands on the sink line.

use crate::common::*;

pub struct Case {
    pub label: &'static str,
    pub source: &'static str,
}

pub fn marker_line(source: &str, marker: &str) -> usize {
    source
        .lines()
        .position(|l| l.contains(marker))
        .unwrap_or_else(|| panic!("fixture has no {marker} marker:\n{source}"))
        + 1
}

pub fn findings(path: &str, language: Language, source: &str) -> prism::slice::SliceResult {
    let parsed = ParsedFile::parse(path, source, language).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([marker_line(source, "SEED")]),
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

pub fn sink_on_marker(path: &str, language: Language, source: &str) -> bool {
    let sink = marker_line(source, "SINK");
    findings(path, language, source)
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == sink)
}

/// Labels of the cases whose result differs from `expect`.
pub fn mismatches(
    path: &str,
    language: Language,
    cases: &[Case],
    expect: bool,
) -> Vec<&'static str> {
    cases
        .iter()
        .filter(|c| sink_on_marker(path, language, c.source) != expect)
        .map(|c| c.label)
        .collect()
}

pub fn assert_all_sinks(path: &str, language: Language, cases: &[Case]) {
    let wrong = mismatches(path, language, cases, true);
    assert!(wrong.is_empty(), "genuine sinks not reported: {wrong:?}");
}

pub fn assert_no_sinks(path: &str, language: Language, cases: &[Case]) {
    let wrong = mismatches(path, language, cases, false);
    assert!(
        wrong.is_empty(),
        "name-fragment shapes reported as sinks: {wrong:?}"
    );
}

/// One matrix row with its own path and language.
pub struct Row {
    pub sink: bool,
    pub label: &'static str,
    pub path: &'static str,
    pub lang: Language,
    pub source: &'static str,
}

/// Rows whose result differs from their `sink` expectation.
pub fn assert_rows(rows: &[Row]) {
    let wrong: Vec<String> = rows
        .iter()
        .filter(|r| sink_on_marker(r.path, r.lang, r.source) != r.sink)
        .map(|r| format!("{} (expected sink={})", r.label, r.sink))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} rows differ:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
