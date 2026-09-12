pub(super) const CASES: &[super::case::Case] = &[];
pub(super) const CURATED: &[(&str, &str)] = &[
    ("formal_parameters", "e0a-x-java-formal_parameters"),
    ("spread_parameter", "e0a-x-java-spread_parameter"),
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "constant_declaration" => "interface C { int VALUE = 1; }",
        "type_pattern" => "class C { void f(Object value) { switch (value) { case String text -> {} default -> {} } } }",
        "underscore_pattern" => "class C { void f(Object value) { switch (value) { case Pair(_, var right) -> {} default -> {} } } } record Pair(Object left, Object right) {}",
        _ => super::case::source(crate::languages::Language::Java),
    }
}
