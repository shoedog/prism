//! Lane 9 parameter-binding contract, including the unchanged baseline controls.

use super::object_literal_arg_tests::build;
use super::{CodePropertyGraph, CpgEdge, FlowConfidence};
use crate::call_graph::CallSite;
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::languages::Language;
use crate::navigation::queries;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

mod route_pins;
mod supplied_site;
pub(super) use supplied_site::assert_supplied_site_reaches_use;

const ACTIVE_SLICES: &[&str] = &["control", "A", "B", "C"];
type AssertionKey<'a> = (&'a str, &'a str, usize);
type ExpectedRed<'a> = (AssertionKey<'a>, &'a str);

const X2_2_RED_REASON: &str = "DESIGN §9 line 314: both step sites resolve only as r6_multi_owner_candidate, excluded from Step 5b. #893 tracks the candidate-binding decision. Green also needs unfiled resolver work to resolve super.m() to the base method and type a receiver through a method that returns this";
const X5_2_RED_REASON: &str = "DESIGN §9 line 315: the constructor site is absent (#892) and the take targets are navigation-only candidates. #893 tracks the candidate-binding decision. Green also needs unfiled resolver work to type the receiver from the constructor's explicit return";

const CONSTRUCTOR_RED_REASON: &str =
    "The Source new-expression site or its singleton Exact constructor implementation is absent (#892). The supplied-site companion exercises binding only.";

/// Mandatory RED assertions from DESIGN §9 and the B constructor ledger.
/// Every pin must remain RED, and every unpinned RED fails the suite.
const EXPECTED_RED: &[ExpectedRed<'_>] = &[
    (
        ("x1-4", "js", 3),
        "DESIGN §9 line 313: an unimported builtin spelling resolves Exact free_single to an unrelated export; fixed by #891",
    ),
    (("x2-2", "js", 1), X2_2_RED_REASON),
    (("x2-2", "js", 2), X2_2_RED_REASON),
    (("x2-2", "ts", 1), X2_2_RED_REASON),
    (("x2-2", "ts", 2), X2_2_RED_REASON),
    (("x2-2", "tsx", 1), X2_2_RED_REASON),
    (("x2-2", "tsx", 2), X2_2_RED_REASON),
    (("x5-2", "js", 1), X5_2_RED_REASON),
    (("x5-2", "js", 2), X5_2_RED_REASON),
    (("t4-3", "js", 2), CONSTRUCTOR_RED_REASON),
    (("t4-3", "ts", 2), CONSTRUCTOR_RED_REASON),
    (("t4-3", "tsx", 2), CONSTRUCTOR_RED_REASON),
    (("s2-4", "ts", 2), CONSTRUCTOR_RED_REASON),
    (("s2-4", "ts", 3), CONSTRUCTOR_RED_REASON),
    (("s2-4", "tsx", 2), CONSTRUCTOR_RED_REASON),
    (("s2-4", "tsx", 3), CONSTRUCTOR_RED_REASON),
    (("s3-3", "js", 1), CONSTRUCTOR_RED_REASON),
    (("s3-3", "ts", 1), CONSTRUCTOR_RED_REASON),
    (("s3-3", "tsx", 1), CONSTRUCTOR_RED_REASON),
    (("s5-6", "js", 2), CONSTRUCTOR_RED_REASON),
    (("s5-6", "ts", 2), CONSTRUCTOR_RED_REASON),
    (("s5-6", "tsx", 2), CONSTRUCTOR_RED_REASON),
    (("x3-5", "js", 1), CONSTRUCTOR_RED_REASON),
    (("x3-5", "js", 2), CONSTRUCTOR_RED_REASON),
];
/// Doubt kinds as `dfg-stats --edges` spells them; `builtin_callback` is added by slice D.
const LABEL_DOUBTS: &[&str] = &[
    "killed",
    "sameline",
    "cfg_incomplete",
    "alias_unstable",
    "call_nameonly",
    "builtin_callback",
];
const ROWS: &str = include_str!("../../tests/fixtures/param_binding_matrix/rows.jsonl");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    id: String,
    group: String,
    langs: Vec<String>,
    files: BTreeMap<String, String>,
    expect: Vec<Assertion>,
    class: String,
    truth: Truth,
    issue: Option<String>,
    unexecuted: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum Truth {
    Correct,
    Wrong,
    Ambiguous,
}

#[derive(Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum Endpoint {
    Plain(String),
    Spanned { at: String, text: String },
}

