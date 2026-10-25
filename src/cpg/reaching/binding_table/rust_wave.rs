use super::{provisional_rows, BindingRow, DeclarationKind, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) const DIGEST: &str = "4b73a1248978340336100db455bf0731c23f9190568c9ae62265fa4a80a327d5";
pub(super) const CENSUS_DIGEST: &str =
    "51a5153bc59df68124f585b893130fe294fbef303930ecdd1d6db1c0e748e44f";

const PATTERN_UNCERTAIN: Ruling = Ruling::Uncertain {
    reason: "pattern identifiers not field-addressable",
    revisit: "future:rust-patterns",
};

const fn pattern_row(kind: &'static str, regression: &'static str) -> BindingRow {
    BindingRow {
        language: Language::Rust,
        kind,
        variant: None,
        roles: RoleSet::of(&[Role::Binding]),
        fields: &[],
        declaration: Some(DeclarationKind::Pattern),
        visibility: Visibility::Header,
        ruling: PATTERN_UNCERTAIN,
        regression,
    }
}

pub(super) static CURATED_ROWS: &[BindingRow] = &[
    pattern_row("captured_pattern", "rs-captured-pattern"),
    pattern_row("mut_pattern", "rs-mut-pattern-inside-arm"),
    pattern_row("or_pattern", "rs-or-pattern-inside-arm"),
    pattern_row("ref_pattern", "rs-ref-pattern-inside-arm"),
    pattern_row("slice_pattern", "rs-slice-pattern-inside-arm"),
    pattern_row("struct_pattern", "rs-struct-pattern-inside-arm"),
    pattern_row("tuple_pattern", "rs-for-pattern-tuple"),
    pattern_row("tuple_struct_pattern", "rs-tuple-struct-pattern-inside-arm"),
];

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Rust,
    "E1c",
    [
        ("assignment_expression", "e0b-rs-assignment_expression"),
        ("async_block", "e0b-rs-async_block"),
        ("block_comment", "e0b-rs-block_comment"),
        ("closure_expression", "e0b-rs-closure_expression"),
        (
            "compound_assignment_expr",
            "e0b-rs-compound_assignment_expr"
        ),
        ("const_parameter", "e0b-rs-const_parameter"),
        ("declaration_list", "e0b-rs-declaration_list"),
        ("enum_item", "e0b-rs-enum_item"),
        (
            "extern_crate_declaration",
            "e0b-rs-extern_crate_declaration"
        ),
        ("field_declaration", "e0b-rs-field_declaration"),
        ("field_declaration_list", "e0b-rs-field_declaration_list"),
        ("field_pattern", "e0b-rs-field_pattern"),
        ("for_expression", "e0b-rs-for_expression"),
        ("for_lifetimes", "e0b-rs-for_lifetimes"),
        ("function_item", "e0b-rs-function_item"),
        ("function_modifiers", "e0b-rs-function_modifiers"),
        ("function_signature_item", "e0b-rs-function_signature_item"),
        ("function_type", "e0b-rs-function_type"),
        ("gen_block", "e0b-rs-gen_block"),
        ("generic_function", "e0b-rs-generic_function"),
        ("generic_pattern", "e0b-rs-generic_pattern"),
        (
            "generic_type_with_turbofish",
            "e0b-rs-generic_type_with_turbofish"
        ),
        ("lifetime_parameter", "e0b-rs-lifetime_parameter"),
        ("match_block", "e0b-rs-match_block"),
        ("match_expression", "e0b-rs-match_expression"),
        ("match_pattern", "e0b-rs-match_pattern"),
        ("mod_item", "e0b-rs-mod_item"),
        (
            "ordered_field_declaration_list",
            "e0b-rs-ordered_field_declaration_list"
        ),
        ("parameter", "e0b-rs-parameter"),
        ("range_expression", "e0b-rs-range_expression"),
        ("range_pattern", "e0b-rs-range_pattern"),
        ("reference_pattern", "e0b-rs-reference_pattern"),
        ("remaining_field_pattern", "e0b-rs-remaining_field_pattern"),
        ("self_parameter", "e0b-rs-self_parameter"),
        ("struct_item", "e0b-rs-struct_item"),
        ("token_binding_pattern", "e0b-rs-token_binding_pattern"),
        (
            "token_repetition_pattern",
            "e0b-rs-token_repetition_pattern"
        ),
        ("token_tree_pattern", "e0b-rs-token_tree_pattern"),
        ("trait_item", "e0b-rs-trait_item"),
        ("type_binding", "e0b-rs-type_binding"),
        ("type_item", "e0b-rs-type_item"),
        ("type_parameter", "e0b-rs-type_parameter"),
        ("type_parameters", "e0b-rs-type_parameters"),
        ("union_item", "e0b-rs-union_item"),
        ("use_as_clause", "e0b-rs-use_as_clause"),
        ("use_declaration", "e0b-rs-use_declaration"),
        ("variadic_parameter", "e0b-rs-variadic_parameter"),
        ("metavariable", "e0b-rs-metavariable"),
    ]
);
