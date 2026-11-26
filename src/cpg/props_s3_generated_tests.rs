//! Compare generated JSX cases against TypeScript syntax, symbol and write expectations.

use super::props_generated_tests::{
    endpoint, exact_node, parse, span, Endpoint, Label, Owner, Span, CASES,
};
use super::props_s2_test_support::{capture, PreBinding};
use super::props_s3_replay_tests::{self, indexed, matching, Replay};
use super::{CodePropertyGraph, CpgEdge, FlowConfidence};
use crate::ast::{JsBindingCache, ParsedFile};
use crate::resolution::ResolutionConfidence;
use petgraph::visit::EdgeRef;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
struct Cell {
    id: String,
    family: String,
    grammar: String,
    files: BTreeMap<String, String>,
    excluded: Option<String>,
    required_evidence: Option<Evidence>,
    expected_outcome: Option<Outcome>,
}

#[derive(Deserialize)]
struct Site {
    bytes: Span,
    expression: Span,
    parent: Span,
    parent_kind: String,
    registered: bool,
}

#[derive(Deserialize)]
struct Operand {
    bytes: Span,
    binding: Span,
    #[serde(rename = "use")]
    source: Option<Endpoint>,
    refused_lookup: Option<RefusedLookup>,
}

#[derive(Deserialize)]
struct RefusedLookup {
    binding: Span,
    disconnected: bool,
}

#[derive(Deserialize)]
struct Evidence {
    site: Site,
    caller: Option<Owner>,
    target: Owner,
    formal: Span,
    defs: Vec<Endpoint>,
    operands: Vec<Operand>,
    replay: Option<Replay>,
    multiline_lookup: Option<MultilineLookup>,
    local_links: Vec<LocalLink>,
}

#[derive(Deserialize)]
struct MultilineLookup {
    lines: Vec<usize>,
    base: Endpoint,
    full_key: Endpoint,
    full: Option<Endpoint>,
    full_disconnected: bool,
}

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Deserialize)]
struct Edge {
    from: Endpoint,
    to: Endpoint,
    label: Label,
}

#[derive(Debug, Deserialize)]
struct Path {
    from: Endpoint,
    via: Endpoint,
    to: Endpoint,
    body: Endpoint,
}

#[derive(Debug, Deserialize)]
struct LocalLink {
    #[serde(flatten)]
    path: Path,
    source_labels: Vec<Label>,
}

#[derive(Debug, Deserialize)]
struct Outcome {
    counters: BTreeMap<String, usize>,
    edges: Vec<Edge>,
    paths: Vec<Path>,
}

