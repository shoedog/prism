use super::{all_cases, CaseClass};
use crate::cpg::reaching::binding_table::{capture_rows, rows, Predicate, Ruling};
use crate::languages::Language;
use std::collections::BTreeSet;

type RowIdentity = (Language, &'static str, Option<Predicate>, &'static str);

include!("placeholder_owners.inc.rs");

const PLACEHOLDER_OWNERS: &[(RowIdentity, &str)] = placeholder_owners!(Language, Predicate);

fn classified_placeholders() -> BTreeSet<RowIdentity> {
    let cases = all_cases();
    Language::all()
        .iter()
        .flat_map(|&language| {
            let cases = &cases;
            rows(language)
                .iter()
                .filter(move |row| {
                    matches!(row.ruling, Ruling::Classified)
                        && !cases.iter().any(|case| {
                            case.language == language
                                && case.id == row.regression
                                && case.kind == row.kind
                                && case.row_variant == row.variant
                                && case.class == CaseClass::Behavioral
                        })
                })
                .map(move |row| (language, row.kind, row.variant, row.regression))
        })
        .collect()
}

fn assert_placeholder_registry_exact(
    placeholders: &BTreeSet<RowIdentity>,
    registry_entries: &[(RowIdentity, &str)],
) {
    let registry_keys: Vec<_> = registry_entries.iter().map(|(key, _)| *key).collect();
    let registry: BTreeSet<_> = registry_keys.iter().copied().collect();
    assert_eq!(
        registry.len(),
        registry_keys.len(),
        "PLACEHOLDER_OWNERS has a duplicate row-identity key"
    );
    assert_eq!(
        placeholders, &registry,
        "classified rows lacking a Behavioral case must equal PLACEHOLDER_OWNERS exactly"
    );
    for (_, owner) in registry_entries {
        let number: u32 = owner
            .strip_prefix("Task ")
            .and_then(|suffix| suffix.parse().ok())
            .unwrap_or_else(|| panic!("{owner}: not \"Task <n>\""));
        assert!(
            (7..=27).contains(&number),
            "{owner}: PLACEHOLDER_OWNERS owner must be Task 7 through Task 27"
        );
    }
}

#[test]
fn classified_placeholders_are_enumerated() {
    let placeholders = classified_placeholders();
    assert_placeholder_registry_exact(&placeholders, PLACEHOLDER_OWNERS);
    assert_eq!(
        placeholders.len(),
        62,
        "placeholder debt count changed; update only as Behavioral cases replace debt"
    );
}

#[test]
#[should_panic(expected = "must equal PLACEHOLDER_OWNERS exactly")]
fn placeholder_registry_rejects_a_missing_owner_entry() {
    let key = (Language::Bash, "fixture", None, "fixture-regression");
    assert_placeholder_registry_exact(&BTreeSet::from([key]), &[]);
}

#[test]
#[should_panic(expected = "must equal PLACEHOLDER_OWNERS exactly")]
fn placeholder_registry_rejects_a_stale_owner_entry() {
    let key = (Language::Bash, "fixture", None, "fixture-regression");
    assert_placeholder_registry_exact(&BTreeSet::new(), &[(key, "Task 7")]);
}

#[test]
fn uncertain_rows_carry_reasons() {
    for language in Language::all() {
        for row in rows(language) {
            if let Ruling::Uncertain { reason, revisit } = row.ruling {
                assert!(!reason.is_empty(), "{language:?} {} reason", row.kind);
                assert!(!revisit.is_empty(), "{language:?} {} revisit", row.kind);
            }
        }
    }
}

#[test]
fn capture_table_covers_every_boundary_kind() {
    for language in Language::all() {
        let actual: Vec<_> = capture_rows(language).iter().map(|row| row.kind).collect();
        assert_eq!(
            actual,
            language.callable_boundary_node_types(),
            "{language:?}"
        );
    }
}

#[test]
fn no_provisional_row_whose_revisit_task_closed() {
    let closed: &[&str] = &[];
    for language in Language::all() {
        for row in rows(language) {
            if let Ruling::Uncertain {
                reason: "not yet curated",
                revisit,
            } = row.ruling
            {
                assert!(
                    !closed.contains(&revisit),
                    "{language:?} {} still provisional after {revisit}",
                    row.kind
                );
            }
        }
    }
}
