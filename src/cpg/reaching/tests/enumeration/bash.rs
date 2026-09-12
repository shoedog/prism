pub(super) const CASES: &[super::case::Case] = &[];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "variable_assignments" => "FIRST=one SECOND=two\n",
        _ => super::case::source(crate::languages::Language::Bash),
    }
}
