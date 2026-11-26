//! Replay the first-wins lookup on the immutable, masked Step 5b input.

use super::props_generated_tests::{endpoint, exact_node, Access, Endpoint, Span};
use super::props_s2_test_support::{capture, PreBinding};
use super::var_index::{VarIndex, VarKeyRef};
use super::{CodePropertyGraph, CpgEdge, FlowConfidence, VarAccess};
use crate::access_path::AccessPath;
use crate::ast::{JsBindingCache, ParsedFile};
use petgraph::graph::NodeIndex;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Deserialize)]
pub(super) struct Replay {
    lines: Vec<usize>,
    root_line: usize,
    root: Span,
    full: Endpoint,
    base: Endpoint,
    def: Endpoint,
    dest: Endpoint,
    binding: Span,
    full_binding: Span,
    write: Span,
    conflict: String,
}

pub(super) fn matching(pre: &PreBinding, want: &Endpoint) -> Option<NodeIndex> {
    let ids: Vec<_> = pre
        .graph
        .node_indices()
        .filter(|&idx| endpoint(&pre.graph[idx]).as_ref() == Some(want))
        .collect();
    assert!(ids.len() <= 1, "ambiguous marked node {want:?}: {ids:?}");
    ids.first().copied()
}

pub(super) fn indexed(
    pre: &PreBinding,
    lookup: &VarIndex,
    want: &Endpoint,
    line: usize,
) -> Option<NodeIndex> {
    let mut fields = want.name.split('.');
    let path = AccessPath::with_fields(fields.next().unwrap(), fields.map(str::to_owned).collect());
    lookup.get_ref(
        &pre.graph,
        VarKeyRef {
            file: &want.file,
            function: &want.owner,
            function_start_line: want.owner_line,
            line,
            path: &path,
            access: VarAccess::Use,
        },
    )
}

pub(super) fn prove_inputs(file: &ParsedFile, pre: &PreBinding, want: &Replay) {
    let mut cache = JsBindingCache::for_sites();
    let mut declaration = |range: Span, binding: Span| {
        let ident = exact_node(file, range);
        let binding_node = exact_node(file, binding);
        let declared = file
            .js_ts_site_declaration(ident, "box", &mut cache)
            .expect("syntax resolves the operand root")
            .1;
        assert!(
            declared.start_byte() <= binding_node.start_byte()
                && binding_node.end_byte() <= declared.end_byte(),
            "{range:?} binds at {binding:?}"
        );
    };
    declaration(want.root, want.binding);
    declaration(want.base.bytes, want.binding);
    declaration(
        [want.full.bytes[0], want.full.bytes[0] + 3],
        want.full_binding,
    );
    let assignment = exact_node(file, want.write);
    assert_eq!(assignment.kind(), "assignment_expression");
    match want.conflict.as_str() {
        "shadow" => assert_ne!(want.binding, want.full_binding),
        "version" => {
            assert_eq!(want.binding, want.full_binding);
            assert!(want.root[1] < want.write[0] && want.write[1] < want.full.bytes[0]);
            declaration([want.write[0], want.write[0] + 3], want.binding);
        }
        other => panic!("unknown conflict {other}"),
    }
    let full = matching(pre, &want.full).expect("actual full Use at competing bytes");
    let base = matching(pre, &want.base).expect("actual base Use at the root-line bytes");
    let def = matching(pre, &want.def).expect("outer Def at marked declaration");
    assert!(matching(pre, &want.dest).is_some(), "marked target Def");
    assert_eq!(
        indexed(pre, &pre.live_lookup, &want.full, want.full.line),
        Some(full)
    );
    assert_eq!(
        indexed(pre, &pre.live_lookup, &want.base, want.root_line),
        Some(base)
    );
    assert!(want.lines.contains(&want.full.line));
    assert!(pre
        .graph
        .edges_connecting(def, base)
        .any(|edge| edge.weight() == &CpgEdge::DataFlow(FlowConfidence::Exact)));
}

