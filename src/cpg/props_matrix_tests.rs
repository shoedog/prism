//! Data-driven coverage for pattern formals, object-literal calls and JSX props.

use super::flow_confidence::{FlowConfidence, FlowDoubt};
use super::object_literal_arg_tests::{build, spell};
use super::trace::BoundaryKind;
use super::{CodePropertyGraph, CpgEdge, CpgNode};
use crate::languages::Language;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

const ENABLED: &[&str] = &["control"];
const ROWS: &str = include_str!("../../tests/fixtures/props_matrix/rows.jsonl");

#[derive(Deserialize)]
struct Row {
    id: String,
    group: String,
    langs: Vec<String>,
    files: BTreeMap<String, String>,
    expect: Vec<Check>,
    class: String,
    truth: String,
    base: Base,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Base {
    Red,
    Green,
}

#[derive(Deserialize)]
struct Check {
    #[serde(default)]
    case: String,
    #[serde(flatten)]
    assertion: Assertion,
}

#[derive(Debug, Deserialize)]
struct Location {
    file: String,
    line: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Label {
    Exact,
    Killed,
    SameLine,
    CfgIncomplete,
    AliasUnstable,
    CallNameOnly,
    BuiltinCallback,
}

impl From<FlowConfidence> for Label {
    fn from(confidence: FlowConfidence) -> Self {
        match confidence {
            FlowConfidence::Exact => Self::Exact,
            FlowConfidence::NameOnly(FlowDoubt::Killed { .. }) => Self::Killed,
            FlowConfidence::NameOnly(FlowDoubt::SameLine) => Self::SameLine,
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => Self::CfgIncomplete,
            FlowConfidence::NameOnly(FlowDoubt::AliasUnstable) => Self::AliasUnstable,
            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly) => Self::CallNameOnly,
            FlowConfidence::NameOnly(FlowDoubt::BuiltinCallback) => Self::BuiltinCallback,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Between {
    from: String,
    to: String,
    label: Option<Label>,
    from_bytes: Option<[usize; 2]>,
    to_bytes: Option<[usize; 2]>,
}

#[derive(Debug, Deserialize)]
struct TraceCheck {
    seed: Location,
    target: Location,
    node: String,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
enum TraceBoundaryKind {
    #[default]
    CrossFunction,
    SelfFunctionParam,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Assertion {
    Def {
        node: String,
        bytes: Option<[usize; 2]>,
        count: Option<usize>,
    },
    NoDef {
        node: String,
    },
    Edge {
        #[serde(flatten)]
        between: Between,
        count: Option<usize>,
    },
    NoEdge {
        #[serde(flatten)]
        between: Between,
    },
    NoEdgeInto {
        to: String,
    },
    Path {
        from: String,
        to: String,
        via: Option<String>,
    },
    NoPath {
        from: String,
        to: String,
    },
    Counter {
        key: String,
        count: u64,
    },
    TraceReaches(TraceCheck),
    TraceMisses(TraceCheck),
    TraceBoundary {
        #[serde(flatten)]
        check: TraceCheck,
        #[serde(default)]
        boundary_kind: TraceBoundaryKind,
        from: Option<String>,
        from_bytes: Option<[usize; 2]>,
    },
}

struct Fixture<'a> {
    graph: &'a CodePropertyGraph,
    ext: &'a str,
    nodes: BTreeMap<String, BTreeSet<NodeIndex>>,
    stats: serde_json::Value,
}

impl<'a> Fixture<'a> {
    fn new(graph: &'a CodePropertyGraph, ext: &'a str) -> Self {
        let mut nodes: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for index in graph.graph.node_indices() {
            if let Some((name, is_use)) = spell(graph, index) {
                let access = if is_use { "use" } else { "def" };
                nodes
                    .entry(format!("{name}({access})"))
                    .or_default()
                    .insert(index);
            }
        }
        let stats = serde_json::to_value(&graph.dfg_label_stats).expect("serialize DFG statistics");
        Self {
            graph,
            ext,
            nodes,
            stats,
        }
    }

    fn named(&self, name: &str) -> &BTreeSet<NodeIndex> {
        static EMPTY: BTreeSet<NodeIndex> = BTreeSet::new();
        self.nodes.get(name).unwrap_or(&EMPTY)
    }

    fn bytes(&self, index: NodeIndex) -> Option<[usize; 2]> {
        match self.graph.node(index) {
            CpgNode::Variable {
                start_byte,
                end_byte,
                ..
            } => Some([*start_byte, *end_byte]),
            _ => None,
        }
    }

    fn flow_from(&self, from: NodeIndex) -> impl Iterator<Item = (NodeIndex, Label)> + '_ {
        self.graph
            .graph
            .edges(from)
            .filter_map(|edge| match *edge.weight() {
                CpgEdge::DataFlow(confidence) => Some((edge.target(), confidence.into())),
                _ => None,
            })
    }

    fn edges(&self, between: &Between) -> Vec<(NodeIndex, NodeIndex, Label)> {
        let mut edges = Vec::new();
        for &from in self.named(&between.from) {
            if between
                .from_bytes
                .is_some_and(|span| self.bytes(from) != Some(span))
            {
                continue;
            }
            for (to, label) in self.flow_from(from) {
                if self.named(&between.to).contains(&to)
                    && between
                        .to_bytes
                        .is_none_or(|span| self.bytes(to) == Some(span))
                {
                    edges.push((from, to, label));
                }
            }
        }
        edges
    }

    fn path(&self, from: &str, to: &str, via: Option<&str>) -> bool {
        let targets = self.named(to);
        let mut pending: Vec<_> = self.named(from).iter().copied().collect();
        if let Some(via) = via {
            let middle = self.named(via);
            return pending.into_iter().any(|start| {
                self.flow_from(start).any(|(hop, _)| {
                    middle.contains(&hop)
                        && self.flow_from(hop).any(|(end, _)| targets.contains(&end))
                })
            });
        }

        let mut seen = BTreeSet::new();
        while let Some(index) = pending.pop() {
            if seen.insert(index) {
                for (next, _) in self.flow_from(index) {
                    if targets.contains(&next) {
                        return true;
                    }
                    pending.push(next);
                }
            }
        }
        false
    }

    fn owner(&self, index: NodeIndex) -> Option<(&str, &str, usize)> {
        match self.graph.node(index) {
            CpgNode::Variable {
                file,
                function,
                function_start_line,
                ..
            }
            | CpgNode::ReturnValue {
                file,
                function,
                function_start_line,
                ..
            } => Some((file, function, *function_start_line)),
            _ => None,
        }
    }

    fn cross_edges_into(&self, target: &str) -> Vec<(NodeIndex, NodeIndex)> {
        let mut edges = Vec::new();
        for from in self.graph.graph.node_indices() {
            let Some(from_owner) = self.owner(from) else {
                continue;
            };
            for (to, _) in self.flow_from(from) {
                let Some(to_owner) = self.owner(to) else {
                    continue;
                };
                if from_owner != to_owner
                    && (to_owner.1 == target || self.named(target).contains(&to))
                {
                    edges.push((from, to));
                }
            }
        }
        edges
    }

    fn trace(&self, assertion: &Assertion, check: &TraceCheck) -> Result<(), String> {
        let seed_file = format!("{}.{}", check.seed.file, self.ext);
        let target_file = format!("{}.{}", check.target.file, self.ext);
        let targets: Vec<_> = self
            .graph
            .nodes_at(&target_file, check.target.line)
            .into_iter()
            .filter(|index| self.named(&check.node).contains(index))
            .collect();
        let trace = self.graph.taint_trace(&[(seed_file, check.seed.line)]);
        let reached = targets.iter().any(|&index| trace.in_frontier(index));
        let (kind, from, from_bytes) = match assertion {
            Assertion::TraceBoundary {
                boundary_kind,
                from,
                from_bytes,
                ..
            } => {
                let kind = match boundary_kind {
                    TraceBoundaryKind::CrossFunction => BoundaryKind::CrossFunction,
                    TraceBoundaryKind::SelfFunctionParam => BoundaryKind::SelfFunctionParam,
                };
                (kind, from.as_deref(), *from_bytes)
            }
            _ => (BoundaryKind::CrossFunction, None, None),
        };
        let boundary = trace.boundary.iter().any(|edge| {
            edge.kind == kind
                && trace.frontier_by_root.contains_key(&edge.root)
                && targets.contains(&edge.to)
                && from.is_none_or(|from| self.named(from).contains(&edge.from))
                && from_bytes.is_none_or(|span| self.bytes(edge.from) == Some(span))
        });
        let matches = match assertion {
            Assertion::TraceReaches(_) => reached,
            Assertion::TraceMisses(_) => !reached,
            Assertion::TraceBoundary { .. } => boundary && !reached,
            _ => unreachable!(),
        };
        if trace.frontier_by_root.is_empty() || targets.is_empty() || !matches {
            return Err(format!(
                "seed roots {}, target nodes {}, reached {reached}, boundary {boundary}, warnings {:?}",
                trace.frontier_by_root.len(),
                targets.len(),
                trace.warnings,
            ));
        }
        Ok(())
    }

    fn check(&self, assertion: &Assertion) -> Result<(), String> {
        match assertion {
            Assertion::Def { node, bytes, count } => {
                let defs = self.named(node);
                let spans: Vec<_> = defs.iter().filter_map(|&index| self.bytes(index)).collect();
                if !node.ends_with("(def)")
                    || defs.is_empty()
                    || count.is_some_and(|count| count != defs.len())
                    || bytes.is_some_and(|span| !spans.contains(&span))
                {
                    return Err(format!("found {} definitions at {spans:?}", defs.len()));
                }
            }
            Assertion::NoDef { node } => {
                if !node.ends_with("(def)") || !self.named(node).is_empty() {
                    return Err(format!("found {} matching nodes", self.named(node).len()));
                }
            }
            Assertion::Edge { between, .. } | Assertion::NoEdge { between } => {
                let edges = self.edges(between);
                let matching = edges
                    .iter()
                    .filter(|(_, _, label)| between.label.is_none_or(|expected| expected == *label))
                    .count();
                let matches = match assertion {
                    Assertion::Edge {
                        count: Some(count), ..
                    } => matching == *count,
                    Assertion::Edge { .. } => matching > 0,
                    Assertion::NoEdge { .. } => matching == 0,
                    _ => unreachable!(),
                };
                if !matches {
                    return Err(format!(
                        "found {matching} matching edges, candidates {edges:?}"
                    ));
                }
            }
            Assertion::NoEdgeInto { to } => {
                let edges = self.cross_edges_into(to);
                if !edges.is_empty() {
                    return Err(format!("found incoming cross-function edges {edges:?}"));
                }
            }
            Assertion::Path { from, to, via } => {
                if !self.path(from, to, via.as_deref()) {
                    return Err("no data-flow path".into());
                }
            }
            Assertion::NoPath { from, to } => {
                if self.path(from, to, None) {
                    return Err("found an unexpected data-flow path".into());
                }
            }
            Assertion::Counter { key, count } => {
                let actual = self.stats["dfg_props"][key].as_u64().unwrap_or(0);
                if actual != *count {
                    return Err(format!("observed {actual}"));
                }
            }
            Assertion::TraceReaches(check)
            | Assertion::TraceMisses(check)
            | Assertion::TraceBoundary { check, .. } => self.trace(assertion, check)?,
        }
        Ok(())
    }
}

impl Row {
    fn run(&self) -> Vec<String> {
        let mut failures = Vec::new();
        let mut cases: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for (name, source) in &self.files {
            let (case, stem) = name.split_once('/').unwrap_or(("", name));
            cases.entry(case).or_default().push((stem, source.as_str()));
        }
        for check in &self.expect {
            if !cases.contains_key(check.case.as_str()) {
                failures.push(format!("unknown fixture case {:?}", check.case));
            }
        }

        let mut used_langs = BTreeSet::new();
        for (case, files) in cases {
            let mut fixed_exts = BTreeSet::new();
            let files: Vec<_> = files
                .into_iter()
                .map(|(name, source)| {
                    if let Some((stem, ext)) = name.rsplit_once('.') {
                        fixed_exts.insert(ext);
                        (stem, source)
                    } else {
                        (name, source)
                    }
                })
                .collect();
            if fixed_exts.len() > 1 {
                failures.push(format!(
                    "{case}: conflicting file extensions {fixed_exts:?}"
                ));
                continue;
            }
            let checks: Vec<_> = self
                .expect
                .iter()
                .filter(|check| check.case == case)
                .collect();
            if checks.is_empty() {
                failures.push(format!("{case:?}: fixture has no assertions"));
            }
            for ext in &self.langs {
                if !fixed_exts.is_empty() && !fixed_exts.contains(ext.as_str()) {
                    continue;
                }
                let Some(language) = Language::from_extension(ext) else {
                    failures.push(format!("{case}: unknown language {ext:?}"));
                    continue;
                };
                used_langs.insert(ext);
                let tag = if case.is_empty() {
                    ext.clone()
                } else {
                    format!("{case}/{ext}")
                };
                // The shared builder panics on parser errors. Continue collecting rows if a
                // fixture is malformed, and distinguish that from an assertion failure.
                let Ok(graph) = std::panic::catch_unwind(|| build(language, &files, ext)) else {
                    failures.push(format!("{tag}: fixture build failed (see panic above)"));
                    continue;
                };
                let fixture = Fixture::new(&graph, ext);
                for check in &checks {
                    if let Err(detail) = fixture.check(&check.assertion) {
                        failures.push(format!("{tag}: {:?}: {detail}", check.assertion));
                    }
                }
            }
            if !fixed_exts.is_empty()
                && !self
                    .langs
                    .iter()
                    .any(|ext| fixed_exts.contains(ext.as_str()))
            {
                failures.push(format!("{case}: fixture extension absent from langs"));
            }
        }
        if used_langs.len() != self.langs.len() || used_langs.is_empty() {
            failures.push(format!("langs {:?}, exercised {used_langs:?}", self.langs));
        }
        failures
    }
}

#[test]
fn props_matrix_trace_boundary_matches_kind_and_source() {
    let source = "def f(p):\n    user = input()\n    f(user)\n    sink(p)\n";
    let graph = build(Language::Python, &[("m", source)], "py");
    let fixture = Fixture::new(&graph, "py");
    let boundary = serde_json::json!({
        "kind": "trace_boundary",
        "seed": {"file": "m", "line": 2},
        "target": {"file": "m", "line": 1},
        "node": "f:p@1(def)",
        "boundary_kind": "self_function_param",
        "from": "f:user@3(use)",
        "from_bytes": [35, 39],
    });
    let run = |value| {
        let check: Check = serde_json::from_value(value).expect("deserialize boundary check");
        fixture.check(&check.assertion)
    };
    run(boundary.clone()).expect("match the recursive call's parameter boundary");

    for (field, wrong_value) in [
        ("boundary_kind", serde_json::json!("cross_function")),
        ("from", serde_json::json!("f:user@2(def)")),
        ("from_bytes", serde_json::json!([34, 38])),
    ] {
        let mut wrong = boundary.clone();
        wrong[field] = wrong_value;
        assert!(run(wrong).is_err(), "unexpectedly matched wrong {field}");
    }
}

#[test]
fn props_matrix_rows() {
    let mut failures = Vec::new();
    let mut ids = BTreeSet::new();
    let mut groups = BTreeSet::new();
    for (line, text) in ROWS.lines().enumerate() {
        let row: Row = serde_json::from_str(text)
            .unwrap_or_else(|err| panic!("rows.jsonl:{}: {err}", line + 1));
        assert!(ids.insert(row.id.clone()), "duplicate row {}", row.id);
        assert!(
            ["control", "S1", "S2", "S3"].contains(&row.group.as_str()),
            "{}: unknown group {:?}",
            row.id,
            row.group
        );
        assert!(!row.class.is_empty() && !row.truth.is_empty(), "{}", row.id);
        if !ENABLED.contains(&row.group.as_str()) {
            continue;
        }
        groups.insert(row.group.clone());
        let issues = row.run();
        let status = if issues.is_empty() { "PASS" } else { "FAIL" };
        println!(
            "PROPS_MATRIX {} {} {status} {:?}",
            row.group, row.id, row.base
        );
        if !issues.is_empty() {
            failures.push(format!(
                "{} {}\n  {}",
                row.group,
                row.id,
                issues.join("\n  ")
            ));
        }
    }
    assert_eq!(
        groups,
        ENABLED.iter().map(|group| group.to_string()).collect()
    );
    assert!(
        failures.is_empty(),
        "{} failing rows\n{}",
        failures.len(),
        failures.join("\n")
    );
}
