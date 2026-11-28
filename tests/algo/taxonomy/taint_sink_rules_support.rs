//! Shared harness for the #622 S1 sink-rule matrices: one single-file fixture per case.
//!
//! A fixture marks its seed line with `SEED` and its sink line with `SINK` (in a
//! comment). The helper seeds taint from the seed line and reports whether a
//! `taint_sink` finding lands on the sink line.

use crate::common::*;
use prism_review::algorithms::taint::{self, TaintConfig};
use std::sync::LazyLock;

pub(super) const EXPECTATION_HEADER: &str = "table\tlabel\trules\tobserved\tclass\tnote";
pub(super) const EXPECTED_RULES: &str =
    include_str!("../../fixtures/taint_sink_rules/expected_rules.tsv");

#[derive(Debug)]
pub struct Expectation {
    pub table: String,
    pub label: String,
    /// An empty list records `-`.
    pub rules: Vec<String>,
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
        let rules = parse_rule_list(rules)
            .map_err(|error| format!("line {number}: {table}/{label}: {error}"))?;
        expectations.push(Expectation {
            table: (*table).to_string(),
            label: (*label).to_string(),
            rules,
            observed: (*observed).to_string(),
            class: (*class).to_string(),
        });
    }
    if expectations.is_empty() {
        return Err("no expectation rows".into());
    }
    Ok(expectations)
}

fn parse_rule_list(rules: &str) -> Result<Vec<String>, String> {
    match rules {
        "?" => return Err("unresolved rules (?)".into()),
        "-" => return Ok(Vec::new()),
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
    for rule in &parsed {
        let identity = rule.split('!').next().unwrap();
        let (id, cwe) = identity
            .split_once('@')
            .map_or((identity, None), |(id, cwe)| (id, Some(cwe)));
        let row =
            prism::rule_registry::lookup(id).ok_or_else(|| format!("unknown rule id {id:?}"))?;
        if row.cwe() != cwe {
            return Err(format!("{id}: expected CWE {:?}, got {cwe:?}", row.cwe()));
        }
    }
    Ok(parsed)
}

/// Require every selected fixture to have an exact ledger key. Each fixture owns its
/// presence assertion, since an empty expected rule list can accompany a finding.
#[track_caller]
pub fn assert_matrix_selection(module: &str, labels: impl IntoIterator<Item = &'static str>) {
    static KEYS: LazyLock<BTreeMap<String, BTreeSet<String>>> = LazyLock::new(|| {
        let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for row in parse_expected_rules(EXPECTED_RULES).unwrap() {
            keys.entry(row.table).or_default().insert(row.label);
        }
        keys
    });

    let table = module.rsplit("::").next().unwrap();
    let mut labels = labels.into_iter().peekable();
    assert!(
        labels.peek().is_some(),
        "empty matrix selection for {table}"
    );
    let keys = KEYS.get(table);
    for label in labels {
        assert!(
            keys.is_some_and(|keys| keys.contains(label)),
            "missing matrix key {table}/{label:?}"
        );
    }
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
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();
    assert_unactivated_rules(&result);
    result
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
    let result = taint::slice(&CpgContext::build(&files, None), &diff, &config).unwrap();
    assert_unactivated_rules(&result);
    result
}

#[track_caller]
fn assert_unactivated_rules(result: &prism::slice::SliceResult) {
    check_matrix_rules(&result.findings).unwrap_or_else(|error| panic!("{error}"));
}

fn check_matrix_rules(findings: &[prism::slice::SliceFinding]) -> Result<(), String> {
    for finding in findings {
        for rule in &finding.rules {
            prism::rule_registry::check_emitted(&finding.algorithm, rule)
                .map_err(|error| format!("{}:{}: {error}", finding.file, finding.line))?;
        }
        if !finding.rules.is_empty() {
            return Err(format!(
                "{}:{}: rules in unactivated fixture",
                finding.file, finding.line
            ));
        }
    }
    Ok(())
}

pub fn sink_on_marker(path: &str, language: Language, source: &str) -> bool {
    let sink = marker_line(source, "SINK");
    findings(path, language, source)
        .findings
        .iter()
        .any(|f| f.category.as_deref() == Some("taint_sink") && f.line == sink)
}

/// Labels of the cases whose result differs from `expect`.
fn mismatches(path: &str, language: Language, cases: &[Case], expect: bool) -> Vec<&'static str> {
    cases
        .iter()
        .filter(|c| sink_on_marker(path, language, c.source) != expect)
        .map(|c| c.label)
        .collect()
}

#[track_caller]
pub fn assert_all_sinks(module: &str, path: &str, language: Language, cases: &[Case]) {
    assert_matrix_selection(module, cases.iter().map(|case| case.label));
    let wrong = mismatches(path, language, cases, true);
    assert!(wrong.is_empty(), "genuine sinks not reported: {wrong:?}");
}

#[track_caller]
pub fn assert_no_sinks(module: &str, path: &str, language: Language, cases: &[Case]) {
    assert_matrix_selection(module, cases.iter().map(|case| case.label));
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

/// Assert each row's presence using the fixture's own path and language.
#[track_caller]
pub fn assert_rows(module: &str, rows: &[Row]) {
    assert_matrix_selection(module, rows.iter().map(|row| row.label));
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

#[test]
fn matrix_rule_reader_checks_unmarked_findings_and_empty_partitions() {
    use prism::slice::SliceFinding;
    let mut findings = vec![SliceFinding::default()];
    assert!(check_matrix_rules(&findings).is_ok());
    findings.push(
        serde_json::from_value(serde_json::json!({
            "algorithm":"taint","file":"unmarked.js","line":99,"severity":"warning",
            "description":"control","rules":[{"id":"legacy"}]
        }))
        .unwrap(),
    );
    let error = check_matrix_rules(&findings).unwrap_err();
    assert!(error.contains("unmarked.js:99") && error.contains("unactivated"));
    findings[1].rules = serde_json::from_str(r#"[{"id":"unknown"}]"#).unwrap();
    assert!(check_matrix_rules(&findings)
        .unwrap_err()
        .contains("unknown rule id"));
}

#[test]
#[should_panic(expected = "empty matrix selection")]
fn matrix_rejects_empty_cases() {
    assert_all_sinks(
        "taint_sink_rules_js_test",
        "a.js",
        Language::JavaScript,
        &[],
    );
}

#[test]
#[should_panic(expected = "empty matrix selection")]
fn matrix_rejects_empty_rows() {
    assert_rows("taint_sink_rules_identity_test", &[]);
}

#[test]
#[should_panic(expected = "missing matrix key")]
fn matrix_requires_exact_row_key() {
    assert_rows(
        "taint_sink_rules_identity_test",
        &[Row {
            sink: false,
            label: "ri27 ",
            path: "a.tf",
            lang: Language::Terraform,
            source: "locals {\n  payload = var.payload # SEED SINK\n}\n",
        }],
    );
}
