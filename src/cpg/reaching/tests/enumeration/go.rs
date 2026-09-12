use super::super::super::binding_table::{rows, DeclarationKind, Role, Visibility};
use crate::languages::Language;

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
            "switch_statement",
            true,
            false,
            None,
            Visibility::WholeScope,
            "e0a-go-switch_statement",
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
    ];

    assert_eq!(go_rows.len(), expected.len());
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
}
