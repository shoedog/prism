use super::super::super::super::binding_table::{capture_rows, rows, Predicate, Ruling};
use super::{all_cases, check_case, Case, CaseClass};
use crate::ast::ParsedFile;
use crate::cpg::reaching::RdFileStats;
use crate::cpg::{DfgLabelStats, FlowConfidence, FlowDoubt};
use crate::languages::Language;

#[derive(Clone, Copy)]
enum CounterField {
    LabelExact,
    LabelLoopCarried,
    LabelKilled,
    LabelOwnershipUncertain,
    LabelSameLine,
    LabelCfgIncomplete,
    LabelAliasUnstable,
    LabelCall,
    LabelCaptureImmediate,
    RdFunctionsOverCap,
    RdFunctionsWithoutCfg,
}

fn select_counter(case_id: &str, field: &str) -> CounterField {
    match field {
        "dfg_label_exact" => CounterField::LabelExact,
        "dfg_label_loop_carried" => CounterField::LabelLoopCarried,
        "dfg_label_nameonly_killed" => CounterField::LabelKilled,
        "dfg_label_nameonly_ownership_uncertain" => CounterField::LabelOwnershipUncertain,
        "dfg_label_nameonly_sameline" => CounterField::LabelSameLine,
        "dfg_label_nameonly_cfg_incomplete" => CounterField::LabelCfgIncomplete,
        "dfg_label_nameonly_alias_unstable" => CounterField::LabelAliasUnstable,
        "dfg_label_nameonly_call" => CounterField::LabelCall,
        "dfg_label_capture_immediate" => CounterField::LabelCaptureImmediate,
        "dfg_rd_functions_over_cap" => CounterField::RdFunctionsOverCap,
        "dfg_rd_functions_without_cfg" => CounterField::RdFunctionsWithoutCfg,
        _ => panic!("{case_id}: unsupported measured counter {field}"),
    }
}

fn counter_value(
    counter: CounterField,
    label_stats: &DfgLabelStats,
    rd_stats: &RdFileStats,
) -> i64 {
    (match counter {
        CounterField::LabelExact => label_stats.dfg_label_exact,
        CounterField::LabelLoopCarried => label_stats.dfg_label_loop_carried,
        CounterField::LabelKilled => label_stats.dfg_label_nameonly_killed,
        CounterField::LabelOwnershipUncertain => label_stats.dfg_label_nameonly_ownership_uncertain,
        CounterField::LabelSameLine => label_stats.dfg_label_nameonly_sameline,
        CounterField::LabelCfgIncomplete => label_stats.dfg_label_nameonly_cfg_incomplete,
        CounterField::LabelAliasUnstable => label_stats.dfg_label_nameonly_alias_unstable,
        CounterField::LabelCall => label_stats.dfg_label_nameonly_call,
        CounterField::LabelCaptureImmediate => label_stats.dfg_label_capture_immediate,
        CounterField::RdFunctionsOverCap => rd_stats.functions_over_cap,
        CounterField::RdFunctionsWithoutCfg => rd_stats.functions_without_cfg,
    }) as i64
}

pub(super) fn assert_behavior(case: &Case, parsed: &ParsedFile) {
    let counter = case
        .expect_counter
        .map(|(field, expected)| (select_counter(case.id, field), field, expected));
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
    let (outcome, rd_stats) = super::super::super::run(parsed, &defs, &edges);
    for (edge, (_, _, expected)) in edges.iter().zip(case.expect.iter()) {
        super::super::super::assert_label(&outcome, edge, *expected);
    }
    if let Some((counter, field, expected)) = counter {
        let super::super::super::RdOutcome::Available(result) = &outcome else {
            panic!("{}: expected measurable outcome, got {outcome:?}", case.id);
        };
        let mut label_stats = DfgLabelStats::default();
        for label in result.labels.values().copied() {
            label_stats.record_label(label);
        }
        label_stats.dfg_label_loop_carried = result.loop_carried_edges.len();
        let actual = counter_value(counter, &label_stats, &rd_stats);
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
fn structural_empty_expectations_measure_supported_zero_rd_counter() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: super::super::javascript::CLASSIFIED_MASK_SOURCE,
        expect: &[],
        expect_counter: Some(("dfg_rd_functions_over_cap", 0)),
    });
}

#[test]
fn supported_nonzero_label_counter_is_measured() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Behavioral,
        src: super::super::javascript::CLASSIFIED_MASK_SOURCE,
        expect: &[
            (
                "2:x",
                "3:x",
                FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 }),
            ),
            ("2:x", "4:x", FlowConfidence::Exact),
        ],
        expect_counter: Some(("dfg_label_nameonly_killed", 1)),
    });
}

#[test]
#[should_panic(expected = "unsupported measured counter unsupported")]
fn structural_empty_expectations_reject_unknown_counter() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: super::super::javascript::CLASSIFIED_MASK_SOURCE,
        expect: &[],
        expect_counter: Some(("unsupported", 0)),
    });
}

#[test]
#[should_panic(expected = "expected measurable outcome")]
fn unavailable_runtime_outcome_fails_closed() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "function f() { let x = source(); }",
        expect: &[],
        expect_counter: Some(("dfg_rd_functions_without_cfg", 0)),
    });
}

#[test]
#[should_panic(expected = "fixture must contain a function")]
fn structural_counter_without_function_fails_closed() {
    super::check_case(&Case {
        id: "e0a-js-lexical_declaration",
        language: crate::languages::Language::JavaScript,
        kind: "lexical_declaration",
        variant: Some("binding"),
        row_variant: None,
        class: CaseClass::Structural,
        src: "let x = source();",
        expect: &[],
        expect_counter: Some(("dfg_rd_functions_over_cap", 0)),
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
