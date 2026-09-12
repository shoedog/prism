use super::super::super::binding_table::{rows, DeclarationKind, Role};
use crate::languages::Language;

pub(super) const SOURCE: &str = r#"from __future__ import annotations
import os as operating_system
from .pkg import item as alias
from pkg import *
top = source()
@decorator
def outer[T](a, typed: int, b: int = 1, *args, c: int = 2, **kwargs):
    global global_name
    def inner():
        nonlocal a
        return a
    class C[T](Base):
        pass
    values = [x for x in args]
    unique = {x for x in args}
    mapping = {x: x for x in args}
    stream = (x for x in args)
    target = lambda p=1, *ps, **ks: p
    for left, *rest in args:
        left += 1
    with context() as (first, second), other() as third:
        first = second
    try:
        risky()
    except Error as error:
        del error
    try:
        grouped()
    except* GroupError as group_error:
        pass
    (tuple_left, tuple_right) = pair
    match value:
        case {"key": [head, *tail], **extra} | C(head, value=tail) as whole:
            result = (named := head)
        case 1+2j:
            result = 0
    return result
"#;

pub(super) const CASES: &[super::case::Case] = &[];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("function_definition", "e0a-py-function_definition"),
    ("lambda", "e0a-py-lambda"),
    ("class_definition", "e0a-py-class_definition"),
    ("list_comprehension", "e0a-py-list_comprehension"),
    ("set_comprehension", "e0a-py-set_comprehension"),
    (
        "dictionary_comprehension",
        "e0a-py-dictionary_comprehension",
    ),
    ("generator_expression", "e0a-py-generator_expression"),
    ("assignment", "e0a-py-assignment"),
    ("augmented_assignment", "e0a-py-augmented_assignment"),
    ("named_expression", "e0a-py-named_expression"),
    ("parameters", "e0a-x-py-parameters"),
];

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
