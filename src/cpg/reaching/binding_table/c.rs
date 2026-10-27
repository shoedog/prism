use super::{BindingRow, DeclarationKind, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) static C_ROWS: &[BindingRow] = &[
    BindingRow {
        language: Language::C,
        kind: "parameter_list",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "e0a-x-c-parameter_list",
    },
    BindingRow {
        language: Language::C,
        kind: "parameter_declaration",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "e0a-x-c-parameter_declaration",
    },
];
