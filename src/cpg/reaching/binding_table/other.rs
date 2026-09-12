use super::{BindingRow, DeclarationKind, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) static JAVA_ROWS: &[BindingRow] = &[
    BindingRow {
        language: Language::Java,
        kind: "formal_parameters",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "e0a-x-java-formal_parameters",
    },
    BindingRow {
        language: Language::Java,
        kind: "spread_parameter",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::Parameter),
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "e0a-x-java-spread_parameter",
    },
];

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

pub(super) static LUA_ROWS: &[BindingRow] = &[BindingRow {
    language: Language::Lua,
    kind: "parameters",
    variant: None,
    roles: RoleSet::of(&[Role::Binding]),
    fields: &["name"],
    declaration: Some(DeclarationKind::Parameter),
    visibility: Visibility::WholeScope,
    ruling: Ruling::Classified,
    regression: "e0a-x-lua-parameters",
}];
