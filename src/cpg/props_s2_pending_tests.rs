//! Tests for position arbitration, concrete edge witnesses and exact-span lookup.

use super::build::props_binding::{literal_pattern_pairs, literal_site_refusal};
use super::var_index::VarIndex;
use super::{CodePropertyGraph, CpgEdge, CpgNode, VarAccess};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::languages::Language;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::BTreeMap;

#[test]
fn props_s2_refusal_priority_uses_check_then_argument_position() {
    let alias = "refused_alias_pattern";
    let export = "refused_member_not_export";
    let identity = "refused_target_identity";
    let wrapper = "refused_unknown_wrapper";
    let cases: &[(&[(usize, bool, u8, &str)], Option<&str>)] = &[
        (&[], None),
        (&[(0, false, 2, export)], None),
        (&[(0, true, 4, alias), (1, true, 2, export)], Some(export)),
        (
            &[(1, true, 3, identity), (0, true, 3, wrapper)],
            Some(wrapper),
        ),
        (
            &[(1, true, 3, wrapper), (0, true, 3, identity)],
            Some(identity),
        ),
        (&[(0, false, 2, export), (1, true, 4, alias)], Some(alias)),
    ];
    let mismatches: Vec<_> = cases
        .iter()
        .filter_map(|(input, want)| {
            let got = literal_site_refusal(input);
            (&got != want).then_some((input, want, got))
        })
        .collect();
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn props_s2_repeated_witnesses_keep_edge_order_and_container_subset() {
    let (first, second, dest) = (NodeIndex::new(5), NodeIndex::new(6), NodeIndex::new(9));
    let witness = [
        (first, dest, false),
        (second, dest, false),
        (first, dest, true),
        (first, dest, true),
    ];
    let (edges, telemetry) = literal_pattern_pairs(&witness);
    assert_eq!(
        (edges, telemetry),
        (
            vec![(first, dest), (second, dest)],
            BTreeMap::from([("edges_to_def".into(), 2), ("edges_to_container".into(), 1)]),
        ),
    );
}

#[test]
fn props_s2_inherited_literal_source_lookup_uses_the_exact_span_per_path() {
    let file = "operand.js";
    let source = "function App(box) {\n  D({a: box.value});\n}\n";
    let inputs = BTreeMap::from([(
        file.to_owned(),
        ParsedFile::parse(file, source, Language::JavaScript).unwrap(),
    )]);
    let cpg = CodePropertyGraph::build(&inputs);
    let caller = &cpg.call_graph.functions["App"][0];
    let parsed = &inputs[file];
    let from = source.find("box.value").unwrap();
    let operand = from..from + "box.value".len();
    let base = AccessPath::simple("box");
    let full = AccessPath::from_expr("box.value");
    let node = |path: &AccessPath, access, start_byte, end_byte| CpgNode::Variable {
        file: file.into(),
        function: "App".into(),
        function_start_line: 1,
        line: 2,
        path: path.clone(),
        access,
        start_byte,
        end_byte,
    };
    let lookup = VarIndex::new();
    let find = |graph: &DiGraph<CpgNode, CpgEdge>, path| {
        CodePropertyGraph::argument_var_node_in_span(caller, parsed, path, &operand, &lookup, graph)
    };
    let mut graph = DiGraph::new();
    graph.add_node(node(&full, VarAccess::Use, operand.end, operand.end + 1));
    assert_eq!(find(&graph, &full), None, "off-span spelling");
    assert_eq!(find(&graph, &base), None, "no exact base");
    let def = graph.add_node(node(&full, VarAccess::Def, operand.start, operand.end));
    assert_eq!(find(&graph, &full), Some(def), "sole Def fallback");
    let use_node = graph.add_node(node(&full, VarAccess::Use, operand.start, operand.end));
    let base_use = graph.add_node(node(
        &base,
        VarAccess::Use,
        operand.start,
        operand.start + 3,
    ));
    assert_eq!(
        find(&graph, &full),
        Some(use_node),
        "prefer even a disconnected Use"
    );
    assert_eq!(
        find(&graph, &base),
        Some(base_use),
        "independent full/base candidates"
    );
    graph.add_node(node(&full, VarAccess::Use, operand.start, operand.end));
    assert_eq!(
        find(&graph, &full),
        None,
        "multiple preferred Uses refuse without falling back"
    );
    assert_eq!(
        find(&graph, &base),
        Some(base_use),
        "full-tier failure leaves base eligible"
    );
}
