//! Public-surface checks of the rule registry: accessors, severity override and the finding wire format.

use prism::rule_registry::{check_emitted, lookup, rules, RuleRef, JS_EVAL};
use prism::slice::SliceFinding;

#[test]
fn registry_rows_expose_their_metadata() {
    let row = lookup("js-eval").expect("js-eval is registered");
    assert_eq!(row.id(), "js-eval");
    assert_eq!(row.name(), "js-eval");
    assert_eq!(row.cwe(), JS_EVAL.cwe());
    assert!(!row.short().is_empty() && !row.help().is_empty());
    assert_eq!(row.algorithms(), ["taint"]);
    assert!(rules().windows(2).all(|pair| pair[0].id() < pair[1].id()));
    assert!(lookup("no-such-rule").is_none());
}

#[test]
fn severity_override_keeps_identity_and_serializes() {
    let weak = JS_EVAL.rule_ref().with_severity(Some("suggestion".into()));
    assert_eq!(weak.id(), "js-eval");
    assert_eq!(weak.cwe(), JS_EVAL.cwe());
    assert_eq!(weak.severity(), Some("suggestion"));
    assert!(check_emitted("taint", &weak).is_ok());
    let cleared = weak.clone().with_severity(None);
    assert_eq!(cleared.severity(), None);

    let mut finding = SliceFinding::default();
    finding.rules.push(weak);
    let json = serde_json::to_value(&finding).unwrap();
    assert_eq!(json["rules"][0]["severity"], "suggestion");
    let round_trip: SliceFinding = serde_json::from_value(json).unwrap();
    assert_eq!(round_trip.rules[0].id(), "js-eval");
}

#[test]
fn unknown_and_misattributed_references_are_rejected() {
    let unknown: RuleRef = serde_json::from_str(r#"{"id":"vendor-check","cwe":"CWE-79"}"#).unwrap();
    assert!(check_emitted("taint", &unknown).is_err());
    assert!(check_emitted("primitive", &JS_EVAL.rule_ref()).is_err());
    let wrong_cwe: RuleRef = serde_json::from_str(r#"{"id":"js-eval","cwe":"CWE-502"}"#).unwrap();
    assert!(check_emitted("taint", &wrong_cwe).is_err());
}
