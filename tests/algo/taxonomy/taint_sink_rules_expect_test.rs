//! Stage T checks for the identity ledger and its coverage of the sink matrices.

use super::taint_sink_rules_support::{parse_expected_rules, EXPECTATION_HEADER, EXPECTED_RULES};
use crate::common::*;
use std::{fs, path::Path};

macro_rules! table_sources {
    ($($table:ident),+ $(,)?) => {
        [ $((stringify!($table), include_str!(concat!(stringify!($table), ".rs")))),+ ]
    };
}

const TABLES: [(&str, &str); 9] = table_sources![
    taint_sink_rules_go_test,
    taint_sink_rules_hybrid_go_test,
    taint_sink_rules_hybrid_js_test,
    taint_sink_rules_hybrid_other_test,
    taint_sink_rules_hybrid_py_test,
    taint_sink_rules_identity_test,
    taint_sink_rules_js_test,
    taint_sink_rules_other_test,
    taint_sink_rules_py_test,
];

fn table_labels(table: &str, source: &str) -> Vec<String> {
    let parsed = ParsedFile::parse(&format!("{table}.rs"), source, Language::Rust).unwrap();
    let root = parsed.tree.root_node();
    assert!(!root.has_error(), "cannot scan Rust table {table}");
    let mut labels = Vec::new();
    let mut nodes = vec![root];
    while let Some(node) = nodes.pop() {
        let label = match node.kind() {
            "field_initializer"
                if node
                    .child_by_field_name("field")
                    .is_some_and(|field| parsed.node_text(&field) == "label") =>
            {
                node.child_by_field_name("value")
            }
            "call_expression"
                if node
                    .child_by_field_name("function")
                    .is_some_and(|callee| parsed.node_text(&callee) == "go") =>
            {
                let args = node.child_by_field_name("arguments").unwrap();
                let mut cursor = args.walk();
                let first = args.named_children(&mut cursor).find(|arg| !arg.is_extra());
                first
            }
            _ => None,
        };
        if let Some(label) = label {
            let literal = parsed.node_text(&label);
            labels.push(serde_json::from_str(literal).unwrap_or_else(|error| {
                panic!("{table}: expected a quoted label, got {literal}: {error}")
            }));
        }
        let mut cursor = node.walk();
        nodes.extend(node.named_children(&mut cursor));
    }
    assert!(!labels.is_empty(), "no labels found in {table}");
    labels
}

#[test]
fn expected_rules_tsv_is_well_formed() {
    assert_eq!(EXPECTED_RULES.lines().next(), Some(EXPECTATION_HEADER));
    let expectations = parse_expected_rules(EXPECTED_RULES).unwrap();
    assert_eq!(expectations.len(), 540);
}

#[test]
fn substitution_only_sprintf_rows_expect_empty_rules() {
    let expectations = parse_expected_rules(EXPECTED_RULES).unwrap();
    for (table, label) in [
        (
            "taint_sink_rules_go_test",
            "Sprintf with a non-constant format",
        ),
        (
            "taint_sink_rules_go_test",
            "Sprintf with a format read from a request field",
        ),
        (
            "taint_sink_rules_hybrid_go_test",
            "Sprintf where a param shadows a const",
        ),
        (
            "taint_sink_rules_hybrid_go_test",
            "r2: range var shadows package const format",
        ),
    ] {
        let row = expectations
            .iter()
            .find(|row| row.table == table && row.label == label)
            .unwrap_or_else(|| panic!("missing matrix key {table}/{label}"));
        assert!(row.rules.is_empty(), "{table}/{label}: {:?}", row.rules);
        assert_eq!(row.observed, "go-format-string@CWE-134", "{table}/{label}");
    }
}

#[test]
fn table_label_extractor_preserves_duplicates_and_ignores_raw_strings() {
    let source = r##"
        fn cases() {
            let _ = Case { label: "field label" };
            go("go label", source, severity);
            go("go label", other_source, severity);
            let _ = r#"Case { label: "decoy field" }; go("decoy call", source, severity)"#;
        }
    "##;
    let mut labels = table_labels("synthetic", source);
    labels.sort_unstable();
    assert_eq!(labels, ["field label", "go label", "go label"]);
}

#[test]
fn table_list_matches_taxonomy_directory() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/algo/taxonomy");
    let mut on_disk = BTreeSet::new();
    for entry in fs::read_dir(directory).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("taint_sink_rules_")
            && name.ends_with("_test.rs")
            && !matches!(
                name.as_ref(),
                "taint_sink_rules_expect_test.rs" | "taint_sink_rules_support_test.rs"
            )
            && fs::metadata(entry.path()).unwrap().is_file()
        {
            on_disk.insert(name.into_owned());
        }
    }
    let listed = TABLES
        .iter()
        .map(|(table, _)| format!("{table}.rs"))
        .collect::<BTreeSet<_>>();
    assert_eq!(on_disk, listed);
}

