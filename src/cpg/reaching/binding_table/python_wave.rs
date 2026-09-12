use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "7ff6914d278faf4254d923e1205b887f940cfb3db46a81eddbde01e782707ac3";
pub(super) const CENSUS_DIGEST: &str =
    "5de434fab529e178786ea4278ecd6cbe158bce41cbbf2e422e9c578034c08b1e";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Python,
    "E1a-1",
    [
        ("aliased_import", "e0b-py-aliased_import"),
        ("as_pattern", "e0b-py-as_pattern"),
        ("block", "e0b-py-block"),
        ("case_clause", "e0b-py-case_clause"),
        ("case_pattern", "e0b-py-case_pattern"),
        ("class_pattern", "e0b-py-class_pattern"),
        ("complex_pattern", "e0b-py-complex_pattern"),
        ("default_parameter", "e0b-py-default_parameter"),
        ("delete_statement", "e0b-py-delete_statement"),
        ("dict_pattern", "e0b-py-dict_pattern"),
        (
            "dictionary_splat_pattern",
            "e0b-py-dictionary_splat_pattern"
        ),
        ("except_clause", "e0b-py-except_clause"),
        ("except_group_clause", "e0b-py-except_group_clause"),
        ("for_in_clause", "e0b-py-for_in_clause"),
        ("for_statement", "e0b-py-for_statement"),
        ("future_import_statement", "e0b-py-future_import_statement"),
        ("global_statement", "e0b-py-global_statement"),
        ("import_from_statement", "e0b-py-import_from_statement"),
        ("import_prefix", "e0b-py-import_prefix"),
        ("import_statement", "e0b-py-import_statement"),
        ("keyword_pattern", "e0b-py-keyword_pattern"),
        ("lambda_parameters", "e0b-py-lambda_parameters"),
        ("list_pattern", "e0b-py-list_pattern"),
        ("list_splat_pattern", "e0b-py-list_splat_pattern"),
        ("match_statement", "e0b-py-match_statement"),
        ("nonlocal_statement", "e0b-py-nonlocal_statement"),
        ("pattern_list", "e0b-py-pattern_list"),
        ("relative_import", "e0b-py-relative_import"),
        ("splat_pattern", "e0b-py-splat_pattern"),
        ("tuple_pattern", "e0b-py-tuple_pattern"),
        ("type_parameter", "e0b-py-type_parameter"),
        ("typed_default_parameter", "e0b-py-typed_default_parameter"),
        ("typed_parameter", "e0b-py-typed_parameter"),
        ("union_pattern", "e0b-py-union_pattern"),
        ("wildcard_import", "e0b-py-wildcard_import"),
        ("with_clause", "e0b-py-with_clause"),
        ("with_item", "e0b-py-with_item"),
        ("with_statement", "e0b-py-with_statement"),
    ]
);