#[derive(Deserialize)]
struct Assertion {
    #[serde(default, deserialize_with = "present_string")]
    slice: Option<String>,
    #[serde(default, deserialize_with = "present_string")]
    loss: Option<String>,
    #[serde(flatten)]
    check: Check,
}

fn present_string<'de, D: serde::Deserializer<'de>>(de: D) -> Result<Option<String>, D::Error> {
    String::deserialize(de).map(Some)
}

const SLICES: &[&str] = &["control", "A", "B", "C", "D", "E2"];

impl Row {
    fn default_slice(&self) -> &str {
        if self.group == "E" {
            "E2"
        } else {
            &self.group
        }
    }

    fn assertions(&self) -> impl Iterator<Item = (usize, &str, &Assertion)> {
        self.expect.iter().enumerate().map(|(i, assertion)| {
            (
                i + 1,
                assertion
                    .slice
                    .as_deref()
                    .unwrap_or_else(|| self.default_slice()),
                assertion,
            )
        })
    }

    fn selected<'a>(
        &'a self,
        active: &'a [&str],
    ) -> impl Iterator<Item = (usize, &'a str, &'a Assertion)> + 'a {
        self.assertions()
            .filter(move |&(_, slice, _)| active.contains(&slice))
    }

    fn fixture<'a>(&'a self, ext: &'a str) -> Fixture<'a> {
        let files: Vec<_> = self
            .files
            .iter()
            .map(|(name, source)| (name.rsplit_once('.').unwrap().0, source.as_str()))
            .collect();
        Fixture {
            cpg: build(language(ext).unwrap(), &files, ext),
            files: &self.files,
            ext,
        }
    }
}
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Check {
    Def {
        at: Endpoint,
    },
    Edge {
        from: Endpoint,
        to: Endpoint,
        label: Option<String>,
        multiplicity: Option<usize>,
    },
    NoEdge {
        from: Endpoint,
        to: Option<Endpoint>,
        label: Option<String>,
    },
    NoEdgeInto {
        to: Endpoint,
        label: Option<String>,
    },
    Resolves {
        call: Site,
        targets: Vec<String>,
        confidence: String,
        route: String,
    },
    Drop {
        call: Site,
        reason: String,
    },
    Reaches {
        from: Endpoint,
        to: Endpoint,
    },
    NotReaches {
        from: Endpoint,
        to: Endpoint,
    },
    TraceBoundary {
        root: Endpoint,
        from: Endpoint,
        to: Endpoint,
    },
    Counter {
        key: String,
        value: u64,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Site {
    site: String,
    callee: String,
    end: Option<String>,
    owner: Option<String>,
}

fn byte_at(source: &str, position: &str) -> Result<(usize, usize), String> {
    let coordinate = || {
        let (line, col) = position.split_once(':')?;
        let line: usize = line.parse().ok()?;
        let col: usize = col.parse().ok()?;
        let line_index = line.checked_sub(1)?;
        let col_index = col.checked_sub(1)?;
        let mut lines = source.split_inclusive('\n');
        let start = (&mut lines).take(line_index).map(str::len).sum::<usize>();
        let text = lines.next()?.trim_end_matches('\n');
        let byte = text
            .char_indices()
            .map(|(byte, _)| byte)
            .chain(std::iter::once(text.len()))
            .nth(col_index)?;
        Some((line, start + byte))
    };
    coordinate().ok_or_else(|| format!("invalid source position {position}"))
}

struct Occurrence<'a> {
    file: String,
    owner: &'a str,
    path: &'a str,
    line: usize,
    start: usize,
    end: usize,
}

impl Occurrence<'_> {
    fn matches(&self, loc: &VarLocation) -> bool {
        self.file == loc.file
            && self.owner == loc.function
            && self.path == loc.path.to_string()
            && self.line == loc.line
            && self.start == loc.start_byte
            && self.end == loc.end_byte
    }
}

struct Fixture<'a> {
    cpg: CodePropertyGraph,
    files: &'a BTreeMap<String, String>,
    ext: &'a str,
}

