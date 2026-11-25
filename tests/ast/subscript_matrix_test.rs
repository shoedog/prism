use crate::common::dfg_labels;
use crate::subscript_matrix_routes::{
    marked_def, no_exact_route, no_route, route_through, uses_at, Bytes,
};
use prism::cpg::{FlowConfidence, FlowDoubt};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::reasoning::seeds::SeedSpec;
use prism::reasoning::taint_reaches::taint_reaches;
use prism::repo_loader::load_repo;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

const ENABLED: &[&str] = &["control"];
// Stage D selects the policy before enabling the policy-dependent rows.
const ALIAS_POLICY: Option<&str> = None;

#[derive(Deserialize)]
struct Row {
    id: String,
    group: String,
    lang: String,
    file: String,
    src_line: usize,
    src_bytes: Bytes,
    w_bytes: Bytes,
    r_bytes: Bytes,
    sink_line: usize,
    sink_bytes: Bytes,
    expect: Option<NegativeCheck>,
    expect_if_not: Option<NegativeCheck>,
    base_verdict: Option<String>,
    label: Option<LabelCheck>,
    pinned_main: Option<Pin>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NegativeCheck {
    NoExactRoute,
    NoRoute,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "snake_case")]
enum Truth {
    Reaches,
    Not,
    Inadmissible,
    #[serde(rename = "n/a")]
    NotApplicable,
}

#[derive(Deserialize)]
struct TruthRow {
    id: String,
    truth: Truth,
}

#[derive(Deserialize)]
struct Pin {
    verdict: Option<String>,
    reason: String,
    route: Option<PinnedRoute>,
}

#[derive(Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum PinnedRoute {
    Exact,
    Nonexact,
    None,
    MissingSinkUse,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum LabelCheck {
    Single(Label),
    Policy(BTreeMap<String, Label>),
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Label {
    Exact,
    Killed,
    Nonexact,
}

impl Label {
    fn matches(self, observed: FlowConfidence) -> bool {
        match self {
            Self::Exact => observed == FlowConfidence::Exact,
            Self::Killed => matches!(observed, FlowConfidence::NameOnly(FlowDoubt::Killed { .. })),
            Self::Nonexact => matches!(observed, FlowConfidence::NameOnly(_)),
        }
    }
}

struct Fixture {
    _dir: TempDir,
    session: NavigationSession,
}

fn fixture(file: &str, source: &str) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(file), source).unwrap();
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    Fixture {
        _dir: dir,
        session: NavigationSession { repo, index },
    }
}

fn matrix_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/subscript_matrix")
}

fn read_jsonl<'a>(path: &'a Path, text: &'a str) -> impl Iterator<Item = serde_json::Value> + 'a {
    text.lines().enumerate().map(move |(index, record)| {
        serde_json::from_str(record)
            .unwrap_or_else(|error| panic!("{}:{}: {error}", path.display(), index + 1))
    })
}

fn read_matrix(root: &Path) -> (Vec<Row>, BTreeMap<String, Truth>) {
    let rows_path = root.join("rows.jsonl");
    let rows_text = std::fs::read_to_string(&rows_path).unwrap();
    let rows: Vec<Row> = read_jsonl(&rows_path, &rows_text)
        .map(|value| serde_json::from_value(value).expect("invalid subscript matrix row"))
        .collect();
    assert!(!rows.is_empty(), "empty subscript matrix");
    let ids: BTreeSet<_> = rows.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(rows.len(), ids.len(), "duplicate subscript matrix row IDs");

    let truth_path = root.join("truth/truth.jsonl");
    let truth_text = std::fs::read_to_string(&truth_path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}. Run truth/run_truth.py cases rows.jsonl from the fixture directory.",
            truth_path.display()
        )
    });
    let mut records = read_jsonl(&truth_path, &truth_text);
    assert!(
        records
            .next()
            .and_then(|record| record.get("versions").cloned())
            .is_some_and(|value| value.is_object()),
        "{}: missing interpreter versions",
        truth_path.display()
    );
    let mut truth = BTreeMap::new();
    for record in records {
        let row: TruthRow = serde_json::from_value(record).expect("invalid row truth");
        assert!(ids.contains(row.id.as_str()), "unknown truth ID {}", row.id);
        assert!(
            truth.insert(row.id.clone(), row.truth).is_none(),
            "duplicate truth for {}",
            row.id
        );
    }
    (rows, truth)
}

