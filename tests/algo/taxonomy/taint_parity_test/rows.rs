use prism::navigation::types::Reachability;
use serde::Deserialize;
use std::collections::BTreeSet;

pub(super) const GROUPS: &[&str] = &[
    "control",
    "binding",
    "same_line",
    "spelling",
    "knobs",
    "catalog",
    "walk",
    "guard",
    "reasons",
    "tbd",
];
const REASONS: &[&str] = &[
    "target_not_binding",
    "no_matching_site",
    "ambiguous_site",
    "not_exact",
    "unproven_slot",
    "depth_cap",
    "policy_ceiling",
    "self_param",
    "self_return",
    "return_not_entered_site",
    "no_graph_edge",
    "shortcut_suppressed",
    "cfg_pruned",
    "sanitizer_model",
    "operand_granularity",
    "unmodelled",
    "review_no_return_flow",
    "review_no_recovered_def_use",
    "review_cfg_filter",
    "review_context_policy",
];
pub(super) type Bytes = [usize; 2];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Row {
    pub(super) id: String,
    pub(super) group: String,
    pub(super) lang: String,
    pub(super) fixture: String,
    axes: Axes,
    pub(super) src: Source,
    pub(super) site: Option<Site>,
    pub(super) sink: Sink,
    pub(super) walk_comparable: bool,
    #[serde(deserialize_with = "Deserialize::deserialize")]
    pub(super) expect: Option<Expectations>,
    reasons: Option<Reasons>,
    knobs: Option<Knobs>,
    truth: Truth,
    known_wrong: Option<String>,
    pending: Option<Pending>,
    class: String,
    note: String,
}