impl Fixture<'_> {
    fn file(&self, name: &str) -> Result<(&str, String), String> {
        let source = self
            .files
            .get(name)
            .ok_or_else(|| format!("unknown fixture file {name}"))?;
        let (stem, _) = name
            .rsplit_once('.')
            .ok_or_else(|| format!("missing file extension in {name}"))?;
        Ok((source, format!("{stem}.{}", self.ext)))
    }

    fn occurrence<'a>(&self, endpoint: &'a Endpoint) -> Result<Occurrence<'a>, String> {
        let at = match endpoint {
            Endpoint::Plain(at) | Endpoint::Spanned { at, .. } => at,
        };
        let parse = || {
            let (owner_path, position) = at.rsplit_once('@')?;
            let (file, owner_path) = owner_path.split_once(':')?;
            let (owner, path) = owner_path.rsplit_once(':')?;
            Some((file, owner, path, position))
        };
        let (name, owner, path, position) =
            parse().ok_or_else(|| format!("invalid endpoint {at}"))?;
        let (source, file) = self.file(name)?;
        let (line, start) = byte_at(source, position)?;
        let text = match endpoint {
            Endpoint::Plain(_) => path,
            Endpoint::Spanned { text, .. } => text,
        };
        if !source[start..].starts_with(text) {
            return Err(format!("{at}: source does not begin with {text:?}"));
        }
        Ok(Occurrence {
            file,
            owner,
            path,
            line,
            start,
            end: start + text.len(),
        })
    }

    fn nodes(&self, endpoint: &Endpoint, access: VarAccessKind) -> Result<Vec<NodeIndex>, String> {
        let occurrence = self.occurrence(endpoint)?;
        Ok(self
            .cpg
            .nodes_at(&occurrence.file, occurrence.line)
            .into_iter()
            .filter(|&index| {
                self.cpg
                    .to_var_location(index)
                    .is_some_and(|loc| occurrence.matches(&loc) && access == loc.kind)
            })
            .collect())
    }

    fn node(&self, endpoint: &Endpoint, access: VarAccessKind) -> Result<NodeIndex, String> {
        let nodes = self.nodes(endpoint, access)?;
        match nodes.as_slice() {
            [node] => Ok(*node),
            _ => Err(format!(
                "expected one {access:?} at {endpoint:?}, found {}",
                nodes.len()
            )),
        }
    }

    fn edge_labels(
        &self,
        from: Option<&Endpoint>,
        to: Option<&Endpoint>,
        crossing_only: bool,
    ) -> Result<Vec<String>, String> {
        let from = from.map(|endpoint| self.occurrence(endpoint)).transpose()?;
        let to = to.map(|endpoint| self.occurrence(endpoint)).transpose()?;
        Ok(self
            .cpg
            .graph
            .edge_references()
            .filter_map(|edge| {
                let CpgEdge::DataFlow(label) = *edge.weight() else {
                    return None;
                };
                let source = self.cpg.to_var_location(edge.source())?;
                let target = self.cpg.to_var_location(edge.target())?;
                let crossing = (&source.file, &source.function, source.function_start_line)
                    != (&target.file, &target.function, target.function_start_line);
                (source.kind == VarAccessKind::Use
                    && (!crossing_only || crossing)
                    && from.as_ref().is_none_or(|occ| occ.matches(&source))
                    && to.as_ref().is_none_or(|occ| occ.matches(&target)))
                .then(|| flow_label(label))
            })
            .collect())
    }

    fn site(&self, site: &Site) -> Result<&CallSite, String> {
        let (name, position) = site
            .site
            .rsplit_once('@')
            .ok_or_else(|| format!("invalid call site {site:?}"))?;
        let (source, file) = self.file(name)?;
        let (line, start) = byte_at(source, position)?;
        let end = site
            .end
            .as_ref()
            .map(|end| byte_at(source, end).map(|(_, byte)| byte))
            .transpose()?;
        let mut sites = self.cpg.call_graph.calls.values().flatten().filter(|s| {
            s.caller.file == file
                && s.line == line
                && s.start_byte == start
                && s.callee_name == site.callee
                && end.is_none_or(|byte| byte == s.end_byte)
                && site
                    .owner
                    .as_ref()
                    .is_none_or(|owner| *owner == s.caller.name)
        });
        match (sites.next(), sites.next()) {
            (Some(found), None) => Ok(found),
            (None, _) => Err(format!("no extracted call at {site:?}")),
            _ => Err(format!("ambiguous call at {site:?}; specify end or owner")),
        }
    }

    fn check(&self, assertion: &Check) -> Result<(bool, String), String> {
        match assertion {
            Check::Def { at } => {
                let count = self.nodes(at, VarAccessKind::Def)?.len();
                Ok((count == 1, format!("Def count {count}")))
            }
            Check::Edge {
                from,
                to,
                label,
                multiplicity,
            } => {
                let labels = self.edge_labels(Some(from), Some(to), false)?;
                let count = labels
                    .iter()
                    .filter(|got| label.as_ref().is_none_or(|want| *got == want))
                    .count();
                let ok = multiplicity.map_or(count > 0, |want| count == want);
                Ok((ok, format!("binding labels {labels:?}")))
            }
            Check::NoEdge { from, to, label } => {
                self.check_absent(Some(from), to.as_ref(), label.as_deref())
            }
            Check::NoEdgeInto { to, label } => self.check_absent(None, Some(to), label.as_deref()),
            Check::Resolves {
                call,
                targets,
                confidence,
                route,
            } => {
                let actual = self.cpg.call_graph.resolve_call_site_full(self.site(call)?);
                let mut got: Vec<_> = actual
                    .resolved
                    .iter()
                    .map(|resolved| {
                        let target = resolved.target;
                        (
                            format!("{}:{}@{}", target.file, target.name, target.start_line),
                            format!("{:?}", resolved.confidence).to_ascii_lowercase(),
                            resolved.kind.as_str(),
                        )
                    })
                    .collect();
                got.sort_unstable();
                let mut want = Vec::new();
                for target in targets {
                    let (name, owner) = target
                        .split_once(':')
                        .ok_or_else(|| format!("invalid target {target}"))?;
                    let (_, file) = self.file(name)?;
                    want.push((
                        format!("{file}:{owner}"),
                        confidence.clone(),
                        route.as_str(),
                    ));
                }
                want.sort_unstable();
                Ok((
                    got == want,
                    format!("targets {got:?}, drop {:?}", actual.drop),
                ))
            }
            Check::Drop { call, reason } => {
                let actual = self.cpg.call_graph.resolve_call_site_full(self.site(call)?);
                let got = actual.drop.map(|drop| format!("{drop:?}"));
                Ok((
                    actual.resolved.is_empty() && got.as_ref() == Some(reason),
                    format!("drop {got:?}, targets {:?}", actual.resolved),
                ))
            }
            Check::Reaches { from, to } | Check::NotReaches { from, to } => {
                let seed_node = self.node(from, VarAccessKind::Def)?;
                let seed = self.cpg.to_var_location(seed_node).unwrap();
                if self.cpg.var_node_for_location(&seed) != Some(seed_node) {
                    return Err(format!(
                        "the forward-reachability seed lookup aliases {from:?}"
                    ));
                }
                let target = self.occurrence(to)?;
                // The public reachability set deduplicates locations on the same line.
                // Read the visited node indices from the same walk to retain byte identity.
                let (_, visited) = self.cpg.dfg_forward_reachable_labeled(&seed);
                let reached = self
                    .cpg
                    .nodes_at(&target.file, target.line)
                    .into_iter()
                    .filter(|node| visited.contains_key(node))
                    .filter_map(|node| self.cpg.to_var_location(node))
                    .any(|loc| target.matches(&loc));
                let want = matches!(assertion, Check::Reaches { .. });
                Ok((reached == want, format!("reachable={reached}")))
            }
            Check::TraceBoundary { root, from, to } => {
                let seed = self.node(root, VarAccessKind::Def)?;
                let seed_loc = self.cpg.to_var_location(seed).unwrap();
                let source = self.node(from, VarAccessKind::Use)?;
                let target = self.node(to, VarAccessKind::Def)?;
                let trace = self.cpg.taint_trace(&[(seed_loc.file, seed_loc.line)]);
                let stopped = trace
                    .boundary
                    .iter()
                    .any(|edge| edge.root == seed && edge.from == source && edge.to == target);
                let crossed = trace
                    .frontier_by_root
                    .get(&seed)
                    .is_some_and(|frontier| frontier.contains(&target));
                Ok((
                    stopped && !crossed,
                    format!("boundary={stopped}, crossed={crossed}"),
                ))
            }
            Check::Counter { key, value } => {
                let stats = if key.starts_with("dfg_labels.") {
                    serde_json::json!({ "dfg_labels": self.cpg.dfg_label_stats })
                } else {
                    let mut stats = queries::call_stats(&self.cpg.call_graph);
                    stats["return_flow"] =
                        serde_json::to_value(&self.cpg.return_flow_stats).unwrap();
                    stats
                };
                let got = key
                    .split('.')
                    .try_fold(&stats, |object, component| object.get(component));
                let got = match got {
                    Some(got) => got
                        .as_u64()
                        .ok_or_else(|| format!("{key}: not an unsigned counter: {got}"))?,
                    None => 0,
                };
                Ok((got == *value, got.to_string()))
            }
        }
    }

    fn check_absent(
        &self,
        from: Option<&Endpoint>,
        to: Option<&Endpoint>,
        label: Option<&str>,
    ) -> Result<(bool, String), String> {
        let labels = self.edge_labels(from, to, from.is_none() || to.is_none())?;
        let absent = !labels
            .iter()
            .any(|got| label.is_none_or(|want| got == want));
        Ok((absent, format!("binding labels {labels:?}")))
    }
}

