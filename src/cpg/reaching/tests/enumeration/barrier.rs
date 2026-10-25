use super::super::grammar_lint::{visibility_span, ByteSpan, SpanKind};
use super::case::{Case, CaseClass};
use crate::cpg::{CodePropertyGraph, DfgLabelStats, FlowConfidence, FlowDoubt};
use crate::data_flow::VarLocation;
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod order;

pub(super) const CASES: &[Case] = &[
    Case {
        id: "e0b-java-record_pattern",
        language: Language::Java,
        kind: "record_pattern",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x)) { sink(x); }\n    return x;\n}\n",
        expect: &[("2:x", "3:x", ou(3)), ("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "e0b-rs-tuple_struct_pattern",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(x) = opt { observe(x); }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 0)),
    },
    Case {
        id: "e0c-1-java-instanceof-same-line-guarded-use",
        language: Language::Java,
        kind: "record_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x) && sink(x)) { }\n    return x;\n}\n",
        expect: &[("2:x", "3:x", ou(3)), ("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    Case {
        id: "e0c-1-rust-if-let-empty-span",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(x) = opt {}\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 0)),
    },
    Case {
        id: "e0b-bash-variable_assignment",
        language: Language::Bash,
        kind: "variable_assignment",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "f() {\n    x=$(source)\n    for x in a b; do :; done\n    sink \"$x\"\n}\n",
        expect: &[("2:x", "4:x", ou(3))],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
];

const JAVA_NEGATED_CASE: Case = supplemental_java_case(
    "java-negated-instanceof-branch-aware",
    "record_pattern",
    "int f(Object o) {\n    int x = source();\n    if (!(o instanceof R(var x))) {\n        sink(x);\n    } else {\n        consume(x);\n    }\n    return x;\n}\n",
    &[
        ("2:x", "4:x", FlowConfidence::Exact),
        ("2:x", "6:x", ou(3)),
        ("2:x", "8:x", FlowConfidence::Exact),
    ],
    1,
);

const fn supplemental_java_case(
    id: &'static str,
    kind: &'static str,
    src: &'static str,
    expect: &'static [(&'static str, &'static str, FlowConfidence)],
    expected_counter: i64,
) -> Case {
    Case {
        id,
        language: Language::Java,
        kind,
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src,
        expect,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", expected_counter)),
    }
}

const fn ou(construct_line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line })
}

const JAVA_DOUBLE_NEGATED_CASE: Case = supplemental_java_case(
    "e0c-1-java-double-negated-pattern",
    "record_pattern",
    "int f(Object o) {\n    int x = source();\n    if (!!((o instanceof R(var x)))) {\n        sink(x);\n    } else {\n        consume(x);\n    }\n    return x;\n}\n",
    &[
        ("2:x", "4:x", ou(3)),
        ("2:x", "6:x", FlowConfidence::Exact),
        ("2:x", "8:x", FlowConfidence::Exact),
    ],
    1,
);

const JAVA_TRIPLE_NEGATED_CASE: Case = supplemental_java_case(
    "e0c-1-java-triple-negated-pattern",
    "record_pattern",
    "int f(Object o) {\n    int x = source();\n    if (!!!((o instanceof R(var x)))) {\n        sink(x);\n    } else {\n        consume(x);\n    }\n    return x;\n}\n",
    &[
        ("2:x", "4:x", FlowConfidence::Exact),
        ("2:x", "6:x", ou(3)),
        ("2:x", "8:x", FlowConfidence::Exact),
    ],
    1,
);

const JAVA_RECORD_BINDER_NAMES_CASE: Case = supplemental_java_case(
    "e0c-1-java-record-pattern-binder-names",
    "record_pattern",
    "int f(Object o) {\n    int R = source();\n    int String = source();\n    int y = source();\n    if (o instanceof R<String>(var y)) {\n        sink(R);\n        consume(String);\n        observe(y);\n    }\n    return R + String + y;\n}\n",
    &[
        ("2:R", "6:R", FlowConfidence::Exact),
        ("3:String", "7:String", FlowConfidence::Exact),
        ("4:y", "8:y", ou(5)),
    ],
    1,
);

const JAVA_NAME_CARRIER_POSITIVE_CASE: Case = supplemental_java_case(
    "e0c-1-java-instanceof-name-positive",
    "instanceof_expression",
    "int f(Object o) {\n    int x = source();\n    if (o instanceof String x && guard(x)) {\n        sink(x);\n    }\n    return x;\n}\n",
    &[
        ("2:x", "3:x", ou(3)),
        ("2:x", "4:x", ou(3)),
        ("2:x", "6:x", FlowConfidence::Exact),
    ],
    2,
);

