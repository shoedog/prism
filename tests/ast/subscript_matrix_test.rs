use crate::common::dfg_labels;
use crate::subscript_matrix_routes::{
    at, marked_def, no_exact_route, no_route, route_through, uses_at, Bytes, Graph,
};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use prism::cpg::{CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::reasoning::seeds::SeedSpec;
use prism::reasoning::taint_reaches::taint_reaches;
use prism::repo_loader::load_repo;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

mod subscript_matrix_q;

const ENABLED: &[&str] = &["control", "baseline-gap", "boundary"];
const ENABLED_MODES: &[&str] = &["py", "js", "ts", "tsx", "go", "c", "java", "rs"];
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
    #[serde(default)]
    w_bytes: Bytes,
    r_bytes: Bytes,
    sink_line: usize,
    sink_bytes: Bytes,
    expect: Option<NegativeCheck>,
    expect_if_not: Option<NegativeCheck>,
    base_verdict: Option<String>,
    label: Option<LabelCheck>,
    accepted_loss: Option<AcceptedLoss>,
    pinned_main: Option<Pin>,
    case_sha256: Option<String>,
    truth_protocol: Option<String>,
    sink: Option<subscript_matrix_q::Site>,
    #[serde(default)]
    markers: Vec<subscript_matrix_q::Marker>,
}

#[derive(Deserialize)]
struct AcceptedLoss {
    reason: String,
    issue: Option<String>,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NegativeCheck {
    NoExactRoute,
    NoRoute,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(try_from = "serde_json::Value")]
enum Truth {
    Reaches,
    Not,
    Inadmissible,
    NotApplicable,
    Mixed([bool; 2]),
}

impl TryFrom<serde_json::Value> for Truth {
    type Error = String;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        use serde_json::Value::Bool;
        match value.as_str() {
            Some("reaches") => Ok(Self::Reaches),
            Some("not") => Ok(Self::Not),
            Some("inadmissible") => Ok(Self::Inadmissible),
            Some("n/a") => Ok(Self::NotApplicable),
            _ => match value.as_array().map(Vec::as_slice) {
                Some([Bool(a), Bool(b)]) => Ok(Self::Mixed([*a, *b])),
                _ => Err(format!("invalid truth {value}")),
            },
        }
    }
}

#[derive(Deserialize)]
struct TruthRow {
    id: String,
    truth: Truth,
    case_sha256: Option<String>,
}