fn observe_evidence(
    file: &ParsedFile,
    graph: &CodePropertyGraph,
    pre: &PreBinding,
    want: &Evidence,
) {
    let opening = exact_node(file, want.site.bytes);
    assert!(matches!(
        opening.kind(),
        "jsx_opening_element" | "jsx_self_closing_element"
    ));
    let expression = exact_node(file, want.site.expression);
    let parent = expression.parent().expect("TS template's parent slot");
    // Different grammars expose intermediary nodes such as JS arguments and
    // parentheses. The containing syntax must still reach the exact TS span.
    assert!(
        std::iter::successors(Some(parent), |node| node.parent())
            .any(|node| span(node) == want.site.parent)
            || want.site.parent == want.site.expression
            || (want.site.parent_kind == "TemplateSpan"
                && parent.kind() == "template_substitution")
            || (want.site.parent_kind == "PropertyDeclaration"
                && matches!(
                    parent.kind(),
                    "public_field_definition" | "field_definition"
                )
                && parent.start_byte() == want.site.parent[0]
                && parent.end_byte() + 1 == want.site.parent[1]),
        "parent {} {:?} around {:?}",
        want.site.parent_kind,
        want.site.parent,
        span(parent)
    );
    let target = exact_node(file, want.target.bytes);
    assert!(
        file.all_functions().contains(&target),
        "selected target callable"
    );
    assert_eq!(exact_node(file, want.formal).kind(), "object_pattern");
    for def in &want.defs {
        assert!(exact_node(file, def.bytes).start_byte() >= want.formal[0]);
    }
    let sites: Vec<_> = graph
        .call_graph
        .calls
        .values()
        .flatten()
        .filter(|site| {
            site.caller.file == file.path
                && site.jsx_element
                && [site.start_byte, site.end_byte] == want.site.bytes
        })
        .collect();
    if want.site.registered {
        let caller = want.caller.as_ref().expect("attributed caller");
        let attributed: Vec<_> = sites
            .iter()
            .filter(|site| site.caller.name == caller.name && site.caller.start_line == caller.line)
            .collect();
        assert_eq!(
            attributed.len(),
            1,
            "executing opening at {:?}: {sites:?}",
            want.site.bytes
        );
        let site = attributed[0];
        assert_eq!(
            (&site.caller.name, site.caller.start_line),
            (&caller.name, caller.line)
        );
        let callable = exact_node(file, caller.bytes);
        assert!(file.all_functions().contains(&callable));
        let targets = graph.call_graph.resolve_call_site(site);
        assert_eq!(targets.len(), 1, "{targets:?}");
        assert_eq!(
            targets[0].confidence,
            ResolutionConfidence::Exact,
            "{targets:?}"
        );
        let selected = targets[0].target;
        assert_eq!(
            (&selected.name, selected.start_line),
            (&want.target.name, want.target.line)
        );
        assert_eq!(selected.file, file.path);
    } else {
        assert!(want.caller.is_none(), "no callable for module site");
        assert_eq!(sites.len(), 1, "one synthetic module site");
        assert_eq!(sites[0].caller.name, "<anon@module>");
    }

    let mut cache = JsBindingCache::for_sites();
    for operand in &want.operands {
        let value = exact_node(file, operand.bytes);
        let binding = exact_node(file, operand.binding);
        assert!(matches!(
            binding.kind(),
            "identifier" | "shorthand_property_identifier_pattern"
        ));
        assert!(opening.byte_range().contains(&value.start_byte()));
        if want.replay.is_none() && value.kind() == "identifier" {
            let name = file.node_text(&value);
            assert_eq!(name, file.node_text(&binding));
            if let Some((_, declaration)) = file.js_ts_site_declaration(value, name, &mut cache) {
                assert!(
                    declaration.byte_range().contains(&binding.start_byte()),
                    "value at {:?} binds to {:?}, expected {:?}",
                    span(value),
                    span(declaration),
                    span(binding)
                );
            }
        }
        if let Some(source) = &operand.source {
            if source.bytes[0] == source.bytes[1] {
                let line_start: usize = file
                    .source
                    .split_inclusive('\n')
                    .take(source.line - 1)
                    .map(str::len)
                    .sum();
                assert_eq!(source.bytes[0], line_start, "marked line-start anchor");
            } else {
                let representative = exact_node(file, source.bytes);
                assert_eq!(file.node_text(&representative), source.name);
            }
        }
        if let Some(witness) = &operand.refused_lookup {
            let source = operand.source.as_ref().expect("marked refused Use");
            let idx = indexed(pre, &pre.live_lookup, source, source.line)
                .expect("refused Use in masked lookup");
            assert_eq!(endpoint(&pre.graph[idx]).as_ref(), Some(source));
            let read = exact_node(file, source.bytes);
            let marked_binding = exact_node(file, witness.binding);
            let (_, declaration) = file
                .js_ts_site_declaration(read, &source.name, &mut cache)
                .expect("refused representative has a lexical binding");
            assert!(
                declaration.start_byte() <= marked_binding.start_byte()
                    && marked_binding.end_byte() <= declaration.end_byte()
            );
            assert_eq!(witness.binding == operand.binding, witness.disconnected);
            if witness.disconnected {
                assert!(
                    !pre.graph
                        .edges_directed(idx, petgraph::Incoming)
                        .any(|edge| edge.weight().is_data_flow()),
                    "refused candidate must have no contributing Def: {source:?}"
                );
            }
        }
    }
    if let Some(lookup) = &want.multiline_lookup {
        let base = matching(pre, &lookup.base).expect("root-line base");
        assert_eq!(
            indexed(pre, &pre.live_lookup, &lookup.base, lookup.base.line),
            Some(base)
        );
        let full = lookup
            .full
            .as_ref()
            .map(|full| matching(pre, full).expect("marked full candidate"));
        if lookup.full_disconnected {
            let full = full.expect("disconnected full candidate");
            assert!(
                !pre.graph
                    .edges_directed(full, petgraph::Incoming)
                    .any(|edge| edge.weight().is_data_flow()),
                "full candidate must have no contributing Def"
            );
        }
        for &line in &lookup.lines {
            let expected = full.filter(|_| lookup.full.as_ref().unwrap().line == line);
            assert_eq!(
                indexed(pre, &pre.live_lookup, &lookup.full_key, line),
                expected,
                "full tier line {line}"
            );
        }
    }
    if let Some(replay) = &want.replay {
        props_s3_replay_tests::prove_inputs(file, pre, replay);
    }
}

