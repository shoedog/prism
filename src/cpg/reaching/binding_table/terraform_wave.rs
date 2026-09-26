use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "d86638c95d20335b960abb62f6758ab53f78fd0efbe4b6669473b5a20dfd1fb5";
pub(super) const CENSUS_DIGEST: &str =
    "17768eaa9626d868165928c23dce529bc360e737525bee83b0327d604ea1b96e";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Terraform,
    "E4e-2",
    [
        ("block", "e0b-tf-block"),
        ("block_end", "e0b-tf-block_end"),
        ("block_start", "e0b-tf-block_start"),
        ("body", "e0b-tf-body"),
        ("for_cond", "e0b-tf-for_cond"),
        ("for_expr", "e0b-tf-for_expr"),
        ("for_intro", "e0b-tf-for_intro"),
        ("for_object_expr", "e0b-tf-for_object_expr"),
        ("for_tuple_expr", "e0b-tf-for_tuple_expr"),
        ("function_arguments", "e0b-tf-function_arguments"),
        ("function_call", "e0b-tf-function_call"),
        ("template_for_end", "e0b-tf-template_for_end"),
        ("template_for_start", "e0b-tf-template_for_start"),
        ("variable_expr", "e0b-tf-variable_expr"),
    ]
);
