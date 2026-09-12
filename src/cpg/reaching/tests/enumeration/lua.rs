pub(super) const CASES: &[super::case::Case] = &[];
pub(super) const CURATED: &[(&str, &str)] = &[("parameters", "e0a-x-lua-parameters")];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "implicit_variable_declaration" => "global <const> *\nfunction f() end\n",
        _ => super::case::source(crate::languages::Language::Lua),
    }
}
