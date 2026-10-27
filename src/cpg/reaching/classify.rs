use super::binding_table::Timing;
use super::grammar_lint::{
    is_python_match_pattern, uncertain_binding_nodes, visibility_span, SpanKind,
};
use super::graph::{definition_reaches_unflagged, path_exists, BitSet};
use super::transfer::HeaderUse;
use super::{scope, statement_lookup, DefSite, FlowConfidence, FlowDoubt, Line};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::data_flow::FlowEdge;
use std::collections::{BTreeMap, BTreeSet};

mod finally_kill;
mod import_exception_gap;

#[derive(Clone, Copy)]
struct KillNodeLines<'a> {
    statements: &'a BTreeMap<Line, usize>,
    synthetic: &'a BTreeMap<usize, Line>,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn classify_edge(
    edge: &FlowEdge,
    parsed: &ParsedFile,
    defs: &[DefSite],
    candidates: &[usize],
    mapped_defs: &[Option<usize>],
    line_index: &BTreeMap<Line, usize>,
    statements: &mut statement_lookup::StatementLookup<'_>,
    in_sets: &[BitSet],
    kill: &[BitSet],
    flat_in_sets: &[BitSet],
    flat_kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    synthetic_nodes: &BTreeMap<usize, Line>,
    collapsed: &BTreeSet<(AccessPath, Line)>,
    capture_timing: Option<Timing>,
    binding_facts: &scope::BindingFacts,
    function_start: Line,
) -> FlowConfidence {
    if capture_timing.is_some() {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }
    if scope::rust_scope::let_else_cfg_gap(parsed, edge, defs) {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }
    if let Some(construct_line) = import_exception_gap::unplaced(parsed, edge) {
        return FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: u32::try_from(construct_line).unwrap_or(u32::MAX),
        });
    }
    if let Some(construct_line) = unplaced_go_parameter(parsed, edge) {
        return FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: u32::try_from(construct_line).unwrap_or(u32::MAX),
        });
    }
    let use_byte = edge_use_byte(parsed, edge);
    if let Some(kill_line) =
        use_byte.and_then(|byte| binding_facts.java_resource_initializer_mask(parsed, edge, byte))
    {
        return FlowConfidence::NameOnly(FlowDoubt::Killed {
            kill_line: u32::try_from(kill_line).unwrap_or(u32::MAX),
        });
    }

    let use_statement = statements.get(edge.to.line);
    let def_statement =
        if edge.from.line == function_start && !line_index.contains_key(&function_start) {
            Some(function_start)
        } else {
            statements.get(edge.from.line)
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

    if candidates.is_empty() {
        return FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
    }
    if let Some(construct_line) = unplaced_go_type_switch(parsed, edge) {
        return FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: u32::try_from(construct_line).unwrap_or(u32::MAX),
        });
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
    if use_byte.is_some_and(|byte| binding_facts.header_use(edge, byte) == Some(HeaderUse::Body)) {
        return FlowConfidence::NameOnly(FlowDoubt::SameLine);
    }
    if let Some(construct_line) =
        use_byte.and_then(|byte| binding_facts.no_entry_use(&edge.to.path.base, byte))
    {
        return FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: u32::try_from(construct_line).unwrap_or(u32::MAX),
        });
    }
    if let Some(kill_line) =
        use_byte.and_then(|byte| binding_facts.header_shadow(parsed, edge, byte, candidates))
    {
        return FlowConfidence::NameOnly(FlowDoubt::Killed {
            kill_line: u32::try_from(kill_line).unwrap_or(u32::MAX),
        });
    }

    let lookup_uncertain = candidates
        .iter()
        .any(|index| binding_facts.lookup_requires_flat_fallback(parsed, edge, *index));
    let recorded_uncertain = binding_facts.unclassified_binding_lines(&edge.to.path.base);
    if let Some(construct_line) = uncertain_construct_on_route(
        parsed,
        edge,
        candidates,
        mapped_defs,
        use_node,
        line_index,
        statements,
        successors,
    ) {
        return FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain {
            construct_line: u32::try_from(construct_line).unwrap_or(u32::MAX),
        });
    }
    let construct_uncertain = recorded_uncertain
        .into_iter()
        .filter_map(|line| statements.get(line))
        .filter_map(|line| line_index.get(&line).copied())
        .any(|construct_node| {
            candidates.iter().any(|index| {
                mapped_defs[*index].is_some_and(|def_node| {
                    path_exists(successors, def_node, construct_node, false)
                        && path_exists(successors, construct_node, use_node, false)
                })
            })
        });
    if let Some(kill_line) =
        edge_use_byte(parsed, edge).and_then(|use_byte| binding_facts.mask_on_route(edge, use_byte))
    {
        return FlowConfidence::NameOnly(FlowDoubt::Killed {
            kill_line: u32::try_from(kill_line).unwrap_or(u32::MAX),
        });
    }
    if lookup_uncertain || construct_uncertain {
        let kill_node_lines = KillNodeLines {
            statements: line_index,
            synthetic: synthetic_nodes,
        };
        return classify_by_reaching_sets(
            parsed,
            edge,
            candidates,
            use_node,
            mapped_defs,
            flat_in_sets,
            flat_kill,
            successors,
            kill_node_lines,
        );
    }

    let mut same_binding = Vec::new();
    let mut boundary_kills = Vec::new();
    for &index in candidates {
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

    let kill_node_lines = KillNodeLines {
        statements: line_index,
        synthetic: synthetic_nodes,
    };
    classify_by_reaching_sets(
        parsed,
        edge,
        &same_binding,
        use_node,
        mapped_defs,
        in_sets,
        kill,
        successors,
        kill_node_lines,
    )
}

