//! Classify S1 body edges without changing the legacy solver's input population.

use super::pattern_formals::{PatternFormal, Selected};
use crate::ast::ParsedFile;
use crate::cpg::{
    reaching_definitions_with_exact, DefSite, FileCfgEdgeIndex, FlowConfidence, FlowDoubt,
    RdOutcome,
};
use crate::data_flow::{ExactFlowEdge, FlowEdge, VarLocation};
use std::collections::BTreeMap;
use tree_sitter::Node;

pub(super) fn body_labels(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    defs: &[DefSite],
    cfg: &FileCfgEdgeIndex<'_>,
    endpoints: &[(&PatternFormal, Selected<'_>)],
    emitted: &[usize],
) -> BTreeMap<(VarLocation, VarLocation), FlowConfidence> {
    let edges: Vec<_> = emitted
        .iter()
        .filter_map(|&index| {
            let (formal, selected) = &endpoints[index];
            (!selected.signature).then(|| FlowEdge {
                from: formal.def.clone(),
                to: selected.loc.clone(),
            })
        })
        .collect();
    if edges.is_empty() {
        return BTreeMap::new();
    }
    let fallback = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    match reaching_definitions_with_exact(parsed, owner, defs, &edges, &[], cfg) {
        RdOutcome::Available(mut result) => {
            // A signature-line kill from the line solver does not prove that an
            // initializer ran or that it replaced the binding. Endpoint selection
            // already suppressed the replacements with an execution proof.
            if let Some(formals) = owner.child_by_field_name("parameters") {
                let (first, last) = parsed.node_line_range(&formals);
                for label in result.labels.values_mut() {
                    if matches!(*label, FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line })
                        if (first..=last).contains(&(kill_line as usize)))
                    {
                        *label = fallback;
                    }
                }
            }
            result.labels
        }
        RdOutcome::Unavailable(_) => edges
            .into_iter()
            .map(|edge| ((edge.from, edge.to), fallback))
            .collect(),
    }
}

pub(super) fn store_body_labels(
    edges: &mut Vec<FlowEdge>,
    legacy_end: usize,
    labels: &mut BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    exact: &mut BTreeMap<ExactFlowEdge, FlowConfidence>,
    body: &BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
) {
    if body.is_empty() {
        return;
    }
    let additions = edges.split_off(legacy_end);
    for edge in additions {
        let key = (edge.from.clone(), edge.to.clone());
        if let Some(&label) = body.get(&key) {
            let stored = labels.entry(key).or_insert(label);
            if *stored != label {
                // Two definitions share the legacy line key. The new occurrence
                // belongs in the exact lane, while the old line label stays intact.
                debug_assert_eq!(edge.from.path, edge.to.path);
                debug_assert!(edge.from.path.is_simple());
                exact.insert(ExactFlowEdge::from_legacy(&edge), label);
                continue;
            }
        }
        edges.push(edge);
    }
}
