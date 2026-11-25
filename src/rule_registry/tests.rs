use crate::rule_registry::*;
use crate::slice::{RuleRef as SliceRuleRef, SlicingAlgorithm};

#[test]
fn registry_rows_are_sorted_and_canonical() {
    let rows = rules();
    assert!(rows.windows(2).all(|pair| pair[0].id() < pair[1].id()));
    assert_eq!(rows.len(), 64);
    for row in rows {
        assert_eq!(row.name(), row.id());
        assert_eq!(row.algorithms(), ["taint"]);
        assert!(row
            .algorithms()
            .iter()
            .all(|a| SlicingAlgorithm::from_str(a).is_some()));
        if let Some(cwe) = row.cwe() {
            let digits = cwe.strip_prefix("CWE-").expect(row.id());
            assert!(!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()));
        }
        for sentence in [row.short(), row.help()] {
            assert!(sentence.ends_with('.'), "{}", row.id());
            assert_eq!(sentence, sentence.trim());
            assert!(!sentence.contains("CWE-"));
            assert!(row.id() == "legacy" || !sentence.contains(row.id()));
        }
        assert!(std::ptr::eq(lookup(row.id()).unwrap(), row));
        let reference: SliceRuleRef = row.rule_ref();
        let identity = (row.id(), row.cwe());
        assert_eq!((reference.id(), reference.cwe()), identity);
        assert_eq!(reference.severity(), None);
        assert!(check_emitted("taint", &reference).is_ok());
        let lowered: RuleRef = reference.clone().with_severity(Some("suggestion".into()));
        assert_eq!((lowered.id(), lowered.cwe()), identity);
        assert_eq!(lowered.severity(), Some("suggestion"));
        assert_eq!(lowered.with_severity(None), reference);
    }
    assert_eq!(
        LEGACY.short(),
        "A legacy sink name matched on a tainted line."
    );
    assert_eq!(LEGACY.help(), "Inspect the matched name and trace the input to the operation. This detector does not identify a particular operand or weakness.");
    assert_eq!(LEGACY.cwe(), None);
    assert!(lookup("JS-eval").is_none());
}

#[test]
fn nominal_sink_descriptions_state_call_shapes() {
    for (row, shape, binding_check) in [
        (&JS_FS, "filesystem-like", "Confirm the filesystem binding"),
        (
            &JS_REDIRECT,
            "redirect-like",
            "Confirm the receiver redirects",
        ),
        (
            &JS_LOCATION,
            "navigation-like",
            "Confirm the navigation or response binding",
        ),
        (
            &JS_LOCATION_HEADER,
            "header-setting call shape",
            "Confirm the response binding",
        ),
        (
            &STRUCTURED_PATH_TRAVERSAL,
            "filesystem-like",
            "Confirm the file API",
        ),
        (
            &TAINT_QUERY_VALUE,
            "database-like or ORM",
            "Confirm the receiver",
        ),
        (&TAINT_WRITE_DATA, "write-like", "Resolve the callee"),
    ] {
        assert!(row.short().contains(shape), "{} short", row.id());
        assert!(row.help().contains(binding_check), "{} help", row.id());
    }
}

#[test]
fn emitted_reference_must_match_registry_and_algorithm() {
    let reference: SliceRuleRef = JS_EVAL.rule_ref().with_severity(Some("suggestion".into()));
    assert_eq!(reference.cwe(), Some("CWE-95"));
    assert!(check_emitted("taint", &reference).is_ok());
    for algorithm in ["primitive", "TAINT"] {
        assert!(check_emitted(algorithm, &reference)
            .unwrap_err()
            .contains("not registered"));
    }

    for (json, error) in [
        (r#"{"id":"nonexistent"}"#, "unknown rule id"),
        (
            r#"{"id":"unknown","cwe":"CWE-95","severity":"info"}"#,
            "unknown rule id",
        ),
        (r#"{"id":"js-eval","cwe":"CWE-89"}"#, "requires CWE"),
        (r#"{"id":"js-eval"}"#, "requires CWE"),
        (
            r#"{"id":"taint-redirect-name","cwe":"CWE-601"}"#,
            "requires CWE",
        ),
    ] {
        let wire: SliceRuleRef = serde_json::from_str(json).unwrap();
        assert_eq!(serde_json::to_string(&wire).unwrap(), json);
        assert!(check_emitted("taint", &wire).unwrap_err().contains(error));
    }
    let neutral = TAINT_REDIRECT_NAME.rule_ref();
    assert_eq!(neutral.cwe(), None);
    assert!(check_emitted("taint", &neutral).is_ok());
}
