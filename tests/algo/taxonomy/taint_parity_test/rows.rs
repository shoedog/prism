use prism::navigation::types::Reachability;
use serde::Deserialize;

const GROUPS: &[&str] = &[
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
    pub(super) expect: Expectations,
    reasons: Option<Reasons>,
    knobs: Option<Knobs>,
    truth: Truth,
    known_wrong: Option<String>,
    pub(super) pending: Option<String>,
    class: String,
    note: String,
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

pub(super) fn rows() -> Vec<Row> {
    include_str!("../../../fixtures/taint_parity/rows.jsonl")
        .lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .unwrap_or_else(|err| panic!("rows.jsonl:{}: {err}", index + 1))
        })
        .collect()
}

pub(super) fn validate_labels(row: &Row) {
    let axes = &row.axes;
    for (value, allowed) in [
        (row.group.as_str(), GROUPS),
        (
            row.lang.as_str(),
            &["javascript", "python", "go", "tsx"][..],
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
    let explained = row.reasons.as_ref().is_some_and(|reasons| {
        let all = [
            &reasons.walk,
            &reasons.review,
            &reasons.trace_review,
            &reasons.catalog,
        ];
        for reason in all.into_iter().flatten() {
            assert!(REASONS.contains(&reason.as_str()), "{}: {reason}", row.id);
        }
        all.iter().any(|reasons| !reasons.is_empty())
    });
    let walk = &row.expect.walk;
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
    let mut levels = vec![
        walk.reaches == Verdict::Reached,
        row.expect.review != Finding::None,
        row.expect.trace_review != Finding::None,
    ];
    if row.group == "catalog" || !row.expect.catalog.is_empty() {
        levels.push(
            row.expect
                .catalog
                .iter()
                .any(|entry| entry.verdict == Verdict::Reached),
        );
    }
    assert!(
        levels.iter().all(|&reaches| reaches == levels[0]) || explained,
        "{}: disagreeing levels need a reason",
        row.id,
    );
    assert!(
        levels
            .iter()
            .all(|&flows| flows == matches!(row.truth, Truth::Flows))
            || explained
            || row.known_wrong.is_some(),
        "{}: truth differs without a reason or known_wrong issue",
        row.id,
    );
    assert!(
        row.note.starts_with("observed on 9d1a7d06f: "),
        "{}",
        row.id
    );
}
