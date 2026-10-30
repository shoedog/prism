//! Pure decision logic for blessing `expected.jsonl` and for the
//! `accepted_changes.jsonl` ledger. No I/O: the baseline (the committed
//! `expected.jsonl`), the working-tree rows and the ledger are passed in.

use super::js_alias_matrix_common::Row;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// One ledger entry: excuses exactly the transition `from` -> `to` for `id`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Accepted {
    pub id: String,
    pub from: String,
    pub to: String,
    #[allow(dead_code)]
    pub reason: String,
}

/// A verdict move is a regression when the new verdict is `WRONG`, `OVERCLAIM`
/// or `NONE+NAMEONLY`; or the old verdict was `CORRECT` or `NONE-OK`; or the new
/// verdict is `NO-RECORD` (and the old one was not). `old` is `None` for a
/// fixture absent from the baseline: a new fixture is never a regression.
pub fn is_regression(old: Option<&str>, new: &str) -> bool {
    let Some(old) = old else { return false };
    old != new
        && (matches!(new, "WRONG" | "OVERCLAIM" | "NONE+NAMEONLY" | "NO-RECORD")
            || matches!(old, "CORRECT" | "NONE-OK"))
}

fn verdicts(rows: &[Row]) -> BTreeMap<&str, &str> {
    rows.iter()
        .map(|r| (r.id.as_str(), r.verdict.as_str()))
        .collect()
}

/// Ledger entries that no longer describe the rows: an id that is not a
/// fixture, or a `to` that is not the verdict currently in `rows`.
pub fn ledger_problems(accepted: &[Accepted], fixture_ids: &[&str], rows: &[Row]) -> Vec<String> {
    let live: BTreeSet<&str> = fixture_ids.iter().copied().collect();
    let now = verdicts(rows);
    let mut out = Vec::new();
    for a in accepted {
        if !live.contains(a.id.as_str()) {
            out.push(format!(
                "accepted_changes.jsonl: `{}` is not a fixture id; delete the entry",
                a.id
            ));
        } else if now.get(a.id.as_str()) != Some(&a.to.as_str()) {
            out.push(format!(
                "accepted_changes.jsonl: stale entry for `{}`: accepts -> {} but the row is {}; \
                 delete the entry in the change that improved the row",
                a.id,
                a.to,
                now.get(a.id.as_str()).copied().unwrap_or("(no row)")
            ));
        }
    }
    out
}

