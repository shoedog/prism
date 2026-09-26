use super::super::super::super::binding_table::{capture_rows, rows, Predicate, Ruling};
use super::{all_cases, check_case, Case, CaseClass};
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::languages::Language;

pub(super) fn assert_behavior(case: &Case, parsed: &ParsedFile) {
    let function = super::super::super::function(parsed);
    let defs = super::super::super::collect_defs(parsed);
    let mut edges = Vec::new();
    for (from, to, _) in case.expect {
        let (def_line, def_name) = from
            .split_once(':')
            .unwrap_or_else(|| panic!("{}: invalid definition expectation {from:?}", case.id));
        let (use_line, use_name) = to
            .split_once(':')
            .unwrap_or_else(|| panic!("{}: invalid use expectation {to:?}", case.id));
        assert_eq!(def_name, use_name, "{}: expectation path", case.id);
        let def_line: usize = def_line
            .parse()
            .unwrap_or_else(|_| panic!("{}: invalid definition line {def_line:?}", case.id));
        let use_line: usize = use_line
            .parse()
            .unwrap_or_else(|_| panic!("{}: invalid use line {use_line:?}", case.id));
        let def = defs
            .iter()
            .find(|def| def.line == def_line && def.path.to_string() == def_name)
            .unwrap_or_else(|| panic!("{}: definition {from:?} missing", case.id));
        edges.push(super::super::super::edge(parsed, &function, def, use_line));
    }
    let (outcome, _) = super::super::super::run(parsed, &defs, &edges);
    for (edge, (_, _, expected)) in edges.iter().zip(case.expect.iter()) {
        super::super::super::assert_label(&outcome, edge, *expected);
    }
    if let Some((field, expected)) = case.expect_counter {
        let actual = match &outcome {
            super::super::super::RdOutcome::Available(result) => result
                .labels
                .values()
                .filter(|label| match field {
                    "dfg_label_exact" => matches!(label, FlowConfidence::Exact),
                    "dfg_label_nameonly_killed" => {
                        matches!(label, FlowConfidence::NameOnly(FlowDoubt::Killed { .. }))
                    }
                    "dfg_label_nameonly_ownership_uncertain" => matches!(
                        label,
                        FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { .. })
                    ),
                    _ => panic!("{}: unsupported measured counter {field}", case.id),
                })
                .count() as i64,
            unavailable => panic!(
                "{}: expected measurable outcome, got {unavailable:?}",
                case.id
            ),
        };
        assert_eq!(actual, expected, "{}: measured {field}", case.id);
    }
}

pub(super) fn assert_ruling(
    case: &Case,
    ruling: Option<Ruling>,
    is_binding: bool,
    is_not_binding: bool,
) {
    assert!(
        case.expect_counter.is_some(),
        "{}: Behavioral case requires a measured counter",
        case.id
    );
    match ruling {
        Some(Ruling::Uncertain { .. }) => assert!(
            case.expect.iter().any(|(_, _, label)| matches!(
                label,
                FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { .. })
            )),
            "{}: uncertain behavior needs OwnershipUncertain",
            case.id
        ),
        Some(Ruling::Classified) if is_binding => {
            assert!(
                case.expect
                    .iter()
                    .any(|(_, _, label)| matches!(label, FlowConfidence::Exact)),
                "{}: classified binding behavior needs Exact",
                case.id
            );
            assert!(
                case.expect.iter().any(|(_, _, label)| matches!(
                    label,
                    FlowConfidence::NameOnly(FlowDoubt::Killed { .. })
                )),
                "{}: classified binding behavior needs Killed",
                case.id
            );
        }
        Some(Ruling::Classified) => assert!(
            case.expect
                .iter()
                .all(|(_, _, label)| matches!(label, FlowConfidence::Exact)),
            "{}: classified {} behavior needs Exact invariance",
            case.id,
            if is_not_binding {
                "NotBinding"
            } else {
                "scope"
            }
        ),
        None => {}
    }
}

pub(super) fn assert_row_case_identities() {
    let cases = all_cases();
    let mut expected: Vec<_> = cases
        .iter()
        .map(|case| {
            (
                case.language,
                case.variant,
                case.kind,
                case.row_variant,
                case.id,
            )
        })
        .collect();
    let mut actual = Vec::new();
    for language in Language::all() {
        for row in rows(language) {
            actual.push((
                language,
                Some("binding"),
                row.kind,
                row.variant,
                row.regression,
            ));
        }
        for row in capture_rows(language) {
            actual.push((
                language,
                Some("capture"),
                row.kind,
                row.variant,
                row.regression,
            ));
        }
    }
    expected.sort_by_key(|entry| format!("{entry:?}"));
    actual.sort_by_key(|entry| format!("{entry:?}"));
    assert_eq!(actual, expected, "case identities");
    for case in cases {
        check_case(case);
    }
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn check_case_rejects_an_impossible_behavioral_expectation() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: super::super::javascript::CLASSIFIED_MASK_SOURCE,
        expect: &[("2:x", "3:x", FlowConfidence::Exact)],
        expect_counter: None,
    });
}

#[test]
#[should_panic(expected = "measured dfg_label_nameonly_killed")]
fn structural_case_with_counter_measures_through_run() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: super::super::javascript::CLASSIFIED_MASK_SOURCE,
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 0)),
    });
}

#[test]
#[should_panic(
    expected = "e0a-x-for_in_statement-predicate: Behavioral case requires nonempty expect"
)]
fn empty_behavioral_case_is_rejected() {
    super::check_case(&Case {
        id: "e0a-x-for_in_statement-predicate",
        language: crate::languages::Language::JavaScript,
        kind: "for_in_statement",
        variant: Some("binding"),
        row_variant: Some(Predicate::FieldTextIs {
            field: "kind",
            any_of: &["let", "const"],
        }),
        class: CaseClass::Behavioral,
        src: "function f(items) { for (const item in items) { sink(item); } }",
        expect: &[],
        expect_counter: None,
    });
}

#[test]
#[should_panic(expected = "classified scope behavior needs Exact invariance")]
fn classified_scope_behavior_rejects_non_exact_invariance() {
    assert_ruling(
        &Case {
            id: "e0a-x-for_in_statement-residual",
            language: Language::JavaScript,
            kind: "for_in_statement",
            variant: Some("binding"),
            row_variant: None,
            class: CaseClass::Behavioral,
            src: "function f(items) { for (item in items) { sink(item); } }",
            expect: &[(
                "2:item",
                "3:item",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            )],
            expect_counter: Some(("dfg_label_nameonly_killed", 1)),
        },
        Some(Ruling::Classified),
        false,
        false,
    );
}
