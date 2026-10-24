use super::super::super::binding_table::{
    rows, DeclarationKind, Predicate, Role, Ruling, Visibility,
};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "e0a-go-block",
        language: Language::Go,
        kind: "block",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  {\n    x := clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("4:x", "5:x", FlowConfidence::Exact), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "e0a-go-if_statement",
        language: Language::Go,
        kind: "if_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  if x := f(); c { use(x) }\n  sink(x)\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "go-range-reuse-inside-loop",
        language: Language::Go,
        kind: "range_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(items []int) {\n  x := source()\n  for _, x = range items {\n    observe(x)\n  }\n  sink(x)\n}\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "go-range-short-var-no-synthesis",
        language: Language::Go,
        kind: "range_clause",
        variant: None,
        row_variant: Some(Predicate::OperatorIs { any_of: &[":="] }),
        class: super::case::CaseClass::Behavioral,
        src: "func f(items []int) {\n  x := source()\n  for _, x := range items {\n    observe(x)\n  }\n  sink(x)\n}\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "go-range-short-var-masks",
        language: Language::Go,
        kind: "range_clause",
        variant: Some("binding"),
        row_variant: Some(Predicate::OperatorIs { any_of: &[":="] }),
        class: super::case::CaseClass::Behavioral,
        src: "func f(items []int) {\n  x := source()\n  for _, x := range items {\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[
            (
                "2:x",
                "4:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "6:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "go-if-init-mask-not-route-kill",
        language: Language::Go,
        kind: "if_statement",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  if x := f(); c { use(x) }\n  sink(x)\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 1)),
    },
    super::case::Case {
        id: "e0a-go-for_statement",
        language: Language::Go,
        kind: "for_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  for x := clean(); x > 0; x-- {\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("3:x", "4:x", FlowConfidence::Exact), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "e0a-go-expression_switch_statement",
        language: Language::Go,
        kind: "expression_switch_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  switch x := clean(); x {\n  case 1:\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("3:x", "5:x", FlowConfidence::Exact), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "e0a-go-select_statement",
        language: Language::Go,
        kind: "select_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(ch chan int) {\n  x := source()\n  select {\n  case <-ch:\n    sink(x)\n  default:\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("2:x", "5:x", FlowConfidence::Exact), ("2:x", "7:x", FlowConfidence::Exact), ("2:x", "9:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 3)),
    },
    super::case::Case {
        id: "e0a-go-short_var_declaration",
        language: Language::Go,
        kind: "short_var_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  {\n    x := clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("2:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-go-var_declaration",
        language: Language::Go,
        kind: "var_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  {\n    var x = clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("2:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-go-const_declaration",
        language: Language::Go,
        kind: "const_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  {\n    const x = 1\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("2:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-x-go-parameter_list",
        language: Language::Go,
        kind: "parameter_list",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  _ = func(x int) {\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[(
            "2:x",
            "4:x",
            FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }),
        ), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    super::case::Case {
        id: "go-parameter-nested-type-name-preserved",
        language: Language::Go,
        kind: "parameter_list",
        variant: None,
        row_variant: None,
        class: super::case::CaseClass::Structural,
        src: "func f() {\n  x := source()\n  _ = func(cb func(x int)) {\n    sink(x)\n  }\n}\n",
        expect: &[(
            "2:x",
            "4:x",
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        )],
        expect_counter: Some(("dfg_label_nameonly_cfg_incomplete", 1)),
    },
    super::case::Case {
        id: "e0a-go-type_switch_statement",
        language: Language::Go,
        kind: "type_switch_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(value any) {\n  x := source()\n  switch x := x.(type) {\n  case int:\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("2:x", "3:x", FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)), ("2:x", "5:x", FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 })), ("2:x", "7:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)),
    },
    super::case::Case {
        id: "go-case-block-scope",
        language: Language::Go,
        kind: "expression_case",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(tag int) {\n  x := source()\n  switch tag {\n  case 1:\n    x := clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("5:x", "6:x", FlowConfidence::Exact), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "go-default-case-block-scope",
        language: Language::Go,
        kind: "default_case",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(tag int) {\n  x := source()\n  switch tag {\n  default:\n    x := clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("5:x", "6:x", FlowConfidence::Exact), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "go-type-case-block-scope",
        language: Language::Go,
        kind: "type_case",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(value any) {\n  x := source()\n  switch value.(type) {\n  case int:\n    x := clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("5:x", "6:x", FlowConfidence::Exact), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "go-communication-case-block-scope",
        language: Language::Go,
        kind: "communication_case",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f(ch chan int) {\n  x := source()\n  select {\n  case <-ch:\n    x := clean()\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("5:x", "6:x", FlowConfidence::Exact), ("2:x", "8:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
    super::case::Case {
        id: "go-for-clause-init",
        language: Language::Go,
        kind: "for_clause",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "func f() {\n  x := source()\n  for x := clean(); x > 0; x-- {\n    sink(x)\n  }\n  sink(x)\n}\n",
        expect: &[("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 })), ("2:x", "6:x", FlowConfidence::Exact)],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("block", "e0a-go-block"),
    ("if_statement", "e0a-go-if_statement"),
    ("for_statement", "e0a-go-for_statement"),
    (
        "expression_switch_statement",
        "e0a-go-expression_switch_statement",
    ),
    ("type_switch_statement", "e0a-go-type_switch_statement"),
    ("select_statement", "e0a-go-select_statement"),
    ("short_var_declaration", "e0a-go-short_var_declaration"),
    ("var_declaration", "e0a-go-var_declaration"),
    ("const_declaration", "e0a-go-const_declaration"),
    ("parameter_list", "e0a-x-go-parameter_list"),
    ("expression_case", "go-case-block-scope"),
    ("default_case", "go-default-case-block-scope"),
    ("type_case", "go-type-case-block-scope"),
    ("communication_case", "go-communication-case-block-scope"),
    ("for_clause", "go-for-clause-init"),
];

#[test]
fn task13_go_binding_cases() {
    for case in CASES {
        super::case::check_case(case);
    }
}

#[test]
fn go_rows_reproduce_the_old_match_arms() {
    let go_rows = rows(Language::Go);
    let expected = [
        (
            "block",
            true,
            false,
            None,
            Visibility::WholeScope,
            "e0a-go-block",
        ),
        (
            "if_statement",
            true,
            false,
            None,
            Visibility::Header,
            "e0a-go-if_statement",
        ),
        (
            "for_statement",
            true,
            false,
            None,
            Visibility::Header,
            "e0a-go-for_statement",
        ),
        (
            "expression_switch_statement",
            true,
            false,
            None,
            Visibility::Header,
            "e0a-go-expression_switch_statement",
        ),
        (
            "type_switch_statement",
            true,
            true,
            Some(DeclarationKind::Other),
            Visibility::Header,
            "e0a-go-type_switch_statement",
        ),
        (
            "select_statement",
            true,
            false,
            None,
            Visibility::Header,
            "e0a-go-select_statement",
        ),
        (
            "short_var_declaration",
            false,
            true,
            Some(DeclarationKind::GoShort),
            Visibility::AfterIntroduction,
            "e0a-go-short_var_declaration",
        ),
        (
            "var_declaration",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::AfterIntroduction,
            "e0a-go-var_declaration",
        ),
        (
            "const_declaration",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::AfterIntroduction,
            "e0a-go-const_declaration",
        ),
        (
            "parameter_list",
            false,
            true,
            Some(DeclarationKind::Parameter),
            Visibility::WholeScope,
            "e0a-x-go-parameter_list",
        ),
    ];

    assert!(go_rows
        .iter()
        .filter(|row| row.regression.starts_with("e0b-go-"))
        .all(|row| matches!(
            row.ruling,
            Ruling::Uncertain {
                reason: "not yet curated",
                ..
            }
        )));
    for (kind, creates_scope, is_binding, declaration, visibility, regression) in expected {
        let row = go_rows
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing row {kind}"));
        assert_eq!(row.roles.has(Role::Scope), creates_scope, "{kind}");
        assert_eq!(row.roles.has(Role::Binding), is_binding, "{kind}");
        assert_eq!(row.declaration, declaration, "{kind}");
        assert_eq!(row.visibility, visibility, "{kind}");
        assert_eq!(row.regression, regression, "{kind}");
    }

    let range_rows: Vec<_> = go_rows
        .iter()
        .filter(|row| row.kind == "range_clause")
        .collect();
    assert_eq!(range_rows.len(), 2);
    assert_eq!(
        range_rows[0].variant,
        Some(Predicate::OperatorIs { any_of: &[":="] })
    );
    assert_eq!(range_rows[0].declaration, Some(DeclarationKind::GoShort));
    assert_eq!(range_rows[0].visibility, Visibility::Header);
    assert_eq!(range_rows[0].regression, "go-range-short-var-masks");
    assert_eq!(range_rows[1].variant, None);
    assert_eq!(range_rows[1].declaration, Some(DeclarationKind::Reuse));
    assert_eq!(range_rows[1].visibility, Visibility::AfterIntroduction);

    let type_switch = go_rows
        .iter()
        .find(|row| row.kind == "type_switch_statement")
        .expect("type-switch row");
    assert_eq!(type_switch.fields, &["alias"]);
    assert!(matches!(
        type_switch.ruling,
        Ruling::Uncertain {
            reason: "type-switch alias has no CFG node",
            revisit: "future:go-type-switch-cfg"
        }
    ));

    let parameters = go_rows
        .iter()
        .find(|row| row.kind == "parameter_list")
        .expect("parameter-list row");
    assert!(matches!(
        parameters.ruling,
        Ruling::Uncertain {
            reason: "parameter list has no CFG binding node",
            revisit: "future:go-parameter-cfg"
        }
    ));

    for kind in [
        "expression_case",
        "default_case",
        "type_case",
        "communication_case",
    ] {
        let row = go_rows
            .iter()
            .find(|row| row.kind == kind)
            .unwrap_or_else(|| panic!("missing row {kind}"));
        assert!(row.roles.has(Role::Scope), "{kind}");
        assert_eq!(row.ruling, Ruling::Classified, "{kind}");
    }

    let for_clause = go_rows
        .iter()
        .find(|row| row.kind == "for_clause")
        .expect("for-clause row");
    assert_eq!(for_clause.fields, &["initializer"]);
    assert_eq!(for_clause.declaration, Some(DeclarationKind::GoShort));
    assert_eq!(for_clause.visibility, Visibility::Header);
}