#[test]
fn tsv_and_tables_name_the_same_labels() {
    let mut by_table: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    for expectation in parse_expected_rules(EXPECTED_RULES).unwrap() {
        by_table
            .entry(expectation.table)
            .or_default()
            .insert(expectation.label);
    }

    let mut differences = Vec::new();
    for (table, source) in TABLES {
        let mut labels = table_labels(table, source);
        labels.sort_unstable();
        for pair in labels.windows(2).filter(|pair| pair[0] == pair[1]) {
            differences.push(format!("{table}: duplicate label in table: {:?}", pair[0]));
        }
        let in_table = labels.into_iter().collect::<BTreeSet<_>>();
        let in_tsv = by_table.remove(table).unwrap_or_default();
        for label in in_table.difference(&in_tsv) {
            differences.push(format!("{table}: label in table but not in TSV: {label:?}"));
        }
        for label in in_tsv.difference(&in_table) {
            differences.push(format!("{table}: label in TSV but not in table: {label:?}"));
        }
    }
    for (table, labels) in by_table {
        for label in labels {
            differences.push(format!("{table}: label in TSV but not in table: {label:?}"));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
}

fn ledger_row_with(rules: &str, observed: &str, class: &str) -> String {
    format!("sample_table\tsample label\t{rules}\t{observed}\t{class}\tparser control\n")
}

fn ledger_row(rules: &str) -> String {
    ledger_row_with(rules, "=", "flat")
}

#[test]
fn parser_rejects_unsorted_rules() {
    for rules in ["z@CWE-2;a@CWE-1", "a;a", "a@CWE-1;a@CWE-2"] {
        let error = parse_expected_rules(&ledger_row(rules)).unwrap_err();
        assert!(
            error.contains("unsorted or duplicate rule id"),
            "{rules}: {error}"
        );
    }
}

#[test]
fn parser_rejects_bad_cwe() {
    for rules in ["rule@cwe-79", "rule@CWE-", "rule@CWE-7x"] {
        let error = parse_expected_rules(&ledger_row(rules)).unwrap_err();
        assert!(error.contains("invalid CWE"), "{rules}: {error}");
    }
}

#[test]
fn parser_rejects_bad_severity_mark() {
    for rules in ["rule!warning", "rule!", "rule!suggestion!suggestion"] {
        let error = parse_expected_rules(&ledger_row(rules)).unwrap_err();
        assert!(error.contains("invalid severity mark"), "{rules}: {error}");
    }
}

#[test]
fn parser_rejects_bad_rule_ids() {
    for rules in ["-;a", "?;a", "---", "-x", "x-", "x--y", "bad_id"] {
        let error = parse_expected_rules(&ledger_row(rules)).unwrap_err();
        assert!(error.contains("invalid rule id"), "{rules}: {error}");
    }
}

#[test]
fn parser_rejects_class_outside_vocabulary() {
    let error = parse_expected_rules(&ledger_row_with("a", "=", "flat-ish")).unwrap_err();
    assert!(error.contains("invalid class"), "{error}");
}

#[test]
fn parser_rejects_bad_observed_syntax() {
    for (rules, observed) in [
        ("?", "="),
        ("a", "a;structured:sqli"),
        ("a", "+structured:sqli"),
    ] {
        let row = ledger_row_with(rules, observed, "flat+structured");
        let error = parse_expected_rules(&row).unwrap_err();
        assert!(error.contains("invalid observed value"), "{row:?}: {error}");
    }
}

#[test]
fn parser_rejects_unresolved_rules() {
    let row = ledger_row_with("?", "structured:sqli", "structured");
    let error = parse_expected_rules(&row).unwrap_err();
    assert!(error.contains("unresolved rules"), "{error}");
}

#[test]
fn parser_rejects_empty_ledger() {
    for tsv in ["", EXPECTATION_HEADER] {
        let error = parse_expected_rules(tsv).unwrap_err();
        assert!(error.contains("no expectation rows"), "{error}");
    }
}

#[test]
fn parser_checks_row_keys_and_preserves_rule_tokens() {
    let row = ledger_row("configured-sink;js-dom-html@CWE-79!suggestion");
    let parsed = parse_expected_rules(&row).unwrap();
    assert_eq!(parsed[0].table, "sample_table");
    assert_eq!(parsed[0].label, "sample label");
    assert_eq!(
        parsed[0].rules,
        ["configured-sink", "js-dom-html@CWE-79!suggestion"]
    );
    assert_eq!(parsed[0].observed, "=");
    assert_eq!(parsed[0].class, "flat");
    for observed in ["a+structured:sqli", "structured:sqli"] {
        let parsed =
            parse_expected_rules(&ledger_row_with("legacy", observed, "flat+structured")).unwrap();
        assert_eq!(parsed[0].rules, ["legacy"]);
        assert_eq!(parsed[0].observed, observed);
    }
    assert!(parse_expected_rules(&ledger_row("-")).unwrap()[0]
        .rules
        .is_empty());

    for (tsv, message) in [
        (
            "table\tlabel\t-\t=\tnegative".to_string(),
            "expected 6 columns",
        ),
        (format!("{row}{row}"), "duplicate table and label"),
    ] {
        let error = parse_expected_rules(&tsv).unwrap_err();
        assert!(error.contains(message), "{tsv:?}: {error}");
    }
}

#[test]
fn tsv_rules_require_the_registered_id_and_cwe() {
    for rule in [
        "not-a-prism-rule",
        "js-dom-html@CWE-78",
        "js-dom-html",
        "legacy@CWE-79",
    ] {
        let tsv = format!("{EXPECTATION_HEADER}\ntable\tlabel\t{rule}\t-\tflat\t\n");
        assert!(parse_expected_rules(&tsv).is_err(), "{rule}");
    }
    let tsv = format!(
        "{EXPECTATION_HEADER}\ntable\tlabel\tjs-dom-html@CWE-79!suggestion;legacy\t-\tflat\t\n"
    );
    assert!(parse_expected_rules(&tsv).is_ok());
}
