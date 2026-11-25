use super::attach_rules;
use crate::rule_registry::{JS_DOM_HTML, JS_EVAL, LEGACY};
use crate::slice::SliceFinding;

#[test]
fn empty_selection_replaces_rules_and_restores_wire_omission() {
    let mut finding = SliceFinding::default();
    finding.rules.push(LEGACY.rule_ref());
    attach_rules(&mut finding, []);
    assert_eq!(
        serde_json::to_string(&finding).unwrap(),
        r#"{"algorithm":"","file":"","line":0,"severity":"","description":""}"#
    );
}

#[test]
fn rows_sort_by_id_and_strong_wins_in_either_order() {
    for first_weak in [true, false] {
        let mut finding = SliceFinding {
            severity: "warning".into(),
            ..SliceFinding::default()
        };
        attach_rules(
            &mut finding,
            [
                (&LEGACY, false),
                (&JS_EVAL, first_weak),
                (&JS_DOM_HTML, true),
                (&JS_EVAL, !first_weak),
                (&JS_DOM_HTML, true),
            ],
        );
        assert_eq!(
            finding.rules,
            [
                JS_DOM_HTML
                    .rule_ref()
                    .with_severity(Some("suggestion".into())),
                JS_EVAL.rule_ref(),
                LEGACY.rule_ref(),
            ]
        );
        assert_eq!(finding.severity, "warning");
    }
}

#[test]
fn weak_row_on_suggestion_needs_no_override() {
    let mut finding = SliceFinding {
        severity: "suggestion".into(),
        ..SliceFinding::default()
    };
    attach_rules(&mut finding, [(&JS_DOM_HTML, true)]);
    assert_eq!(finding.rules, [JS_DOM_HTML.rule_ref()]);
}
