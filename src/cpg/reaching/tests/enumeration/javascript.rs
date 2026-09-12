use super::super::super::binding_table::{rows, DeclarationKind, Predicate, Role, Visibility};
use crate::languages::Language;

#[test]
fn javascript_rows_reproduce_the_old_match_arms() {
    let javascript_rows = rows(Language::JavaScript);
    assert!(std::ptr::eq(javascript_rows, rows(Language::TypeScript)));
    assert!(std::ptr::eq(javascript_rows, rows(Language::Tsx)));

    for k in [
        "statement_block",
        "class_body",
        "function_declaration",
        "function_expression",
        "arrow_function",
        "for_statement",
    ] {
        assert!(
            javascript_rows
                .iter()
                .any(|r| r.kind == k && r.roles.has(Role::Scope)),
            "missing scope row {k}"
        );
    }

    for (k, declaration) in [
        ("variable_declaration", DeclarationKind::JavaScriptVar),
        ("lexical_declaration", DeclarationKind::Other),
        ("class_declaration", DeclarationKind::Other),
    ] {
        let r = javascript_rows
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(declaration));
        assert!(r.roles.has(Role::Binding));
    }

    let for_in_rows: Vec<_> = javascript_rows
        .iter()
        .filter(|r| r.kind == "for_in_statement")
        .collect();
    assert_eq!(for_in_rows.len(), 2);
    let header = for_in_rows
        .iter()
        .find(|r| r.variant.is_some())
        .expect("missing for_in_statement predicate row");
    assert_eq!(header.declaration, Some(DeclarationKind::Other));
    assert_eq!(header.visibility, Visibility::Header);
    assert!(header.roles.has(Role::Scope));
    assert!(header.roles.has(Role::Binding));
    assert!(matches!(
        header.variant,
        Some(Predicate::FieldTextIs {
            field: "kind",
            any_of: &["let", "const"]
        })
    ));
    let residual = for_in_rows
        .iter()
        .find(|r| r.variant.is_none())
        .expect("missing for_in_statement residual row");
    assert_eq!(residual.declaration, None);
    assert!(residual.roles.has(Role::Scope));
    assert!(!residual.roles.has(Role::Binding));
    assert!(residual.fields.is_empty());
    assert_eq!(residual.visibility, Visibility::WholeScope);
}
