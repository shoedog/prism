pub(super) const CASES: &[super::case::Case] = &[];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "class" => "const C = class Named { method(): void {} };",
        "import" => "async function f() { return import('pkg'); }",
        _ => super::typescript::SOURCE,
    }
}
