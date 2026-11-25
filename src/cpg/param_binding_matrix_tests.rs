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

const ENABLED: &[&str] = &["control"];
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

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Assertion {
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

    fn node(&self, endpoint: &Endpoint, access: VarAccessKind) -> Result<NodeIndex, String> {
        let occurrence = self.occurrence(endpoint)?;
        let nodes: Vec<_> = self
            .cpg
            .nodes_at(&occurrence.file, occurrence.line)
            .into_iter()
            .filter(|&index| {
                self.cpg
                    .to_var_location(index)
                    .is_some_and(|loc| occurrence.matches(&loc) && access == loc.kind)
            })
            .collect();
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

    fn check(&self, assertion: &Assertion) -> Result<(), String> {
        match assertion {
            Assertion::Def { at } => self.node(at, VarAccessKind::Def).map(|_| ()),
            Assertion::Edge {
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
                require(ok, || {
                    format!("expected binding edge {from:?} -> {to:?} ({label:?}, multiplicity {multiplicity:?}); found {labels:?}")
                })
            }
            Assertion::NoEdge { from, to, label } => {
                self.check_absent(Some(from), to.as_ref(), label.as_deref())
            }
            Assertion::NoEdgeInto { to, label } => {
                self.check_absent(None, Some(to), label.as_deref())
            }
            Assertion::Resolves {
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
                require(got == want, || {
                    format!(
                        "{call:?}: expected {want:?}, got {got:?} (drop {:?})",
                        actual.drop
                    )
                })
            }
            Assertion::Drop { call, reason } => {
                let actual = self.cpg.call_graph.resolve_call_site_full(self.site(call)?);
                let got = actual.drop.map(|drop| format!("{drop:?}"));
                require(
                    actual.resolved.is_empty() && got.as_ref() == Some(reason),
                    || {
                        format!(
                            "{call:?}: expected drop {reason}, got {got:?} with {} targets",
                            actual.resolved.len()
                        )
                    },
                )
            }
            Assertion::Reaches { from, to } | Assertion::NotReaches { from, to } => {
                let seed_node = self.node(from, VarAccessKind::Def)?;
                let seed = self.cpg.to_var_location(seed_node).unwrap();
                require(
                    self.cpg.var_node_for_location(&seed) == Some(seed_node),
                    || format!("the forward-reachability seed lookup aliases {from:?}"),
                )?;
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
                let want = matches!(assertion, Assertion::Reaches { .. });
                require(reached == want, || {
                    format!("{from:?} -> {to:?}: expected reachable={want}, got {reached}")
                })
            }
            Assertion::TraceBoundary { root, from, to } => {
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
                require(stopped && !crossed, || {
                    format!("trace from {root:?} at {from:?} -> {to:?}: boundary={stopped}, crossed={crossed}")
                })
            }
            Assertion::Counter { key, value } => {
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
                require(got == *value, || {
                    format!("{key}: expected {value}, got {got}")
                })
            }
        }
    }

    fn check_absent(
        &self,
        from: Option<&Endpoint>,
        to: Option<&Endpoint>,
        label: Option<&str>,
    ) -> Result<(), String> {
        let labels = self.edge_labels(from, to, from.is_none() || to.is_none())?;
        require(
            !labels
                .iter()
                .any(|got| label.is_none_or(|want| got == want)),
            || format!("expected no binding edge {from:?} -> {to:?} ({label:?}); found {labels:?}"),
        )
    }
}

fn require(ok: bool, message: impl FnOnce() -> String) -> Result<(), String> {
    if ok {
        Ok(())
    } else {
        Err(message())
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
            label
        }
    }
}

fn evaluate(row: &Row) -> Vec<String> {
    let mut failures = Vec::new();
    for ext in &row.langs {
        let language = match ext.as_str() {
            "js" => Language::JavaScript,
            "ts" => Language::TypeScript,
            "tsx" => Language::Tsx,
            "go" => Language::Go,
            "py" => Language::Python,
            _ => panic!("{}: unknown language {ext}", row.id),
        };
        let files: Vec<_> = row
            .files
            .iter()
            .map(|(name, source)| {
                let (stem, _) = name.rsplit_once('.').expect("fixture filename extension");
                (stem, source.as_str())
            })
            .collect();
        let fixture = Fixture {
            cpg: build(language, &files, ext),
            files: &row.files,
            ext,
        };
        for (index, assertion) in row.expect.iter().enumerate() {
            if let Err(error) = fixture.check(assertion) {
                failures.push(format!("[{ext}] assertion {}: {error}", index + 1));
            }
        }
    }
    failures
}

#[test]
fn param_binding_matrix_rows() {
    let mut ids = BTreeSet::new();
    let mut groups = BTreeSet::new();
    let mut failures = Vec::new();
    for (index, line) in ROWS.lines().enumerate() {
        let row: Row = serde_json::from_str(line)
            .unwrap_or_else(|error| panic!("rows.jsonl:{}: {error}", index + 1));
        assert!(ids.insert(row.id.clone()), "duplicate row {}", row.id);
        assert!(
            !row.langs.is_empty()
                && !row.files.is_empty()
                && !row.expect.is_empty()
                && !row.class.is_empty(),
            "incomplete row {}",
            row.id
        );
        assert!(
            matches!(row.group.as_str(), "control" | "A" | "B" | "C" | "D" | "E"),
            "{}: invalid group {}",
            row.id,
            row.group
        );
        if matches!(row.truth, Truth::Wrong) {
            assert!(
                row.issue.as_ref().is_some_and(|issue| !issue.is_empty()),
                "{}: wrong row needs an issue",
                row.id
            );
        }
        if !ENABLED.contains(&row.group.as_str()) {
            continue;
        }
        groups.insert(row.group.clone());
        if let Some(reason) = &row.unexecuted {
            println!("MATRIX\t{}\t{}\tunexecuted\t{reason}", row.id, row.group);
            continue;
        }
        let result = std::panic::catch_unwind(|| evaluate(&row));
        match result {
            Ok(errors) if errors.is_empty() => {
                println!("MATRIX\t{}\t{}\talready correct", row.id, row.group);
            }
            Ok(errors) => {
                println!("MATRIX\t{}\t{}\tRED\t{}", row.id, row.group, errors[0]);
                failures.push(format!("{}\n  {}", row.id, errors.join("\n  ")));
            }
            Err(panic) => {
                let message = panic
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| panic.downcast_ref::<&str>().copied())
                    .unwrap_or("unknown panic");
                println!(
                    "MATRIX\t{}\t{}\terror\t{}",
                    row.id,
                    row.group,
                    message.replace('\n', " ")
                );
                failures.push(format!("{}: fixture failed: {message}", row.id));
            }
        }
    }
    assert!(
        ENABLED.iter().all(|group| groups.contains(*group)),
        "empty enabled matrix group"
    );
    assert!(
        failures.is_empty(),
        "matrix failures:\n{}",
        failures.join("\n")
    );
}
