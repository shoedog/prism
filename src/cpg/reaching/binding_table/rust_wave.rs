use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "4b73a1248978340336100db455bf0731c23f9190568c9ae62265fa4a80a327d5";
pub(super) const CENSUS_DIGEST: &str =
    "a8c32a150f5bdcf1c895f714b6bed26402a7e08cfde1258fb1a641a775cc3f6c";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Rust,
    "E1c",
    [
        ("assignment_expression", "e0b-rs-assignment_expression"),
        ("async_block", "e0b-rs-async_block"),
        ("block_comment", "e0b-rs-block_comment"),
        ("captured_pattern", "e0b-rs-captured_pattern"),
        ("closure_expression", "e0b-rs-closure_expression"),
        ("closure_parameters", "e0b-rs-closure_parameters"),
        (
            "compound_assignment_expr",
            "e0b-rs-compound_assignment_expr"
        ),
        ("const_block", "e0b-rs-const_block"),
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
        ("let_chain", "e0b-rs-let_chain"),
        ("let_condition", "e0b-rs-let_condition"),
        ("lifetime_parameter", "e0b-rs-lifetime_parameter"),
        ("match_arm", "e0b-rs-match_arm"),
        ("match_block", "e0b-rs-match_block"),
        ("match_expression", "e0b-rs-match_expression"),
        ("match_pattern", "e0b-rs-match_pattern"),
        ("mod_item", "e0b-rs-mod_item"),
        ("mut_pattern", "e0b-rs-mut_pattern"),
        ("or_pattern", "e0b-rs-or_pattern"),
        (
            "ordered_field_declaration_list",
            "e0b-rs-ordered_field_declaration_list"
        ),
        ("parameter", "e0b-rs-parameter"),
        ("range_expression", "e0b-rs-range_expression"),
        ("range_pattern", "e0b-rs-range_pattern"),
        ("ref_pattern", "e0b-rs-ref_pattern"),
        ("reference_pattern", "e0b-rs-reference_pattern"),
        ("remaining_field_pattern", "e0b-rs-remaining_field_pattern"),
        ("self_parameter", "e0b-rs-self_parameter"),
        ("slice_pattern", "e0b-rs-slice_pattern"),
        ("struct_item", "e0b-rs-struct_item"),
        ("struct_pattern", "e0b-rs-struct_pattern"),
        ("token_binding_pattern", "e0b-rs-token_binding_pattern"),
        (
            "token_repetition_pattern",
            "e0b-rs-token_repetition_pattern"
        ),
        ("token_tree_pattern", "e0b-rs-token_tree_pattern"),
        ("trait_item", "e0b-rs-trait_item"),
        ("try_block", "e0b-rs-try_block"),
        ("tuple_pattern", "e0b-rs-tuple_pattern"),
        ("tuple_struct_pattern", "e0b-rs-tuple_struct_pattern"),
        ("type_binding", "e0b-rs-type_binding"),
        ("type_item", "e0b-rs-type_item"),
        ("type_parameter", "e0b-rs-type_parameter"),
        ("type_parameters", "e0b-rs-type_parameters"),
        ("union_item", "e0b-rs-union_item"),
        ("unsafe_block", "e0b-rs-unsafe_block"),
        ("use_as_clause", "e0b-rs-use_as_clause"),
        ("use_declaration", "e0b-rs-use_declaration"),
        ("variadic_parameter", "e0b-rs-variadic_parameter"),
        ("metavariable", "e0b-rs-metavariable"),
    ]
);
