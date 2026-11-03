//! Go-only cycle membership for RD and public loop-carried telemetry.
use super::reaching::join_kind::{is_incomplete_join, StatementKinds};
use crate::ast::ParsedFile;
use crate::cfg::{self, ArmProvenance, CfgEdge};
use crate::languages::Language;
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use std::collections::BTreeMap;

pub(super) fn components(successors: &[Vec<(usize, bool)>]) -> Vec<Option<usize>> {
    let mut graph = DiGraph::<(), ()>::new();
    let nodes: Vec<_> = successors.iter().map(|_| graph.add_node(())).collect();
    for (from, edges) in successors.iter().enumerate() {
        for &(to, incomplete) in edges {
            // Incomplete joins may carry may-reaching facts, but cannot prove a cycle.
            if !incomplete {
                graph.add_edge(nodes[from], nodes[to], ());
            }
        }
    }
    let mut result = vec![None; nodes.len()];
    for (id, group) in kosaraju_scc(&graph).into_iter().enumerate() {
        if group.len() > 1 || graph.contains_edge(group[0], group[0]) {
            for node in group {
                result[node.index()] = Some(id);
            }
        }
    }
    result
}

struct Owner {
    // Same innermost statement-line projection used by the RD solver.
    spans: Vec<(usize, usize, usize, usize)>,
    lines: BTreeMap<usize, usize>,
    components: Vec<Option<usize>>,
}
impl Owner {
    fn node(&self, line: usize) -> Option<usize> {
        let span = self
            .spans
            .iter()
            .filter(|s| s.0 <= line && line <= s.1)
            .min_by_key(|s| (s.3 - s.2, std::cmp::Reverse(s.2), std::cmp::Reverse(s.0)))?;
        self.lines.get(&span.0).copied()
    }
    fn same_cycle(&self, from: usize, to: usize) -> bool {
        let (Some(from), Some(to)) = (self.node(from), self.node(to)) else {
            return false;
        };
        self.components[from].is_some() && self.components[from] == self.components[to]
    }
}

#[derive(Default)]
pub(super) struct GoLoopCycles {
    edges: BTreeMap<String, Vec<(CfgEdge, ArmProvenance)>>,
    owners: BTreeMap<(String, usize), Option<Owner>>,
}
impl GoLoopCycles {
    pub(super) fn is_loop_carried(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        file: &str,
        owner_start: usize,
        from: usize,
        to: usize,
    ) -> bool {
        if to >= from {
            return false;
        }
        let Some(parsed) = files.get(file) else {
            return true;
        };
        if parsed.language != Language::Go {
            return true;
        }
        let owner = self
            .owners
            .entry((file.to_owned(), owner_start))
            .or_insert_with(|| {
                let mut candidates = parsed
                    .all_functions()
                    .into_iter()
                    .filter(|f| f.start_position().row + 1 == owner_start);
                let function = candidates.next()?;
                if candidates.next().is_some() {
                    return None;
                }
                let statements = parsed.statements_in_function(&function);
                let kinds = StatementKinds::new(&statements);
                let lines: BTreeMap<_, _> = statements
                    .iter()
                    .enumerate()
                    .map(|(index, (line, _))| (*line, index))
                    .collect();
                let mut successors = vec![Vec::new(); lines.len()];
                let edges = self
                    .edges
                    .entry(file.to_owned())
                    .or_insert_with(|| cfg::build_cfg_edges_with_arms(parsed));
                for (edge, provenance) in edges {
                    if let (Some(&from), Some(&to)) =
                        (lines.get(&edge.from_line), lines.get(&edge.to_line))
                    {
                        successors[from]
                            .push((to, is_incomplete_join(parsed, &kinds, edge, *provenance)));
                    }
                }
                let spans = parsed
                    .statement_spans_in_function(&function)
                    .into_iter()
                    .map(|span| {
                        (
                            span.line,
                            parsed.line_for_byte(span.end_byte.saturating_sub(1)),
                            span.start_byte,
                            span.end_byte,
                        )
                    })
                    .collect();
                Some(Owner {
                    spans,
                    lines,
                    components: components(&successors),
                })
            });
        owner.as_ref().is_some_and(|o| o.same_cycle(from, to))
    }
}

#[cfg(test)]
mod tests {
    use super::components;

    #[test]
    fn go_fold_incomplete_edge_cannot_close_cycle() {
        assert_eq!(
            components(&[vec![(1, false)], vec![(0, true)]]),
            vec![None, None]
        );
    }

    #[test]
    fn go_fold_incomplete_self_edge_is_not_cycle() {
        assert_eq!(components(&[vec![(0, true)]]), vec![None]);
    }

    #[test]
    fn go_fold_admissible_cycles_survive_extra_incomplete_edges() {
        let groups = components(&[
            vec![(1, false), (2, true)],
            vec![(0, false)],
            vec![(2, false)],
        ]);
        assert!(groups[0].is_some());
        assert_eq!(groups[0], groups[1]);
        assert!(groups[2].is_some());
        assert_ne!(groups[0], groups[2]);
    }
}
