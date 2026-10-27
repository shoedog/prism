use super::{BindingRow, Role, RoleSet, Ruling, Visibility};
use crate::languages::Language;

pub(super) const DIGEST: &str = "b644629f7f9460cb0af080cb88f3bef898e6c47b6f8a76ed512b67ae16ef5ac5";
pub(super) const CENSUS_DIGEST: &str =
    "aea5b0dc547dce88a30f44bd60151e6f3e8bba586746662ed9d72fdf57aec269";

macro_rules! census_only_row {
    ($kind:literal, $fields:expr, $regression:literal) => {
        BindingRow {
            language: Language::Bash,
            kind: $kind,
            variant: None,
            roles: RoleSet::of(&[Role::Binding]),
            fields: $fields,
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
    census_only_row!(
        "c_style_for_statement",
        &[],
        "e0b-bash-c_style_for_statement"
    ),
    census_only_row!("case_item", &[], "e0b-bash-case_item"),
    census_only_row!("case_statement", &[], "e0b-bash-case_statement"),
    census_only_row!("declaration_command", &[], "e0b-bash-declaration_command"),
    census_only_row!("for_statement", &["variable"], "e0b-bash-for_statement"),
    census_only_row!("function_definition", &[], "e0b-bash-function_definition"),
    census_only_row!("heredoc_body", &[], "e0b-bash-heredoc_body"),
    census_only_row!(
        "variable_assignment",
        &["name"],
        "e0b-bash-variable_assignment"
    ),
    census_only_row!("variable_assignments", &[], "e0b-bash-variable_assignments"),
    census_only_row!("extglob_pattern", &[], "e0b-bash-extglob_pattern"),
    census_only_row!(
        "special_variable_name",
        &[],
        "e0b-bash-special_variable_name"
    ),
    census_only_row!("variable_name", &[], "e0b-bash-variable_name"),
];