fn check_row(root: &Path, row: &Row, truth: &Truth) -> Result<(), String> {
    let source = std::fs::read_to_string(root.join("cases").join(&row.file))
        .map_err(|error| format!("reading {}: {error}", row.file))?;
    let file = format!("case.{}", row.lang);
    let fixture = fixture(&file, &source);
    let cpg = fixture.session.index.cpg();
    let sink = uses_at(&cpg.graph, &file, row.sink_bytes)?;
    let marked = marked_def(&cpg.graph, &file, row.src_bytes)?;

    let seed = |line| SeedSpec::Loc {
        file: file.clone(),
        line,
    };
    let evidence = taint_reaches(
        &fixture.session,
        &[seed(row.src_line)],
        Some(&[seed(row.sink_line)]),
    )
    .map_err(|error| format!("taint_reaches: {error:?}"))?;
    let verdict = evidence
        .reasoning
        .and_then(|reasoning| reasoning.reachability)
        .map(|reachability| format!("{reachability:?}"))
        .ok_or("taint_reaches returned no verdict")?;

    if row.group == "control" {
        let base = row
            .base_verdict
            .as_deref()
            .ok_or("control lacks base_verdict")?;
        if verdict != base {
            return Err(format!("expected base verdict {base}, got {verdict}"));
        }
    }

    if let Some(pin) = &row.pinned_main {
        let expected = pin
            .verdict
            .as_deref()
            .ok_or("pinned_main lacks a base verdict")?;
        if verdict != expected {
            return Err(format!("expected pinned verdict {expected}, got {verdict}"));
        }
        if let Some(expected) = &pin.route {
            let actual = if !no_exact_route(&cpg.graph, marked, &sink) {
                PinnedRoute::Exact
            } else if !no_route(&cpg.graph, marked, &sink) {
                PinnedRoute::Nonexact
            } else {
                PinnedRoute::None
            };
            if *expected != actual {
                return Err(format!(
                    "expected pinned route {expected:?}, got {actual:?}"
                ));
            }
        }
        return Ok(());
    }

    match truth {
        Truth::Reaches if verdict != "Reached" => {
            return Err(format!("truth reaches, taint_reaches returned {verdict}"));
        }
        Truth::Reaches if !route_through(&cpg.graph, marked, row.w_bytes, &sink) => {
            return Err(format!("no route through W at {:?}", row.w_bytes));
        }
        Truth::Reaches => {}
        Truth::Not => match row.expect.or(row.expect_if_not) {
            Some(NegativeCheck::NoExactRoute) if !no_exact_route(&cpg.graph, marked, &sink) => {
                return Err("unexpected all-exact route".into());
            }
            Some(NegativeCheck::NoRoute) if !no_route(&cpg.graph, marked, &sink) => {
                return Err("unexpected route".into());
            }
            Some(_) => {}
            None => return Err("truth is not, but neither expect nor expect_if_not is set".into()),
        },
        _ => return Err(format!("invalid truth for an unpinned row: {truth:?}")),
    }

    if let Some(label) = &row.label {
        let expected = match label {
            LabelCheck::Single(label) => *label,
            LabelCheck::Policy(policies) => {
                let policy = ALIAS_POLICY.ok_or("stage D must set ALIAS_POLICY for this row")?;
                *policies
                    .get(policy)
                    .ok_or("label is missing for active alias policy")?
            }
        };
        // Compare the spans on the stored label endpoints explicitly. VarLocation
        // equality groups occurrences by line and path and omits their bytes.
        let labels: BTreeSet<_> = dfg_labels(&cpg.dfg)
            .into_iter()
            .filter(|((from, to), _)| {
                from.file == file
                    && to.file == file
                    && from.kind == prism::data_flow::VarAccessKind::Def
                    && to.kind == prism::data_flow::VarAccessKind::Use
                    && [from.start_byte, from.end_byte] == row.w_bytes
                    && [to.start_byte, to.end_byte] == row.r_bytes
            })
            .map(|(_, label)| label)
            .collect();
        if labels.is_empty() || labels.iter().any(|&label| !expected.matches(label)) {
            return Err(format!(
                "W {:?} to R {:?}: expected {expected:?}, found {labels:?}",
                row.w_bytes, row.r_bytes
            ));
        }
    }
    Ok(())
}

mod subscript_matrix {
    use super::*;

    #[test]
    fn subscript_matrix_rows() {
        let root = matrix_dir();
        let (rows, truth) = read_matrix(&root);
        let mut failures = Vec::new();
        let mut enabled = 0;
        for row in rows
            .iter()
            .filter(|row| ENABLED.contains(&row.group.as_str()))
        {
            enabled += 1;
            if let Some(pin) = &row.pinned_main {
                eprintln!("{} pinned_main: {}", row.id, pin.reason);
            }
            let result = match truth.get(&row.id) {
                None => Err("missing executed truth".into()),
                Some(Truth::Inadmissible) => Err("inadmissible executed truth".into()),
                Some(truth) => check_row(&root, row, truth),
            };
            if let Err(error) = result {
                failures.push(format!("{}: {error}", row.id));
            }
        }
        assert!(enabled > 0, "no subscript matrix rows enabled");
        assert!(
            failures.is_empty(),
            "{} of {enabled} enabled subscript matrix rows failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    // This gate is expected to be RED until the runner has produced truth.jsonl.
    #[test]
    fn subscript_matrix_rows_have_truth() {
        let (rows, truth) = read_matrix(&matrix_dir());
        let mut failures = Vec::new();
        for row in &rows {
            let valid = match row.lang.as_str() {
                "py" | "js" | "ts" | "go" => {
                    matches!(truth.get(&row.id), Some(Truth::Reaches | Truth::Not))
                }
                "rs" | "java" | "c" => truth.get(&row.id) == Some(&Truth::NotApplicable),
                _ => false,
            };
            if !valid {
                failures.push(format!(
                    "{}: missing or invalid truth {:?}",
                    row.id,
                    truth.get(&row.id)
                ));
            }
            if row.group == "control" && row.base_verdict.is_none() {
                failures.push(format!("{}: control lacks base_verdict", row.id));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