fn observe_paths(pre: &PreBinding, required: &[LocalLink]) {
    for link in required {
        let path = &link.path;
        let from = matching(pre, &path.from)
            .unwrap_or_else(|| panic!("missing contributing Def {:?}", path.from));
        let via =
            matching(pre, &path.via).unwrap_or_else(|| panic!("missing source Use {:?}", path.via));
        let to =
            matching(pre, &path.to).unwrap_or_else(|| panic!("missing target Def {:?}", path.to));
        let body = matching(pre, &path.body)
            .unwrap_or_else(|| panic!("missing target read {:?}", path.body));
        assert_eq!(
            indexed(pre, &pre.live_lookup, &path.via, path.via.line),
            Some(via)
        );
        let incoming: Vec<_> = pre
            .graph
            .edges_directed(via, petgraph::Incoming)
            .filter_map(|edge| match edge.weight() {
                CpgEdge::DataFlow(label) => Some((edge.source(), Label::from(*label))),
                _ => None,
            })
            .collect();
        assert!(
            !incoming.is_empty()
                && incoming
                    .iter()
                    .all(|(id, label)| { *id == from && link.source_labels.contains(label) }),
            "contributing Def/label {from:?} {:?}, observed {incoming:?} for {path:?}",
            link.source_labels
        );
        assert!(
            pre.graph
                .edges_connecting(to, body)
                .any(|edge| edge.weight() == &CpgEdge::DataFlow(FlowConfidence::Exact)),
            "target Def does not reach the marked body read: {path:?}"
        );
    }
}

fn compare(
    graph: &CodePropertyGraph,
    pre: &PreBinding,
    want: &Evidence,
    outcome: &Outcome,
) -> Vec<String> {
    let nodes: BTreeMap<_, _> = graph
        .graph
        .node_indices()
        .filter_map(|index| Some((index, endpoint(graph.node(index))?)))
        .collect();
    let mut actual: Vec<_> =
        graph
            .graph
            .edge_references()
            .filter_map(|edge| {
                let CpgEdge::DataFlow(label) = *edge.weight() else {
                    return None;
                };
                let from = nodes.get(&edge.source())?;
                let to = nodes.get(&edge.target())?;
                (want.caller.as_ref().is_some_and(|caller| {
                    from.owner == caller.name && from.owner_line == caller.line
                }) && to.owner == want.target.name
                    && to.owner_line == want.target.line)
                    .then(|| Edge {
                        from: from.clone(),
                        to: to.clone(),
                        label: label.into(),
                    })
            })
            .collect();
    actual.sort();
    let mut expected = outcome.edges.clone();
    expected.sort();
    let mut errors = Vec::new();
    if expected != actual {
        errors.push(format!("edges expected {expected:?}, observed {actual:?}"));
    }
    let stats: BTreeMap<_, _> = graph
        .dfg_label_stats
        .dfg_props
        .iter()
        .filter(|(_, count)| **count > 0)
        .map(|(key, count)| (key.clone(), *count))
        .collect();
    if stats != outcome.counters {
        errors.push(format!(
            "counters expected {:?}, observed {stats:?}",
            outcome.counters
        ));
    }
    let source_defs: BTreeSet<_> = outcome.paths.iter().map(|path| &path.from).collect();
    for source in source_defs {
        let from = matching(pre, source).unwrap();
        for target in &want.defs {
            let Some(to) = matching(pre, target) else {
                continue;
            };
            let expected = outcome
                .paths
                .iter()
                .any(|path| &path.from == source && &path.to == target);
            if graph.has_path(from, to, &CpgEdge::is_data_flow) != expected {
                errors.push(format!(
                    "path {source:?} -> {target:?}: expected {expected}"
                ));
            }
        }
    }
    for path in &outcome.paths {
        let from = matching(pre, &path.from).unwrap();
        let to = matching(pre, &path.body).unwrap();
        if !graph.has_path(from, to, &CpgEdge::is_data_flow) {
            errors.push(format!("missing body path {path:?}"));
        }
    }
    errors
}

