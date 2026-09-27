//! Intraprocedural reaching definitions over the existing line-granular CFG.

use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cfg::{self, ArmProvenance, CfgEdge, EdgeOrigin};
use crate::data_flow::{ExactFlowEdge, FlowEdge, VarLocation};
use crate::languages::Language;
#[cfg(test)]
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use tree_sitter::Node;

use super::{FlowConfidence, FlowDoubt};

mod binding_table;
mod capture;
mod cfg_edge_index;
mod classify;
mod compact_wire;
mod def_buckets;
mod grammar_lint;
mod graph;
mod join_kind;
mod scope;
mod scope_python;
mod statement_lookup;
mod transfer;

pub(crate) use cfg_edge_index::FileCfgEdgeIndex;
use classify::classify_edge;
#[cfg(test)]
use classify::matching_defs;
use graph::{reverse_postorder, BitSet};

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
    pub(crate) synthetic: bool,
}

pub(super) struct CfgView<'a> {
    pub(super) line_index: &'a BTreeMap<Line, usize>,
    pub(super) successors: &'a [Vec<(usize, bool)>],
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
    pub(crate) exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    pub(crate) loop_carried_edges: BTreeSet<(VarLocation, VarLocation)>,
    /// May-reaching facts, independent of confidence (including flagged CFG
    /// routes). Used only to admit implicit parameter-to-body entry copies.
    pub(crate) reaching_edges: BTreeSet<(VarLocation, VarLocation)>,
}

type RdFunctionKey = (String, usize);

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RdFileStats {
    pub functions_over_cap: usize,
    pub functions_without_cfg: usize,
    // These identity sets are the persisted source of truth. The public counts
    // remain derived mirrors for the approved Task 2 interface.
    over_cap_function_keys: BTreeSet<RdFunctionKey>,
    without_cfg_function_keys: BTreeSet<RdFunctionKey>,
}

impl RdFileStats {
    // Native compact decoder consistency guard.
    pub(crate) fn counts_match_keys(&self) -> bool {
        self.functions_over_cap == self.over_cap_function_keys.len()
            && self.functions_without_cfg == self.without_cfg_function_keys.len()
    }

    pub(crate) fn record_over_cap(&mut self, function: String, start_line: usize) {
        self.over_cap_function_keys.insert((function, start_line));
        self.refresh_counts();
    }

    pub(crate) fn record_without_cfg(&mut self, function: String, start_line: usize) {
        self.without_cfg_function_keys
            .insert((function, start_line));
        self.refresh_counts();
    }

    pub(crate) fn merge(&mut self, other: Self) {
        self.over_cap_function_keys
            .extend(other.over_cap_function_keys);
        self.without_cfg_function_keys
            .extend(other.without_cfg_function_keys);
        self.refresh_counts();
    }

    fn refresh_counts(&mut self) {
        self.functions_over_cap = self.over_cap_function_keys.len();
        self.functions_without_cfg = self.without_cfg_function_keys.len();
    }
}

#[allow(dead_code)]
pub(crate) fn reaching_definitions(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    dfg_edges: &[FlowEdge],
) -> RdOutcome {
    let cfg_edges = cfg::build_cfg_edges_with_arms(parsed);
    reaching_definitions_with_exact(
        parsed,
        func_node,
        defs,
        dfg_edges,
        &[],
        &FileCfgEdgeIndex::new(&cfg_edges),
    )
}

pub(crate) fn reaching_definitions_with_exact(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    dfg_edges: &[FlowEdge],
    exact_edges: &[FlowEdge],
    file_cfg_edges: &FileCfgEdgeIndex<'_>,
) -> RdOutcome {
    let outcome = reaching_definitions_impl(
        parsed,
        func_node,
        defs,
        dfg_edges,
        exact_edges,
        file_cfg_edges,
    );
    #[cfg(test)]
    cfg_edge_index::tests::record_outcome(&outcome);
    #[cfg(test)]
    join_kind_tests::record_outcome(&outcome);
    #[cfg(test)]
    statement_lookup_tests::record_outcome(&outcome);
    #[cfg(test)]
    def_bucket_tests::record_outcome(&outcome);
    outcome
}

pub(crate) fn classified_masked_reads(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    spans: &[crate::ast::PathSpan],
) -> Vec<crate::ast::PathSpan> {
    scope::classified_masked_reads(parsed, func_node, spans)
}

