use super::{provisional_rows, BindingRow, DeclarationKind, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) const DIGEST: &str = "a5dd5e5316dc9c00d6e370673260a1ba35fa8d422e6150debf5dea527aae7130";
pub(super) const CENSUS_DIGEST: &str =
    "1333d848dbf6de55318ae14b9932ae9c75bc162faec9651ba152248699b0166c";

pub(super) static CURATED_ROWS: &[BindingRow] = &[
    scope_row("expression_case", "go-case-block-scope"),
    scope_row("default_case", "go-default-case-block-scope"),
    scope_row("type_case", "go-type-case-block-scope"),
    scope_row("communication_case", "go-communication-case-block-scope"),
    BindingRow {
        language: Language::Go,
        kind: "for_clause",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["initializer"],
        declaration: Some(DeclarationKind::GoShort),
        visibility: Visibility::Header,
        ruling: Ruling::Classified,
        regression: "go-for-clause-init",
    },
];

const fn scope_row(kind: &'static str, regression: &'static str) -> BindingRow {
    BindingRow {
        language: Language::Go,
        kind,
        variant: None,
        roles: RoleSet::of(&[Role::Scope]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression,
    }
}

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Go,
    "E1b",
    [
        ("assignment_statement", "e0b-go-assignment_statement"),
        ("field_declaration", "e0b-go-field_declaration"),
        ("field_declaration_list", "e0b-go-field_declaration_list"),
        ("func_literal", "e0b-go-func_literal"),
        ("function_declaration", "e0b-go-function_declaration"),
        ("function_type", "e0b-go-function_type"),
        ("import_declaration", "e0b-go-import_declaration"),
        ("import_spec", "e0b-go-import_spec"),
        ("import_spec_list", "e0b-go-import_spec_list"),
        ("method_declaration", "e0b-go-method_declaration"),
        ("method_elem", "e0b-go-method_elem"),
        ("parameter_declaration", "e0b-go-parameter_declaration"),
        ("type_declaration", "e0b-go-type_declaration"),
        (
            "type_parameter_declaration",
            "e0b-go-type_parameter_declaration"
        ),
        ("type_parameter_list", "e0b-go-type_parameter_list"),
        (
            "variadic_parameter_declaration",
            "e0b-go-variadic_parameter_declaration"
        ),
    ]
);