const JAVA_NAME_CARRIER_NEGATED_CASE: Case = supplemental_java_case(
    "e0c-1-java-instanceof-name-negated",
    "instanceof_expression",
    "int f(Object o) {\n    int x = source();\n    if (!(o instanceof String x)) {\n        sink(x);\n    } else {\n        consume(x);\n    }\n    return x;\n}\n",
    &[
        ("2:x", "4:x", FlowConfidence::Exact),
        ("2:x", "6:x", ou(3)),
        ("2:x", "8:x", FlowConfidence::Exact),
    ],
    1,
);

const JAVA_NESTED_NONCOMPLETING_CASE: Case = supplemental_java_case(
    "e0c-1-java-nested-noncompleting-consequence",
    "record_pattern",
    "int f(Object o, boolean flag) {\n    int x = source();\n    if (!(o instanceof R(var x))) {\n        if (flag) { return 0; } else { return 1; }\n    }\n    sink(x);\n    return x;\n}\n",
    &[("2:x", "6:x", ou(3))],
    1,
);

const JAVA_NESTED_CAN_COMPLETE_CASE: Case = supplemental_java_case(
    "e0c-1-java-nested-completing-consequence",
    "record_pattern",
    "int f(Object o, boolean flag) {\n    int x = source();\n    if (!(o instanceof R(var x))) {\n        if (flag) { return 0; }\n    }\n    sink(x);\n    return x;\n}\n",
    &[("2:x", "6:x", FlowConfidence::Exact)],
    0,
);

const JAVA_NEGATED_EMPTY_CASE: Case = supplemental_java_case(
    "e0c-1-java-negated-empty-span",
    "record_pattern",
    "int f(Object o) {\n    int x = source();\n    if (!(o instanceof R(var x))) {}\n    sink(x);\n    return x;\n}\n",
    &[("2:x", "4:x", FlowConfidence::Exact)],
    0,
);

const JAVA_MIXED_COMPOUND_CASE: Case = supplemental_java_case(
    "e0c-1-java-mixed-compound-unknown",
    "instanceof_expression",
    "int f(Object o, boolean flag) {\n    int x = source();\n    if (!(o instanceof String x) || (flag && guard(x))) {}\n    return x;\n}\n",
    &[("2:x", "3:x", ou(3))],
    1,
);

const JAVA_ENCLOSING_NOT_NAME_CASE: Case = supplemental_java_case(
    "e0c-1-java-enclosing-not-name-unknown",
    "instanceof_expression",
    "int f(Object o) {\n    int x = source();\n    if (!(o instanceof String x && guard(x))) {}\n    return x;\n}\n",
    &[("2:x", "3:x", ou(3))],
    1,
);

const JAVA_ENCLOSING_NOT_RECORD_CASE: Case = supplemental_java_case(
    "e0c-1-java-enclosing-not-record-unknown",
    "record_pattern",
    "int f(Object o) {\n    int x = source();\n    if (!(o instanceof R(var x) && guard(x))) {}\n    return x;\n}\n",
    &[("2:x", "3:x", ou(3))],
    1,
);

fn assert_barrier_case(case: &Case) {
    let parsed = super::super::parsed(case.src, case.language);
    let function = super::super::function(&parsed);
    let defs = super::super::collect_defs(&parsed);
    let mut edges = Vec::new();
    for (from, to, _) in case.expect {
        let (def_line, def_name) = from.split_once(':').unwrap();
        let (use_line, use_name) = to.split_once(':').unwrap();
        assert_eq!(def_name, use_name, "{}: expectation path", case.id);
        let def_line: usize = def_line.parse().unwrap();
        let use_line: usize = use_line.parse().unwrap();
        let def = defs
            .iter()
            .find(|def| def.line == def_line && def.path.to_string() == def_name)
            .unwrap_or_else(|| panic!("{}: definition {from:?} missing", case.id));
        edges.push(super::super::edge(&parsed, &function, def, use_line));
    }
    let (outcome, _) = super::super::run(&parsed, &defs, &edges);
    for (edge, (_, _, expected)) in edges.iter().zip(case.expect.iter()) {
        super::super::assert_label(&outcome, edge, *expected);
    }
    if let Some((field, expected)) = case.expect_counter {
        assert_eq!(field, "dfg_label_nameonly_ownership_uncertain");
        let super::super::RdOutcome::Available(result) = outcome else {
            panic!("{}: expected measurable outcome, got {outcome:?}", case.id);
        };
        let mut stats = DfgLabelStats::default();
        for label in result.labels.values().copied() {
            stats.record_label(label);
        }
        assert_eq!(
            stats.dfg_label_nameonly_ownership_uncertain as i64, expected,
            "{}: measured {field}",
            case.id
        );
    }
}

macro_rules! barrier_tests {
    ($($name:ident => [$($case:expr),+];)+) => {$(
        #[test]
        fn $name() {
            $(assert_barrier_case($case);)+
        }
    )+};
}