fn unplaced_go_parameter(parsed: &ParsedFile, edge: &FlowEdge) -> Option<Line> {
    if parsed.language != crate::languages::Language::Go {
        return None;
    }
    let use_byte = edge_use_byte(parsed, edge)?;
    let mut current = Some(
        parsed
            .tree
            .root_node()
            .descendant_for_byte_range(use_byte, use_byte.saturating_add(1))?,
    );
    let function = loop {
        let node = current?;
        if matches!(
            node.kind(),
            "function_declaration" | "method_declaration" | "func_literal"
        ) {
            break node;
        }
        current = node.parent();
    };
    if function.kind() != "func_literal" {
        return None;
    }
    let parameters = parsed.find_parameters_node(&function)?;
    if use_byte < parameters.end_byte() || function.end_byte() <= use_byte {
        return None;
    }
    let construct_line = parameters.start_position().row + 1;
    (edge.from.line < construct_line
        && parameter_declares_name(parsed, parameters, &edge.to.path.base))
    .then_some(construct_line)
}

fn parameter_declares_name(parsed: &ParsedFile, node: tree_sitter::Node<'_>, name: &str) -> bool {
    let mut cursor = node.walk();
    let declares = node
        .named_children(&mut cursor)
        .filter(|child| {
            matches!(
                child.kind(),
                "parameter_declaration" | "variadic_parameter_declaration"
            )
        })
        .any(|parameter| {
            let mut names = parameter.walk();
            let found = parameter
                .children_by_field_name("name", &mut names)
                .any(|identifier| parsed.node_text(&identifier) == name);
            found
        });
    declares
}

fn unplaced_go_type_switch(parsed: &ParsedFile, edge: &FlowEdge) -> Option<Line> {
    if parsed.language != crate::languages::Language::Go {
        return None;
    }
    let use_byte = edge_use_byte(parsed, edge)?;
    let use_node = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(use_byte, use_byte.saturating_add(1))?;
    uncertain_binding_nodes(parsed, &edge.to.path.base)
        .into_iter()
        .filter(|binder| binder.kind() == "type_switch_statement")
        .filter_map(|binder| {
            let construct_line = binder.start_position().row + 1;
            let mut current = Some(use_node);
            let mut inside_case = false;
            let mut inside_switch = false;
            while let Some(node) = current {
                if node.id() == binder.id() {
                    inside_switch = true;
                    break;
                }
                inside_case |= matches!(node.kind(), "type_case" | "default_case");
                current = node.parent();
            }
            let def_outside_switch = edge.from.start_byte < binder.start_byte()
                || binder.end_byte() <= edge.from.start_byte;
            let mut def_current = Some(parsed.tree.root_node().descendant_for_byte_range(
                edge.from.start_byte,
                edge.from.start_byte.saturating_add(1),
            )?);
            let mut def_case = None;
            let def_case_of_binder = loop {
                let Some(node) = def_current else {
                    break None;
                };
                if matches!(node.kind(), "type_case" | "default_case") && def_case.is_none() {
                    def_case = Some(node);
                }
                if node.kind() == "type_switch_statement" {
                    break (node.id() == binder.id()).then_some(def_case?);
                }
                def_current = node.parent();
            };
            let arm_def_reaches_outside = def_case_of_binder.is_some_and(|case| {
                !(case.start_byte() <= use_byte && use_byte < case.end_byte())
                    && ((inside_switch && inside_case) || binder.end_byte() <= use_byte)
            });
            ((def_outside_switch && inside_switch && inside_case) || arm_def_reaches_outside)
                .then_some(construct_line)
        })
        .min()
}