pub(super) fn compare_replay(
    file: &ParsedFile,
    opening: Span,
    graph: &CodePropertyGraph,
    pre: &PreBinding,
    want: &Replay,
) -> Result<(), String> {
    let base = matching(pre, &want.base).unwrap();
    let def = matching(pre, &want.def).unwrap();
    let dest = matching(pre, &want.dest).unwrap();
    let full_ids: BTreeSet<_> = want
        .lines
        .iter()
        .filter_map(|&line| indexed(pre, &pre.live_lookup, &want.full, line))
        .collect();
    assert!(!full_ids.is_empty());
    let mut masked = pre.live_lookup.clone();
    masked.hide(full_ids);
    for &line in &want.lines {
        assert_eq!(indexed(pre, &masked, &want.full, line), None);
        assert_eq!(
            indexed(pre, &masked, &want.base, line),
            indexed(pre, &pre.live_lookup, &want.base, line)
        );
    }
    let replay = |lookup, mutant| {
        super::jsx_props_replay::replay(
            file,
            opening[0]..opening[1],
            graph,
            &pre.graph,
            lookup,
            dest,
            mutant,
        )
    };
    let untouched = replay(&pre.live_lookup, false);
    let hidden_full = replay(&masked, false);
    let mutant = replay(&pre.live_lookup, true);
    let expected = [(base, dest, CpgEdge::DataFlow(FlowConfidence::Exact))];
    let refusal = graph
        .dfg_label_stats
        .dfg_props
        .get("refused_source_ambiguous")
        .copied()
        .unwrap_or_default();
    let live_path = graph.has_path(def, dest, &CpgEdge::is_data_flow);
    let live_edge = graph
        .graph
        .edges_connecting(base, dest)
        .any(|edge| edge.weight().is_data_flow());
    if untouched.is_empty()
        && hidden_full == expected
        && mutant == expected
        && refusal == 1
        && !live_path
        && !live_edge
    {
        return Ok(());
    }
    Err(format!(
        "live refusal={refusal}, edge={live_edge}, path={live_path}, replay={untouched:?}; masked={hidden_full:?}, fall-through={mutant:?}, expected={expected:?}"
    ))
}

#[test]
fn props_s3_live_lookup_pinned_multiline_replays() {
    let rows = include_str!("../../tests/fixtures/props_matrix/rows.jsonl");
    let cases = [
        ("d1-multiline-selection", "shadow", "shadow"),
        ("d1-multiline-selection", "stale", "version"),
        ("d2-multiline-shadow", "conflict", "shadow"),
        ("d2-multiline-version", "conflict", "version"),
    ];
    let mut errors = Vec::new();
    for (id, case, conflict) in cases {
        let row = rows
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .find(|row| row["id"] == id)
            .unwrap();
        let source = row["files"][format!("{case}/m")].as_str().unwrap();
        for ext in ["jsx", "tsx"] {
            let path = format!("m.{ext}");
            let files = super::props_generated_tests::parse(&BTreeMap::from([(
                path.clone(),
                source.into(),
            )]));
            let file = &files[&path];
            assert_eq!(file.parse_error_count, 0);
            let (graph, capture) = capture(&files, || CodePropertyGraph::build(&files));
            let pre = capture.pre_binding.as_ref().unwrap();
            let var = |owner: &str, owner_line, line, name: &str, access, bytes| Endpoint {
                file: path.clone(),
                owner: owner.into(),
                owner_line,
                line,
                name: name.into(),
                access,
                bytes,
            };
            let (full, full_binding, write) = if conflict == "shadow" {
                ([169, 178], [135, 138], [150, 163])
            } else {
                ([146, 155], [63, 66], [127, 140])
            };
            let want = Replay {
                lines: vec![6, 7],
                root_line: 6,
                root: [107, 110],
                full: var("App", 4, 7, "box.value", Access::Use, full),
                base: var("App", 4, 6, "box", Access::Use, [83, 86]),
                def: var("App", 4, 5, "box", Access::Def, [63, 66]),
                dest: var("D", 1, 1, "a", Access::Def, [12, 13]),
                binding: [63, 66],
                full_binding,
                write,
                conflict: conflict.into(),
            };
            let start = source.find("<D").unwrap();
            let end = start + source[start..].find("/>").unwrap() + 2;
            let tag = format!("{id}/{case}/{ext}");
            let evidence = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prove_inputs(file, pre, &want);
            }));
            let (status, issue) = if evidence.is_err() {
                (
                    "UNMET",
                    Some("lookup/byte witness precondition failed".into()),
                )
            } else {
                let result = compare_replay(file, [start, end], &graph, pre, &want);
                (if result.is_ok() { "PASS" } else { "RED" }, result.err())
            };
            println!("S3_REPLAY {tag} {status} {issue:?}");
            if let Some(issue) = issue {
                errors.push(format!("{tag}: {issue}"));
            }
        }
    }
    assert!(errors.is_empty(), "{}", errors.join("\n"));
}
