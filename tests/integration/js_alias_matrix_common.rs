//! Pure model for the JS/TS alias variant matrix: row and fixture types, the
//! verdict function, and the comparison and bless-guard logic.
//!
//! `verdict` and `hit` are a port of `verdict()` and `matcher()` in
//! `tests/fixtures/js_alias_matrix/generator/run.cjs`; keep them in step.
//! Nothing here touches the filesystem or prism, so the guard is unit-tested
//! on synthetic rows.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub file: String,
    pub name: String,
    pub line: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Truth {
    One { targets: Vec<Target> },
    Set { targets: Vec<Target> },
    May { targets: Vec<Target> },
    None,
    Anon { file: String, line: Option<u64> },
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Fixture {
    pub id: String,
    pub step: String,
    pub shape: String,
    pub scope: String,
    pub form: String,
    pub naming: String,
    pub lang: String,
    pub contract: Option<String>,
    pub site_file: String,
    pub site_line: Option<u64>,
    pub truth: Truth,
    pub files: BTreeMap<String, String>,
}

/// A resolved callee as reported by `call-stats --dump-sites`.
/// Field order is the serialized order (it fixes the sort key and the file bytes).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hit {
    pub file: String,
    pub name: String,
    pub line: u64,
    pub kind: String,
}

/// One `expected.jsonl` row. Field order is the serialized order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    pub id: String,
    pub verdict: String,
    pub record: bool,
    pub exact: Vec<Hit>,
    pub nameonly: Vec<Hit>,
    pub drop: Option<String>,
    pub origins: Vec<String>,
}

/// What the call-site records at the marked line say, before judging.
#[derive(Debug, Clone, Default)]
pub struct Observed {
    pub record: bool,
    pub exact: Vec<Hit>,
    pub nameonly: Vec<Hit>,
    pub drop: Option<String>,
    pub origins: Vec<String>,
}

fn sort_hits(hits: &mut [Hit]) {
    hits.sort_by_cached_key(|h| serde_json::to_string(h).expect("serialize hit"));
}

/// Fold the `call_site` records at the marked site into one observation.
/// Mirrors `prismSite` in run.cjs plus the sorting done by `export.cjs`.
pub fn observe(records: &[serde_json::Value]) -> Observed {
    if records.is_empty() {
        return Observed::default();
    }
    let mut exact = Vec::new();
    let mut nameonly = Vec::new();
    let mut drops: Vec<String> = Vec::new();
    let mut origins = BTreeSet::new();
    for rec in records {
        origins.insert(match rec["origin"].as_str() {
            Some(s) => s.to_string(),
            None => rec["origin"].to_string(),
        });
        if let Some(d) = rec["drop"].as_str() {
            if !d.is_empty() && !drops.iter().any(|x| x == d) {
                drops.push(d.to_string());
            }
        }
        for t in rec["resolved_targets"].as_array().into_iter().flatten() {
            let id = &t["function_id"];
            let hit = Hit {
                file: id["file"].as_str().unwrap_or_default().to_string(),
                name: id["name"].as_str().unwrap_or_default().to_string(),
                line: id["start_line"].as_u64().unwrap_or_default(),
                kind: t["kind"].as_str().unwrap_or_default().to_string(),
            };
            if t["confidence"] == "exact" {
                exact.push(hit);
            } else {
                nameonly.push(hit);
            }
        }
    }
    sort_hits(&mut exact);
    sort_hits(&mut nameonly);
    Observed {
        record: true,
        exact,
        nameonly,
        drop: if drops.is_empty() {
            None
        } else {
            Some(drops.join("+"))
        },
        origins: origins.into_iter().collect(),
    }
}

fn hit(t: &Hit, target: &Target) -> bool {
    t.file == target.file
        && (t.name == target.name || (target.line.is_some() && Some(t.line) == target.line))
}