#[derive(Default, Debug)]
struct Receipt {
    planned: usize,
    excluded: usize,
    emitted: usize,
    parsed: usize,
    precondition_passing: usize,
    compared: usize,
    outcomes: BTreeMap<&'static str, usize>,
}

#[test]
fn props_generated_s3_jsx_cells() {
    let mut receipts = BTreeMap::<String, Receipt>::new();
    let mut ids = BTreeSet::new();
    let mut failures = 0;
    for row in CASES
        .lines()
        .filter(|row| row.starts_with(r#"{"id":"jsx-"#))
    {
        let cell: Cell = serde_json::from_str(row).expect("S3 generated case schema");
        assert!(ids.insert(cell.id.clone()), "duplicate {}", cell.id);
        let receipt = receipts.entry(cell.family).or_default();
        receipt.planned += 1;
        if cell.excluded.is_some() {
            receipt.excluded += 1;
            assert!(
                cell.files.is_empty()
                    && cell.required_evidence.is_none()
                    && cell.expected_outcome.is_none()
            );
            continue;
        }
        receipt.emitted += 1;
        let want = cell.required_evidence.unwrap();
        let outcome = cell.expected_outcome.unwrap();
        let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let files = parse(&cell.files);
            let file = &files[&format!("case.{}", cell.grammar)];
            assert_eq!(file.parse_error_count, 0, "generated parser recovery");
            receipt.parsed += 1;
            let (graph, assembly) = capture(&files, || CodePropertyGraph::build(&files));
            let pre = assembly.pre_binding.as_ref().unwrap();
            observe_evidence(file, &graph, pre, &want);
            observe_paths(pre, &want.local_links);
            receipt.precondition_passing += 1;

            let mut errors = compare(&graph, pre, &want, &outcome);
            if let Some(replay) = &want.replay {
                if let Err(error) = props_s3_replay_tests::compare_replay(
                    file,
                    want.site.bytes,
                    &graph,
                    pre,
                    replay,
                ) {
                    errors.push(error);
                }
            }
            receipt.compared += 1;
            errors
        }));
        let (status, detail) = match run {
            Ok(errors) if errors.is_empty() => ("PASS", String::new()),
            Ok(errors) => ("RED", errors.join(" | ")),
            Err(payload) => (
                "UNMET",
                payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| {
                        payload
                            .downcast_ref::<&str>()
                            .map(|message| (*message).into())
                    })
                    .unwrap_or_else(|| "precondition panic".into()),
            ),
        };
        *receipt.outcomes.entry(status).or_default() += 1;
        failures += usize::from(status != "PASS");
        println!("S3_GENERATED {} {status} {detail}", cell.id);
    }
    for (family, expected) in [
        ("jsx-core", (3840, 0)),
        ("jsx-region", (176, 0)),
        ("jsx-multiline", (24, 0)),
        ("jsx-replacement", (48, 0)),
        ("jsx-names", (80, 0)),
        ("jsx-t3", (1044, 268)),
        ("jsx-region-host", (0, 176)),
    ] {
        let receipt = &receipts[family];
        println!("S3_GENERATED_TOTAL {family} {receipt:?}");
        assert_eq!((receipt.emitted, receipt.excluded), expected, "{family}");
    }
    assert_eq!(ids.len(), 5656);
    assert_eq!(failures, 0, "{failures} failed generated programs");
}
