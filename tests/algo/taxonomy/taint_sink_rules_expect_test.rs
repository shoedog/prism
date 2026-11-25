//! Stage T checks for the identity ledger and its coverage of the sink matrices.

use super::taint_sink_rules_support::parse_expected_rules;
use crate::common::*;

const EXPECTED_RULES: &str = include_str!("../../fixtures/taint_sink_rules/expected_rules.tsv");

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

fn table_labels(table: &str, source: &str) -> BTreeSet<String> {
    let parsed = ParsedFile::parse(&format!("{table}.rs"), source, Language::Rust).unwrap();
    let root = parsed.tree.root_node();
    assert!(!root.has_error(), "cannot scan Rust table {table}");
    let mut labels = BTreeSet::new();
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
            labels.insert(serde_json::from_str(literal).unwrap_or_else(|error| {
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
    let expectations = parse_expected_rules(EXPECTED_RULES).unwrap();
    assert!(!expectations.is_empty());
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
        let in_table = table_labels(table, source);
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

fn ledger_row(rules: &str) -> String {
    format!("sample_table\tsample label\t{rules}\t=\tflat\tparser control\n")
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
fn parser_checks_row_keys_and_preserves_rule_tokens() {
    let row = ledger_row("a;b-2@CWE-79!suggestion");
    let parsed = parse_expected_rules(&row).unwrap();
    assert_eq!(parsed[0].table, "sample_table");
    assert_eq!(parsed[0].label, "sample label");
    assert_eq!(
        parsed[0].rules,
        Some(vec!["a".into(), "b-2@CWE-79!suggestion".into()])
    );
    assert_eq!(parsed[0].observed, "=");
    assert_eq!(parsed[0].class, "flat");
    assert_eq!(
        parse_expected_rules(&ledger_row("?")).unwrap()[0].rules,
        None
    );
    assert_eq!(
        parse_expected_rules(&ledger_row("-")).unwrap()[0].rules,
        Some(vec![])
    );

    for (tsv, message) in [
        (
            "table\tlabel\t-\t=\tnegative".to_string(),
            "expected 6 columns",
        ),
        (ledger_row("bad_id"), "invalid rule id"),
        (format!("{row}{row}"), "duplicate table and label"),
    ] {
        let error = parse_expected_rules(&tsv).unwrap_err();
        assert!(error.contains(message), "{tsv:?}: {error}");
    }
}