/// Everything that stops a bless. `baseline` is the committed `expected.jsonl`
/// (the old state), `pinned` the working-tree one, `actual` the new rows.
/// Empty means the bless may write `actual`.
pub fn bless_problems(
    fixture_ids: &[&str],
    baseline: &[Row],
    pinned: &[Row],
    actual: &[Row],
    accepted: &[Accepted],
) -> Vec<String> {
    let live: BTreeSet<&str> = fixture_ids.iter().copied().collect();
    let mut out = Vec::new();
    for r in pinned.iter().filter(|r| !live.contains(r.id.as_str())) {
        out.push(format!(
            "pinned row `{}` names no fixture; delete that row from expected.jsonl by hand in \
             the same change (bless will not drop it silently)",
            r.id
        ));
    }
    out.extend(ledger_problems(accepted, fixture_ids, actual));
    let old = verdicts(baseline);
    for a in actual {
        let from = old.get(a.id.as_str()).copied();
        if !is_regression(from, &a.verdict) {
            continue;
        }
        let excused = accepted
            .iter()
            .any(|e| e.id == a.id && Some(e.from.as_str()) == from && e.to == a.verdict);
        if !excused {
            out.push(format!(
                "regression not accepted: `{}` {} -> {}; add {{\"id\":\"{}\",\"from\":\"{}\",\
                 \"to\":\"{}\",\"reason\":...}} to accepted_changes.jsonl",
                a.id,
                from.unwrap_or("?"),
                a.verdict,
                a.id,
                from.unwrap_or("?"),
                a.verdict
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, verdict: &str) -> Row {
        Row {
            id: id.into(),
            verdict: verdict.into(),
            record: true,
            exact: vec![],
            nameonly: vec![],
            drop: None,
            origins: vec![],
        }
    }
    fn rows(v: &[(&str, &str)]) -> Vec<Row> {
        v.iter().map(|(i, d)| row(i, d)).collect()
    }
    fn acc(id: &str, from: &str, to: &str) -> Accepted {
        Accepted {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            reason: "r".into(),
        }
    }
    /// Bless problems with working tree == baseline.
    fn bless(base: &[(&str, &str)], actual: &[(&str, &str)], a: &[Accepted]) -> Vec<String> {
        let ids: Vec<&str> = actual.iter().map(|(i, _)| *i).collect();
        let b = rows(base);
        bless_problems(&ids, &b, &b, &rows(actual), a)
    }

    #[test]
    fn regression_set() {
        for old in ["MISSING", "DEMOTED", "PARTIAL", "MAY-NONE"] {
            for new in ["WRONG", "OVERCLAIM", "NONE+NAMEONLY", "NO-RECORD"] {
                assert!(is_regression(Some(old), new), "{old}->{new}");
            }
        }
        for new in [
            "PARTIAL",
            "DEMOTED",
            "MISSING",
            "WRONG",
            "NO-RECORD",
            "NONE+NAMEONLY",
        ] {
            assert!(is_regression(Some("CORRECT"), new), "CORRECT->{new}");
        }
        for new in ["NONE+NAMEONLY", "WRONG", "NO-RECORD", "MISSING"] {
            assert!(is_regression(Some("NONE-OK"), new), "NONE-OK->{new}");
        }
    }

    #[test]
    fn improvements_lateral_moves_and_unchanged_rows_are_not_regressions() {
        assert!(!is_regression(Some("MISSING"), "CORRECT"));
        assert!(!is_regression(Some("WRONG"), "CORRECT"));
        assert!(!is_regression(Some("OVERCLAIM"), "MAY-NAMEONLY"));
        assert!(!is_regression(Some("NONE+NAMEONLY"), "NONE-OK"));
        assert!(!is_regression(Some("MISSING"), "DEMOTED"));
        assert!(!is_regression(Some("NO-RECORD"), "NO-RECORD"));
        for v in ["WRONG", "OVERCLAIM", "NONE+NAMEONLY", "CORRECT", "NONE-OK"] {
            assert!(!is_regression(Some(v), v), "{v} unchanged");
        }
    }

    #[test]
    fn a_new_fixture_is_never_a_regression() {
        for v in [
            "WRONG",
            "OVERCLAIM",
            "NONE+NAMEONLY",
            "NO-RECORD",
            "CORRECT",
        ] {
            assert!(!is_regression(None, v), "new fixture landing {v}");
        }
        assert!(bless(
            &[("a", "CORRECT")],
            &[("a", "CORRECT"), ("n", "WRONG")],
            &[]
        )
        .is_empty());
    }

    #[test]
    fn unaccepted_regressions_are_all_listed_and_improvements_pass() {
        let p = bless(
            &[
                ("a", "CORRECT"),
                ("b", "MISSING"),
                ("c", "MISSING"),
                ("d", "NONE-OK"),
            ],
            &[
                ("a", "MISSING"),
                ("b", "CORRECT"),
                ("c", "WRONG"),
                ("d", "NONE+NAMEONLY"),
            ],
            &[],
        );
        assert_eq!(p.len(), 3, "{p:?}");
        assert!(p[0].contains("`a`") && p[1].contains("`c`") && p[2].contains("`d`"));
    }

    #[test]
    fn deleted_working_tree_row_uses_the_baseline_verdict() {
        let base = rows(&[("a", "CORRECT")]);
        let p = bless_problems(&["a"], &base, &[], &rows(&[("a", "MISSING")]), &[]);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("CORRECT -> MISSING"));
    }

    #[test]
    fn an_accepted_transition_passes() {
        let a = [acc("a", "CORRECT", "MISSING")];
        assert!(bless(&[("a", "CORRECT")], &[("a", "MISSING")], &a).is_empty());
    }

    #[test]
    fn an_entry_with_the_wrong_from_or_to_does_not_excuse() {
        let base = [("a", "CORRECT")];
        let actual = [("a", "MISSING")];
        assert_eq!(
            bless(&base, &actual, &[acc("a", "PARTIAL", "MISSING")]).len(),
            1
        );
        // wrong `to` is also a stale entry (its `to` is not the new verdict)
        let p = bless(&base, &actual, &[acc("a", "CORRECT", "DEMOTED")]);
        assert!(
            p.iter().any(|m| m.contains("regression not accepted")),
            "{p:?}"
        );
        assert!(p.iter().any(|m| m.contains("stale entry")), "{p:?}");
    }

    #[test]
    fn an_entry_for_another_id_does_not_excuse() {
        let a = [acc("b", "CORRECT", "MISSING")];
        let ids = ["a", "b"];
        let base = rows(&[("a", "CORRECT"), ("b", "CORRECT")]);
        let p = bless_problems(
            &ids,
            &base,
            &base,
            &rows(&[("a", "MISSING"), ("b", "CORRECT")]),
            &a,
        );
        assert!(
            p.iter().any(|m| m.contains("`a` CORRECT -> MISSING")),
            "{p:?}"
        );
    }

    #[test]
    fn a_pinned_row_with_no_fixture_blocks_the_bless() {
        let base = rows(&[("a", "CORRECT"), ("gone", "CORRECT")]);
        let p = bless_problems(&["a"], &base, &base, &rows(&[("a", "CORRECT")]), &[]);
        assert_eq!(p.len(), 1, "{p:?}");
        assert!(p[0].contains("`gone`") && p[0].contains("by hand"));
    }

    #[test]
    fn ledger_flags_unknown_ids_and_stale_entries() {
        let pinned = rows(&[("a", "MISSING"), ("b", "CORRECT")]);
        let ids = ["a", "b"];
        assert!(ledger_problems(&[acc("a", "CORRECT", "MISSING")], &ids, &pinned).is_empty());
        let unknown = ledger_problems(&[acc("ghost", "CORRECT", "MISSING")], &ids, &pinned);
        assert!(unknown[0].contains("not a fixture id"), "{unknown:?}");
        // row improved since the entry was written: the entry is stale
        let stale = ledger_problems(&[acc("b", "CORRECT", "MISSING")], &ids, &pinned);
        assert!(stale[0].contains("stale entry"), "{stale:?}");
        // an id that is a fixture but has no pinned row is stale too
        let none = ledger_problems(&[acc("a", "CORRECT", "MISSING")], &ids, &pinned[1..]);
        assert!(none[0].contains("(no row)"), "{none:?}");
    }
}
