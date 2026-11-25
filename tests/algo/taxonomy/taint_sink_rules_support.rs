//! Shared harness for the #622 S1 sink-rule matrices: one single-file fixture per case.
//!
//! A fixture marks its seed line with `SEED` and its sink line with `SINK` (in a
//! comment). The helper seeds taint from the seed line and reports whether a
//! `taint_sink` finding lands on the sink line.

use crate::common::*;
use prism_review::algorithms::taint::{self, TaintConfig};

pub(super) const EXPECTATION_HEADER: &str = "table\tlabel\trules\tobserved\tclass\tnote";

#[derive(Debug)]
pub struct Expectation {
    pub table: String,
    pub label: String,
    /// `None` defers the rule list to stage D. An empty list records `-`.
    pub rules: Option<Vec<String>>,
    pub observed: String,
    pub class: String,
}

/// Parse and validate the identity ledger rows.
pub fn parse_expected_rules(tsv: &str) -> Result<Vec<Expectation>, String> {
    let mut expectations = Vec::new();
    let mut keys = BTreeSet::new();
    for (index, line) in tsv.lines().enumerate() {
        if index == 0 && line == EXPECTATION_HEADER {
            continue;
        }
        let number = index + 1;
        let columns: Vec<_> = line.split('\t').collect();
        let [table, label, rules, observed, class, _note] = columns.as_slice() else {
            return Err(format!(
                "line {number}: expected 6 columns, got {}",
                columns.len()
            ));
        };
        if [table, label, observed, class]
            .iter()
            .any(|value| value.is_empty())
        {
            return Err(format!(
                "line {number}: empty table, label, observed, or class"
            ));
        }
        if !matches!(
            *class,
            "flat"
                | "flat-multi"
                | "flat-dup"
                | "structured"
                | "structured-multi"
                | "flat+structured"
                | "cleansed-structured"
                | "continued"
                | "sibling"
                | "legacy"
                | "source-line"
                | "yaml-inline"
                | "wrapper"
                | "caller-sink"
                | "weak-mixed"
                | "negative"
        ) {
            return Err(format!("line {number}: invalid class {class:?}"));
        }
        if (*rules == "?" && *observed == "=")
            || observed.contains(";structured:")
            || observed.starts_with('+')
        {
            return Err(format!(
                "line {number}: invalid observed value {observed:?}"
            ));
        }
        if !keys.insert((*table, *label)) {
            return Err(format!(
                "line {number}: duplicate table and label {table}/{label}"
            ));
        }
        let rules = parse_rule_list(rules).map_err(|error| format!("line {number}: {error}"))?;
        expectations.push(Expectation {
            table: (*table).to_string(),
            label: (*label).to_string(),
            rules,
            observed: (*observed).to_string(),
            class: (*class).to_string(),
        });
    }
    Ok(expectations)
}

fn parse_rule_list(rules: &str) -> Result<Option<Vec<String>>, String> {
    match rules {
        "?" => return Ok(None),
        "-" => return Ok(Some(Vec::new())),
        _ => {}
    }

    let mut previous = None;
    let mut parsed = Vec::new();
    for rule in rules.split(';') {
        let identity = match rule.split_once('!') {
            Some((identity, "suggestion")) => identity,
            Some((_, severity)) => return Err(format!("invalid severity mark !{severity}")),
            None => rule,
        };
        let (id, cwe) = match identity.split_once('@') {
            Some((id, cwe)) => (id, Some(cwe)),
            None => (identity, None),
        };
        if !id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        }) {
            return Err(format!("invalid rule id {id:?}"));
        }
        if let Some(cwe) = cwe {
            let valid = cwe.strip_prefix("CWE-").is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
            });
            if !valid {
                return Err(format!("invalid CWE {cwe:?}"));
            }
        }
        if previous.is_some_and(|preceding| preceding >= id) {
            return Err(format!("unsorted or duplicate rule id {id:?}"));
        }
        previous = Some(id);
        parsed.push(rule.to_string());
    }
    Ok(Some(parsed))
}

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

fn fixture_inputs(
    path: &str,
    language: Language,
    source: &str,
) -> (BTreeMap<String, ParsedFile>, DiffInput) {
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
    (files, diff)
}

pub fn findings(path: &str, language: Language, source: &str) -> prism::slice::SliceResult {
    let (files, diff) = fixture_inputs(path, language, source);
    algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

/// Run taint with caller-supplied sink names. This skips evidence alignment and diagram
/// finalisation, so only the presence of `taint_sink` findings is comparable to the
/// compatibility runner.
pub fn findings_with_sinks(
    path: &str,
    language: Language,
    source: &str,
    extra_sinks: &[&str],
) -> prism::slice::SliceResult {
    let (files, diff) = fixture_inputs(path, language, source);
    let config = TaintConfig {
        extra_sinks: extra_sinks.iter().map(|&name| name.to_string()).collect(),
        ..TaintConfig::default()
    };
    taint::slice(&CpgContext::build(&files, None), &diff, &config).unwrap()
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
