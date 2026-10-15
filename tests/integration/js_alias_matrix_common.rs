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
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CallerTruth {
    Anon {
        file: String,
        line: usize,
        col: usize,
    },
    Named {
        name: String,
        line: usize,
    },
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
    pub caller: Option<CallerTruth>,
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

/// Callable identity: same file, same name, and the same declaration line when
/// the truth records one.
fn hit(t: &Hit, target: &Target) -> bool {
    t.file == target.file && t.name == target.name && target.line.is_none_or(|line| t.line == line)
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
    fn identity_needs_name_and_line_not_either() {
        let t = |name: &str, line: Option<u64>| Target {
            file: "a.ts".into(),
            name: name.into(),
            line,
        };
        let h = |name: &str, line: u64| Hit {
            file: "a.ts".into(),
            name: name.into(),
            line,
            kind: "k".into(),
        };
        let obs = |exact: Vec<Hit>| Observed {
            record: true,
            exact,
            ..Default::default()
        };
        // outer `function f` on line 1, inner `function f` on line 3; truth is the inner one
        let inner = Truth::One {
            targets: vec![t("f", Some(3))],
        };
        assert_eq!(verdict(&inner, &obs(vec![h("f", 1)])), "WRONG");
        assert_eq!(verdict(&inner, &obs(vec![h("f", 3)])), "CORRECT");
        // two functions on one line: a target named `g` does not match a truth for `f`
        let f_only = Truth::One {
            targets: vec![t("f", Some(2))],
        };
        assert_eq!(verdict(&f_only, &obs(vec![h("g", 2)])), "WRONG");
        // a truth without a line matches on file and name alone
        let unlined = Truth::One {
            targets: vec![t("f", None)],
        };
        assert_eq!(verdict(&unlined, &obs(vec![h("f", 9)])), "CORRECT");
        assert_eq!(verdict(&unlined, &obs(vec![h("g", 9)])), "WRONG");
        // another file never matches
        let other = Hit {
            file: "b.ts".into(),
            ..h("f", 3)
        };
        assert_eq!(verdict(&inner, &obs(vec![other])), "WRONG");
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
        // a line match alone is not identity: the name must agree too
        assert_eq!(verdict(&one, &obs(vec![h("renamed", 1)], vec![])), "WRONG");
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
