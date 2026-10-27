use super::case::{Case, CaseClass};
use crate::languages::Language;

pub(super) const CASES: &[Case] = &[Case {
    id: "hcl-variable-expr-not-binding",
    language: Language::Terraform,
    kind: "variable_expr",
    variant: Some("binding"),
    row_variant: None,
    class: CaseClass::Structural,
    src: "locals {\n  x = source()\n  through = var.input\n  y = x\n}\n",
    expect: &[],
    expect_counter: None,
}];

#[test]
fn task27_terraform_rows_are_census_only_except_variable_reference() {
    use crate::cpg::reaching::binding_table::{rows, Role, Ruling};
    for row in rows(Language::Terraform) {
        assert_eq!(
            row.ruling,
            Ruling::Uncertain {
                reason: "census-only language",
                revisit: "future:scripting",
            },
            "{}",
            row.kind
        );
        if row.kind == "variable_expr" {
            assert!(row.roles.has(Role::NotBinding));
        } else {
            assert!(row.roles.has(Role::Binding));
        }
    }
}
