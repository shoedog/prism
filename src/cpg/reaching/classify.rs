use super::graph::{definition_reaches_unflagged, path_exists, BitSet};
use super::{
    capture, innermost_statement, scope, DefSite, FlowConfidence, FlowDoubt, Line,
    StatementLineSpan,
};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use std::collections::{BTreeMap, BTreeSet};

#[allow(clippy::too_many_arguments)]
pub(super) fn classify_edge(
    edge: &FlowEdge,
    parsed: &ParsedFile,
    defs: &[DefSite],
    mapped_defs: &[Option<usize>],
    line_index: &BTreeMap<Line, usize>,
    spans: &[StatementLineSpan],
    in_sets: &[BitSet],
    kill: &[BitSet],
    flat_in_sets: &[BitSet],
    flat_kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    collapsed: &BTreeSet<(AccessPath, Line)>,
    capture_facts: &capture::CaptureFacts,
    binding_facts: &scope::BindingFacts,
    function_start: Line,
) -> FlowConfidence {
    if capture::is_capture(edge, capture_facts) {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }

    let use_statement = innermost_statement(edge.to.line, spans);
    let def_statement =
        if edge.from.line == function_start && !line_index.contains_key(&function_start) {
            Some(function_start)
        } else {
            innermost_statement(edge.from.line, spans)
        };
    let (Some(use_line), Some(def_line)) = (use_statement, def_statement) else {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    };
    let Some(&use_node) = line_index.get(&use_line) else {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    };
    let def_node_mapped = if def_line == function_start && !line_index.contains_key(&def_line) {
        true
    } else {
        line_index.contains_key(&def_line)
    };
    if !def_node_mapped {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }

    let candidates = matching_defs(edge, defs);
    if candidates.is_empty() {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }

    // Rule 3: no downstream classification is admissible unless this exact
    // def-to-use query has an unflagged CFG route. Kills are checked separately
    // below and must also reach this use through unflagged edges.
    if !candidates.iter().any(|index| {
        mapped_defs[*index]
            .is_some_and(|def_node| path_exists(successors, def_node, use_node, false))
    }) {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }
    if def_line == use_line
        || collapsed.contains(&(edge.from.path.clone(), edge.from.line))
        || collapsed.contains(&(edge.to.path.clone(), edge.to.line))
    {
        return FlowConfidence::NameOnly(FlowDoubt::SameLine);
    }
    if candidates.iter().any(|index| defs[*index].alias_derived) {
        return FlowConfidence::NameOnly(FlowDoubt::AliasUnstable);
    }

    let lookup_uncertain = candidates
        .iter()
        .any(|index| binding_facts.lookup_requires_flat_fallback(parsed, edge, *index));
    let construct_uncertain = binding_facts
        .unclassified_binding_lines(&edge.to.path.base)
        .into_iter()
        .filter_map(|line| innermost_statement(line, spans))
        .filter_map(|line| line_index.get(&line).copied())
        .any(|construct_node| {
            candidates.iter().any(|index| {
                mapped_defs[*index].is_some_and(|def_node| {
                    path_exists(successors, def_node, construct_node, false)
                        && path_exists(successors, construct_node, use_node, false)
                })
            })
        });
    if lookup_uncertain || construct_uncertain {
        return classify_by_reaching_sets(
            &candidates,
            use_node,
            mapped_defs,
            flat_in_sets,
            flat_kill,
            successors,
            line_index,
        );
    }

    let mut same_binding = Vec::new();
    let mut boundary_kills = Vec::new();
    for index in candidates {
        match binding_facts.relation(parsed, edge, index) {
            scope::BindingRelation::Same => same_binding.push(index),
            scope::BindingRelation::KilledAt(line) => boundary_kills.push(line),
            scope::BindingRelation::Unresolved => {}
        }
    }
    if let Some(kill_line) = boundary_kills.into_iter().min() {
        return FlowConfidence::NameOnly(FlowDoubt::Killed {
            kill_line: u32::try_from(kill_line).unwrap_or(u32::MAX),
        });
    }
    if same_binding.is_empty() {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }

    classify_by_reaching_sets(
        &same_binding,
        use_node,
        mapped_defs,
        in_sets,
        kill,
        successors,
        line_index,
    )
}

fn classify_by_reaching_sets(
    candidates: &[usize],
    use_node: usize,
    mapped_defs: &[Option<usize>],
    in_sets: &[BitSet],
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    line_index: &BTreeMap<Line, usize>,
) -> FlowConfidence {
    let reaching_candidates: Vec<usize> = candidates
        .iter()
        .copied()
        .filter(|index| in_sets[use_node].contains(*index))
        .collect();
    if !reaching_candidates.is_empty()
        && !reaching_candidates.iter().any(|index| {
            mapped_defs[*index].is_some_and(|def_node| {
                definition_reaches_unflagged(*index, def_node, use_node, kill, successors)
            })
        })
    {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }
    if !reaching_candidates.is_empty() {
        return FlowConfidence::Exact;
    }

    let Some(kill_line) = candidates
        .iter()
        .filter_map(|index| {
            lowest_reachable_kill(
                *index,
                mapped_defs[*index],
                use_node,
                kill,
                successors,
                line_index,
            )
        })
        .min()
    else {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    };
    FlowConfidence::NameOnly(FlowDoubt::Killed {
        kill_line: u32::try_from(kill_line).unwrap_or(u32::MAX),
    })
}

fn matching_defs(edge: &FlowEdge, defs: &[DefSite]) -> Vec<usize> {
    let mut matches: Vec<usize> = defs
        .iter()
        .enumerate()
        .filter_map(|(index, def)| {
            (def.path == edge.from.path
                && def.line == edge.from.line
                && def.start_byte == edge.from.start_byte)
                .then_some(index)
        })
        .collect();
    if matches.is_empty() {
        matches.extend(defs.iter().enumerate().filter_map(|(index, def)| {
            (def.path == edge.from.path && def.line == edge.from.line).then_some(index)
        }));
    }
    matches
}

fn lowest_reachable_kill(
    def_index: usize,
    def_node: Option<usize>,
    use_node: usize,
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    line_index: &BTreeMap<Line, usize>,
) -> Option<Line> {
    let def_node = def_node?;
    line_index
        .iter()
        .filter_map(|(line, node)| {
            (kill[*node].contains(def_index)
                && path_exists(successors, def_node, *node, false)
                && path_exists(successors, *node, use_node, false))
            .then_some(*line)
        })
        .min()
}
