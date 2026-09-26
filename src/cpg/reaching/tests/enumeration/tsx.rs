use super::super::super::binding_table::Predicate;
use crate::cpg::{FlowConfidence, FlowDoubt};
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
        class: super::case::CaseClass::Behavioral,
        src: "function f(items: number[]) {\n  let item = source();\n  for (const item in items) { sink(item); }\n  sink(item);\n}",
        expect: &[
            (
                "2:item",
                "3:item",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:item", "4:item", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    },
    super::case::Case {
        id: "e0a-x-for_in_statement-residual",
        language: Language::Tsx,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: None,
        class: super::case::CaseClass::Behavioral,
        src: "function f(items: number[]) {\n  let outer = source();\n  for (item in items) { sink(outer); }\n  sink(outer);\n}",
        expect: &[
            ("2:outer", "3:outer", FlowConfidence::Exact),
            ("2:outer", "4:outer", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_exact", 2)),
    },
];

pub(super) fn source_for_kind(kind: &str) -> &'static str {
    match kind {
        "class" => "const C = class Named { method(): void {} };",
        "import" => "async function f() { return import('pkg'); }",
        _ => super::typescript::SOURCE,
    }
}