fn reaching_definitions_impl(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    dfg_edges: &[FlowEdge],
    exact_edges: &[FlowEdge],
    file_cfg_edges: &FileCfgEdgeIndex<'_>,
) -> RdOutcome {
    #[cfg(test)]
    let _work = work_tests::Pass::begin(parsed, func_node);
    #[cfg(test)]
    work_tests::bump(|counts| counts.cfg_file_edges = file_cfg_edges.len());
    let mut defs = deduplicate_definitions(defs);
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

    #[cfg(test)]
    work_tests::bump(|counts| counts.statement_entries = statements.len());
    let lines: Vec<Line> = statements.iter().map(|(line, _)| *line).collect();
    let line_index: BTreeMap<Line, usize> = lines
        .iter()
        .enumerate()
        .map(|(index, line)| (*line, index + 1))
        .collect();
    let mut cfg_edges = Vec::new();
    let kinds = join_kind::StatementKinds::new(&statements);
    for (from, to, edge, provenance) in file_cfg_edges.for_owner(&line_index) {
        cfg_edges.push((
            from,
            to,
            join_kind::is_incomplete_join(parsed, &kinds, &edge, provenance),
        ));
    }
    if cfg_edges.is_empty() {
        return RdOutcome::Unavailable(RdUnavailable::NoCfgEdges);
    }

    // The operation-only census stops after the measured CFG selection. It
    // retains real producer inputs and caps, but does not measure the solver.
    #[cfg(test)]
    if work_tests::count_only() {
        return RdOutcome::Available(RdResult {
            labels: BTreeMap::new(),
            exact_labels: BTreeMap::new(),
            loop_carried_edges: BTreeSet::new(),
            reaching_edges: BTreeSet::new(),
        });
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
    #[cfg(test)]
    work_tests::bump(|counts| counts.statement_span_entries = spans.len());
    let mut statements = statement_lookup::StatementLookup::new(&spans);
    let (function_start, _) = parsed.node_line_range(func_node);
    let mut mapped_defs: Vec<Option<usize>> = defs
        .iter()
        .map(|def| {
            if def.line == function_start && !line_index.contains_key(&def.line) {
                Some(entry)
            } else {
                statements
                    .get(def.line)
                    .and_then(|line| line_index.get(&line).copied())
            }
        })
        .collect();
    let original_successors = successors.clone();
    let cfg_view = CfgView {
        line_index: &line_index,
        successors: &original_successors,
    };
    let missing_real_defs = transfer::discover(parsed, func_node, &defs);
    transfer::materialize_existing_def_sites(
        &missing_real_defs,
        &line_index,
        &mut defs,
        &mut mapped_defs,
    );
    let binders = transfer::discover(parsed, func_node, &defs);
    let applied = transfer::apply(
        parsed,
        &binders,
        &line_index,
        &mut successors,
        &mut predecessors,
        &mut defs,
        &mut mapped_defs,
    );
    if defs.len() > RD_MAX_DEFS {
        return RdOutcome::Unavailable(RdUnavailable::DefinitionsCapExceeded {
            actual: defs.len(),
        });
    }
    if successors.len() > RD_MAX_LINES + 1 {
        return RdOutcome::Unavailable(RdUnavailable::StatementLinesCapExceeded {
            actual: successors.len(),
        });
    }
    let binding_facts = scope::BindingFacts::new(parsed, func_node, &defs, &cfg_view);

    let node_count = successors.len();
    let mut gen = vec![BitSet::new(defs.len()); node_count];
    for (index, mapped) in mapped_defs.iter().enumerate() {
        if let Some(node) = mapped {
            gen[*node].insert(index);
        }
    }
    let buckets = def_buckets::DefinitionBuckets::new(&defs);
    let (kill, flat_kill) = buckets.kill_sets(&defs, &gen, &binding_facts);

    let in_sets = solve_reaching_sets(&gen, &kill, &predecessors, &successors, entry, defs.len());
    let flat_in_sets = solve_reaching_sets(
        &gen,
        &flat_kill,
        &predecessors,
        &successors,
        entry,
        defs.len(),
    );

    #[cfg(test)]
    join_kind_tests::record_sets(&in_sets, &flat_in_sets);
    #[cfg(test)]
    statement_lookup_tests::record_sets(&in_sets, &flat_in_sets);
    #[cfg(test)]
    def_bucket_tests::record_sets(&in_sets, &flat_in_sets, &kill, &flat_kill);
    let collapsed = collapsed_groups(&defs);
    let capture_facts = capture::capture_facts(parsed, *func_node);
    let mut labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence> = BTreeMap::new();
    let mut exact_labels = BTreeMap::new();
    let mut loop_carried_edges = BTreeSet::new();
    let mut reaching_edges = BTreeSet::new();
    for edge in dfg_edges {
        let key = (edge.from.clone(), edge.to.clone());
        let candidates = buckets.matching_defs(edge);
        if statements
            .get(edge.to.line)
            .and_then(|line| line_index.get(&line))
            .is_some_and(|node| {
                candidates
                    .iter()
                    .any(|index| in_sets[*node].contains(*index))
            })
        {
            reaching_edges.insert(key.clone());
        }
        let label = classify_edge(
            edge,
            parsed,
            &defs,
            &candidates,
            &mapped_defs,
            &line_index,
            &mut statements,
            &in_sets,
            &kill,
            &flat_in_sets,
            &flat_kill,
            &successors,
            &applied.synthetic_nodes,
            &collapsed,
            &capture_facts,
            &binding_facts,
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
    for edge in exact_edges {
        let key = ExactFlowEdge::from_legacy(edge);
        let candidates = buckets.matching_defs(edge);
        let label = classify_edge(
            edge,
            parsed,
            &defs,
            &candidates,
            &mapped_defs,
            &line_index,
            &mut statements,
            &in_sets,
            &kill,
            &flat_in_sets,
            &flat_kill,
            &successors,
            &applied.synthetic_nodes,
            &collapsed,
            &capture_facts,
            &binding_facts,
            function_start,
        );
        exact_labels
            .entry(key)
            .and_modify(|stored: &mut FlowConfidence| *stored = stored.worst(label))
            .or_insert(label);
    }

    RdOutcome::Available(RdResult {
        labels,
        exact_labels,
        loop_carried_edges,
        reaching_edges,
    })
}

fn solve_reaching_sets(
    gen: &[BitSet],
    kill: &[BitSet],
    predecessors: &[Vec<usize>],
    successors: &[Vec<(usize, bool)>],
    entry: usize,
    def_count: usize,
) -> Vec<BitSet> {
    let mut in_sets = vec![BitSet::new(def_count); gen.len()];
    let mut out_sets = vec![BitSet::new(def_count); gen.len()];
    let order = reverse_postorder(successors, entry);
    let mut worklist: VecDeque<usize> = order.iter().copied().collect();
    let mut queued = vec![false; gen.len()];
    for node in &order {
        queued[*node] = true;
    }
    while let Some(node) = worklist.pop_front() {
        queued[node] = false;
        let mut incoming = BitSet::new(def_count);
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

    in_sets
}

fn deduplicate_definitions(defs: &[DefSite]) -> Vec<DefSite> {
    let mut unique = Vec::<DefSite>::new();
    let mut occurrences = BTreeMap::<(AccessPath, Line, usize), usize>::new();
    for def in defs.iter().filter(|def| !def.synthetic) {
        let occurrence = (def.path.clone(), def.line, def.start_byte);
        if let Some(index) = occurrences.get(&occurrence).copied() {
            unique[index].alias_derived |= def.alias_derived;
        } else {
            occurrences.insert(occurrence, unique.len());
            unique.push(def.clone());
        }
    }
    unique
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

// Frozen pre-memo scanner: independent differential oracle.
#[cfg(test)]
fn innermost_statement(line: Line, spans: &[StatementLineSpan]) -> Option<Line> {
    spans
        .iter()
        .filter(|span| {
            #[cfg(test)]
            work_tests::bump(|counts| counts.statement_span_visits += 1);
            span.start_line <= line && line <= span.end_line
        })
        .min_by_key(|span| {
            (
                span.end_byte.saturating_sub(span.start_byte),
                Reverse(span.start_byte),
                Reverse(span.start_line),
            )
        })
        .map(|span| span.start_line)
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod work_tests;

#[cfg(test)]
mod join_kind_tests;

#[cfg(test)]
mod statement_lookup_tests;

#[cfg(test)]
mod def_bucket_tests;