/// Verdict vocabulary: see `tests/fixtures/js_alias_matrix/README.md`.
pub fn verdict(truth: &Truth, o: &Observed) -> &'static str {
    if !o.record {
        return "NO-RECORD";
    }
    let (ex, no) = (&o.exact, &o.nameonly);
    match truth {
        Truth::Anon { file, line } => {
            let on_line = |t: &Hit| &t.file == file && Some(t.line) == *line;
            if !ex.is_empty() && ex.iter().all(on_line) {
                "CORRECT"
            } else if !ex.is_empty() {
                "WRONG"
            } else if no.iter().any(on_line) {
                "DEMOTED"
            } else {
                "MISSING"
            }
        }
        Truth::None => {
            if !ex.is_empty() {
                "WRONG"
            } else if !no.is_empty() {
                "NONE+NAMEONLY"
            } else {
                "NONE-OK"
            }
        }
        Truth::May { targets } => {
            let in_truth = |t: &Hit| targets.iter().any(|g| hit(t, g));
            if ex.iter().any(|t| !in_truth(t)) {
                "WRONG"
            } else if !ex.is_empty() {
                "OVERCLAIM"
            } else if no.iter().any(in_truth) {
                "MAY-NAMEONLY"
            } else {
                "MAY-NONE"
            }
        }
        Truth::One { targets } | Truth::Set { targets } => {
            let in_truth = |t: &Hit| targets.iter().any(|g| hit(t, g));
            if ex.iter().any(|t| !in_truth(t)) {
                "WRONG"
            } else if !ex.is_empty() {
                let covered = targets
                    .iter()
                    .filter(|g| ex.iter().any(|t| hit(t, g)))
                    .count();
                if covered == targets.len() {
                    "CORRECT"
                } else {
                    "PARTIAL"
                }
            } else if no.iter().any(in_truth) {
                "DEMOTED"
            } else {
                "MISSING"
            }
        }
    }
}

pub fn row_for(fixture: &Fixture, o: Observed) -> Row {
    Row {
        id: fixture.id.clone(),
        verdict: verdict(&fixture.truth, &o).to_string(),
        record: o.record,
        exact: o.exact,
        nameonly: o.nameonly,
        drop: o.drop,
        origins: o.origins,
    }
}

/// A row whose actual value differs from the pinned one.
#[derive(Debug, Clone)]
pub struct Change {
    pub id: String,
    pub expected: Option<Row>,
    pub actual: Row,
}

/// Every row (in fixture order) whose actual value differs from the pinned row,
/// including fixtures that have no pinned row. Never stops at the first.
pub fn changes(expected: &[Row], actual: &[Row]) -> Vec<Change> {
    let pinned: BTreeMap<&str, &Row> = expected.iter().map(|r| (r.id.as_str(), r)).collect();
    actual
        .iter()
        .filter(|a| pinned.get(a.id.as_str()).copied() != Some(*a))
        .map(|a| Change {
            id: a.id.clone(),
            expected: pinned.get(a.id.as_str()).map(|r| (*r).clone()),
            actual: a.clone(),
        })
        .collect()
}

/// Pinned rows with no fixture (the fixture list is the source of truth).
pub fn orphan_expected(expected: &[Row], actual: &[Row]) -> Vec<String> {
    let live: BTreeSet<&str> = actual.iter().map(|r| r.id.as_str()).collect();
    expected
        .iter()
        .filter(|r| !live.contains(r.id.as_str()))
        .map(|r| r.id.clone())
        .collect()
}

/// Moving INTO WRONG or OVERCLAIM, or OUT OF CORRECT, is a regression.
/// A fixture with no pinned row has nothing to move from; it counts as a
/// regression only if it lands in WRONG or OVERCLAIM.
pub fn is_regression(old: Option<&str>, new: &str) -> bool {
    match old {
        Some(old) if old == new => false,
        Some(old) => matches!(new, "WRONG" | "OVERCLAIM") || old == "CORRECT",
        None => matches!(new, "WRONG" | "OVERCLAIM"),
    }
}

/// The bless guard: ids of changed rows that regress and are not listed as accepted.
pub fn unaccepted_regressions(changes: &[Change], accepted: &BTreeSet<String>) -> Vec<String> {
    changes
        .iter()
        .filter(|c| {
            is_regression(
                c.expected.as_ref().map(|r| r.verdict.as_str()),
                &c.actual.verdict,
            ) && !accepted.contains(&c.id)
        })
        .map(|c| c.id.clone())
        .collect()
}

/// Accepted ids that name no fixture.
pub fn unknown_accepted(accepted: &BTreeSet<String>, fixtures: &[Fixture]) -> Vec<String> {
    let live: BTreeSet<&str> = fixtures.iter().map(|f| f.id.as_str()).collect();
    accepted
        .iter()
        .filter(|id| !live.contains(id.as_str()))
        .cloned()
        .collect()
}

