use super::super::super::binding_table::{rows, DeclarationKind, Role};
use crate::languages::Language;

#[test]
fn python_rows_reproduce_the_old_match_arms() {
    let scope_kinds = [
        "function_definition",
        "lambda",
        "class_definition",
        "list_comprehension",
        "set_comprehension",
        "dictionary_comprehension",
        "generator_expression",
    ];
    for k in scope_kinds {
        assert!(
            rows(Language::Python)
                .iter()
                .any(|r| r.kind == k && r.roles.has(Role::Scope)),
            "missing scope row {k}"
        );
    }
    for k in ["assignment", "augmented_assignment", "named_expression"] {
        let r = rows(Language::Python)
            .iter()
            .find(|r| r.kind == k)
            .unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(DeclarationKind::PythonAssignment));
        assert!(r.roles.has(Role::Binding));
    }
}
