//! #660 S1b-2: D3, the narrow same-node order refusal (DESIGN.md 7, 8).
//!
//! Reaching definitions sees a CFG node's input set, so it cannot order a write against a read inside
//! one node. For an edge D->U that would be `exact`, D3 asks the S0 `precedes` relation:
//! - use side (U in node N, D outside): every counting write W of N that affects U's path must be
//!   proven after U (`precedes(U, W)`), except the single-site operand exception (E2);
//! - def side (D in N, U outside): every other counting write W of N that affects D's path must be
//!   proven before D (`precedes(W, D)`).
//!
//! Anything unproven is `NameOnly(SameLine)`: label-only, never availability or an edge. A use with
//! no real bytes is placed on its read occurrences by the line-keyed lookup S1a uses (`read_index`);
//! one that cannot be placed on any occurrence, or whose occurrences disagree, refuses.
use super::read_index::{read_node, ReadIndex};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingRelation};
use crate::data_flow::{FlowEdge, VarLocation};
use crate::js_write_census::counting::{Counting, Counts};
use crate::js_write_census::order::{order, Order, Position};
use crate::js_write_census::tables::Tables;
use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;

/// Trace mechanisms (DESIGN.md 10).
pub(super) const D3_USE: &str = "d3-use";
pub(super) const D3_DEF: &str = "d3-def";
pub(super) const D3_LOOP: &str = "d3-loop";
pub(super) const D3_UNKNOWN: &str = "d3-unknown";

type EdgeKey = (VarLocation, VarLocation);

/// One owner's D3 state, shared (not copied) by the census facts' clones.
pub(super) struct D3 {
    counts: Counts,
    /// The owner callable (`kind`, byte range): found again by bytes when the reads are first needed.
    owner: (&'static str, usize, usize),
    reads: OnceCell<ReadIndex>,
    /// The mechanism of each edge D3 refused, kept only while `PRISM_RD_CENSUS_TRACE` is on.
    log: Option<RefCell<BTreeMap<EdgeKey, &'static str>>>,
}

/// What the unproven positions of one query are, strongest first: a byte order that is wrong, a loop
/// chain, anything else.
#[derive(Default)]
struct Failures {
    byte: bool,
    repeated: bool,
    unknown: bool,
}

impl Failures {
    fn note(&mut self, o: Order) {
        match o {
            Order::Proven => {}
            Order::Byte => self.byte = true,
            Order::Loop => self.repeated = true,
            Order::Unknown => self.unknown = true,
        }
    }

    fn any(&self) -> bool {
        self.byte || self.repeated || self.unknown
    }

    fn mechanism(&self, byte: &'static str) -> &'static str {
        if self.byte {
            byte
        } else if self.repeated {
            D3_LOOP
        } else {
            D3_UNKNOWN
        }
    }
}

impl D3 {
    pub(super) fn new(counts: Counts, owner: &tree_sitter::Node<'_>, trace: bool) -> Option<Self> {
        (!counts.is_empty()).then(|| D3 {
            counts,
            owner: (owner.kind(), owner.start_byte(), owner.end_byte()),
            reads: OnceCell::new(),
            log: trace.then(|| RefCell::new(BTreeMap::new())),
        })
    }

    #[cfg(test)]
    pub(super) fn counts(&self) -> &Counts {
        &self.counts
    }

    /// The mechanism D3 recorded for an edge it refused (trace on).
    pub(super) fn logged(&self, edge: &FlowEdge) -> Option<&'static str> {
        self.log
            .as_ref()?
            .borrow()
            .get(&(edge.from.clone(), edge.to.clone()))
            .copied()
    }

    /// `Some(mechanism)` when the edge D->U, classified `exact`, is not proven ordered inside its
    /// node. `use_line` and `def_line` are the CFG nodes of U and D.
    pub(super) fn refuse(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        use_line: usize,
        def_line: usize,
        candidates: &[usize],
        facts: &BindingFacts,
    ) -> Option<&'static str> {
        if use_line == def_line {
            return None; // one node: main's collapse and same-line rules own it
        }
        let hit = self
            .use_side(parsed, edge, use_line, candidates, facts)
            .or_else(|| self.def_side(edge, def_line, facts))?;
        if let Some(log) = &self.log {
            log.borrow_mut()
                .insert((edge.from.clone(), edge.to.clone()), hit);
        }
        Some(hit)
    }

