use super::{
    declarator_chain_ruling, provisional_rows, BindingRow, DeclarationKind, Predicate, Role,
    RoleSet, Ruling, Visibility, CHAIN_RECOVERY_ENABLED,
};
use crate::languages::Language;

pub(super) const DIGEST: &str = "fdfd4b1f3dca1516616a1eb615bb6c1ad3082b8937ad21070d0def5dcfe7e535";
pub(super) const CENSUS_DIGEST: &str =
    "6e900fe653d61e792f535b1599ecfe3725c1b26868758dccc10a874e81fcfda3";

const fn scope_row(kind: &'static str, regression: &'static str) -> BindingRow {
    BindingRow {
        language: Language::Cpp,
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

pub(super) fn row_is_active(row: &BindingRow) -> bool {
    CHAIN_RECOVERY_ENABLED || row.regression != "cpp-block-shadow" || row.variant.is_some()
}

pub(super) static CURATED_ROWS: &[BindingRow] = &[
    scope_row("compound_statement", "cpp-lambda-body-block-scope"),
    scope_row("for_statement", "cpp-for-init-scope"),
    scope_row("condition_clause", "cpp-if-init-declaration"),
    BindingRow {
        language: Language::Cpp,
        kind: "for_range_loop",
        variant: None,
        roles: RoleSet::of(&[Role::Scope, Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::Header,
        ruling: Ruling::Classified,
        regression: "cpp-range-for-masks",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "catch_clause",
        variant: None,
        roles: RoleSet::of(&[Role::Scope, Role::Binding]),
        fields: &["parameters"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::Header,
        ruling: Ruling::Classified,
        regression: "cpp-catch-param-masks",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "declaration",
        variant: if CHAIN_RECOVERY_ENABLED {
            Some(Predicate::FieldChildKindIs {
                field: "declarator",
                any_of: &["function_declarator"],
            })
        } else {
            Some(Predicate::FieldChildCountIs {
                field: "declarator",
                count: 1,
            })
        },
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::AfterIntroduction,
        ruling: if CHAIN_RECOVERY_ENABLED {
            Ruling::Uncertain {
                reason: "direct-initialization and function-declaration syntax are ambiguous",
                revisit: "future:cpp-declarators",
            }
        } else {
            Ruling::Classified
        },
        regression: if CHAIN_RECOVERY_ENABLED {
            "cpp-direct-init-uncertain"
        } else {
            "cpp-block-shadow"
        },
    },
    BindingRow {
        language: Language::Cpp,
        kind: "declaration",
        variant: if CHAIN_RECOVERY_ENABLED {
            Some(Predicate::FieldChildCountAtLeastExcludingKinds {
                field: "declarator",
                count: 2,
                excluded: &["function_declarator"],
            })
        } else {
            None
        },
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::AfterIntroduction,
        ruling: declarator_chain_ruling(),
        regression: "cpp-declarator-chain-uncertain",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "declaration",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &["declarator"],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::AfterIntroduction,
        ruling: Ruling::Classified,
        regression: "cpp-block-shadow",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "structured_binding_declarator",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::Other),
        visibility: Visibility::AfterIntroduction,
        ruling: Ruling::Uncertain {
            reason: "N bindings at one line; per-occurrence DefId is a future item",
            revisit: "future:cpp-structured",
        },
        regression: "cpp-structured-binding-uncertain-inside",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "lambda_capture_specifier",
        variant: Some(Predicate::CppReferenceCaptureOnly),
        roles: RoleSet::of(&[Role::NotBinding]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "cpp-capture-by-reference",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "lambda_capture_specifier",
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::CaptureCopy),
        visibility: Visibility::AfterIntroduction,
        ruling: Ruling::Classified,
        regression: "cpp-capture-specifier-multiline",
    },
    BindingRow {
        language: Language::Cpp,
        kind: "lambda_default_capture",
        variant: None,
        roles: RoleSet::of(&[Role::NotBinding]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "cpp-default-capture-not-binding",
    },
];

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Cpp,
    "future:cpp-residual",
    [
        (
            "abstract_array_declarator",
            "e0b-cpp-abstract_array_declarator"
        ),
        (
            "abstract_function_declarator",
            "e0b-cpp-abstract_function_declarator"
        ),
        (
            "abstract_parenthesized_declarator",
            "e0b-cpp-abstract_parenthesized_declarator"
        ),
        (
            "abstract_pointer_declarator",
            "e0b-cpp-abstract_pointer_declarator"
        ),
        (
            "abstract_reference_declarator",
            "e0b-cpp-abstract_reference_declarator"
        ),
        ("alias_declaration", "e0b-cpp-alias_declaration"),
        ("array_declarator", "e0b-cpp-array_declarator"),
        ("assignment_expression", "e0b-cpp-assignment_expression"),
        ("attribute_declaration", "e0b-cpp-attribute_declaration"),
        ("attributed_declarator", "e0b-cpp-attributed_declarator"),
        ("base_class_clause", "e0b-cpp-base_class_clause"),
        ("case_statement", "e0b-cpp-case_statement"),
        ("class_specifier", "e0b-cpp-class_specifier"),
        ("declaration_list", "e0b-cpp-declaration_list"),
        ("delete_expression", "e0b-cpp-delete_expression"),
        ("delete_method_clause", "e0b-cpp-delete_method_clause"),
        (
            "explicit_function_specifier",
            "e0b-cpp-explicit_function_specifier"
        ),
        ("field_declaration", "e0b-cpp-field_declaration"),
        ("field_declaration_list", "e0b-cpp-field_declaration_list"),
        ("friend_declaration", "e0b-cpp-friend_declaration"),
        ("function_declarator", "e0b-cpp-function_declarator"),
        ("function_definition", "e0b-cpp-function_definition"),
        ("init_declarator", "e0b-cpp-init_declarator"),
        (
            "lambda_capture_initializer",
            "e0b-cpp-lambda_capture_initializer"
        ),
        ("lambda_expression", "e0b-cpp-lambda_expression"),
        ("new_declarator", "e0b-cpp-new_declarator"),
        ("noexcept", "e0b-cpp-noexcept"),
        (
            "optional_parameter_declaration",
            "e0b-cpp-optional_parameter_declaration"
        ),
        (
            "optional_type_parameter_declaration",
            "e0b-cpp-optional_type_parameter_declaration"
        ),
        (
            "parameter_pack_expansion",
            "e0b-cpp-parameter_pack_expansion"
        ),
        (
            "parenthesized_declarator",
            "e0b-cpp-parenthesized_declarator"
        ),
        ("pointer_declarator", "e0b-cpp-pointer_declarator"),
        ("pointer_type_declarator", "e0b-cpp-pointer_type_declarator"),
        ("preproc_function_def", "e0b-cpp-preproc_function_def"),
        ("reference_declarator", "e0b-cpp-reference_declarator"),
        ("requires_expression", "e0b-cpp-requires_expression"),
        ("seh_except_clause", "e0b-cpp-seh_except_clause"),
        (
            "static_assert_declaration",
            "e0b-cpp-static_assert_declaration"
        ),
        ("storage_class_specifier", "e0b-cpp-storage_class_specifier"),
        (
            "subscript_range_designator",
            "e0b-cpp-subscript_range_designator"
        ),
        ("template_declaration", "e0b-cpp-template_declaration"),
        ("template_function", "e0b-cpp-template_function"),
        ("template_parameter_list", "e0b-cpp-template_parameter_list"),
        (
            "template_template_parameter_declaration",
            "e0b-cpp-template_template_parameter_declaration"
        ),
        (
            "type_parameter_declaration",
            "e0b-cpp-type_parameter_declaration"
        ),
        ("using_declaration", "e0b-cpp-using_declaration"),
        ("variadic_declarator", "e0b-cpp-variadic_declarator"),
        (
            "variadic_parameter_declaration",
            "e0b-cpp-variadic_parameter_declaration"
        ),
        (
            "variadic_type_parameter_declaration",
            "e0b-cpp-variadic_type_parameter_declaration"
        ),
    ]
);
