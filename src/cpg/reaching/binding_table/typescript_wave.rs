use super::{
    provisional_rows, BindingRow, DeclarationKind, Predicate, Role, RoleSet, Ruling, Visibility,
};
use crate::languages::Language;

pub(super) const DIGEST: &str = "c790a733fc756b54d4e54dceeb7d2d51e40d8b57136e70277753a75804cce3e3";
pub(super) const CENSUS_DIGEST: &str =
    "ccc10d7bddb99d2e1a13c5bc9eccb01d9cb8e4523f60a5923bc7f174c770355c";

const PARAMETER_PATTERN_UNCERTAIN: Ruling = Ruling::Uncertain {
    reason: "pattern identifiers not field-addressable",
    revisit: "future:ts-destructuring",
};

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
        language: Language::TypeScript,
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
    row(
        "required_parameter",
        Some(Predicate::TypeScriptParameterPattern { kind: "identifier" }),
        RoleSet::of(&[Role::Binding]),
        &["pattern"],
        Some(DeclarationKind::Parameter),
        Visibility::WholeScope,
        Ruling::Classified,
        "ts-required-parameter-seeds",
    ),
    row(
        "required_parameter",
        Some(Predicate::TypeScriptParameterPattern {
            kind: "rest_pattern",
        }),
        RoleSet::of(&[Role::Binding]),
        &["pattern"],
        Some(DeclarationKind::Parameter),
        Visibility::WholeScope,
        Ruling::Classified,
        "ts-rest-parameter",
    ),
    row(
        "required_parameter",
        None,
        RoleSet::of(&[Role::Binding]),
        &["pattern"],
        None,
        Visibility::WholeScope,
        PARAMETER_PATTERN_UNCERTAIN,
        "ts-required-parameter-pattern-uncertain",
    ),
    row(
        "optional_parameter",
        Some(Predicate::TypeScriptParameterPattern { kind: "identifier" }),
        RoleSet::of(&[Role::Binding]),
        &["pattern"],
        Some(DeclarationKind::Parameter),
        Visibility::WholeScope,
        Ruling::Classified,
        "ts-optional-parameter",
    ),
    row(
        "optional_parameter",
        None,
        RoleSet::of(&[Role::Binding]),
        &["pattern"],
        None,
        Visibility::WholeScope,
        PARAMETER_PATTERN_UNCERTAIN,
        "ts-optional-parameter-pattern-uncertain",
    ),
    row(
        "rest_pattern",
        Some(Predicate::ParentKindIs {
            kind: "required_parameter",
        }),
        RoleSet::of(&[Role::NotBinding]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "ts-rest-pattern-owned-by-parameter",
    ),
    row(
        "enum_assignment",
        None,
        RoleSet::of(&[Role::NotBinding]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "ts-enum-assignment-not-binding",
    ),
    row(
        "abstract_class_declaration",
        None,
        RoleSet::of(&[Role::Scope]),
        &[],
        None,
        Visibility::WholeScope,
        Ruling::Classified,
        "ts-abstract-class-scope",
    ),
    not_binding("ambient_declaration", "ts-ambient-not-binding"),
    not_binding("enum_declaration", "ts-enum-declaration-not-binding"),
    not_binding("interface_declaration", "ts-interface-not-binding"),
    not_binding("type_alias_declaration", "ts-type-alias-not-binding"),
    not_binding("type_parameter", "ts-type-parameter-not-binding"),
    not_binding("type_parameters", "ts-type-parameters-not-binding"),
    not_binding("function_type", "ts-function-type-not-binding"),
    not_binding("method_signature", "ts-method-signature-not-binding"),
    not_binding("constructor_type", "ts-constructor-type-not-binding"),
    not_binding("call_signature", "ts-call-signature-not-binding"),
    not_binding("construct_signature", "ts-construct-signature-not-binding"),
    not_binding("function_signature", "ts-function-signature-not-binding"),
    not_binding(
        "abstract_method_signature",
        "ts-abstract-method-signature-not-binding",
    ),
];

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::TypeScript,
    "E2b-1",
    [
        ("array_pattern", "e0b-ts-array_pattern"),
        ("assignment_expression", "e0b-ts-assignment_expression"),
        ("assignment_pattern", "e0b-ts-assignment_pattern"),
        (
            "augmented_assignment_expression",
            "e0b-ts-augmented_assignment_expression"
        ),
        ("catch_clause", "e0b-ts-catch_clause"),
        ("class", "e0b-ts-class"),
        ("class_heritage", "e0b-ts-class_heritage"),
        ("class_static_block", "e0b-ts-class_static_block"),
        ("enum_body", "e0b-ts-enum_body"),
        ("generator_function", "e0b-ts-generator_function"),
        (
            "generator_function_declaration",
            "e0b-ts-generator_function_declaration"
        ),
        ("import", "e0b-ts-import"),
        ("import_alias", "e0b-ts-import_alias"),
        ("import_attribute", "e0b-ts-import_attribute"),
        ("import_clause", "e0b-ts-import_clause"),
        ("import_require_clause", "e0b-ts-import_require_clause"),
        ("import_specifier", "e0b-ts-import_specifier"),
        ("import_statement", "e0b-ts-import_statement"),
        ("interface_body", "e0b-ts-interface_body"),
        ("mapped_type_clause", "e0b-ts-mapped_type_clause"),
        ("method_definition", "e0b-ts-method_definition"),
        ("named_imports", "e0b-ts-named_imports"),
        ("namespace_import", "e0b-ts-namespace_import"),
        (
            "object_assignment_pattern",
            "e0b-ts-object_assignment_pattern"
        ),
        ("object_pattern", "e0b-ts-object_pattern"),
        ("pair_pattern", "e0b-ts-pair_pattern"),
        ("rest_pattern", "e0b-ts-rest_pattern"),
        ("switch_body", "e0b-ts-switch_body"),
        ("switch_case", "e0b-ts-switch_case"),
        ("variable_declarator", "e0b-ts-variable_declarator"),
        ("with_statement", "e0b-ts-with_statement"),
        ("regex_pattern", "e0b-ts-regex_pattern"),
        (
            "shorthand_property_identifier_pattern",
            "e0b-ts-shorthand_property_identifier_pattern"
        ),
    ]
);
