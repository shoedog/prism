//! Intraprocedural reaching definitions over the existing line-granular CFG.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cfg::{self, ArmProvenance, CfgEdge, EdgeOrigin};
use crate::data_flow::{FlowEdge, VarLocation};
use crate::languages::Language;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tree_sitter::Node;

use super::{FlowConfidence, FlowDoubt};

mod capture;

/// Hard caps from the authorised measurement pass
/// (~/code/tools/logs/item2-census/REPORT.md §2.3, 92,338 functions).
/// RD_MAX_LINES bounds `stmt_lines.len()` — the CFG statement-line universe
/// returned by `ParsedFile::statements_in_function` — NOT the function's line
/// span (`end - start + 1`). Measured worst case: 590 defs, 331 statement
/// lines; 0 of 92,338 functions exceed either cap.
pub(crate) const RD_MAX_DEFS: usize = 2048;
pub(crate) const RD_MAX_LINES: usize = 4096;

pub(crate) type Line = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct DefId(pub(crate) u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DefSite {
    pub(crate) id: DefId,
    pub(crate) path: AccessPath,
    pub(crate) line: Line,
    pub(crate) start_byte: usize,
    pub(crate) alias_derived: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RdUnavailable {
    DefinitionsCapExceeded { actual: usize },
    StatementLinesCapExceeded { actual: usize },
    NoCfgEdges,
}

impl RdUnavailable {
    pub(crate) fn is_def_cap(self) -> bool {
        matches!(self, Self::DefinitionsCapExceeded { .. })
    }

    pub(crate) fn is_line_cap(self) -> bool {
        matches!(self, Self::StatementLinesCapExceeded { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RdOutcome {
    Available(RdResult),
    Unavailable(RdUnavailable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RdResult {
    pub(crate) labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    pub(crate) loop_carried_edges: BTreeSet<(VarLocation, VarLocation)>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RdFileStats {
    pub functions_over_cap: usize,
    pub functions_without_cfg: usize,
}

pub(crate) fn reaching_definitions(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    dfg_edges: &[FlowEdge],
) -> RdOutcome {
    if defs.len() > RD_MAX_DEFS {
        return RdOutcome::Unavailable(RdUnavailable::DefinitionsCapExceeded {
            actual: defs.len(),
        });
    }

    let statements = parsed.statements_in_function(func_node);
    if statements.len() > RD_MAX_LINES {
        return RdOutcome::Unavailable(RdUnavailable::StatementLinesCapExceeded {
            actual: statements.len(),
        });
    }

    let lines: Vec<Line> = statements.iter().map(|(line, _)| *line).collect();
    let line_index: BTreeMap<Line, usize> = lines
        .iter()
        .enumerate()
        .map(|(index, line)| (*line, index + 1))
        .collect();
    let mut cfg_edges = Vec::new();
    for (edge, provenance) in cfg::build_cfg_edges_with_arms(parsed) {
        let (Some(&from), Some(&to)) = (
            line_index.get(&edge.from_line),
            line_index.get(&edge.to_line),
        ) else {
            continue;
        };
        cfg_edges.push((
            from,
            to,
            is_incomplete_join(parsed, &statements, &edge, provenance),
        ));
    }
    if cfg_edges.is_empty() {
        return RdOutcome::Unavailable(RdUnavailable::NoCfgEdges);
    }

    let node_count = lines.len() + 1;
    let entry = 0;
    let mut successors = vec![Vec::new(); node_count];
    let mut predecessors = vec![Vec::new(); node_count];
    for (from, to, incomplete) in cfg_edges {
        successors[from].push((to, incomplete));
        predecessors[to].push(from);
    }
    if let Some(first) = lines.first().and_then(|line| line_index.get(line)) {
        successors[entry].push((*first, false));
        predecessors[*first].push(entry);
    }

    let spans = statement_line_spans(parsed, func_node);
    let (function_start, _) = parsed.node_line_range(func_node);
    let mapped_defs: Vec<Option<usize>> = defs
        .iter()
        .map(|def| {
            if def.line == function_start && !line_index.contains_key(&def.line) {
                Some(entry)
            } else {
                innermost_statement(def.line, &spans)
                    .and_then(|line| line_index.get(&line).copied())
            }
        })
        .collect();

    let mut gen = vec![BitSet::new(defs.len()); node_count];
    for (index, mapped) in mapped_defs.iter().enumerate() {
        if let Some(node) = mapped {
            gen[*node].insert(index);
        }
    }
    let mut kill = vec![BitSet::new(defs.len()); node_count];
    for node in 0..node_count {
        let generated: Vec<usize> = gen[node].members().collect();
        for &new_def in &generated {
            if defs[new_def].alias_derived {
                continue;
            }
            for old_def in 0..defs.len() {
                if !gen[node].contains(old_def)
                    && !defs[old_def].alias_derived
                    && defs[old_def].path == defs[new_def].path
                {
                    kill[node].insert(old_def);
                }
            }
        }
    }

    let mut in_sets = vec![BitSet::new(defs.len()); node_count];
    let mut out_sets = vec![BitSet::new(defs.len()); node_count];
    let order = reverse_postorder(&successors, entry);
    let mut worklist: VecDeque<usize> = order.iter().copied().collect();
    let mut queued = vec![false; node_count];
    for node in &order {
        queued[*node] = true;
    }
    while let Some(node) = worklist.pop_front() {
        queued[node] = false;
        let mut incoming = BitSet::new(defs.len());
        for predecessor in &predecessors[node] {
            incoming.union_with(&out_sets[*predecessor]);
        }
        let mut outgoing = incoming.clone();
        outgoing.subtract(&kill[node]);
        outgoing.union_with(&gen[node]);
        in_sets[node] = incoming;
        if outgoing != out_sets[node] {
            out_sets[node] = outgoing;
            for (successor, _) in &successors[node] {
                if !queued[*successor] {
                    queued[*successor] = true;
                    worklist.push_back(*successor);
                }
            }
        }
    }

    let collapsed = collapsed_groups(defs);
    let capture_facts = capture::capture_facts(parsed, *func_node);
    let mut labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence> = BTreeMap::new();
    let mut loop_carried_edges = BTreeSet::new();
    for edge in dfg_edges {
        let key = (edge.from.clone(), edge.to.clone());
        let label = classify_edge(
            edge,
            defs,
            &mapped_defs,
            &line_index,
            &spans,
            &in_sets,
            &kill,
            &successors,
            &collapsed,
            &capture_facts,
            function_start,
        );
        if label.is_exact() && edge.to.line < edge.from.line {
            loop_carried_edges.insert(key.clone());
        }
        labels
            .entry(key)
            .and_modify(|stored| *stored = stored.worst(label))
            .or_insert(label);
    }

    RdOutcome::Available(RdResult {
        labels,
        loop_carried_edges,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BitSet(Vec<u64>);

impl BitSet {
    fn new(bits: usize) -> Self {
        Self(vec![0; bits.div_ceil(64)])
    }

    fn insert(&mut self, bit: usize) {
        self.0[bit / 64] |= 1 << (bit % 64);
    }

    fn contains(&self, bit: usize) -> bool {
        self.0
            .get(bit / 64)
            .is_some_and(|word| word & (1 << (bit % 64)) != 0)
    }

    fn members(&self) -> impl Iterator<Item = usize> + '_ {
        self.0.iter().enumerate().flat_map(|(word_index, word)| {
            (0..64).filter_map(move |bit| (word & (1 << bit) != 0).then_some(word_index * 64 + bit))
        })
    }

    fn union_with(&mut self, other: &Self) {
        for (left, right) in self.0.iter_mut().zip(&other.0) {
            *left |= right;
        }
    }

    fn subtract(&mut self, other: &Self) {
        for (left, right) in self.0.iter_mut().zip(&other.0) {
            *left &= !right;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn classify_edge(
    edge: &FlowEdge,
    defs: &[DefSite],
    mapped_defs: &[Option<usize>],
    line_index: &BTreeMap<Line, usize>,
    spans: &[StatementLineSpan],
    in_sets: &[BitSet],
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    collapsed: &BTreeSet<(AccessPath, Line)>,
    capture_facts: &capture::CaptureFacts,
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
    if def_line == use_line
        || collapsed.contains(&(edge.from.path.clone(), edge.from.line))
        || collapsed.contains(&(edge.to.path.clone(), edge.to.line))
    {
        return FlowConfidence::NameOnly(FlowDoubt::SameLine);
    }

    if candidates.iter().any(|index| defs[*index].alias_derived) {
        return FlowConfidence::NameOnly(FlowDoubt::AliasUnstable);
    }
    if !reaching_candidates.is_empty() {
        return FlowConfidence::Exact;
    }

    let kill_line = candidates
        .iter()
        .filter_map(|index| {
            lowest_reachable_kill(
                *index,
                defs[*index].line,
                mapped_defs[*index],
                kill,
                successors,
                line_index,
            )
        })
        .min()
        .unwrap_or(edge.from.line);
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
    def_line: Line,
    def_node: Option<usize>,
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
    line_index: &BTreeMap<Line, usize>,
) -> Option<Line> {
    let def_node = def_node?;
    line_index
        .iter()
        .filter_map(|(line, node)| {
            (*line > def_line
                && kill[*node].contains(def_index)
                && path_exists(successors, def_node, *node, true))
            .then_some(*line)
        })
        .min()
}

fn collapsed_groups(defs: &[DefSite]) -> BTreeSet<(AccessPath, Line)> {
    let mut counts = BTreeMap::new();
    for def in defs {
        *counts.entry((def.path.clone(), def.line)).or_insert(0usize) += 1;
    }
    counts
        .into_iter()
        .filter_map(|(group, count)| (count >= 2).then_some(group))
        .collect()
}

#[derive(Clone, Copy)]
struct StatementLineSpan {
    start_line: Line,
    end_line: Line,
    start_byte: usize,
    end_byte: usize,
}

fn statement_line_spans(parsed: &ParsedFile, func_node: &Node<'_>) -> Vec<StatementLineSpan> {
    parsed
        .statement_spans_in_function(func_node)
        .into_iter()
        .map(|span| StatementLineSpan {
            start_line: span.line,
            end_line: parsed.line_for_byte(span.end_byte.saturating_sub(1)),
            start_byte: span.start_byte,
            end_byte: span.end_byte,
        })
        .collect()
}

fn innermost_statement(line: Line, spans: &[StatementLineSpan]) -> Option<Line> {
    spans
        .iter()
        .filter(|span| span.start_line <= line && line <= span.end_line)
        .min_by_key(|span| {
            (
                span.end_byte.saturating_sub(span.start_byte),
                Reverse(span.start_byte),
                Reverse(span.start_line),
            )
        })
        .map(|span| span.start_line)
}

fn reverse_postorder(successors: &[Vec<(usize, bool)>], entry: usize) -> Vec<usize> {
    let mut seen = vec![false; successors.len()];
    let mut stack = vec![(entry, false)];
    let mut postorder = Vec::new();
    while let Some((node, expanded)) = stack.pop() {
        if expanded {
            postorder.push(node);
            continue;
        }
        if seen[node] {
            continue;
        }
        seen[node] = true;
        stack.push((node, true));
        for (successor, _) in successors[node].iter().rev() {
            if !seen[*successor] {
                stack.push((*successor, false));
            }
        }
    }
    postorder.reverse();
    postorder
}

fn path_exists(
    successors: &[Vec<(usize, bool)>],
    from: usize,
    to: usize,
    allow_incomplete: bool,
) -> bool {
    let mut seen = vec![false; successors.len()];
    let mut queue = VecDeque::from([from]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            return true;
        }
        if seen[node] {
            continue;
        }
        seen[node] = true;
        for (successor, incomplete) in &successors[node] {
            if (allow_incomplete || !incomplete) && !seen[*successor] {
                queue.push_back(*successor);
            }
        }
    }
    false
}

fn definition_reaches_unflagged(
    def_index: usize,
    from: usize,
    to: usize,
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
) -> bool {
    let mut seen = vec![false; successors.len()];
    let mut queue = VecDeque::from([from]);
    while let Some(node) = queue.pop_front() {
        if node == to {
            return true;
        }
        if seen[node] {
            continue;
        }
        seen[node] = true;
        for (successor, incomplete) in &successors[node] {
            if !incomplete && !seen[*successor] {
                if *successor == to {
                    return true;
                }
                if !kill[*successor].contains(def_index) {
                    queue.push_back(*successor);
                }
            }
        }
    }
    false
}

fn is_incomplete_join(
    parsed: &ParsedFile,
    statements: &[(Line, String)],
    edge: &CfgEdge,
    provenance: ArmProvenance,
) -> bool {
    if provenance.crosses_lexical_arm() {
        return true;
    }
    if !matches!(provenance.origin, EdgeOrigin::Structured) {
        return false;
    }
    let kinds: BTreeMap<Line, &str> = statements
        .iter()
        .map(|(line, kind)| (*line, kind.as_str()))
        .collect();
    let from_kind = kinds.get(&edge.from_line).copied().unwrap_or("");
    let to_kind = kinds.get(&edge.to_line).copied().unwrap_or("");
    let try_join = matches!(
        parsed.language,
        Language::Python
            | Language::JavaScript
            | Language::TypeScript
            | Language::Tsx
            | Language::Java
    ) && from_kind == "try_statement";
    let go_defer = parsed.language == Language::Go
        && parsed.language.is_return_node(from_kind)
        && to_kind == "defer_statement";
    try_join || go_defer
}

#[cfg(test)]
mod tests;
