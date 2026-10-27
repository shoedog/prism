use super::{BindingRow, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) const DIGEST: &str = "d86638c95d20335b960abb62f6758ab53f78fd0efbe4b6669473b5a20dfd1fb5";
pub(super) const CENSUS_DIGEST: &str =
    "a4f20f5792875813aa883a0c6e6d7872a4da6c7db892b2ae83a04b313451a083";

macro_rules! census_only_row {
    ($kind:literal, $regression:literal) => {
        BindingRow {
            language: Language::Terraform,
            kind: $kind,
            variant: None,
            roles: RoleSet::of(&[Role::Binding]),
            fields: &[],
            declaration: None,
            visibility: Visibility::WholeScope,
            ruling: Ruling::Uncertain {
                reason: "census-only language",
                revisit: "future:scripting",
            },
            regression: $regression,
        }
    };
}

pub(super) static ROWS: &[BindingRow] = &[
    census_only_row!("block", "e0b-tf-block"),
    census_only_row!("block_end", "e0b-tf-block_end"),
    census_only_row!("block_start", "e0b-tf-block_start"),
    census_only_row!("body", "e0b-tf-body"),
    census_only_row!("for_cond", "e0b-tf-for_cond"),
    census_only_row!("for_expr", "e0b-tf-for_expr"),
    census_only_row!("for_intro", "e0b-tf-for_intro"),
    census_only_row!("for_object_expr", "e0b-tf-for_object_expr"),
    census_only_row!("for_tuple_expr", "e0b-tf-for_tuple_expr"),
    census_only_row!("function_arguments", "e0b-tf-function_arguments"),
    census_only_row!("function_call", "e0b-tf-function_call"),
    census_only_row!("template_for_end", "e0b-tf-template_for_end"),
    census_only_row!("template_for_start", "e0b-tf-template_for_start"),
    BindingRow {
        language: Language::Terraform,
        kind: "variable_expr",
        variant: None,
        roles: RoleSet::of(&[Role::NotBinding]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Uncertain {
            reason: "census-only language",
            revisit: "future:scripting",
        },
        regression: "hcl-variable-expr-not-binding",
    },
];