fn flow_label(label: FlowConfidence) -> String {
    match label {
        FlowConfidence::Exact => "exact".to_owned(),
        FlowConfidence::NameOnly(doubt) => {
            // Use the serialized variant name so future doubts can be fixture expectations
            // as soon as the product emits them.
            let value = serde_json::to_value(doubt).unwrap();
            let name = value
                .as_str()
                .unwrap_or_else(|| value.as_object().unwrap().keys().next().unwrap());
            let mut label = String::from("nameonly/");
            for (index, c) in name.chars().enumerate() {
                if c.is_ascii_uppercase() && index != 0 {
                    label.push('_');
                }
                label.push(c.to_ascii_lowercase());
            }
            // The established spelling (`dfg-stats --edges`, navigation::queries) is one word.
            label
                .replace("nameonly/call_name_only", "nameonly/call_nameonly")
                .replace("nameonly/same_line", "nameonly/sameline")
        }
    }
}

fn language(ext: &str) -> Option<Language> {
    Some(match ext {
        "js" => Language::JavaScript,
        "ts" => Language::TypeScript,
        "tsx" => Language::Tsx,
        "go" => Language::Go,
        "py" => Language::Python,
        _ => return None,
    })
}

fn read_rows(input: &str) -> Result<Vec<Row>, String> {
    let mut ids = BTreeSet::new();
    input
        .lines()
        .enumerate()
        .map(|(index, line)| {
            let row: Row = serde_json::from_str(line)
                .map_err(|error| format!("rows.jsonl:{}: {error}", index + 1))?;
            let invalid = |message: &str| format!("{}: {message}", row.id);
            if row.id.trim().is_empty() || !ids.insert(row.id.clone()) {
                return Err(invalid("empty or duplicate id"));
            }
            if row.langs.is_empty()
                || row.files.is_empty()
                || row.expect.is_empty()
                || row.class.is_empty()
            {
                return Err(invalid("incomplete row"));
            }
            let stems: BTreeSet<_> = row
                .files
                .keys()
                .filter_map(|name| name.rsplit_once('.'))
                .map(|(stem, _)| stem)
                .collect();
            if stems.len() != row.files.len() || stems.contains("") {
                return Err(invalid("invalid or duplicate fixture filename stem"));
            }
            if row.langs.iter().any(|ext| language(ext).is_none())
                || row.langs.iter().collect::<BTreeSet<_>>().len() != row.langs.len()
            {
                return Err(invalid("unknown or duplicate language"));
            }
            if !matches!(row.group.as_str(), "control" | "A" | "B" | "C" | "D" | "E") {
                return Err(invalid("invalid row group"));
            }
            if matches!(row.truth, Truth::Wrong)
                && row
                    .issue
                    .as_ref()
                    .is_none_or(|issue| issue.trim().is_empty())
            {
                return Err(invalid("wrong row needs an issue"));
            }
            if row
                .unexecuted
                .as_ref()
                .is_some_and(|reason| reason.trim().is_empty())
            {
                return Err(invalid("empty unexecuted reason"));
            }
            for (ordinal, slice, assertion) in row.assertions() {
                if !SLICES.contains(&slice)
                    || assertion
                        .loss
                        .as_ref()
                        .is_some_and(|reason| reason.trim().is_empty())
                {
                    return Err(invalid(&format!("#{ordinal}: invalid slice or loss")));
                }
                let label = match &assertion.check {
                    Check::Edge { label, .. }
                    | Check::NoEdge { label, .. }
                    | Check::NoEdgeInto { label, .. } => label.as_deref(),
                    _ => None,
                };
                if let Some(label) = label {
                    if label != "exact"
                        && !label
                            .strip_prefix("nameonly/")
                            .is_some_and(|doubt| LABEL_DOUBTS.contains(&doubt))
                    {
                        return Err(invalid(&format!("#{ordinal}: unknown label {label}")));
                    }
                }
            }
            Ok(row)
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Pass,
    Red,
    Loss,
    UnexpectedPass,
    Error,
    Skipped,
}

impl Outcome {
    fn fails(self) -> bool {
        matches!(self, Self::Red | Self::UnexpectedPass | Self::Error)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Red => "RED",
            Self::Loss => "LOSS",
            Self::UnexpectedPass => "unexpected pass",
            Self::Error => "error",
            Self::Skipped => "skipped",
        }
    }
}

fn catching<T>(
    run: impl FnOnce() -> Result<T, String> + std::panic::UnwindSafe,
) -> Result<T, String> {
    std::panic::catch_unwind(run).unwrap_or_else(|panic| {
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or("unknown panic");
        Err(format!("panic: {message}"))
    })
}

fn evaluate(
    row: &Row,
    active: &[&str],
    mut emit: impl FnMut(&str, usize, &str, &Assertion, Outcome, &str),
) {
    for ext in &row.langs {
        let mut fixture = None;
        for (ordinal, slice, assertion) in row.selected(active) {
            let (outcome, observation) = if let Some(reason) = &row.unexecuted {
                (Outcome::Skipped, reason.clone())
            } else {
                let fixture = fixture.get_or_insert_with(|| catching(|| Ok(row.fixture(ext))));
                match fixture
                    .as_ref()
                    .map_err(Clone::clone)
                    .and_then(|fixture| catching(|| fixture.check(&assertion.check)))
                {
                    Ok((true, actual)) if assertion.loss.is_some() => {
                        (Outcome::UnexpectedPass, actual)
                    }
                    Ok((true, actual)) => (Outcome::Pass, actual),
                    Ok((false, actual)) if assertion.loss.is_some() => (Outcome::Loss, actual),
                    Ok((false, actual)) => (Outcome::Red, actual),
                    Err(error) => (Outcome::Error, error),
                }
            };
            emit(ext, ordinal, slice, assertion, outcome, &observation);
        }
    }
}

fn matrix_failures(rows: &[Row], active: &[&str], expected_red: &[ExpectedRed<'_>]) -> Vec<String> {
    let mut failures = Vec::new();
    let mut pending = expected_red.to_vec();
    for row in rows {
        evaluate(
            row,
            active,
            |ext, ordinal, slice, assertion, outcome, actual| {
                let actual = actual.replace(['\t', '\n', '\r'], " ");
                let loss = assertion
                    .loss
                    .as_deref()
                    .unwrap_or("")
                    .replace(['\t', '\n', '\r'], " ");
                let record = format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t{}",
                    row.id,
                    row.group,
                    outcome.label(),
                    ext,
                    ordinal,
                    slice,
                    assertion.check,
                    actual,
                    loss
                );
                println!("MATRIX\t{record}");
                if outcome == Outcome::Red {
                    let key = (row.id.as_str(), ext, ordinal);
                    if let Some(index) = pending.iter().position(|(pin, _)| *pin == key) {
                        pending.remove(index);
                        println!("MATRIX-EXPECTED-RED\t{}", row.id);
                        return;
                    }
                }
                if outcome.fails() {
                    failures.push(record);
                }
            },
        );
    }
    for ((id, ext, ordinal), why) in pending {
        failures.push(format!(
            "{id}/{ext} #{ordinal}: expected RED ({why}); update EXPECTED_RED"
        ));
    }
    failures
}

#[test]
fn param_binding_matrix_rows() {
    let rows = read_rows(ROWS).unwrap();
    let failures = matrix_failures(&rows, ACTIVE_SLICES, EXPECTED_RED);
    let seen: BTreeSet<_> = rows
        .iter()
        .flat_map(|row| row.selected(ACTIVE_SLICES).map(|(_, slice, _)| slice))
        .collect();
    assert!(
        ACTIVE_SLICES.iter().all(|slice| seen.contains(slice)),
        "empty active slice"
    );
    assert!(
        failures.is_empty(),
        "matrix failures:\n{}",
        failures.join("\n")
    );
}

mod harness {
    use super::*;
    use serde_json::json;

    fn row(checks: serde_json::Value) -> serde_json::Value {
        json!({
            "id": "synthetic", "group": "E", "langs": ["js"], "class": "test",
            "truth": "correct", "files": {"m.js": "function f(a) { use(a); }\nfunction g(v) { f(v); }\n"},
            "expect": checks
        })
    }

    fn outcomes(row: &serde_json::Value, active: &[&str]) -> Vec<(usize, Outcome)> {
        let rows = read_rows(&row.to_string()).unwrap();
        let mut actual = Vec::new();
        evaluate(&rows[0], active, |_, n, _, _, outcome, _| {
            actual.push((n, outcome))
        });
        actual
    }

    fn synthetic_matrix_failures(kinds: &[&str], langs: &[&str]) -> Vec<String> {
        let checks: Vec<_> = kinds
            .iter()
            .map(|kind| {
                json!({
                    "kind": kind, "from": "m.js:g:v@2:19", "to": "m.js:f:a@1:12", "slice": "A"
                })
            })
            .collect();
        let mut fixture = row(json!(checks));
        fixture["langs"] = json!(langs);
        let rows = read_rows(&fixture.to_string()).unwrap();
        matrix_failures(
            &rows,
            &["A"],
            &[
                (("synthetic", "js", 1), "known RED"),
                (("synthetic", "js", 2), "known RED"),
            ],
        )
    }

    #[test]
    fn expected_red_passes() {
        let failures = synthetic_matrix_failures(&["no_edge", "no_edge", "edge"], &["js"]);
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn expected_red_turning_green_fails_with_a_sibling_still_red() {
        let failures = synthetic_matrix_failures(&["no_edge", "edge"], &["js"]);
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(failures[0].contains("synthetic/js #2"));
        assert!(failures[0].contains("update EXPECTED_RED"));
    }

    #[test]
    fn unexpected_red_siblings_fail_in_the_same_row() {
        let ordinal = synthetic_matrix_failures(&["no_edge", "no_edge", "no_edge"], &["js"]);
        let language = synthetic_matrix_failures(&["no_edge", "no_edge"], &["js", "ts"]);
        assert_eq!([ordinal.len(), language.len()], [1, 2]);
        assert!(ordinal[0].starts_with("synthetic\tE\tRED\tjs\t3\t"));
        for (index, failure) in language.iter().enumerate() {
            assert!(failure.starts_with(&format!("synthetic\tE\tRED\tts\t{}\t", index + 1)));
        }
    }

    #[test]
    fn outcomes_keep_running_siblings() {
        use Outcome::*;
        let row = row(json!([
            {"kind":"no_edge", "from":"m.js:g:v@2:19", "to":"m.js:f:a@1:12", "slice":"D", "loss":"known excess"},
            {"kind":"edge", "from":"m.js:g:v@2:19", "to":"m.js:f:a@1:12", "slice":"D", "loss":"obsolete"},
            {"kind":"edge", "from":"m.js:g:v@2:19", "to":"m.js:f:a@1:12", "slice":"D", "label":"nameonly/builtin_callback"},
            {"kind":"edge", "from":"m.js:g:v@2:1", "to":"m.js:f:a@1:12", "slice":"D", "loss":"invalid source"},
            {"kind":"resolves", "call":{"site":"m.js@2:17","callee":"missing"}, "targets":[], "confidence":"exact","route":"local_def","slice":"D","loss":"missing site"},
            {"kind":"edge", "from":"m.js:g:v@2:19", "to":"m.js:f:a@1:12", "slice":"D", "label":"exact"},
            {"kind":"no_edge", "from":"m.js:g:v@2:19", "to":"m.js:f:a@1:12", "label":"nameonly/builtin_callback"}
        ]));
        let d = [
            (1, Loss),
            (2, UnexpectedPass),
            (3, Red),
            (4, Error),
            (5, Error),
            (6, Pass),
        ];
        assert_eq!(outcomes(&row, &["D"]), d);
        assert_eq!(
            outcomes(&row, &["D", "E2"]),
            [d.as_slice(), &[(7, Pass)]].concat()
        );
        for outcome in [UnexpectedPass, Red, Error] {
            assert!(outcome.fails());
        }
        for outcome in [Pass, Loss, Skipped] {
            assert!(!outcome.fails());
        }
        let mut broken = row.clone();
        broken["files"]["m.js"] = json!("function {");
        assert_eq!(
            outcomes(&broken, &["D"]),
            (1..=6).map(|n| (n, Error)).collect::<Vec<_>>()
        );
        broken["unexecuted"] = json!("historical boundary");
        assert_eq!(outcomes(&broken, &["E2"]), [(7, Skipped)]);
    }

    #[test]
    fn validates_inactive_metadata() {
        for (field, value) in [
            ("slice", json!("E")),
            ("slice", json!(null)),
            ("loss", json!(" ")),
            ("loss", json!(null)),
            ("label", json!("nameonly/call_name_only")),
            ("unexpected", json!(true)),
        ] {
            let mut row = row(json!([{"kind":"no_edge_into","to":"m.js:f:a@1:12"}]));
            row["expect"][0][field] = value;
            row["unexecuted"] = json!("even skipped rows must validate");
            assert!(read_rows(&row.to_string()).is_err(), "{field}");
        }
        let mut row = row(json!([{"kind":"def","at":"m.js:f:a@1:12"}]));
        assert!(read_rows(&format!("{row}\n{row}")).is_err());
        row["group"] = json!("E2");
        assert!(read_rows(&row.to_string()).is_err());
    }

    #[test]
    fn staged_ordinals_and_integrated_coverage() {
        let rows = read_rows(ROWS).unwrap();
        assert_eq!(rows.len(), 425);
        let ordinals = |id, slices: &[&str]| {
            rows.iter()
                .find(|row| row.id == id)
                .unwrap()
                .selected(slices)
                .map(|(n, _, _)| n)
                .collect::<Vec<_>>()
        };
        for (id, expected) in [("k06a", 2), ("k07a", 1), ("k07b", 1)] {
            assert_eq!(ordinals(id, &["B"]), [expected], "{id}");
            assert_eq!(ordinals(id, &["B", "C"]), [1, 2], "{id}");
        }
        for (id, d, both) in [
            ("s3-14", &[1, 3, 5][..], 5),
            ("s4-12", &[1][..], 4),
            ("s4-13", &[3, 4][..], 4),
            ("x2-11", &[1, 2, 3][..], 7),
            ("x4-9", &[1, 2][..], 6),
            ("s2-15", &[1, 2, 3][..], 5),
        ] {
            assert_eq!(ordinals(id, &["D"]), d, "{id}");
            assert_eq!(
                ordinals(id, &["D", "E2"]),
                (1..=both).collect::<Vec<_>>(),
                "{id}"
            );
        }
        for row in &rows {
            let expected = (1..=row.expect.len()).collect::<Vec<_>>();
            assert_eq!(ordinals(row.id.as_str(), SLICES), expected, "{}", row.id);
        }
    }

    #[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
    #[serde(deny_unknown_fields)]
    struct Record {
        id: String,
        ordinal: usize,
        slice: String,
        kind: String,
        reason: String,
    }

    #[test]
    fn manifest_reconciles() {
        let manifest = include_str!("../../tests/fixtures/param_binding_matrix/manifest.jsonl");
        let rows = read_rows(ROWS).unwrap();
        let by_id: BTreeMap<_, _> = rows.iter().map(|row| (row.id.as_str(), row)).collect();
        let mut records = BTreeSet::new();
        let mut metadata = BTreeSet::new();
        let mut ledger_rows = BTreeSet::new();
        for line in manifest.lines() {
            let r: Record = serde_json::from_str(line).unwrap();
            let row = by_id[r.id.as_str()];
            let (_, slice, assertion) = row.assertions().find(|(n, _, _)| *n == r.ordinal).unwrap();
            assert_eq!(r.slice, slice, "{r:?}");
            assert!(!r.reason.trim().is_empty());
            assert!(
                records.insert((r.id.clone(), r.ordinal, r.kind.clone())),
                "{r:?}"
            );
            match r.kind.as_str() {
                "slice" | "loss" => assert!(metadata.insert(r)),
                "loss-row" | "set-row" => assert!(ledger_rows.insert((r.id, r.kind))),
                "skip" => assert_eq!(row.unexecuted.as_deref(), Some(r.reason.as_str())),
                "blocked" | "sentinel" => {
                    assert!(assertion.loss.is_none() && row.unexecuted.is_none())
                }
                "corrected" => (),
                _ => panic!("unknown manifest record {r:?}"),
            }
        }
        let mut actual = BTreeSet::new();
        for row in &rows {
            for (ordinal, slice, assertion) in row.assertions() {
                let mut record = |kind: &str, reason: String| {
                    actual.insert(Record {
                        id: row.id.clone(),
                        ordinal,
                        slice: slice.to_owned(),
                        kind: kind.to_owned(),
                        reason,
                    });
                };
                if let Some(reason) = &assertion.loss {
                    record("loss", reason.clone());
                }
                if assertion.slice.is_some() {
                    record("slice", format!("Explicit slice {slice}"));
                }
                if row.unexecuted.is_some() {
                    assert!(records.contains(&(row.id.clone(), ordinal, "skip".to_owned())));
                }
            }
        }
        assert_eq!(actual, metadata);
        assert_eq!(
            ledger_rows
                .iter()
                .filter(|(_, kind)| kind == "loss-row")
                .count(),
            136
        );
        assert_eq!(rows.iter().filter(|r| r.unexecuted.is_some()).count(), 20);
        for id in ["x1-4", "x2-2", "x5-2"] {
            for (n, slice, a) in by_id[id].assertions() {
                assert_eq!(slice, "A");
                assert!(a.loss.is_none() && by_id[id].unexecuted.is_none());
                assert!(records.contains(&(id.to_owned(), n, "sentinel".to_owned())));
            }
        }
    }
}

#[path = "../../tests/support/param_binding_matrix_prerequisites.rs"]
mod prerequisites;
