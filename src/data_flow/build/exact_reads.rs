//! Supplemental exact producer facts and RD definition helpers.
use super::Producer;
use crate::access_path::AccessPath;
use crate::ast::{ParsedFile, PathSpan};
use crate::cpg::{DefId, DefSite, FlowConfidence, FlowDoubt};
use crate::data_flow::{ExactFlowEdge, ExactVarLocation, FlowEdge, VarAccessKind, VarLocation};
use std::collections::{BTreeMap, BTreeSet};

impl Producer {
    pub(super) fn push_rd_def(rd_defs: &mut Vec<DefSite>, loc: &VarLocation, alias_derived: bool) {
        let id = DefId(
            u32::try_from(rd_defs.len())
                .expect("reaching-definition count exceeds the DefId representation"),
        );
        rd_defs.push(DefSite {
            id,
            path: loc.path.clone(),
            line: loc.line,
            start_byte: loc.start_byte,
            alias_derived,
            synthetic: false,
        });
    }

    pub(super) fn exact_read_candidates(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
        owner_is_unique: bool,
        file: &str,
        function: &str,
        function_start_line: usize,
        parameter_occurrences: &[(String, usize, usize)],
        parameter_defs: &[VarLocation],
        raw_aliases: &[(String, String, usize)],
        lvalue_spans: &[PathSpan],
        rvalue_spans: &[PathSpan],
        legacy_edges: &[FlowEdge],
    ) -> Vec<FlowEdge> {
        if !owner_is_unique {
            return Vec::new();
        }

        let mut candidates = BTreeMap::<ExactFlowEdge, FlowEdge>::new();
        for read in crate::cpg::classified_masked_reads(parsed, func_node, rvalue_spans) {
            if !read.path.is_simple()
                || read.start_byte >= read.end_byte
                || lvalue_spans.iter().any(|span| {
                    (span.start_byte, span.end_byte) == (read.start_byte, read.end_byte)
                })
                || raw_aliases.iter().any(|(alias, target, _)| {
                    alias == &read.path.base || AccessPath::from_expr(target).base == read.path.base
                })
                || lvalue_spans
                    .iter()
                    .chain(rvalue_spans)
                    .any(|span| span.path.has_fields() && span.path.base == read.path.base)
            {
                continue;
            }
            let mut prior_defs = BTreeMap::<ExactVarLocation, VarLocation>::new();
            for def in parameter_defs
                .iter()
                .filter(|def| def.path == read.path && def.line < read.line)
            {
                prior_defs.insert(ExactVarLocation::from_legacy(def), def.clone());
            }
            for span in lvalue_spans
                .iter()
                .filter(|span| span.path == read.path && span.line < read.line)
                .filter(|span| {
                    crate::profile::gates::def_scope(
                        parsed,
                        func_node,
                        &span.path.base,
                        span.start_byte,
                    )
                    .is_none_or(|(start, end)| start <= read.start_byte && read.end_byte <= end)
                })
            {
                let def = VarLocation {
                    file: file.to_string(),
                    function: function.to_string(),
                    function_start_line,
                    line: span.line,
                    path: span.path.clone(),
                    start_byte: span.start_byte,
                    end_byte: span.end_byte,
                    kind: VarAccessKind::Def,
                };
                prior_defs.insert(ExactVarLocation::from_legacy(&def), def);
            }
            let to = VarLocation {
                file: file.to_string(),
                function: function.to_string(),
                function_start_line,
                line: read.line,
                path: read.path,
                start_byte: read.start_byte,
                end_byte: read.end_byte,
                kind: VarAccessKind::Use,
            };
            for from in prior_defs.into_values() {
                let edge = FlowEdge {
                    from,
                    to: to.clone(),
                };
                candidates.insert(ExactFlowEdge::from_legacy(&edge), edge);
            }
        }
        if !parsed.exact_read_callable_is_straight_line(func_node) {
            return candidates.into_values().collect();
        }

        let mut occurrences: BTreeMap<(AccessPath, usize), BTreeSet<(usize, usize)>> =
            BTreeMap::new();
        for span in rvalue_spans {
            if span.path.is_simple() && span.start_byte < span.end_byte {
                occurrences
                    .entry((span.path.clone(), span.line))
                    .or_default()
                    .insert((span.start_byte, span.end_byte));
            }
        }

        for ((path, line), spans) in occurrences {
            if spans.len() < 2
                || raw_aliases.iter().any(|(alias, target, _)| {
                    alias == &path.base || AccessPath::from_expr(target).base == path.base
                })
                || lvalue_spans
                    .iter()
                    .chain(rvalue_spans)
                    .any(|span| span.path.has_fields() && span.path.base == path.base)
            {
                continue;
            }

            let mut exact_defs = BTreeMap::<ExactVarLocation, VarLocation>::new();
            for edge in legacy_edges.iter().filter(|edge| {
                edge.from.kind == VarAccessKind::Def
                    && edge.from.path == path
                    && edge.to.kind == VarAccessKind::Use
                    && edge.to.path == path
                    && edge.to.line == line
            }) {
                exact_defs
                    .entry(ExactVarLocation::from_legacy(&edge.from))
                    .or_insert_with(|| edge.from.clone());
            }
            let mut defs_for_line: Vec<_> = exact_defs.into_values().collect();
            if defs_for_line.len() != 1 {
                continue;
            }
            let def = defs_for_line.pop().expect("one exact producer definition");

            let matching_parameters: Vec<_> = parameter_occurrences
                .iter()
                .filter(|(name, start, end)| {
                    name == &path.base && (*start, *end) == (def.start_byte, def.end_byte)
                })
                .collect();
            let parameter_span = PathSpan {
                path: path.clone(),
                line: def.line,
                start_byte: def.start_byte,
                end_byte: def.end_byte,
            };
            let is_parameter = matching_parameters.len() == 1
                && parsed.exact_read_is_plain_required_parameter(func_node, &parameter_span);
            if !matching_parameters.is_empty() && !is_parameter {
                continue;
            }
            let matching_lvalues: Vec<_> = lvalue_spans
                .iter()
                .filter(|span| span.path == path)
                .collect();
            if is_parameter {
                if !matching_lvalues.is_empty() {
                    continue;
                }
            } else {
                if matching_lvalues.len() != 1
                    || (matching_lvalues[0].start_byte, matching_lvalues[0].end_byte)
                        != (def.start_byte, def.end_byte)
                {
                    continue;
                }
                let def_span = PathSpan {
                    path: path.clone(),
                    line: def.line,
                    start_byte: def.start_byte,
                    end_byte: def.end_byte,
                };
                if def.line >= line
                    || !parsed.exact_read_is_simple_local_declaration(func_node, &def_span)
                {
                    continue;
                }
            }

            for (start_byte, end_byte) in spans {
                let edge = FlowEdge {
                    from: def.clone(),
                    to: VarLocation {
                        file: def.file.clone(),
                        function: def.function.clone(),
                        function_start_line: def.function_start_line,
                        line,
                        path: path.clone(),
                        start_byte,
                        end_byte,
                        kind: VarAccessKind::Use,
                    },
                };
                candidates.insert(ExactFlowEdge::from_legacy(&edge), edge);
            }
        }
        candidates.into_values().collect()
    }

