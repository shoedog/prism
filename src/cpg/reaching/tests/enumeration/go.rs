use super::super::super::binding_table::{
    rows, DeclarationKind, Predicate, Role, Ruling, Visibility,
};
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

pub(super) const CASES: &[super::case::Case] = &[
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
        variant: Some("binding"),
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
];

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
            Visibility::WholeScope,
            "e0a-go-if_statement",
        ),
        (
            "for_statement",
            true,
            false,
            None,
            Visibility::WholeScope,
            "e0a-go-for_statement",
        ),
        (
            "expression_switch_statement",
            true,
            false,
            None,
            Visibility::WholeScope,
            "e0a-go-expression_switch_statement",
        ),
        (
            "type_switch_statement",
            true,
            false,
            None,
            Visibility::WholeScope,
            "e0a-go-type_switch_statement",
        ),
        (
            "select_statement",
            true,
            false,
            None,
            Visibility::WholeScope,
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

    assert!(go_rows[expected.len() + 2..].iter().all(|row| matches!(
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
    assert_eq!(range_rows[1].variant, None);
    assert_eq!(range_rows[1].declaration, Some(DeclarationKind::Reuse));
    assert_eq!(range_rows[1].visibility, Visibility::AfterIntroduction);
}