#[derive(Deserialize)]
struct Observation {
    verdict: String,
    truth: Option<Truth>,
    route: Option<PinnedRoute>,
    through_write: Option<bool>,
    sink_uses: Option<BTreeSet<SinkUse>>,
    edges: Vec<EdgeObservation>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct SinkUse {
    owner: String,
    path: String,
    bytes: Bytes,
}

#[derive(Debug, Deserialize)]
struct Endpoint {
    access: VarAccess,
    bytes: Bytes,
    node_count: Option<usize>,
}

#[derive(Deserialize)]
struct EdgeObservation {
    from: Endpoint,
    to: Endpoint,
    labels: Vec<Label>,
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
    NameonlyCfgIncomplete,
}

impl Label {
    fn matches(self, observed: FlowConfidence) -> bool {
        match self {
            Self::Exact => observed == FlowConfidence::Exact,
            Self::Killed => matches!(observed, FlowConfidence::NameOnly(FlowDoubt::Killed { .. })),
            Self::Nonexact => matches!(observed, FlowConfidence::NameOnly(_)),
            Self::NameonlyCfgIncomplete => {
                observed == FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
            }
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
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/subscript_matrix")
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
    let ids: BTreeMap<_, _> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
    assert_eq!(rows.len(), ids.len(), "duplicate subscript matrix row IDs");

    let truth_path = root.join("truth/truth.jsonl");
    let truth_text = std::fs::read_to_string(&truth_path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}. Run python3 -B truth/run_truth.py from the matrix directory.",
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
        let spec = ids
            .get(row.id.as_str())
            .unwrap_or_else(|| panic!("unknown truth ID {}", row.id));
        assert_eq!(
            row.case_sha256, spec.case_sha256,
            "{}: receipt SHA-256 differs",
            row.id
        );
        if let Some(expected) = &spec.case_sha256 {
            let bytes = std::fs::read(root.join("cases").join(&spec.file)).unwrap();
            assert_eq!(
                &format!("{:x}", Sha256::digest(&bytes)),
                expected,
                "{}: case SHA-256 differs",
                row.id
            );
        }
        assert!(
            truth.insert(row.id.clone(), row.truth).is_none(),
            "duplicate truth for {}",
            row.id
        );
    }
    (rows, truth)
}

fn read_observations(root: &Path) -> BTreeMap<String, Observation> {
    #[derive(Deserialize)]
    struct Record {
        id: String,
        #[serde(flatten)]
        observation: Observation,
    }

    let path = root.join("observations.jsonl");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut observations = BTreeMap::new();
    for value in read_jsonl(&path, &text) {
        let record: Record = serde_json::from_value(value).expect("invalid observation");
        assert!(
            observations
                .insert(record.id.clone(), record.observation)
                .is_none(),
            "duplicate observation for {}",
            record.id
        );
    }
    observations
}

fn check_route(
    graph: &Graph,
    marked: NodeIndex,
    sink: &[NodeIndex],
    expected: &PinnedRoute,
) -> Result<(), String> {
    let actual = if sink.is_empty() {
        PinnedRoute::MissingSinkUse
    } else if !no_exact_route(graph, marked, sink) {
        PinnedRoute::Exact
    } else if !no_route(graph, marked, sink) {
        PinnedRoute::Nonexact
    } else {
        PinnedRoute::None
    };
    if *expected != actual {
        return Err(format!(
            "expected pinned route {expected:?}, got {actual:?}"
        ));
    }
    Ok(())
}

fn check_negative(
    graph: &Graph,
    marked: NodeIndex,
    sink: &[NodeIndex],
    expected: NegativeCheck,
) -> Result<(), String> {
    match expected {
        NegativeCheck::NoExactRoute if !no_exact_route(graph, marked, sink) => {
            Err("unexpected all-exact route".into())
        }
        NegativeCheck::NoRoute if !no_route(graph, marked, sink) => Err("unexpected route".into()),
        _ => Ok(()),
    }
}

fn check_sink_inventory(
    graph: &Graph,
    file: &str,
    row: &Row,
    expected: &BTreeSet<SinkUse>,
) -> Result<(), String> {
    let actual: BTreeSet<_> = graph
        .node_weights()
        .filter_map(|node| match node {
            CpgNode::Variable {
                file: node_file,
                function,
                line,
                path,
                access: VarAccess::Use,
                start_byte,
                end_byte,
                ..
            } if node_file == file && *line == row.sink_line => Some(SinkUse {
                owner: function.clone(),
                path: path.to_string(),
                bytes: [*start_byte, *end_byte],
            }),
            _ => None,
        })
        .collect();
    if actual != *expected {
        return Err(format!(
            "expected sink-line Uses {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn check_edge(graph: &Graph, file: &str, expected: &EdgeObservation) -> Result<(), String> {
    for endpoint in [&expected.from, &expected.to] {
        let actual = graph
            .node_weights()
            .filter(|node| at(node, file, endpoint.bytes, endpoint.access))
            .count();
        let count = endpoint.node_count.unwrap_or(1);
        if actual != count {
            return Err(format!(
                "{endpoint:?}: expected {count} nodes, found {actual}"
            ));
        }
    }
    let mut found: Vec<_> = graph
        .edge_references()
        .filter_map(|edge| match edge.weight() {
            CpgEdge::DataFlow(label)
                if at(
                    &graph[edge.source()],
                    file,
                    expected.from.bytes,
                    expected.from.access,
                ) && at(
                    &graph[edge.target()],
                    file,
                    expected.to.bytes,
                    expected.to.access,
                ) =>
            {
                Some(*label)
            }
            _ => None,
        })
        .collect();
    for label in &expected.labels {
        if let Some(index) = found.iter().position(|actual| label.matches(*actual)) {
            found.remove(index);
        } else {
            return Err(format!(
                "{:?} to {:?}: missing {label:?}, remaining labels {found:?}",
                expected.from, expected.to
            ));
        }
    }
    if !found.is_empty() {
        return Err(format!(
            "{:?} to {:?}: unexpected labels {found:?}",
            expected.from, expected.to
        ));
    }
    Ok(())
}

fn check_row(
    root: &Path,
    row: &Row,
    truth: &Truth,
    observation: Option<&Observation>,
) -> Result<(), String> {
    let source = std::fs::read_to_string(root.join("cases").join(&row.file))
        .map_err(|error| format!("reading {}: {error}", row.file))?;
    let file = format!("case.{}", row.lang);
    let fixture = fixture(&file, &source);
    if !row.markers.is_empty() {
        subscript_matrix_q::check_truth(row, truth)?;
        return subscript_matrix_q::check(&fixture, &file, row);
    }
    let cpg = fixture.session.index.cpg();
    if row.group.starts_with("recall-") {
        let observation = observation.ok_or("missing recall observation")?;
        let missing: Vec<_> = observation
            .edges
            .iter()
            .filter_map(|edge| check_edge(&cpg.graph, &file, edge).err())
            .collect();
        if !missing.is_empty() {
            return Err(missing.join(" | "));
        }
    }
    let exact_sink = uses_at(&cpg.graph, &file, row.sink_bytes);
    let sink = if row.group == "baseline-gap"
        && observation.and_then(|pin| pin.route.as_ref()) == Some(&PinnedRoute::MissingSinkUse)
    {
        exact_sink.unwrap_or_default()
    } else {
        exact_sink?
    };
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

    if matches!(row.group.as_str(), "baseline-gap" | "boundary") {
        let pin = observation.ok_or("missing baseline observation")?;
        if verdict != pin.verdict {
            return Err(format!(
                "expected baseline verdict {}, got {verdict}",
                pin.verdict
            ));
        }
        if let Some(expected) = &pin.truth {
            if truth != expected {
                return Err(format!(
                    "expected executed truth {expected:?}, got {truth:?}"
                ));
            }
        }
        if let Some(expected) = &pin.route {
            check_route(&cpg.graph, marked, &sink, expected)?;
        }
        if let Some(expected) = pin.through_write {
            if route_through(&cpg.graph, marked, row.w_bytes, &sink) != expected {
                return Err(format!(
                    "expected route through W {:?}: {expected}",
                    row.w_bytes
                ));
            }
        }
        if let Some(expected) = &pin.sink_uses {
            check_sink_inventory(&cpg.graph, &file, row, expected)?;
        }
        for edge in &pin.edges {
            check_edge(&cpg.graph, &file, edge)?;
        }
        if row.group == "boundary" && *truth == Truth::Not {
            if let Some(expected) = row.expect.or(row.expect_if_not) {
                check_negative(&cpg.graph, marked, &sink, expected)?;
            }
        }
        return Ok(());
    }

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
            check_route(&cpg.graph, marked, &sink, expected)?;
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
            Some(expected) => check_negative(&cpg.graph, marked, &sink, expected)?,
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
        let pins = read_observations(&root);
        let parent = std::env::var("PRISM_T2_PARENT_RED").ok();
        let groups_to_run: &[&str] = if let Some(parent) = &parent {
            let expected = "14c74b4cd39264f6cdff1b3eb07c88b8e234f463";
            assert_eq!(parent, expected, "incorrect lane T2 parent");
            let build = prism::api::build_info();
            assert_eq!(build.git_sha.trim_end_matches("-dirty"), &expected[..12]);
            eprintln!(
                "T2_PARENT {} build {} {}",
                parent, build.git_sha, build.build_identity
            );
            &["recall-py"]
        } else {
            ENABLED
        };
        let mut failures = Vec::new();
        let mut enabled = 0;
        let mut groups = BTreeMap::new();
        for row in rows
            .iter()
            .filter(|row| groups_to_run.contains(&row.group.as_str()))
        {
            assert!(
                ENABLED_MODES.contains(&row.lang.as_str()),
                "{}: unsupported enabled mode {}",
                row.id,
                row.lang
            );
        }
        for row in rows
            .iter()
            .filter(|row| groups_to_run.contains(&row.group.as_str()))
        {
            enabled += 1;
            *groups.entry(row.group.as_str()).or_insert(0usize) += 1;
            if let Some(pin) = &row.pinned_main {
                eprintln!("{} pinned_main: {}", row.id, pin.reason);
            }
            let result = match truth.get(&row.id) {
                None => Err("missing executed truth".into()),
                Some(Truth::Inadmissible) => Err("inadmissible executed truth".into()),
                Some(truth) => check_row(&root, row, truth, pins.get(&row.id)),
            };
            match (parent.is_some(), result) {
                (true, Err(error)) => {
                    let mut reasons = error.split(" | ");
                    let missing_pair = reasons.any(|reason| {
                        reason.contains(
                            "expected Some(NameonlyCfgIncomplete), got DataFlow labels []",
                        ) || reason.contains("missing NameonlyCfgIncomplete, remaining labels []")
                    });
                    if !missing_pair
                        || error.contains("unbound production query")
                        || error.contains("production query omitted")
                        || error.contains("production query did not isolate")
                        || error.contains("production query duplicated")
                        || error.contains("production query returned no reasoning")
                        || error.contains("wrong witness terminal")
                        || error.contains("missing typed node")
                        || error.contains("nodes, found")
                    {
                        failures.push(format!("{}: no admissible parent RED: {error}", row.id));
                    } else {
                        let bytes = std::fs::read(root.join("cases").join(&row.file)).unwrap();
                        eprintln!(
                            "T2_PARENT_RED {} {:x} {error}",
                            row.id,
                            Sha256::digest(&bytes)
                        );
                    }
                }
                (true, Ok(())) => failures.push(format!("{}: expected RED on parent", row.id)),
                (false, Err(error)) => failures.push(format!("{}: {error}", row.id)),
                (false, Ok(())) => {}
            }
        }
        if parent.is_some() {
            eprintln!("T2_PARENT_TOTAL {enabled}");
        }
        assert!(enabled > 0, "no subscript matrix rows enabled");
        for group in groups_to_run {
            assert!(
                groups.get(group).copied().unwrap_or(0) > 0,
                "no rows ran for group {group}"
            );
        }
        assert!(
            failures.is_empty(),
            "{} of {enabled} enabled subscript matrix rows failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn subscript_matrix_rows_have_truth() {
        let root = matrix_dir();
        let (rows, truth) = read_matrix(&root);
        let observations = read_observations(&root);
        let expected_observation_ids: BTreeSet<_> = rows
            .iter()
            .filter(|row| row.group != "control" && row.markers.is_empty())
            .map(|row| &row.id)
            .collect();
        assert_eq!(
            observations.keys().collect::<BTreeSet<_>>(),
            expected_observation_ids,
            "observations must cover every non-control row"
        );
        let mut failures = Vec::new();
        for row in &rows {
            if let Some(loss) = &row.accepted_loss {
                if loss.reason.trim().is_empty()
                    || loss
                        .issue
                        .as_ref()
                        .is_some_and(|issue| issue.trim().is_empty())
                {
                    failures.push(format!("{}: invalid accepted_loss", row.id));
                }
            }
            if !row.markers.is_empty() {
                let truth = &truth[&row.id];
                if let Err(error) = subscript_matrix_q::check_truth(row, truth) {
                    failures.push(format!("{}: {error}", row.id));
                }
                continue;
            }
            let valid = match row.lang.as_str() {
                "py" | "js" | "ts" | "tsx" | "go" => {
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

    #[test]
    fn structural_gap_cannot_pass_a_behavioural_row() {
        let root = matrix_dir();
        let (rows, truth) = read_matrix(&root);
        let pins = read_observations(&root);
        let mut row = rows.into_iter().find(|row| row.id == "M04.js").unwrap();
        let pin = pins.get(&row.id);
        let truth = &truth[&row.id];
        check_row(&root, &row, truth, pin).unwrap();

        row.group = "boundary".into();
        let error = check_row(&root, &row, truth, pin).unwrap_err();
        assert!(error.contains("missing sink argument Use"), "{error}");
    }

    #[test]
    fn forbidden_pair_fails_with_exact_or_weak_labels() {
        let root = matrix_dir();
        let (rows, _) = read_matrix(&root);
        for id in ["M01.py", "P26.py"] {
            let row = rows.iter().find(|row| row.id == id).unwrap();
            let source = std::fs::read_to_string(root.join("cases").join(id)).unwrap();
            let file = "case.py";
            let fixture = fixture(file, &source);
            let graph = &fixture.session.index.cpg().graph;
            let mut forbidden = EdgeObservation {
                from: Endpoint {
                    access: VarAccess::Def,
                    bytes: row.w_bytes,
                    node_count: None,
                },
                to: Endpoint {
                    access: VarAccess::Use,
                    bytes: row.r_bytes,
                    node_count: None,
                },
                labels: vec![],
            };
            let error = check_edge(graph, file, &forbidden).unwrap_err();
            assert!(error.contains("unexpected labels"), "{id}: {error}");

            forbidden.to.bytes = row.src_bytes;
            let error = check_edge(graph, file, &forbidden).unwrap_err();
            assert!(error.contains("expected 1 nodes, found 0"), "{id}: {error}");
            forbidden.to.node_count = Some(0);
            check_edge(graph, file, &forbidden).unwrap();
            forbidden.to.access = VarAccess::Def;
            forbidden.to.node_count = None;
            check_edge(graph, file, &forbidden).unwrap();
        }
    }
}