    pub(super) fn store_supplemental_exact(
        destination: &mut BTreeMap<ExactFlowEdge, FlowConfidence>,
        legacy_edges: &[FlowEdge],
        legacy_labels: &BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
        classified: BTreeMap<ExactFlowEdge, FlowConfidence>,
    ) {
        let legacy_exact: BTreeMap<_, _> = legacy_edges
            .iter()
            .map(|edge| {
                let label = legacy_labels
                    .get(&(edge.from.clone(), edge.to.clone()))
                    .copied()
                    .unwrap_or(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete));
                (ExactFlowEdge::from_legacy(edge), label)
            })
            .collect();
        let conflicting_bindings: BTreeSet<_> = classified
            .iter()
            .filter_map(|(edge, label)| {
                legacy_exact
                    .get(edge)
                    .is_some_and(|legacy| legacy != label)
                    .then(|| edge.binding_key())
            })
            .collect();
        if !conflicting_bindings.is_empty() {
            eprintln!(
                "prism: refused {} conflicting exact producer binding(s)",
                conflicting_bindings.len()
            );
        }

        for (edge, label) in classified {
            if legacy_exact.contains_key(&edge)
                || conflicting_bindings.contains(&edge.binding_key())
            {
                continue;
            }
            destination
                .entry(edge)
                .and_modify(|stored| *stored = stored.worst(label))
                .or_insert(label);
        }
    }
}
