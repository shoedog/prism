use super::{provisional_rows, BindingRow};
use crate::languages::Language;

pub(super) const DIGEST: &str = "12ea1259c9e644754a13068eafd6160a9c5561c7a69e9cb9c8cd6c968b5e09fc";
pub(super) const CENSUS_DIGEST: &str =
    "3975070aa620bb4fd44dbe753873b431ec9966d6b299ecaba185b364be0ac831";

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Lua,
    "E4e-1",
    [
        ("assignment_statement", "e0b-lua-assignment_statement"),
        ("block", "e0b-lua-block"),
        ("for_generic_clause", "e0b-lua-for_generic_clause"),
        ("for_numeric_clause", "e0b-lua-for_numeric_clause"),
        ("for_statement", "e0b-lua-for_statement"),
        ("function_call", "e0b-lua-function_call"),
        ("function_declaration", "e0b-lua-function_declaration"),
        ("function_definition", "e0b-lua-function_definition"),
        (
            "implicit_variable_declaration",
            "e0b-lua-implicit_variable_declaration"
        ),
        ("variable_declaration", "e0b-lua-variable_declaration"),
        ("variable_list", "e0b-lua-variable_list"),
    ]
);