barrier_tests! {
    java_instanceof_pattern_uncertain => [&CASES[0]];
    rust_if_let_uncertain_outside_scope => [&CASES[1]];
    java_instanceof_same_line_guarded_use => [&CASES[2]];
    rust_if_let_empty_span => [&CASES[3]];
    unknown_scope_fail_closed => [&CASES[4]];
    java_negated_instanceof_is_branch_aware => [&JAVA_NEGATED_CASE];
    java_negation_parity_through_parentheses_and_unary => [&JAVA_DOUBLE_NEGATED_CASE, &JAVA_TRIPLE_NEGATED_CASE];
    java_record_pattern_excludes_type_and_generic_identifiers => [&JAVA_RECORD_BINDER_NAMES_CASE];
    java_instanceof_name_carrier_obeys_guard_spans => [&JAVA_NAME_CARRIER_POSITIVE_CASE, &JAVA_NAME_CARRIER_NEGATED_CASE];
    java_nested_noncompleting_consequence_controls_post_if_span => [&JAVA_NESTED_NONCOMPLETING_CASE, &JAVA_NESTED_CAN_COMPLETE_CASE];
}

#[test]
fn empty_span_is_direct_and_behaviorally_preserved() {
    let rust = super::super::parsed(CASES[3].src, Language::Rust);
    let binder = super::case::nodes_of_kind(&rust, "tuple_struct_pattern")[0];
    assert_eq!(visibility_span(&rust, binder), SpanKind::Empty);
    assert_barrier_case(&JAVA_NEGATED_EMPTY_CASE);
}

fn java_visibility_span((src, kind): (&str, &str)) -> SpanKind {
    let parsed = super::super::parsed(src, Language::Java);
    let binder = super::case::nodes_of_kind(&parsed, kind)[0];
    visibility_span(&parsed, binder)
}

fn assert_java_span_unknown(case: &Case) {
    assert_eq!(
        java_visibility_span((case.src, case.kind)),
        SpanKind::Unknown
    );
}

#[test]
fn java_compound_conditions_fail_closed() {
    assert_barrier_case(&JAVA_MIXED_COMPOUND_CASE);
    let compound =
        "int f(Object o, boolean flag) {\n    if (!(!(o instanceof String x) && flag)) { sink(x); }\n}\n";
    let spans = [
        (JAVA_MIXED_COMPOUND_CASE.src, "instanceof_expression"),
        (compound, "instanceof_expression"),
    ]
    .map(java_visibility_span);
    assert_eq!(spans, [SpanKind::Unknown, SpanKind::Unknown]);
}

#[test]
fn java_enclosing_not_after_name_and_stays_uncertain() {
    assert_barrier_case(&JAVA_ENCLOSING_NOT_NAME_CASE);
    assert_java_span_unknown(&JAVA_ENCLOSING_NOT_NAME_CASE);
}

#[test]
fn java_enclosing_not_after_record_and_stays_uncertain() {
    assert_barrier_case(&JAVA_ENCLOSING_NOT_RECORD_CASE);
    assert_java_span_unknown(&JAVA_ENCLOSING_NOT_RECORD_CASE);
}

fn ancestor_of_kind<'a>(mut node: tree_sitter::Node<'a>, kind: &str) -> tree_sitter::Node<'a> {
    while let Some(parent) = node.parent() {
        if parent.kind() == kind {
            return parent;
        }
        node = parent;
    }
    panic!("{} has no {kind} ancestor", node.kind());
}

#[test]
fn java_pattern_guard_spans_have_exact_disjoint_byte_ranges() {
    let positive = super::super::parsed(CASES[2].src, Language::Java);
    let binder = super::case::nodes_of_kind(&positive, "record_pattern")[0];
    let owner = ancestor_of_kind(binder, "if_statement");
    let condition = owner.child_by_field_name("condition").unwrap();
    let consequence = owner.child_by_field_name("consequence").unwrap();
    assert_eq!(
        visibility_span(&positive, binder),
        SpanKind::Known(vec![
            ByteSpan {
                start: binder.end_byte(),
                end: condition.end_byte(),
            },
            ByteSpan {
                start: consequence.start_byte(),
                end: consequence.end_byte(),
            },
        ])
    );

    let negated = super::super::parsed(JAVA_NEGATED_CASE.src, Language::Java);
    let binder = super::case::nodes_of_kind(&negated, "record_pattern")[0];
    let owner = ancestor_of_kind(binder, "if_statement");
    let alternative = owner.child_by_field_name("alternative").unwrap();
    assert_eq!(
        visibility_span(&negated, binder),
        SpanKind::Known(vec![ByteSpan {
            start: alternative.start_byte(),
            end: alternative.end_byte(),
        }])
    );
}
