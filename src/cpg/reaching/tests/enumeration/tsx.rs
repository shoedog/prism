use super::super::super::binding_table::Predicate;
use crate::languages::Language;

pub(super) const CASES: &[super::case::Case] = &[
    super::case::Case {
        id: "e0a-x-for_in_statement-predicate",
        language: Language::Tsx,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: Some(Predicate::FieldTextIs {
            field: "kind",
            any_of: &["let", "const"],
        }),
        src: "function f(items: number[]) { for (const item of items) { sink(item); } }",
        expect: &[],
        expect_counter: None,
    },
    super::case::Case {
        id: "e0a-x-for_in_statement-residual",
        language: Language::Tsx,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: None,
        src: "function f(items: number[]) { for (item of items) { sink(item); } }",
        expect: &[],
        expect_counter: None,
    },
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "class" => "const C = class Named { method(): void {} };",
        "import" => "async function f() { return import('pkg'); }",
        _ => super::typescript::SOURCE,
    }
}
