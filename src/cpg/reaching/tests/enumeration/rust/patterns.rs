use super::super::case::{Case, CaseClass};
use crate::cpg::reaching::binding_table::{rows, DeclarationKind, Role, Ruling, Visibility};
use crate::cpg::reaching::grammar_lint::{visibility_span, ByteSpan, SpanKind};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

const EXACT: FlowConfidence = FlowConfidence::Exact;
const CFG_INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);

const fn uncertain(line: u32) -> FlowConfidence {
    FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
        construct_line: line,
    })
}

macro_rules! pattern_case {
    ($id:literal, $kind:literal, $src:literal, $expect:expr, $counter:expr) => {
        Case {
            id: $id,
            language: Language::Rust,
            kind: $kind,
            variant: Some("binding"),
            row_variant: None,
            class: CaseClass::Behavioral,
            src: $src,
            expect: $expect,
            expect_counter: Some($counter),
        }
    };
}

pub(crate) const CASES: &[Case] = &[
    pattern_case!(
        "rs-captured-pattern",
        "captured_pattern",
        "fn f(value: Option<i32>) {\n    let x = source();\n    match value {\n        x @ Some(_) => {\n            sink(x);\n        }\n        _ => {}\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "9:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-mut-pattern-inside-arm",
        "mut_pattern",
        "fn f(value: i32) {\n    let x = source();\n    match value {\n        mut x => {\n            sink(x);\n        }\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "8:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-or-pattern-inside-arm",
        "or_pattern",
        "fn f(value: Option<i32>) {\n    let x = source();\n    match value {\n        Some(x) | Ok(x) => {\n            sink(x);\n        }\n        _ => {}\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "9:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-ref-pattern-inside-arm",
        "ref_pattern",
        "fn f(value: i32) {\n    let x = source();\n    match value {\n        ref x => {\n            sink(x);\n        }\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "8:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-slice-pattern-inside-arm",
        "slice_pattern",
        "fn f(value: &[i32]) {\n    let x = source();\n    match value {\n        [x, ..] => {\n            sink(x);\n        }\n        _ => {}\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "9:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-struct-pattern-inside-arm",
        "struct_pattern",
        "fn f(value: S) {\n    let x = source();\n    match value {\n        S { field: x } => {\n            sink(x);\n        }\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "8:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-for-pattern-tuple",
        "tuple_pattern",
        "fn f(items: Vec<(i32, i32)>) {\n    let x = source();\n    for (x, y) in items {\n        sink(x);\n        consume(y);\n    }\n    sink(x);\n}\n",
        &[("2:x", "4:x", uncertain(3)), ("2:x", "7:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
    pattern_case!(
        "rs-tuple-struct-pattern-inside-arm",
        "tuple_struct_pattern",
        "fn f(value: Option<i32>) {\n    let x = source();\n    match value {\n        Some(x) => {\n            sink(x);\n        }\n        None => {}\n    }\n    sink(x);\n}\n",
        &[("2:x", "5:x", uncertain(4)), ("2:x", "9:x", EXACT)],
        ("dfg_label_nameonly_ownership_uncertain", 1)
    ),
];

fn supplemental(case: &Case) -> Case {
    Case {
        variant: None,
        ..*case
    }
}

const GUARD_EXPECT: &[(&str, &str, FlowConfidence)] =
    &[("2:x", "6:x", uncertain(4)), ("2:x", "10:x", EXACT)];
const LET_CHAIN_EXPECT: &[(&str, &str, FlowConfidence)] = &[
    ("2:x", "4:x", uncertain(3)),
    ("2:x", "6:x", uncertain(3)),
    ("2:x", "8:x", EXACT),
];
const LET_SCOPE_EXPECT: &[(&str, &str, FlowConfidence)] =
    &[("2:x", "5:x", uncertain(4)), ("2:x", "8:x", EXACT)];

#[test]
fn task15_rust_pattern_cases() {
    for case in CASES {
        super::super::case::check_case(case);
    }
}

macro_rules! row_case_test {
    ($test:ident, $index:expr) => {
        #[test]
        fn $test() {
            super::super::case::check_case(&supplemental(&CASES[$index]));
        }
    };
}

row_case_test!(captured_pattern_behavior, 0);
row_case_test!(mut_pattern_behavior, 1);
row_case_test!(or_pattern_behavior, 2);
row_case_test!(ref_pattern_behavior, 3);
row_case_test!(slice_pattern_behavior, 4);
row_case_test!(struct_pattern_behavior, 5);
row_case_test!(tuple_pattern_behavior, 6);
row_case_test!(tuple_struct_pattern_behavior, 7);

#[test]
fn task15_pattern_behavior_is_independent_of_row_identity() {
    for case in CASES {
        super::super::case::check_case(&supplemental(case));
    }
}

#[test]
fn rust_guard_if_let_is_uncertain_only_in_its_arm() {
    super::super::case::check_case(&Case {
        id: "rs-guard-if-let-inside-arm",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(value: Option<i32>, gate: Option<i32>) {\n    let x = source();\n    match value {\n        Some(y) if let Some(x) = gate => {\n            consume(y);\n            sink(x);\n        }\n        _ => {}\n    }\n    sink(x);\n}\n",
        expect: GUARD_EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    });
}

#[test]
fn rust_let_chain_binding_reaches_later_condition_and_body_only() {
    super::super::case::check_case(&Case {
        id: "rs-let-chain-pattern-scope",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(value: Option<i32>) {\n    let x = source();\n    if let Some(x) = value\n        && check(x)\n    {\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: LET_CHAIN_EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 2)),
    });
}

#[test]
fn rust_let_pattern_doubt_ends_at_lexical_scope_exit() {
    super::super::case::check_case(&Case {
        id: "rs-let-pattern-scope-exit",
        language: Language::Rust,
        kind: "tuple_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(pair: (i32, i32)) {\n    let x = source();\n    {\n        let (x, y) = pair;\n        sink(x);\n        consume(y);\n    }\n    sink(x);\n}\n",
        expect: LET_SCOPE_EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    });
}

#[test]
fn rust_let_pattern_visibility_is_the_following_inner_scope_only() {
    let parsed = super::super::super::parsed(
        "fn f(pair: (i32, i32)) {\n    let x = source();\n    {\n        let (x, y) = pair;\n        sink(x);\n    }\n    sink(x);\n}\n",
        Language::Rust,
    );
    let pattern = super::super::case::nodes_of_kind(&parsed, "tuple_pattern")[0];
    let declaration = pattern.parent().expect("let declaration");
    let block = declaration.parent().expect("inner block");
    assert_eq!(declaration.kind(), "let_declaration");
    assert_eq!(block.kind(), "block");
    assert_eq!(
        visibility_span(&parsed, pattern),
        SpanKind::Known(vec![ByteSpan {
            start: declaration.end_byte(),
            end: block.end_byte(),
        }])
    );
}

#[test]
fn rust_closure_pattern_capture_is_cfg_incomplete_before_pattern_doubt() {
    super::super::case::check_case(&Case {
        id: "rs-closure-param-capture",
        language: Language::Rust,
        kind: "closure_expression",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    let closure = |(x, y): (i32, i32)| {\n        sink(x);\n        consume(y);\n    };\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", CFG_INCOMPLETE), ("2:x", "7:x", EXACT)],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    });
}

#[test]
fn rust_pattern_row_does_not_claim_an_arm_assignment() {
    super::super::case::check_case(&Case {
        id: "rs-pattern-arm-assignment-preserved",
        language: Language::Rust,
        kind: "assignment_expression",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(value: (i32, i32)) {\n    let mut x = source();\n    match value {\n        (left, right) => {\n            x = clean();\n            sink(x);\n            consume(left + right);\n        }\n    }\n    sink(x);\n}\n",
        expect: &[
            (
                "2:x",
                "6:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 5 }),
            ),
            ("5:x", "6:x", EXACT),
            ("5:x", "10:x", EXACT),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    });
}

#[test]
fn rust_pattern_grammar_has_no_per_identifier_name_field() {
    let parsed = super::super::super::parsed(
        "fn f(value: Option<i32>) { match value { bound @ Some(inner) => consume(bound + inner), _ => {} } }",
        Language::Rust,
    );
    let captured = super::super::case::nodes_of_kind(&parsed, "captured_pattern")[0];
    assert!(captured.child_by_field_name("name").is_none());
    assert!(captured.child_by_field_name("pattern").is_none());
    let identifiers: Vec<_> = captured
        .named_children(&mut captured.walk())
        .map(|node| parsed.node_text(&node))
        .collect();
    assert_eq!(identifiers, ["bound", "Some(inner)"]);
}

#[test]
fn rust_if_let_initializer_and_else_bypass_preserve_outer_binding() {
    const EXPECT: &[(&str, &str, FlowConfidence)] = &[
        ("2:x", "4:x", EXACT),
        ("2:x", "6:x", uncertain(3)),
        ("2:x", "8:x", EXACT),
        ("2:x", "10:x", EXACT),
    ];
    super::super::case::check_case(&Case {
        id: "rs-if-let-initializer-else-bypass",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(value: Option<i32>) {\n    let x = source();\n    if let Some(x) =\n        prepare(x)\n    {\n        sink(x);\n    } else {\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    });
}

#[test]
fn rust_let_pattern_initializer_reads_outer_binding() {
    super::super::case::check_case(&Case {
        id: "rs-let-pattern-initializer-control",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Structural,
        src: "fn f() {\n    let x = source();\n    let y = source();\n    let Some(a) = x;\n    let Some(b) = y;\n    consume(a + b);\n}\n",
        expect: &[("2:x", "4:x", EXACT), ("3:y", "5:y", EXACT)],
        expect_counter: Some(("dfg_label_exact", 2)),
    });
}

#[test]
fn rust_while_let_pattern_is_uncertain_in_body_and_discharged_after_loop() {
    const EXPECT: &[(&str, &str, FlowConfidence)] =
        &[("2:x", "4:x", uncertain(3)), ("2:x", "6:x", EXACT)];
    super::super::case::check_case(&Case {
        id: "rs-while-let-pattern-loop",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f() {\n    let x = source();\n    while let Some(x) = next() {\n        sink(x);\n    }\n    sink(x);\n}\n",
        expect: EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    });
}

#[test]
fn rust_for_pattern_does_not_taint_iterable_or_unrelated_body_name() {
    super::super::case::check_case(&Case {
        id: "rs-for-pattern-initializer-control",
        language: Language::Rust,
        kind: "for_expression",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(items: Vec<(i32, i32)>) {\n    let x = source();\n    for (left, right) in\n        select(items, x)\n    {\n        sink(x);\n        consume(left + right);\n    }\n    sink(x);\n}\n",
        expect: &[("2:x", "4:x", EXACT), ("2:x", "6:x", EXACT), ("2:x", "9:x", EXACT)],
        expect_counter: Some(("dfg_label_exact", 3)),
    });
}

#[test]
fn rust_nested_tuple_in_tuple_struct_inherits_outer_pattern_span() {
    const EXPECT: &[(&str, &str, FlowConfidence)] =
        &[("2:x", "5:x", uncertain(4)), ("2:x", "9:x", EXACT)];
    super::super::case::check_case(&Case {
        id: "rs-nested-tuple-pattern",
        language: Language::Rust,
        kind: "tuple_struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(value: Option<(i32, i32)>) {\n    let x = source();\n    match value {\n        Some((x, y)) => {\n            sink(x);\n            consume(y);\n        }\n        None => {}\n    }\n    sink(x);\n}\n",
        expect: EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    });
}

#[test]
fn rust_nested_ref_in_struct_inherits_outer_pattern_span() {
    const EXPECT: &[(&str, &str, FlowConfidence)] =
        &[("2:y", "5:y", uncertain(4)), ("2:y", "8:y", EXACT)];
    super::super::case::check_case(&Case {
        id: "rs-nested-ref-struct-pattern",
        language: Language::Rust,
        kind: "struct_pattern",
        variant: None,
        row_variant: None,
        class: CaseClass::Behavioral,
        src: "fn f(value: S) {\n    let y = source();\n    match value {\n        S { x: ref y, .. } => {\n            sink(y);\n        }\n    }\n    sink(y);\n}\n",
        expect: EXPECT,
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    });
}

#[test]
fn rust_constructor_and_field_labels_are_not_pattern_bindings() {
    for case in [
        Case {
            id: "rs-tuple-struct-constructor-control",
            language: Language::Rust,
            kind: "match_expression",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "fn f(value: Option<i32>) {\n    let Some = source();\n    match value {\n        Some(x) => {\n            sink(Some);\n            consume(x);\n        }\n        None => {}\n    }\n    sink(Some);\n}\n",
            expect: &[("2:Some", "5:Some", EXACT), ("2:Some", "10:Some", EXACT)],
            expect_counter: Some(("dfg_label_exact", 2)),
        },
        Case {
            id: "rs-struct-field-label-control",
            language: Language::Rust,
            kind: "match_expression",
            variant: None,
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "fn f(value: S) {\n    let field = source();\n    match value {\n        S { field: x } => {\n            sink(field);\n            consume(x);\n        }\n    }\n    sink(field);\n}\n",
            expect: &[("2:field", "5:field", EXACT), ("2:field", "9:field", EXACT)],
            expect_counter: Some(("dfg_label_exact", 2)),
        },
    ] {
        super::super::case::check_case(&case);
    }
}

#[test]
fn rust_pattern_definition_cells_are_bounded_for_every_kind() {
    for (case, pattern_line, inside_line, outside_line) in [
        (&CASES[0], 4, 5, 9),
        (&CASES[1], 4, 5, 8),
        (&CASES[2], 4, 5, 9),
        (&CASES[3], 4, 5, 8),
        (&CASES[4], 4, 5, 9),
        (&CASES[5], 4, 5, 8),
        (&CASES[6], 3, 4, 7),
        (&CASES[7], 4, 5, 9),
    ] {
        let parsed = super::super::super::parsed(case.src, Language::Rust);
        let function = super::super::super::function(&parsed);
        let pattern_def = super::super::super::def(&parsed, 0, "x", pattern_line, 0, false);
        let inside = super::super::super::edge(&parsed, &function, &pattern_def, inside_line);
        let outside = super::super::super::edge(&parsed, &function, &pattern_def, outside_line);
        let outcome = super::super::super::run(
            &parsed,
            std::slice::from_ref(&pattern_def),
            &[inside.clone(), outside.clone()],
        )
        .0;
        super::super::super::assert_label(&outcome, &inside, EXACT);
        let crate::cpg::reaching::RdOutcome::Available(result) = outcome else {
            panic!("{}: expected available result", case.id);
        };
        assert!(
            matches!(
                result.labels.get(&(outside.from, outside.to)),
                Some(FlowConfidence::NameOnly(_))
            ),
            "{}: pattern definition reaching outside its scope must be NameOnly",
            case.id
        );
    }
}

#[test]
fn rust_pattern_rows_are_fail_closed_and_fieldless() {
    let expected = [
        ("captured_pattern", "rs-captured-pattern"),
        ("mut_pattern", "rs-mut-pattern-inside-arm"),
        ("or_pattern", "rs-or-pattern-inside-arm"),
        ("ref_pattern", "rs-ref-pattern-inside-arm"),
        ("slice_pattern", "rs-slice-pattern-inside-arm"),
        ("struct_pattern", "rs-struct-pattern-inside-arm"),
        ("tuple_pattern", "rs-for-pattern-tuple"),
        ("tuple_struct_pattern", "rs-tuple-struct-pattern-inside-arm"),
    ];
    for (kind, regression) in expected {
        let row = rows(Language::Rust)
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing {kind}"));
        assert_eq!(row.regression, regression, "{kind}");
        assert!(row.roles.has(Role::Binding), "{kind}");
        assert_eq!(row.fields, &[] as &[&str], "{kind}");
        assert_eq!(row.declaration, Some(DeclarationKind::Pattern), "{kind}");
        assert_eq!(row.visibility, Visibility::Header, "{kind}");
        assert_eq!(
            row.ruling,
            Ruling::Uncertain {
                reason: "pattern identifiers not field-addressable",
                revisit: "future:rust-patterns",
            },
            "{kind}"
        );
    }
}
