use super::super::super::binding_table::{rows, DeclarationKind, Role, Ruling, Visibility};
use crate::languages::Language;

pub(super) const CASES: &[super::case::Case] = &[];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("block", "e0a-rs-block"),
    ("let_declaration", "e0a-rs-let_declaration"),
    ("const_item", "e0a-rs-const_item"),
    ("static_item", "e0a-rs-static_item"),
    ("parameters", "e0a-x-rs-parameters"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "generic_pattern" => "fn f<T>(value: T) { match value { Choice::<T> => {}, _ => {} } }",
        "generic_type_with_turbofish" => "fn f<T>() { let value = Type::<T> { field: item }; }",
        "reference_pattern" => "fn f(value: &i32) { let &inner = value; }",
        _ => super::case::source(Language::Rust),
    }
}

#[test]
fn rust_rows_reproduce_the_old_match_arms() {
    let rust_rows = rows(Language::Rust);
    let expected = [
        (
            "block",
            true,
            false,
            None,
            Visibility::WholeScope,
            "e0a-rs-block",
        ),
        (
            "let_declaration",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::AfterIntroduction,
            "e0a-rs-let_declaration",
        ),
        (
            "const_item",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::AfterIntroduction,
            "e0a-rs-const_item",
        ),
        (
            "static_item",
            false,
            true,
            Some(DeclarationKind::Other),
            Visibility::AfterIntroduction,
            "e0a-rs-static_item",
        ),
        (
            "parameters",
            false,
            true,
            Some(DeclarationKind::Parameter),
            Visibility::WholeScope,
            "e0a-x-rs-parameters",
        ),
    ];

    assert!(rust_rows[expected.len()..].iter().all(|row| matches!(
        row.ruling,
        Ruling::Uncertain {
            reason: "not yet curated",
            ..
        }
    )));
    for (kind, creates_scope, is_binding, declaration, visibility, regression) in expected {
        let row = rust_rows
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
