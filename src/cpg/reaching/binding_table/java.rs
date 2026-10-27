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