#[allow(clippy::too_many_arguments)]
fn uncertain_construct_on_route(
    parsed: &ParsedFile,
    edge: &FlowEdge,
    candidates: &[usize],
    mapped_defs: &[Option<usize>],
    use_node: usize,
    line_index: &BTreeMap<Line, usize>,
    statements: &mut statement_lookup::StatementLookup<'_>,
    successors: &[Vec<(usize, bool)>],
) -> Option<Line> {
    let use_byte = edge_use_byte(parsed, edge);
    let function = parsed
        .all_functions()
        .into_iter()
        .find(|node| parsed.node_line_range(node).0 == edge.to.function_start_line)?;
    uncertain_binding_nodes(parsed, &edge.to.path.base)
        .into_iter()
        .filter(|binder| {
            function.start_byte() <= binder.start_byte() && binder.end_byte() <= function.end_byte()
        })
        .filter_map(|binder| {
            let construct_line = binder.start_position().row + 1;
            if construct_line <= edge.from.line {
                return None;
            }
            let explicit_uncertain_binder = matches!(
                (parsed.language, binder.kind()),
                (
                    crate::languages::Language::Java,
                    "record_pattern" | "type_pattern" | "instanceof_expression"
                ) | (crate::languages::Language::Bash, "for_statement")
                    | (
                        crate::languages::Language::Rust,
                        "captured_pattern"
                            | "mut_pattern"
                            | "or_pattern"
                            | "ref_pattern"
                            | "slice_pattern"
                            | "struct_pattern"
                            | "tuple_pattern"
                            | "tuple_struct_pattern"
                    )
                    | (
                        crate::languages::Language::Python,
                        "global_statement"
                            | "nonlocal_statement"
                            | "wildcard_import"
                            | "case_pattern"
                            | "class_pattern"
                            | "as_pattern"
                            | "keyword_pattern"
                    )
                    | (
                        crate::languages::Language::JavaScript,
                        "object_pattern"
                            | "array_pattern"
                            | "assignment_pattern"
                            | "rest_pattern"
                            | "shorthand_property_identifier_pattern"
                    )
            ) || (parsed.language
                == crate::languages::Language::Python
                && matches!(binder.kind(), "tuple_pattern" | "list_pattern")
                && is_python_match_pattern(binder))
                || (matches!(
                    parsed.language,
                    crate::languages::Language::TypeScript | crate::languages::Language::Tsx
                )
                    && matches!(binder.kind(), "required_parameter" | "optional_parameter")
                    && crate::cpg::reaching::grammar_lint::visibility::typescript_parameter_has_body_owner(binder));
            if !explicit_uncertain_binder {
                return None;
            }
            let unplaced_python_pattern = parsed.language == crate::languages::Language::Python
                && is_python_match_pattern(binder);
            let construct_node = (!unplaced_python_pattern)
                .then(|| statements.get(construct_line))
                .flatten()
                .and_then(|line| line_index.get(&line).copied());
            let omitted_python_declaration = parsed.language == crate::languages::Language::Python
                && (matches!(binder.kind(), "global_statement" | "nonlocal_statement")
                    || unplaced_python_pattern)
                && construct_node.is_none();
            let on_route = candidates.iter().any(|index| {
                mapped_defs[*index].is_some_and(|def_node| match construct_node {
                    Some(construct_node) => {
                        path_exists(successors, def_node, construct_node, false)
                            && path_exists(successors, construct_node, use_node, false)
                    }
                    None => {
                        omitted_python_declaration
                            && path_exists(successors, def_node, use_node, false)
                    }
                })
            });
            if !on_route {
                return None;
            }
            let visible = match visibility_span(parsed, binder) {
                SpanKind::Known(spans) => use_byte.map_or_else(
                    || {
                        if !unplaced_python_pattern {
                            return false;
                        }
                        let identifiers: Vec<_> = parsed
                            .identifiers_on_line(edge.to.line)
                            .into_iter()
                            .filter(|node| parsed.node_text(node) == edge.to.path.base)
                            .collect();
                        identifiers.len() > 1
                            && identifiers.iter().any(|identifier| {
                                spans
                                    .iter()
                                    .any(|span| span.contains(identifier.start_byte()))
                            })
                    },
                    |use_byte| spans.iter().any(|span| span.contains(use_byte)),
                ),
                SpanKind::Empty => false,
                SpanKind::Unknown => use_byte.is_some_and(|use_byte| {
                    binder.start_byte() <= use_byte && use_byte < function.end_byte()
                }),
            };
            visible.then_some(construct_line)
        })
        .min()
}

