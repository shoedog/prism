//! Go-only cycle membership for RD and public loop-carried telemetry.
use crate::ast::ParsedFile;
use crate::cfg::{self, CfgEdge};
use crate::languages::Language;
use petgraph::{algo::kosaraju_scc, graph::DiGraph};
use std::collections::BTreeMap;

pub(super) fn components(successors: &[Vec<(usize, bool)>]) -> Vec<Option<usize>> {
    let mut graph = DiGraph::<(), ()>::new();
    let nodes: Vec<_> = successors.iter().map(|_| graph.add_node(())).collect();
    for (from, edges) in successors.iter().enumerate() {
        for &(to, _) in edges {
            graph.add_edge(nodes[from], nodes[to], ());
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
    edges: BTreeMap<String, Vec<CfgEdge>>,
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
                let lines: BTreeMap<_, _> = parsed
                    .statements_in_function(&function)
                    .into_iter()
                    .enumerate()
                    .map(|(index, (line, _))| (line, index))
                    .collect();
                let mut successors = vec![Vec::new(); lines.len()];
                let edges = self
                    .edges
                    .entry(file.to_owned())
                    .or_insert_with(|| cfg::build_cfg_edges(parsed));
                for edge in edges {
                    if let (Some(&from), Some(&to)) =
                        (lines.get(&edge.from_line), lines.get(&edge.to_line))
                    {
                        successors[from].push((to, false));
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
