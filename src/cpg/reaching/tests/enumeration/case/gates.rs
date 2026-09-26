use crate::cpg::reaching::binding_table::{capture_rows, rows, Ruling};
use crate::languages::Language;

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
