use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "b644629f7f9460cb0af080cb88f3bef898e6c47b6f8a76ed512b67ae16ef5ac5";
pub(super) const CENSUS_DIGEST: &str =
    "c828ffe90436967140f4af809cdd704d2ab908a66d59020ca6035d06a3151510";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Bash,
    "E4e-2",
    [
        ("c_style_for_statement", "e0b-bash-c_style_for_statement"),
        ("case_item", "e0b-bash-case_item"),
        ("case_statement", "e0b-bash-case_statement"),
        ("declaration_command", "e0b-bash-declaration_command"),
        ("for_statement", "e0b-bash-for_statement"),
        ("function_definition", "e0b-bash-function_definition"),
        ("heredoc_body", "e0b-bash-heredoc_body"),
        ("variable_assignment", "e0b-bash-variable_assignment"),
        ("variable_assignments", "e0b-bash-variable_assignments"),
        ("extglob_pattern", "e0b-bash-extglob_pattern"),
        ("special_variable_name", "e0b-bash-special_variable_name"),
        ("variable_name", "e0b-bash-variable_name"),
    ]
);
