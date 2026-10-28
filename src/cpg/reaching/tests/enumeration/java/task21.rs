use crate::cpg::reaching::binding_table::{rows, Predicate, Role, Ruling};
use crate::cpg::reaching::grammar_lint::{visibility_span, SpanKind};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

use super::super::case::{nodes_of_kind, Case, CaseClass};

const CLASSIFIED_IF_PATTERN: Predicate = Predicate::JavaInstanceofGuard;
const OU_3: FlowConfidence =
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 });
const KILLED_3: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 });

pub(super) const CASES: &[Case] = &[
    Case {
        id: "java-instanceof-unsupported-owner-uncertain",
        language: Language::Java,
        kind: "instanceof_expression",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "int f(Object o) { while (o instanceof String text) { sink(text); break; } return 0; }\n",
        expect: &[],
        expect_counter: None,
    },
    Case {
        id: "java-instanceof-and-chained-guard",
        language: Language::Java,
        kind: "instanceof_expression",
        variant: Some("binding"),
        row_variant: Some(CLASSIFIED_IF_PATTERN),
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    String text = source();\n    if (o instanceof String text && guard(text)) { sink(text); }\n    return text.length();\n}\n",
        expect: &[
            ("2:text", "3:text", KILLED_3),
            ("2:text", "4:text", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    Case {
        id: "java-switch-pattern-uncertain",
        language: Language::Java,
        kind: "type_pattern",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    String text = source();\n    switch (o) { case String text -> sink(text); default -> work(); }\n    return text.length();\n}\n",
        expect: &[
            ("2:text", "3:text", OU_3),
            ("2:text", "4:text", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "java-record-pattern-body-structural",
        language: Language::Java,
        kind: "record_pattern_body",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(_)) { sink(x); }\n    return x;\n}\n",
        expect: &[
            ("2:x", "3:x", FlowConfidence::Exact),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    Case {
        id: "java-underscore-pattern-not-binding",
        language: Language::Java,
        kind: "underscore_pattern",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(_)) { sink(x); }\n    return x;\n}\n",
        expect: &[
            ("2:x", "3:x", FlowConfidence::Exact),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    Case {
        id: "java-instanceof-outside-guard-preserved",
        language: Language::Java,
        kind: "record_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x)) { sink(x); }\n    return x;\n}\n",
        expect: &[
            ("2:x", "3:x", OU_3),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "java-record-pattern-nested",
        language: Language::Java,
        kind: "record_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof Outer(Inner(var x))) { sink(x); }\n    return x;\n}\n",
        expect: &[
            ("2:x", "3:x", OU_3),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "java-instanceof-negated-return-scope",
        language: Language::Java,
        kind: "instanceof_expression",
        variant: None,
        row_variant: Some(CLASSIFIED_IF_PATTERN),
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    String text = source();\n    if (!(o instanceof String text)) {\n        sink(text);\n        return 0;\n    }\n    sink(text);\n    return text.length();\n}\n",
        expect: &[
            ("2:text", "4:text", FlowConfidence::Exact),
            ("2:text", "7:text", KILLED_3),
            ("2:text", "8:text", KILLED_3),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 2)),
    },
    Case {
        id: "java-instanceof-negated-completing-consequence",
        language: Language::Java,
        kind: "instanceof_expression",
        variant: None,
        row_variant: Some(CLASSIFIED_IF_PATTERN),
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    String text = source();\n    if (o instanceof String text) { consume(text); }\n    if (!(o instanceof String text)) {\n        sink(text);\n    }\n    sink(text);\n    return text.length();\n}\n",
        expect: &[
            ("2:text", "3:text", KILLED_3),
            ("2:text", "5:text", FlowConfidence::Exact),
            ("2:text", "7:text", FlowConfidence::Exact),
            ("2:text", "8:text", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 3)),
    },
];

#[test]
fn task21_java_binding_cases() {
    super::super::case::check_case(&super::super::barrier::CASES[0]);
    for case in CASES {
        super::super::case::check_case(case);
    }
}

#[test]
fn task21_java_rows_are_curated() {
    let java = rows(Language::Java);
    let row = |kind, variant| {
        java.iter()
            .find(|row| row.kind == kind && row.variant == variant)
            .unwrap_or_else(|| panic!("missing Java row {kind} {variant:?}"))
    };

    let instanceof = row("instanceof_expression", Some(CLASSIFIED_IF_PATTERN));
    assert_eq!(instanceof.fields, ["name"]);
    assert!(instanceof.roles.has(Role::Binding));
    assert_eq!(instanceof.ruling, Ruling::Classified);

    for kind in ["record_pattern", "type_pattern"] {
        let pattern = row(kind, None);
        assert!(pattern.roles.has(Role::Binding));
        assert_eq!(pattern.fields, [] as [&str; 0]);
        assert!(matches!(
            pattern.ruling,
            Ruling::Uncertain {
                revisit: "future:java-patterns",
                ..
            }
        ));
    }

    for kind in ["record_pattern_body", "underscore_pattern"] {
        let structural = row(kind, None);
        assert!(structural.roles.has(Role::NotBinding));
        assert_eq!(structural.ruling, Ruling::Classified);
    }
}

#[test]
fn task21_java_pattern_visibility_is_guard_bounded() {
    let switch_case = CASES
        .iter()
        .find(|case| case.id == "java-switch-pattern-uncertain")
        .unwrap();
    let parsed = super::super::super::parsed(switch_case.src, Language::Java);
    let pattern = nodes_of_kind(&parsed, "type_pattern")[0];
    let span = visibility_span(&parsed, pattern);
    assert!(matches!(span, SpanKind::Known(ref spans) if !spans.is_empty()));
    assert!(!span_contains_source_exit(&span, switch_case.src));

    let negated = CASES
        .iter()
        .find(|case| case.id == "java-instanceof-negated-return-scope")
        .unwrap();
    let parsed = super::super::super::parsed(negated.src, Language::Java);
    let binder = nodes_of_kind(&parsed, "instanceof_expression")[0];
    assert!(matches!(
        visibility_span(&parsed, binder),
        SpanKind::Known(_)
    ));
}

fn span_contains_source_exit(span: &SpanKind, src: &str) -> bool {
    let return_byte = src.rfind("return text.length").unwrap();
    matches!(span, SpanKind::Known(spans) if spans.iter().any(|span| span.contains(return_byte)))
}
