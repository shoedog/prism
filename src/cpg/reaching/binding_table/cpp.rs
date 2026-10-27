use super::{BindingRow, DeclarationKind, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) static CPP_ROWS: &[BindingRow] = &[
    BindingRow {
        language: Language::Cpp,
        kind: "parameter_list",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "e0a-x-cpp-parameter_list",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "parameter_declaration",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "e0a-x-cpp-parameter_declaration",
    },
];
