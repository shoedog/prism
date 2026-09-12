use super::super::super::binding_table::{rows, DeclarationKind, Role};
use crate::languages::Language;

#[test]
fn rust_rows_reproduce_the_old_match_arms() {
    let rust_rows = rows(Language::Rust);

    let block = rust_rows
        .iter()
        .find(|r| r.kind == "block")
        .expect("missing scope row block");
    assert!(block.roles.has(Role::Scope));
    assert_eq!(block.declaration, None);
    assert_eq!(block.regression, "e0a-rs-block");

    for k in ["let_declaration", "const_item", "static_item"] {
        let r = rust_rows
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(DeclarationKind::Other));
        assert!(r.roles.has(Role::Binding));
        assert_eq!(r.regression, format!("e0a-rs-{k}"));
    }
}