    fn affecting<'s>(
        &'s self,
        node: usize,
        path: &'s AccessPath,
    ) -> impl Iterator<Item = &'s Counting> {
        self.counts
            .by_node
            .get(&node)
            .into_iter()
            .flatten()
            .filter(move |w| super::def_buckets::affects(&w.path, path))
    }

    fn use_side(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        use_line: usize,
        candidates: &[usize],
        facts: &BindingFacts,
    ) -> Option<&'static str> {
        let writes: Vec<&Counting> = self.affecting(use_line, &edge.to.path).collect();
        if writes.is_empty() && !self.counts.unplaced.contains(&edge.to.path.base) {
            return None;
        }
        if writes.is_empty() {
            return Some(D3_UNKNOWN);
        }
        let Some(occurrences) = self.occurrences(parsed, edge, use_line) else {
            return None; // anchored at a write's own target: no read to order
        };
        if occurrences.is_empty() {
            return Some(D3_UNKNOWN);
        }
        let t = Tables::shared();
        let mut failures = Failures::default();
        for (start, end) in occurrences {
            // A read of another binding of the name (an inner `let` shadowing D's) is not D's use.
            let base = &edge.to.path.base;
            if !candidates.is_empty()
                && candidates.iter().all(|&c| {
                    matches!(
                        facts.relation_at(base, start, c),
                        BindingRelation::KilledAt(_)
                    )
                })
            {
                continue;
            }
            let Some(node) = read_node(parsed, start, end) else {
                failures.unknown = true;
                continue;
            };
            let place = Position::of_read(node, &self.counts.spans);
            let counting: Vec<&&Counting> = writes
                .iter()
                .filter(|w| !facts.proven_distinct_at(&edge.to.path.base, w.probe, start))
                .collect();
            if let [only] = counting[..] {
                if only.is_operand_of(start) {
                    continue; // E2: the one write reads its operand before it writes
                }
            }
            for w in counting {
                failures.note(order(t, &place, &w.pos));
            }
        }
        failures.any().then(|| failures.mechanism(D3_USE))
    }

    fn def_side(
        &self,
        edge: &FlowEdge,
        def_line: usize,
        facts: &BindingFacts,
    ) -> Option<&'static str> {
        let writes: Vec<&Counting> = self.affecting(def_line, &edge.from.path).collect();
        let d_start = edge.from.start_byte;
        // D's own site: a recorded def (a declarator, a plain write) is positioned at its construct.
        let d = writes.iter().find(|w| w.target_start == d_start);
        let d_probe = d.map_or(d_start, |d| d.probe);
        let others: Vec<&&Counting> = writes
            .iter()
            .filter(|w| w.target_start != d_start)
            .filter(|w| !facts.proven_distinct_at(&edge.from.path.base, w.probe, d_probe))
            .collect();
        if others.is_empty() {
            return None;
        }
        // A def with no census site in this node (a formal on the node's line) is unplaced.
        let Some(d) = d else {
            return Some(D3_UNKNOWN);
        };
        let t = Tables::shared();
        let mut failures = Failures::default();
        for w in others {
            failures.note(order(t, &w.pos, &d.pos));
        }
        failures.any().then(|| failures.mechanism(D3_DEF))
    }

    /// The read occurrences of the edge's use: every read on its line that spells its path (a write
    /// target, which is no read, never counts) and the use's own bytes. The DFG keeps one use per
    /// path and line, at its first occurrence, so the edge speaks for the line's reads: a later
    /// occurrence the write precedes makes the whole edge unproven.
    fn occurrences(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        use_line: usize,
    ) -> Option<Vec<(usize, usize)>> {
        let to = &edge.to;
        let reads = self.reads.get_or_init(|| {
            let (kind, start, end) = self.owner;
            let mut node = parsed
                .tree
                .root_node()
                .descendant_for_byte_range(start, end);
            while let Some(n) = node {
                if n.kind() == kind && n.start_byte() == start && n.end_byte() == end {
                    return ReadIndex::build(n);
                }
                node = n.parent();
            }
            ReadIndex::build(parsed.tree.root_node())
        });
        // Only reads of this node: its innermost statement span starts on the node's line.
        let on_line: Vec<(usize, usize)> = reads
            .on_line(parsed, &to.path, to.line)
            .into_iter()
            .filter(|(start, _)| self.span_line_of(parsed, *start) == Some(use_line))
            .collect();
        let (targets, mut found): (Vec<_>, Vec<_>) = on_line
            .into_iter()
            .partition(|r| self.counts.write_only.contains(r));
        if found.is_empty() && !targets.is_empty() {
            // The DFG also anchors a use at a write's own target (`value = other`, `var [f] = ..`):
            // no read of the line spells the path, so there is nothing to order.
            return None;
        }
        let own = (to.start_byte, to.end_byte);
        if to.end_byte > to.start_byte && !found.contains(&own) {
            found.push(own);
        }
        Some(found)
    }

    fn span_line_of(&self, parsed: &ParsedFile, byte: usize) -> Option<usize> {
        self.counts
            .spans
            .iter()
            .filter(|(a, b)| *a <= byte && byte < *b)
            .min_by_key(|(a, b)| (b - a, std::cmp::Reverse(*a)))
            .map(|(a, _)| parsed.line_for_byte(*a))
    }
}
