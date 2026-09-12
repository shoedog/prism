use super::super::super::binding_table::{rows, DeclarationKind, Role, Visibility};
use crate::languages::Language;

#[test]
fn go_rows_reproduce_the_old_match_arms() {
    let go_rows = rows(Language::Go);

    for k in [
        "block",
        "if_statement",
        "for_statement",
        "expression_switch_statement",
        "type_switch_statement",
        "switch_statement",
        "select_statement",
    ] {
        let r = go_rows
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing scope row {k}"));
        assert!(r.roles.has(Role::Scope));
        assert_eq!(r.declaration, None);
        assert_eq!(r.regression, format!("e0a-go-{k}"));
    }

    for k in [
        "if_statement",
        "for_statement",
        "expression_switch_statement",
        "type_switch_statement",
        "select_statement",
    ] {
        let r = go_rows.iter().find(|r| r.kind == k).unwrap();
        assert_eq!(r.visibility, Visibility::WholeScope);
    }

    for (k, declaration) in [
        ("short_var_declaration", DeclarationKind::GoShort),
        ("var_declaration", DeclarationKind::Other),
        ("const_declaration", DeclarationKind::Other),
    ] {
        let r = go_rows
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(declaration));
        assert!(r.roles.has(Role::Binding));
        assert_eq!(r.regression, format!("e0a-go-{k}"));
    }
}