/// Counts by `(step, expected verdict -> actual verdict)`, rendered one per line.
pub fn summary(changes: &[Change], step_of: &BTreeMap<&str, &str>) -> String {
    let mut counts: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for c in changes {
        let step = step_of.get(c.id.as_str()).copied().unwrap_or("?");
        let was = c
            .expected
            .as_ref()
            .map_or("(no pinned row)", |r| r.verdict.as_str());
        *counts
            .entry((step.to_string(), was.to_string(), c.actual.verdict.clone()))
            .or_default() += 1;
    }
    counts
        .iter()
        .map(|((step, was, now), n)| format!("  {step}: {was} -> {now}: {n}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render rows exactly as the pinned file stores them (compact JSON, one per line).
pub fn render(rows: &[Row]) -> String {
    let mut out = String::new();
    for r in rows {
        out.push_str(&serde_json::to_string(r).expect("serialize row"));
        out.push('\n');
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
            origins: vec!["Source".into()],
        }
    }
    fn change(id: &str, old: Option<&str>, new: &str) -> Change {
        Change {
            id: id.into(),
            expected: old.map(|v| row(id, v)),
            actual: row(id, new),
        }
    }
    fn set(ids: &[&str]) -> BTreeSet<String> {
        ids.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn regression_into_wrong_or_overclaim() {
        for old in ["MISSING", "DEMOTED", "NONE-OK", "MAY-NONE", "PARTIAL"] {
            assert!(is_regression(Some(old), "WRONG"), "{old}->WRONG");
            assert!(is_regression(Some(old), "OVERCLAIM"), "{old}->OVERCLAIM");
        }
    }

    #[test]
    fn regression_out_of_correct() {
        for new in [
            "PARTIAL",
            "DEMOTED",
            "MISSING",
            "WRONG",
            "NO-RECORD",
            "NONE-OK",
        ] {
            assert!(is_regression(Some("CORRECT"), new), "CORRECT->{new}");
        }
    }

    #[test]
    fn improvements_and_lateral_moves_are_not_regressions() {
        assert!(!is_regression(Some("MISSING"), "CORRECT"));
        assert!(!is_regression(Some("WRONG"), "CORRECT"));
        assert!(!is_regression(Some("OVERCLAIM"), "MAY-NAMEONLY"));
        assert!(!is_regression(Some("MISSING"), "DEMOTED"));
        assert!(!is_regression(Some("DEMOTED"), "MISSING"));
        assert!(!is_regression(Some("PARTIAL"), "CORRECT"));
    }

    #[test]
    fn unchanged_verdict_is_not_a_regression_even_when_wrong() {
        assert!(!is_regression(Some("WRONG"), "WRONG"));
        assert!(!is_regression(Some("OVERCLAIM"), "OVERCLAIM"));
        assert!(!is_regression(Some("CORRECT"), "CORRECT"));
    }

    #[test]
    fn new_fixture_regresses_only_when_it_lands_wrong_or_overclaim() {
        assert!(is_regression(None, "WRONG"));
        assert!(is_regression(None, "OVERCLAIM"));
        assert!(!is_regression(None, "CORRECT"));
        assert!(!is_regression(None, "MISSING"));
    }

    #[test]
    fn guard_refuses_unaccepted_regressions_and_lists_all() {
        let cs = vec![
            change("a", Some("CORRECT"), "MISSING"),
            change("b", Some("MISSING"), "CORRECT"),
            change("c", Some("MISSING"), "WRONG"),
        ];
        assert_eq!(unaccepted_regressions(&cs, &set(&[])), vec!["a", "c"]);
    }

    #[test]
    fn guard_allows_regressions_listed_as_accepted() {
        let cs = vec![
            change("a", Some("CORRECT"), "MISSING"),
            change("c", Some("MISSING"), "WRONG"),
        ];
        assert_eq!(unaccepted_regressions(&cs, &set(&["a"])), vec!["c"]);
        assert!(unaccepted_regressions(&cs, &set(&["a", "c"])).is_empty());
    }

    #[test]
    fn accepting_an_id_does_not_excuse_other_rows() {
        let cs = vec![change("x", Some("CORRECT"), "WRONG")];
        assert_eq!(unaccepted_regressions(&cs, &set(&["y"])), vec!["x"]);
    }

    #[test]
    fn accepted_id_that_names_no_fixture_is_reported() {
        let fx = vec![fixture("a"), fixture("b")];
        assert_eq!(
            unknown_accepted(&set(&["a", "ghost"]), &fx),
            vec!["ghost".to_string()]
        );
        assert!(unknown_accepted(&set(&["a", "b"]), &fx).is_empty());
    }

    fn fixture(id: &str) -> Fixture {
        Fixture {
            id: id.into(),
            step: "s".into(),
            shape: "x".into(),
            scope: "x".into(),
            form: "x".into(),
            naming: "x".into(),
            lang: "ts".into(),
            contract: None,
            site_file: "a.ts".into(),
            site_line: Some(1),
            truth: Truth::None,
            files: BTreeMap::new(),
        }
    }

    #[test]
    fn changes_reports_every_differing_row_not_just_the_first() {
        let expected = vec![
            row("a", "CORRECT"),
            row("b", "MISSING"),
            row("c", "CORRECT"),
        ];
        let mut actual = expected.clone();
        actual[0].verdict = "WRONG".into();
        actual[2].exact = vec![Hit {
            file: "f.ts".into(),
            name: "f".into(),
            line: 1,
            kind: "k".into(),
        }];
        let ids: Vec<_> = changes(&expected, &actual)
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, vec!["a", "c"]);
    }

    #[test]
    fn changes_flags_fixture_without_pinned_row_and_orphan_pinned_row() {
        let expected = vec![row("a", "CORRECT"), row("old", "CORRECT")];
        let actual = vec![row("a", "CORRECT"), row("new", "CORRECT")];
        let cs = changes(&expected, &actual);
        assert_eq!(cs.len(), 1);
        assert_eq!(cs[0].id, "new");
        assert!(cs[0].expected.is_none());
        assert_eq!(orphan_expected(&expected, &actual), vec!["old"]);
    }

    #[test]
    fn identical_rows_produce_no_changes() {
        let rows = vec![row("a", "CORRECT"), row("b", "NONE-OK")];
        assert!(changes(&rows, &rows).is_empty());
    }

    #[test]
    fn summary_groups_by_step_and_verdict_move() {
        let cs = vec![
            change("a", Some("MISSING"), "CORRECT"),
            change("b", Some("MISSING"), "CORRECT"),
            change("c", Some("CORRECT"), "WRONG"),
        ];
        let steps = BTreeMap::from([("a", "local"), ("b", "local"), ("c", "paths")]);
        assert_eq!(
            summary(&cs, &steps),
            "  local: MISSING -> CORRECT: 2\n  paths: CORRECT -> WRONG: 1"
        );
    }

    #[test]
    fn verdicts_follow_the_reference_definitions() {
        let t = |name: &str, line: u64| Target {
            file: "callee.ts".into(),
            name: name.into(),
            line: Some(line),
        };
        let h = |name: &str, line: u64| Hit {
            file: "callee.ts".into(),
            name: name.into(),
            line,
            kind: "k".into(),
        };
        let obs = |exact: Vec<Hit>, nameonly: Vec<Hit>| Observed {
            record: true,
            exact,
            nameonly,
            ..Default::default()
        };
        let one = Truth::One {
            targets: vec![t("f", 1)],
        };
        assert_eq!(verdict(&one, &obs(vec![h("f", 1)], vec![])), "CORRECT");
        assert_eq!(verdict(&one, &obs(vec![h("g", 2)], vec![])), "WRONG");
        assert_eq!(verdict(&one, &obs(vec![], vec![h("f", 1)])), "DEMOTED");
        assert_eq!(verdict(&one, &obs(vec![], vec![])), "MISSING");
        let set = Truth::Set {
            targets: vec![t("f", 1), t("g", 2)],
        };
        assert_eq!(verdict(&set, &obs(vec![h("f", 1)], vec![])), "PARTIAL");
        // a line match counts even when the name differs
        assert_eq!(
            verdict(&one, &obs(vec![h("renamed", 1)], vec![])),
            "CORRECT"
        );
        let may = Truth::May {
            targets: vec![t("f", 1)],
        };
        assert_eq!(verdict(&may, &obs(vec![h("f", 1)], vec![])), "OVERCLAIM");
        assert_eq!(verdict(&may, &obs(vec![h("g", 2)], vec![])), "WRONG");
        assert_eq!(verdict(&may, &obs(vec![], vec![h("f", 1)])), "MAY-NAMEONLY");
        assert_eq!(verdict(&may, &obs(vec![], vec![])), "MAY-NONE");
        assert_eq!(
            verdict(&Truth::None, &obs(vec![h("f", 1)], vec![])),
            "WRONG"
        );
        assert_eq!(
            verdict(&Truth::None, &obs(vec![], vec![h("f", 1)])),
            "NONE+NAMEONLY"
        );
        assert_eq!(verdict(&Truth::None, &obs(vec![], vec![])), "NONE-OK");
        let anon = Truth::Anon {
            file: "callee.ts".into(),
            line: Some(3),
        };
        assert_eq!(verdict(&anon, &obs(vec![h("x", 3)], vec![])), "CORRECT");
        assert_eq!(verdict(&anon, &obs(vec![h("x", 4)], vec![])), "WRONG");
        assert_eq!(verdict(&anon, &obs(vec![], vec![h("x", 3)])), "DEMOTED");
        assert_eq!(verdict(&anon, &obs(vec![], vec![])), "MISSING");
        assert_eq!(verdict(&one, &Observed::default()), "NO-RECORD");
    }
}