impl Row {
    pub(super) fn pending_issue(&self, level: Level) -> Option<&str> {
        self.pending
            .as_ref()
            .filter(|pending| pending.levels.contains(&level))
            .map(|pending| pending.issue.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Level {
    Walk,
    Review,
    TraceReview,
    Catalog,
    Reasons,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pending {
    levels: Vec<Level>,
    issue: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Axes {
    mechanism: String,
    layout: String,
    resolution: String,
    depth: usize,
    sanitizer: String,
    direction: String,
    distance: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) bytes: Bytes,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum Slot {
    Argument(usize),
    Return(ReturnSlot),
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum ReturnSlot {
    Return,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Site {
    pub(super) bytes: Bytes,
    pub(super) slot: Slot,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Sink {
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) operand: Bytes,
    pub(super) nodes: Vec<Bytes>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Expectations {
    pub(super) walk: Walk,
    pub(super) review: Finding,
    pub(super) trace_review: Finding,
    pub(super) catalog: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(super) enum Verdict {
    Reached,
    NotReached,
    BoundaryExited,
    Sanitized,
    SeedError,
}

impl From<Reachability> for Verdict {
    fn from(value: Reachability) -> Self {
        match value {
            Reachability::Reached => Self::Reached,
            Reachability::NotReached => Self::NotReached,
            Reachability::BoundaryExited => Self::BoundaryExited,
            Reachability::Sanitized => Self::Sanitized,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Walk {
    pub(super) reaches: Verdict,
    pub(super) source: Option<Bytes>,
    pub(super) terminal: Option<Bytes>,
    pub(super) descent_depth: Option<usize>,
}

impl Walk {
    pub(super) fn no_witness(reaches: Verdict) -> Self {
        Self {
            reaches,
            source: None,
            terminal: None,
            descent_depth: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(super) enum Finding {
    Exact,
    NameOnly,
    Unlabeled,
    None,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CatalogEntry {
    pub(super) file: String,
    pub(super) line: usize,
    pub(super) rule: String,
    pub(super) operand: Option<Bytes>,
    pub(super) verdict: Verdict,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reasons {
    walk: Vec<String>,
    review: Vec<String>,
    trace_review: Vec<String>,
    catalog: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Knobs {
    edges: String,
    depth: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Truth {
    Flows,
    NoFlow,
}

/// Row files in reading order. `PARITY_ROWS=a.jsonl,b.jsonl` (names under `tests/fixtures/taint_parity/`)
/// replaces the list at run time, so a worker can check its own rows file next to `rows.jsonl`.
const ROW_FILES: &[(&str, &str)] = &[
    (
        "rows.jsonl",
        include_str!("../../../fixtures/taint_parity/rows.jsonl"),
    ),
    (
        "rows-t2a.jsonl",
        include_str!("../../../fixtures/taint_parity/rows-t2a.jsonl"),
    ),
    (
        "rows-t2b.jsonl",
        include_str!("../../../fixtures/taint_parity/rows-t2b.jsonl"),
    ),
    (
        "rows-t2c.jsonl",
        include_str!("../../../fixtures/taint_parity/rows-t2c.jsonl"),
    ),
];

pub(super) fn rows() -> Vec<Row> {
    let sources: Vec<(String, String)> = match std::env::var("PARITY_ROWS") {
        Ok(list) => list
            .split(',')
            .map(|name| {
                let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/taint_parity")
                    .join(name);
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
                (name.to_string(), text)
            })
            .collect(),
        Err(_) => ROW_FILES
            .iter()
            .map(|(name, text)| (name.to_string(), text.to_string()))
            .collect(),
    };
    let mut rows = Vec::new();
    for (name, text) in &sources {
        for (index, line) in text
            .lines()
            .enumerate()
            .filter(|(_, line)| !line.trim().is_empty())
        {
            rows.push(
                serde_json::from_str(line)
                    .unwrap_or_else(|err| panic!("{name}:{}: {err}", index + 1)),
            );
        }
    }
    rows
}

pub(super) fn validate_labels(row: &Row) {
    let axes = &row.axes;
    for (value, allowed) in [
        (row.group.as_str(), GROUPS),
        (
            row.lang.as_str(),
            &["javascript", "python", "go", "typescript", "tsx"][..],
        ),
        (
            row.class.as_str(),
            &["research", "seed", "design_review", "second_review"][..],
        ),
        (
            axes.mechanism.as_str(),
            &[
                "intra",
                "argument",
                "binding",
                "return",
                "capture",
                "callback",
                "field",
                "sanitizer",
                "dispatch",
                "catalog",
            ][..],
        ),
        (
            axes.layout.as_str(),
            &["same_line", "separate_lines", "multiline"][..],
        ),
        (
            axes.resolution.as_str(),
            &["none", "exact", "nameonly", "multi_exact", "unresolved"][..],
        ),
        (
            axes.sanitizer.as_str(),
            &["none", "on_path", "off_path", "other_category"][..],
        ),
        (
            axes.direction.as_str(),
            &["intra", "descent", "ascent", "both"][..],
        ),
        (
            axes.distance.as_str(),
            &["same_function", "same_file", "cross_file"][..],
        ),
    ] {
        assert!(
            allowed.contains(&value),
            "{}: invalid value {value}",
            row.id
        );
    }
    if let Some(knobs) = &row.knobs {
        assert!(
            ["exact", "nameonly"].contains(&knobs.edges.as_str()),
            "{}",
            row.id
        );
        assert_eq!(knobs.depth, axes.depth, "{}", row.id);
    }
    if let Some(pending) = &row.pending {
        let distinct: BTreeSet<_> = pending.levels.iter().collect();
        assert!(
            !pending.issue.trim().is_empty()
                && !distinct.is_empty()
                && distinct.len() == pending.levels.len(),
            "{}: pending needs an issue and distinct levels",
            row.id
        );
    }
    let observations = row
        .note
        .strip_prefix("observed on 9d1a7d06f: ")
        .unwrap_or_else(|| panic!("{}: missing base observation", row.id));
    let Some(expect) = &row.expect else {
        let observed_cells = ["walk=", "review=", "trace_review="].iter().all(|&label| {
            observations.split("; ").any(|cell| {
                cell.strip_prefix(label)
                    .is_some_and(|value| !value.trim().is_empty())
            })
        });
        assert!(
            row.group == "tbd" && row.pending.is_none() && row.reasons.is_none() && observed_cells,
            "{}: null expect requires a tbd row with observed cells in its note",
            row.id
        );
        return;
    };
    let walk = &expect.walk;
    let has_witness = matches!(walk.reaches, Verdict::Reached | Verdict::Sanitized);
    assert_eq!(
        walk.source,
        has_witness.then_some(row.src.bytes),
        "{}",
        row.id
    );
    assert_eq!(walk.terminal.is_some(), has_witness, "{}", row.id);
    assert_eq!(walk.descent_depth.is_some(), has_witness, "{}", row.id);
    assert!(
        walk.terminal
            .is_none_or(|span| row.sink.nodes.contains(&span)),
        "{}",
        row.id
    );
    validate_differences(row, expect);
}

fn validate_differences(row: &Row, expect: &Expectations) {
    let catalog = (row.group == "catalog" || !expect.catalog.is_empty()).then(|| {
        expect.catalog.iter().any(|entry| {
            entry.file == row.sink.file
                && entry.line == row.sink.line
                && entry.verdict == Verdict::Reached
        })
    });
    let reasons = row.reasons.as_ref();
    let levels = [
        (
            Some(expect.walk.reaches == Verdict::Reached),
            reasons.map(|r| &r.walk),
        ),
        (
            Some(expect.review != Finding::None),
            reasons.map(|r| &r.review),
        ),
        (
            Some(expect.trace_review != Finding::None),
            reasons.map(|r| &r.trace_review),
        ),
        (catalog, reasons.map(|r| &r.catalog)),
    ];
    let truth = matches!(row.truth, Truth::Flows);
    let mut agrees = false;
    let mut differs = false;
    let mut explained = false;
    for (flows, reasons) in levels {
        for reason in reasons.into_iter().flatten() {
            assert!(REASONS.contains(&reason.as_str()), "{}: {reason}", row.id);
        }
        if let Some(flows) = flows {
            agrees |= flows == truth;
            differs |= flows != truth;
            explained |= flows != truth && reasons.is_some_and(|reasons| !reasons.is_empty());
        }
    }
    assert!(
        !(agrees && differs) || explained,
        "{}: disagreeing levels need a reason on a level that differs from truth",
        row.id,
    );
    assert!(
        !differs || explained || row.known_wrong.is_some(),
        "{}: truth differs without a reason or known_wrong issue",
        row.id,
    );
}
