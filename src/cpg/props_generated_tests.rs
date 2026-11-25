//! DESIGN §5.8. Check source and graph evidence before comparing the S1 outcome.

use super::{CodePropertyGraph, CpgEdge, CpgNode, FlowConfidence, FlowDoubt, VarAccess};
use crate::resolution::ResolutionConfidence;
use crate::{ast::ParsedFile, languages::Language};
use petgraph::visit::EdgeRef;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[path = "props_generated_cache_tests.rs"]
mod cache;
#[path = "props_generated_outcome.rs"]
mod outcome;
use outcome::{compare, Outcome};

const CASES: &str = include_str!("../../tests/props_matrix_gen/generated-cases.jsonl");
type Span = [usize; 2];

#[derive(Debug, Deserialize)]
struct Cell {
    id: String,
    family: String,
    dimensions: Dimensions,
    grammar: String,
    generator: u32,
    typescript: String,
    files: BTreeMap<String, String>,
    excluded: Option<String>,
    required_evidence: Option<Evidence>,
    expected_outcome: Option<Outcome>,
}

#[derive(Debug, Deserialize)]
struct Dimensions {
    terminal: String,
    depth: usize,
    neighbor: String,
    default: String,
}

#[derive(Debug, Deserialize)]
struct Evidence {
    parser_recovery: bool,
    caller: Owner,
    target: Owner,
    dfg_owner: Option<String>,
    duplicates: Vec<String>,
    site: Site,
    formal: Formal,
    initializer: Option<Span>,
    provider: Option<Span>,
    bound: Vec<Binding>,
    reads: Vec<Endpoint>,
    sources: Vec<Source>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Owner {
    name: String,
    bytes: Span,
    line: usize,
    end_line: usize,
}

#[derive(Debug, Deserialize)]
struct Site {
    bytes: Span,
    argument: Span,
    resolution: ResolutionConfidence,
}

#[derive(Debug, Deserialize)]
struct Formal {
    position: usize,
    bytes: Span,
    designated: Option<Span>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Binding {
    name: String,
    bytes: Span,
    array: bool,
    position: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Access {
    Def,
    Use,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct Endpoint {
    file: String,
    owner: String,
    owner_line: usize,
    name: String,
    line: usize,
    access: Access,
    bytes: Span,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum Label {
    Exact,
    CfgIncomplete,
    SameLine,
    Killed,
    AliasUnstable,
    CallNameOnly,
}

impl From<FlowConfidence> for Label {
    fn from(label: FlowConfidence) -> Self {
        match label {
            FlowConfidence::Exact => Self::Exact,
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => Self::CfgIncomplete,
            FlowConfidence::NameOnly(FlowDoubt::SameLine) => Self::SameLine,
            FlowConfidence::NameOnly(FlowDoubt::Killed { .. }) => Self::Killed,
            FlowConfidence::NameOnly(FlowDoubt::AliasUnstable) => Self::AliasUnstable,
            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly) => Self::CallNameOnly,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Source {
    path: Vec<String>,
    full: Option<Endpoint>,
    base: Endpoint,
    incoming: Incoming,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Incoming {
    from: Endpoint,
    label: Label,
}

fn span(node: Node<'_>) -> Span {
    [node.start_byte(), node.end_byte()]
}

fn parse(files: &BTreeMap<String, String>) -> BTreeMap<String, ParsedFile> {
    files
        .iter()
        .map(|(path, source)| {
            let language = Language::from_path(path).expect("designated grammar");
            assert!(
                language.is_js_family(),
                "unexpected fixture grammar {language:?}"
            );
            (
                path.clone(),
                ParsedFile::parse(path, source, language).expect("parse fixture"),
            )
        })
        .collect()
}

fn owner<'a>(file: &'a ParsedFile, wanted: &Owner) -> Node<'a> {
    let owners: Vec<_> = file
        .all_functions()
        .into_iter()
        .filter(|node| {
            node.child_by_field_name("name")
                .is_some_and(|name| file.node_text(&name) == wanted.name)
        })
        .collect();
    assert_eq!(owners.len(), 1, "unique lexical owner {wanted:?}");
    let node = owners[0];
    assert_eq!(span(node), wanted.bytes);
    assert_eq!(file.node_line_range(&node), (wanted.line, wanted.end_line));
    node
}

fn endpoint(node: &CpgNode) -> Option<Endpoint> {
    let CpgNode::Variable {
        file,
        path,
        function,
        function_start_line,
        line,
        access,
        start_byte,
        end_byte,
    } = node
    else {
        return None;
    };
    Some(Endpoint {
        file: file.clone(),
        owner: function.clone(),
        owner_line: *function_start_line,
        name: path.to_string(),
        line: *line,
        access: match access {
            VarAccess::Def => Access::Def,
            VarAccess::Use => Access::Use,
        },
        bytes: [*start_byte, *end_byte],
    })
}

fn named_children(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|child| !child.is_extra())
        .collect()
}

fn parameter(node: Node<'_>) -> (Node<'_>, Option<Node<'_>>) {
    match node.kind() {
        "required_parameter" => (
            node.child_by_field_name("pattern").unwrap(),
            node.child_by_field_name("value"),
        ),
        "assignment_pattern" => (
            node.child_by_field_name("left").unwrap(),
            node.child_by_field_name("right"),
        ),
        _ => (node, None),
    }
}

fn bindings(
    file: &ParsedFile,
    node: Node<'_>,
    position: usize,
    in_array: bool,
    out: &mut Vec<Binding>,
) {
    match node.kind() {
        "identifier" | "shorthand_property_identifier_pattern" => out.push(Binding {
            name: file.node_text(&node).to_owned(),
            bytes: span(node),
            array: in_array,
            position,
        }),
        "pair_pattern" => bindings(
            file,
            node.child_by_field_name("value").unwrap(),
            position,
            in_array,
            out,
        ),
        "object_pattern" | "array_pattern" | "rest_pattern" => {
            for child in named_children(node) {
                bindings(
                    file,
                    child,
                    position,
                    in_array || node.kind() == "array_pattern",
                    out,
                );
            }
        }
        kind => panic!("unexpected binding {kind} at {:?}", span(node)),
    }
}

fn exact_node(file: &ParsedFile, bytes: Span) -> Node<'_> {
    let node = file
        .tree
        .root_node()
        .descendant_for_byte_range(bytes[0], bytes[1])
        .expect("marked token");
    assert_eq!(span(node), bytes, "exact token bytes");
    node
}

fn observe_evidence(file: &ParsedFile, graph: &CodePropertyGraph, want: &Evidence) {
    assert_eq!(file.tree.root_node().has_error(), want.parser_recovery);
    assert_eq!(file.parse_error_count, 0);
    let target = owner(file, &want.target);
    let caller = owner(file, &want.caller);
    assert!(target.end_byte() <= caller.start_byte(), "disjoint owners");
    assert_eq!(file.dfg_owner_name(&target), want.dfg_owner);
    assert_eq!(
        file.dfg_owner_name(&caller).as_deref(),
        Some(want.caller.name.as_str())
    );

    let formals = named_children(file.find_parameters_node(&target).unwrap());
    let (binding, init) = parameter(formals[want.formal.position]);
    assert_eq!(span(binding), want.formal.bytes);
    assert_eq!(binding.kind(), "object_pattern");
    assert_eq!(init.map(span), want.initializer);
    let mut bound = Vec::new();
    for (position, formal) in formals.into_iter().enumerate() {
        bindings(file, parameter(formal).0, position, false, &mut bound);
    }
    assert_eq!(bound, want.bound, "whole-list declarations");
    let mut names = BTreeSet::new();
    let duplicates: BTreeSet<_> = bound
        .iter()
        .filter_map(|b| (!names.insert(&b.name)).then_some(&b.name))
        .collect();
    assert_eq!(
        duplicates.into_iter().collect::<Vec<_>>(),
        want.duplicates.iter().collect::<Vec<_>>()
    );
    if let Some(designated) = want.formal.designated {
        assert!(bound
            .iter()
            .any(|b| b.position == want.formal.position && b.bytes == designated));
    } else {
        assert!(bound.iter().all(|b| b.position != want.formal.position));
    }
    let provider: Vec<_> = init
        .filter(|node| node.kind() == "identifier")
        .into_iter()
        .flat_map(|node| {
            bound
                .iter()
                .filter(move |b| b.name == file.node_text(&node))
                .map(|b| b.bytes)
        })
        .collect();
    assert_eq!(provider, want.provider.into_iter().collect::<Vec<_>>());
    if let Some(node) = init.filter(|_| want.provider.is_some()) {
        assert!(file.js_ts_occurrence_binds_formal(
            &target,
            file.node_text(&node),
            node.start_byte()
        ));
    }

    for read in &want.reads {
        let node = exact_node(file, read.bytes);
        let body = target.child_by_field_name("body").unwrap();
        assert!(body.start_byte() < node.start_byte() && node.end_byte() < body.end_byte());
        assert_eq!(node.kind(), "identifier");
        assert_eq!(read.file, file.path);
        assert_eq!(read.owner, want.target.name);
        assert_eq!(read.owner_line, want.target.line);
        assert_eq!(read.access, Access::Use);
        assert_eq!(file.node_text(&node), read.name);
        assert_eq!(node.start_position().row + 1, read.line);
        assert_eq!(node.parent().unwrap().kind(), "arguments");
        if !want.duplicates.contains(&read.name) {
            assert!(
                file.js_ts_occurrence_binds_formal(&target, &read.name, node.start_byte()),
                "{read:?}"
            );
        }
    }

    let calls: Vec<_> = graph
        .call_graph
        .calls
        .iter()
        .filter(|(owner, _)| {
            owner.name == want.caller.name
                && owner.file == file.path
                && owner.start_line == want.caller.line
                && owner.end_line == want.caller.end_line
        })
        .flat_map(|(_, sites)| sites)
        .filter(|site| site.callee_name == want.target.name)
        .collect();
    assert_eq!(calls.len(), 1, "recorded call sites {calls:?}");
    assert_eq!([calls[0].start_byte, calls[0].end_byte], want.site.bytes);
    let resolved = graph.call_graph.resolve_call_site(calls[0]);
    assert_eq!(resolved.len(), 1, "resolution of marked call");
    let target_id = resolved[0].target;
    assert_eq!(target_id.file, file.path);
    assert_eq!(target_id.name, want.target.name);
    assert_eq!(
        (target_id.start_line, target_id.end_line),
        (want.target.line, want.target.end_line)
    );
    assert_eq!(resolved[0].confidence, want.site.resolution);
    let call = exact_node(file, want.site.bytes);
    assert_eq!(call.kind(), "call_expression");
    let args = named_children(call.child_by_field_name("arguments").unwrap());
    let argument = args[want.formal.position];
    assert_eq!(span(argument), want.site.argument);
    assert_eq!(argument.kind(), "object");

    for source in &want.sources {
        let mut operand = argument;
        for key in &source.path {
            let pairs: Vec<_> = named_children(operand)
                .into_iter()
                .filter(|node| {
                    node.child_by_field_name("key")
                        .is_some_and(|name| file.node_text(&name) == key)
                })
                .collect();
            assert_eq!(pairs.len(), 1, "ordered source path {:?}", source.path);
            operand = pairs[0].child_by_field_name("value").unwrap();
        }
        assert_eq!(span(operand), source.base.bytes);
        assert_eq!(operand.kind(), "identifier");
        assert_eq!(file.node_text(&operand), source.base.name);
        assert!(file.js_ts_occurrence_binds_formal(
            &caller,
            &source.base.name,
            operand.start_byte()
        ));
        let candidates: Vec<_> = graph
            .graph
            .node_indices()
            .filter_map(|index| {
                let at = endpoint(graph.node(index))?;
                (at.owner == want.caller.name
                    && at.access == Access::Use
                    && at.bytes == source.base.bytes)
                    .then_some((index, at))
            })
            .collect();
        let bases: Vec<_> = candidates
            .iter()
            .filter(|(_, at)| at.name == source.base.name)
            .collect();
        assert_eq!(
            bases.len(),
            1,
            "base {:?}, candidates {candidates:?}",
            source.base
        );
        assert_eq!(bases[0].1, source.base);
        let full: Vec<_> = candidates
            .iter()
            .map(|(_, at)| at)
            .filter(|at| *at != &source.base)
            .collect();
        assert_eq!(
            full,
            source.full.iter().collect::<Vec<_>>(),
            "full candidates"
        );
        let incoming: Vec<_> = graph
            .graph
            .edges_directed(bases[0].0, petgraph::Incoming)
            .filter_map(|edge| {
                let CpgEdge::DataFlow(label) = *edge.weight() else {
                    return None;
                };
                let from =
                    endpoint(graph.node(edge.source())).expect("operand input is a variable");
                Some(Incoming {
                    from,
                    label: label.into(),
                })
            })
            .collect();
        assert_eq!(
            incoming.as_slice(),
            std::slice::from_ref(&source.incoming),
            "incoming to {:?}",
            source.base
        );
    }
}

#[derive(Debug, Default)]
struct Receipt {
    planned: usize,
    excluded: usize,
    emitted: usize,
    parsed: usize,
    preconditions: usize,
    compared: usize,
    expected_outcomes: BTreeMap<String, usize>,
}

#[test]
fn generated_pattern_default_cells() {
    let mut previous = String::new();
    let mut receipts = BTreeMap::<String, Receipt>::new();
    let mut failures = Vec::new();
    for line in CASES.lines() {
        let cell: Cell = serde_json::from_str(line).expect("generated JSONL schema");
        let Cell {
            id,
            family,
            dimensions,
            grammar,
            generator,
            typescript,
            files,
            excluded,
            required_evidence,
            expected_outcome,
        } = cell;
        assert_eq!(generator, 1);
        assert_eq!(typescript, "5.9.3");
        assert_eq!(
            id,
            format!(
                "{family}/{}/{}/{}/{}/{grammar}",
                dimensions.terminal, dimensions.depth, dimensions.neighbor, dimensions.default
            )
        );
        assert!(id > previous, "duplicate or out-of-order cell {id}");
        previous = id.clone();
        let receipt = receipts.entry(family).or_default();
        receipt.planned += 1;
        if let Some(reason) = excluded {
            assert!(
                !reason.is_empty()
                    && files.is_empty()
                    && required_evidence.is_none()
                    && expected_outcome.is_none(),
                "{id}"
            );
            receipt.excluded += 1;
            continue;
        }
        receipt.emitted += 1;
        let evidence = required_evidence.expect("applicable evidence");
        let outcome = expected_outcome.expect("applicable outcome");
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            assert_eq!(files.len(), 1);
            let parsed = parse(&files);
            let file = &parsed[&format!("case.{grammar}")];
            assert!(!file.tree.root_node().has_error(), "unexpected recovery");
            receipt.parsed += 1;
            let graph = CodePropertyGraph::build(&parsed);
            observe_evidence(file, &graph, &evidence);
            receipt.preconditions += 1;
            compare(file, &graph, &evidence, &outcome);
            receipt.compared += 1;
            *receipt
                .expected_outcomes
                .entry(outcome.disposition)
                .or_default() += 1;
        }));
        if result.is_err() {
            eprintln!("failing cell {id}, {dimensions:?}\nfiles {files:#?}");
            failures.push(id);
        }
    }
    eprintln!(
        "Generator v1, TypeScript 5.9.3, SHA-256 {:x}",
        Sha256::digest(CASES.as_bytes())
    );
    for (family, receipt) in &receipts {
        eprintln!("{family}: {receipt:?}");
    }
    let receipt = &receipts["pattern-defaults"];
    assert_eq!(
        (receipt.planned, receipt.excluded, receipt.emitted),
        (576, 48, 528)
    );
    assert!(failures.is_empty(), "failing cell IDs: {failures:?}");
    assert_eq!(
        (receipt.parsed, receipt.preconditions, receipt.compared),
        (528, 528, 528)
    );
}
