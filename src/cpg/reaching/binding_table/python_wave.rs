use super::{
    provisional_rows, BindingRow, DeclarationKind, Predicate, Role, RoleSet, Ruling, Visibility,
};
use crate::languages::Language;

pub(super) const DIGEST: &str = "7ff6914d278faf4254d923e1205b887f940cfb3db46a81eddbde01e782707ac3";
pub(super) const CENSUS_DIGEST: &str =
    "d6f9cf15bb6191845d42c1ae36a448403b246408553120f4942884dea453f6c5";

const PATTERN_UNCERTAIN: Ruling = Ruling::Uncertain {
    reason: "pattern identifiers not field-addressable",
    revisit: "future:python-patterns",
};

pub(super) static CURATED_ROWS: &[BindingRow] = &[
    BindingRow {
        language: Language::Python,
        kind: "match_statement",
        variant: None,
        roles: RoleSet::of(&[Role::Scope]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "py-match-statement-scope",
    },
    BindingRow {
        language: Language::Python,
        kind: "case_clause",
        variant: None,
        roles: RoleSet::of(&[Role::Scope]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "py-case-clause-scope",
    },
    pattern_row("case_pattern", None, &[], "py-case-pattern-uncertain"),
    pattern_row("class_pattern", None, &[], "py-class-pattern-uncertain"),
    pattern_row(
        "as_pattern",
        Some(Predicate::ParentKindIs {
            kind: "case_pattern",
        }),
        &["alias"],
        "py-case-as-pattern-inside-arm",
    ),
    BindingRow {
        language: Language::Python,
        kind: "as_pattern",
        variant: None,
        roles: RoleSet::of(&[Role::NotBinding]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression: "py-as-pattern-non-case-transparent",
    },
    pattern_row("keyword_pattern", None, &[], "py-keyword-pattern-uncertain"),
    transparent_row("pattern_list", "py-pattern-list-non-case-transparent"),
    pattern_row(
        "tuple_pattern",
        Some(Predicate::AncestorKindIs {
            kind: "case_clause",
        }),
        &[],
        "py-tuple-pattern-uncertain",
    ),
    transparent_row("tuple_pattern", "py-tuple-pattern-non-case-transparent"),
    pattern_row(
        "list_pattern",
        Some(Predicate::AncestorKindIs {
            kind: "case_clause",
        }),
        &[],
        "py-list-pattern-uncertain",
    ),
    transparent_row("list_pattern", "py-list-pattern-non-case-transparent"),
];

const fn pattern_row(
    kind: &'static str,
    variant: Option<Predicate>,
    fields: &'static [&'static str],
    regression: &'static str,
) -> BindingRow {
    BindingRow {
        language: Language::Python,
        kind,
        variant,
        roles: RoleSet::of(&[Role::Binding]),
        fields,
        declaration: Some(DeclarationKind::Pattern),
        visibility: Visibility::Header,
        ruling: PATTERN_UNCERTAIN,
        regression,
    }
}

const fn transparent_row(kind: &'static str, regression: &'static str) -> BindingRow {
    BindingRow {
        language: Language::Python,
        kind,
        variant: None,
        roles: RoleSet::of(&[Role::NotBinding]),
        fields: &[],
        declaration: None,
        visibility: Visibility::WholeScope,
        ruling: Ruling::Classified,
        regression,
    }
}

pub(super) static ROWS: &[BindingRow] = provisional_rows!(
    Language::Python,
    "E1a-later",
    [
        ("block", "e0b-py-block"),
        ("complex_pattern", "e0b-py-complex_pattern"),
        ("default_parameter", "e0b-py-default_parameter"),
        ("dict_pattern", "e0b-py-dict_pattern"),
        (
            "dictionary_splat_pattern",
            "e0b-py-dictionary_splat_pattern"
        ),
        ("for_in_clause", "e0b-py-for_in_clause"),
        ("future_import_statement", "e0b-py-future_import_statement"),
        ("import_prefix", "e0b-py-import_prefix"),
        ("lambda_parameters", "e0b-py-lambda_parameters"),
        ("list_splat_pattern", "e0b-py-list_splat_pattern"),
        ("relative_import", "e0b-py-relative_import"),
        ("splat_pattern", "e0b-py-splat_pattern"),
        ("type_parameter", "e0b-py-type_parameter"),
        ("typed_default_parameter", "e0b-py-typed_default_parameter"),
        ("typed_parameter", "e0b-py-typed_parameter"),
        ("union_pattern", "e0b-py-union_pattern"),
        ("wildcard_import", "e0b-py-wildcard_import"),
        ("with_clause", "e0b-py-with_clause"),
    ]
);