fn edge_use_byte(parsed: &ParsedFile, edge: &FlowEdge) -> Option<usize> {
    if edge.to.start_byte < edge.to.end_byte {
        return Some(edge.to.start_byte);
    }
    let mut identifiers = parsed
        .identifiers_on_line(edge.to.line)
        .into_iter()
        .filter(|node| parsed.node_text(node) == edge.to.path.base);
    let first = identifiers.next()?;
    identifiers.next().is_none().then_some(first.start_byte())
}

#[allow(clippy::too_many_arguments)]
fn classify_by_reaching_sets(
    parsed: &ParsedFile,
    edge: &FlowEdge,
    candidates: &[usize],
    use_node: usize,
    mapped_defs: &[Option<usize>],
    in_sets: &[BitSet],
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    kill_node_lines: KillNodeLines<'_>,
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

    if let Some(kill_line) = finally_kill::dominant(
        parsed,
        edge,
        candidates,
        use_node,
        mapped_defs,
        kill,
        successors,
        kill_node_lines.statements,
    ) {
        return FlowConfidence::NameOnly(FlowDoubt::Killed {
            kill_line: u32::try_from(kill_line).unwrap_or(u32::MAX),
        });
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
                kill_node_lines,
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

// Frozen main scanner, independent of bucket construction.
#[cfg(test)]
pub(super) fn matching_defs(edge: &FlowEdge, defs: &[DefSite]) -> Vec<usize> {
    super::work_tests::bump(|counts| {
        counts.matching_defs_calls += 1;
        counts.matching_defs_items += defs.len();
    });
    let mut matches: Vec<usize> = defs
        .iter()
        .enumerate()
        .filter_map(|(index, def)| {
            (!def.synthetic
                && def.path == edge.from.path
                && def.line == edge.from.line
                && def.start_byte == edge.from.start_byte)
                .then_some(index)
        })
        .collect();
    if matches.is_empty() {
        super::work_tests::bump(|counts| {
            counts.matching_defs_fallbacks += 1;
            counts.matching_defs_items += defs.len();
        });
        matches.extend(defs.iter().enumerate().filter_map(|(index, def)| {
            (!def.synthetic && def.path == edge.from.path && def.line == edge.from.line)
                .then_some(index)
        }));
    }
    super::work_tests::bump(|counts| counts.projected_matching_defs_items += matches.len());
    matches
}

fn lowest_reachable_kill(
    def_index: usize,
    def_node: Option<usize>,
    use_node: usize,
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    node_lines: KillNodeLines<'_>,
) -> Option<Line> {
    let def_node = def_node?;
    node_lines
        .statements
        .iter()
        .map(|(line, node)| (*node, *line))
        .chain(
            node_lines
                .synthetic
                .iter()
                .map(|(node, line)| (*node, *line)),
        )
        .filter_map(|(node, line)| {
            (kill[node].contains(def_index)
                && path_exists(successors, def_node, node, false)
                && path_exists(successors, node, use_node, false))
            .then_some(line)
        })
        .min()
}
