use super::{
    provisional_rows, BindingRow, DeclarationKind, Predicate, Role, RoleSet, Ruling, Visibility,
};
use crate::languages::Language;

pub(super) const DIGEST: &str = "78b5789145286799a27a0a7ecc36cc1bcb151f94ec7fa631b248459867010c8c";
pub(super) const CENSUS_DIGEST: &str =
    "cff617f7e7e30b9c0a2d40c709689c72102842d0cc3662dfeb33f65f20904cc6";

pub(super) fn replaces_javascript_row(kind: &str) -> bool {
    matches!(
        kind,
        "statement_block"
            | "class_body"
            | "function_declaration"
            | "function_expression"
            | "arrow_function"
            | "for_statement"
            | "formal_parameters"
            | "variable_declaration"
            | "lexical_declaration"
    )
}

#[allow(clippy::too_many_arguments)]
const fn row(
    kind: &'static str,
    variant: Option<Predicate>,
    roles: RoleSet,
    fields: &'static [&'static str],
    declaration: Option<DeclarationKind>,
    visibility: Visibility,
    ruling: Ruling,
    regression: &'static str,
) -> BindingRow {
    BindingRow {
        language: Language::Tsx,
        kind,
        variant,
        roles,
        fields,
        declaration,
        visibility,
        ruling,
        regression,
    }
}

const fn not_binding(kind: &'static str, regression: &'static str) -> BindingRow {
    row(
        kind,
        None,
        RoleSet::of(&[Role::NotBinding]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        regression,
    )
}

pub(super) static CURATED_ROWS: &[BindingRow] = &[
    row(
        "statement_block",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-statement_block",
    ),
    row(
        "class_body",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-class_body",
    ),
    row(
        "function_declaration",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-function_declaration",
    ),
    row(
        "function_expression",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-function_expression",
    ),
    row(
        "arrow_function",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-arrow_function",
    ),
    row(
        "for_statement",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-for_statement",
    ),
    row(
        "formal_parameters",
        None,
        RoleSet::of(&[Role::Binding]),
        &[],
        Some(DeclarationKind::Parameter),
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-x-js-formal_parameters",
    ),
    row(
        "variable_declaration",
        None,
        RoleSet::of(&[Role::Binding]),
        &[],
        Some(DeclarationKind::JavaScriptVar),
        Visibility::WholeScope,
        Ruling::Classified,
        "e0a-js-variable_declaration",
    ),
    row(
        "lexical_declaration",
        None,
        RoleSet::of(&[Role::Binding]),
        &[],
        Some(DeclarationKind::Other),
        Visibility::AfterIntroduction,
        Ruling::Classified,
        "e0a-js-lexical_declaration",
    ),
    not_binding("ambient_declaration", "tsx-ambient-not-binding"),
    not_binding("enum_declaration", "tsx-enum-declaration-not-binding"),
    not_binding("interface_declaration", "tsx-interface-not-binding"),
    not_binding("type_alias_declaration", "tsx-type-alias-not-binding"),
    not_binding("type_parameter", "tsx-type-parameter-not-binding"),
    not_binding("type_parameters", "tsx-type-parameters-not-binding"),
    not_binding("function_type", "tsx-function-type-not-binding"),
    not_binding("method_signature", "tsx-method-signature-not-binding"),
    not_binding("constructor_type", "tsx-constructor-type-not-binding"),
    not_binding("call_signature", "tsx-call-signature-not-binding"),
    not_binding("construct_signature", "tsx-construct-signature-not-binding"),
    not_binding("function_signature", "tsx-function-signature-not-binding"),
    not_binding(
        "abstract_method_signature",
        "tsx-abstract-method-signature-not-binding",
    ),
    not_binding("jsx_attribute", "tsx-jsx-attribute-not-binding"),
    not_binding("jsx_expression", "tsx-jsx-expression-not-binding"),
    not_binding("jsx_namespace_name", "tsx-jsx-namespace-name-not-binding"),
];

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Tsx,
    "future:tsx-residual",
    [
        (
            "abstract_class_declaration",
            "e0b-tsx-abstract_class_declaration"
        ),
        ("array_pattern", "e0b-tsx-array_pattern"),
        ("assignment_expression", "e0b-tsx-assignment_expression"),
        ("assignment_pattern", "e0b-tsx-assignment_pattern"),
        (
            "augmented_assignment_expression",
            "e0b-tsx-augmented_assignment_expression"
        ),
        ("catch_clause", "e0b-tsx-catch_clause"),
        ("class", "e0b-tsx-class"),
        ("class_heritage", "e0b-tsx-class_heritage"),
        ("class_static_block", "e0b-tsx-class_static_block"),
        ("enum_assignment", "e0b-tsx-enum_assignment"),
        ("enum_body", "e0b-tsx-enum_body"),
        ("generator_function", "e0b-tsx-generator_function"),
        (
            "generator_function_declaration",
            "e0b-tsx-generator_function_declaration"
        ),
        ("import", "e0b-tsx-import"),
        ("import_alias", "e0b-tsx-import_alias"),
        ("import_attribute", "e0b-tsx-import_attribute"),
        ("import_clause", "e0b-tsx-import_clause"),
        ("import_require_clause", "e0b-tsx-import_require_clause"),
        ("import_specifier", "e0b-tsx-import_specifier"),
        ("import_statement", "e0b-tsx-import_statement"),
        ("interface_body", "e0b-tsx-interface_body"),
        ("mapped_type_clause", "e0b-tsx-mapped_type_clause"),
        ("method_definition", "e0b-tsx-method_definition"),
        ("named_imports", "e0b-tsx-named_imports"),
        ("namespace_import", "e0b-tsx-namespace_import"),
        (
            "object_assignment_pattern",
            "e0b-tsx-object_assignment_pattern"
        ),
        ("object_pattern", "e0b-tsx-object_pattern"),
        ("optional_parameter", "e0b-tsx-optional_parameter"),
        ("pair_pattern", "e0b-tsx-pair_pattern"),
        ("required_parameter", "e0b-tsx-required_parameter"),
        ("rest_pattern", "e0b-tsx-rest_pattern"),
        ("switch_body", "e0b-tsx-switch_body"),
        ("switch_case", "e0b-tsx-switch_case"),
        ("variable_declarator", "e0b-tsx-variable_declarator"),
        ("with_statement", "e0b-tsx-with_statement"),
        ("regex_pattern", "e0b-tsx-regex_pattern"),
        (
            "shorthand_property_identifier_pattern",
            "e0b-tsx-shorthand_property_identifier_pattern"
        ),
    ]
);
