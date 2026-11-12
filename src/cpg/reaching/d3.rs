//! #660 S1b-2: D3, the narrow same-node order refusal (DESIGN.md 7, 8).
//!
//! Reaching definitions sees a CFG node's input set, so it cannot order a write against a read inside
//! one node. For an edge D->U that would be `exact`, D3 asks the S0 `precedes` relation:
//! - use side (U in node N, D outside): every counting write W of N that affects U's path must be
//!   proven after U (`precedes(U, W)`), except the single-site operand exception (E2);
//! - def side (D in N, U outside): every other counting write W of N that affects D's path must be
//!   proven before D (`precedes(W, D)`).
//!
//! Which node a position is in is RD's answer and nobody else's (#696): a write is in the node
//! `StatementLookup` gives its line (the rule `mapped_defs` uses for a def), a read is in the node of the
//! line the DFG put the use on. The census's E18 placement and `Position` only order two positions
//! inside one node.
//!
//! Anything unproven is `NameOnly(SameLine)`: label-only, never availability or an edge. A use with
//! no real bytes is placed on its read occurrences by the line-keyed lookup S1a uses (`read_index`);
//! one that cannot be placed on any occurrence, or whose occurrences disagree, refuses.
use super::read_index::{read_node, ReadIndex};
use super::statement_lookup::StatementLookup;
use super::{FlowConfidence, FlowDoubt, Line};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingRelation};
use crate::data_flow::{FlowEdge, VarLocation};
use crate::js_write_census::counting::{Counting, Counts};
use crate::js_write_census::order::{order, Order, Position};
use crate::js_write_census::tables::Tables;
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

/// Trace mechanisms (DESIGN.md 10).
pub(super) const D3_USE: &str = "d3-use";
pub(super) const D3_DEF: &str = "d3-def";
pub(super) const D3_LOOP: &str = "d3-loop";
pub(super) const D3_UNKNOWN: &str = "d3-unknown";

#[cfg(test)]
thread_local! {
    /// Test seam: D3 off, to compare the solver with and without it.
    pub(super) static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

type EdgeKey = (VarLocation, VarLocation);

/// One owner's D3 state, shared (not copied) by the census facts' clones.
pub(super) struct D3 {
    counts: Rc<Counts>,
    /// #783 option A: the owner's alias-resolved writes (`u.x = 1` through a certified alias, as the
    /// aliased path `o.x`) that join `counts.writes` in node order, and the bases of those with no node.
    /// Empty for the census's own D3; the alias refinement's D3 shares the counts and the read index.
    extra: Vec<Counting>,
    extra_unplaced: BTreeSet<String>,
    /// The owner callable (`kind`, byte range): found again by bytes when the reads are first needed.
    owner: (&'static str, usize, usize),
    reads: Rc<OnceCell<ReadIndex>>,
    /// The counting writes by RD's node, built once per owner on the first edge D3 sees.
    nodes: OnceCell<NodeIndex>,
    /// The mechanism of each edge D3 refused, kept only while `PRISM_RD_CENSUS_TRACE` is on.
    log: Option<RefCell<BTreeMap<EdgeKey, &'static str>>>,
}

/// The counting writes of one owner by the node RD maps their line to.
pub(super) struct NodeIndex {
    /// Indices into `Counts::writes`, then (past its length) into `D3::extra`.
    pub(super) by_node: HashMap<usize, Vec<usize>>,
    /// Bases with a counting write on a line RD maps to no node: unknown, like an unplaced write.
    pub(super) unmapped: BTreeSet<String>,
}

impl NodeIndex {
    pub(super) fn build(
        counts: &Counts,
        extra: &[Counting],
        statements: &mut StatementLookup<'_>,
    ) -> Self {
        let mut by_node: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut unmapped = BTreeSet::new();
        for (i, w) in counts.writes.iter().chain(extra).enumerate() {
            match statements.get(w.line) {
                Some(node) => by_node.entry(node).or_default().push(i),
                None => {
                    unmapped.insert(w.path.base.clone());
                }
            }
        }
        Self { by_node, unmapped }
    }

    /// A counting write of `base` has no node (E18 could not place it, or RD maps its line to none):
    /// every D3 query of the base is unknown.
    pub(super) fn base_is_unknown(&self, counts: &Counts, base: &str) -> bool {
        counts.unplaced.contains(base) || self.unmapped.contains(base)
    }
}

/// What the unproven positions of one query are, strongest first: a byte order that is wrong, a loop
/// chain, anything else.
#[derive(Default)]
struct Failures {
    /// A plain byte-order failure on the use side / the def side.
    byte_use: bool,
    byte_def: bool,
    repeated: bool,
    unknown: bool,
}

impl Failures {
    fn note(&mut self, o: Order, side: Side) {
        match (o, side) {
            (Order::Proven, _) => {}
            (Order::Byte, Side::Use) => self.byte_use = true,
            (Order::Byte, Side::Def) => self.byte_def = true,
            (Order::Loop, _) => self.repeated = true,
            (Order::Unknown, _) => self.unknown = true,
        }
    }

    /// The strongest reason across both sides: a wrong byte order, a loop, anything else.
    fn mechanism(&self) -> Option<&'static str> {
        if self.byte_use {
            Some(D3_USE)
        } else if self.byte_def {
            Some(D3_DEF)
        } else if self.repeated {
            Some(D3_LOOP)
        } else if self.unknown {
            Some(D3_UNKNOWN)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
enum Side {
    Use,
    Def,
}

impl D3 {
    pub(super) fn new(counts: Counts, owner: &tree_sitter::Node<'_>, trace: bool) -> Option<Self> {
        (!counts.is_empty()).then(|| D3 {
            counts: Rc::new(counts),
            extra: Vec::new(),
            extra_unplaced: BTreeSet::new(),
            owner: (owner.kind(), owner.start_byte(), owner.end_byte()),
            reads: Rc::new(OnceCell::new()),
            nodes: OnceCell::new(),
            log: trace.then(|| RefCell::new(BTreeMap::new())),
        })
    }

    /// The owner's counting writes (the alias may-kill rule, #783, derives its own from them).
    pub(super) fn counts_ref(&self) -> &Counts {
        &self.counts
    }

    #[cfg(test)]
    pub(super) fn counts(&self) -> &Counts {
        &self.counts
    }

    /// This owner's D3 with `extra` writes (and `unplaced` bases) added: it shares the counts and the read
    /// index, so the alias refinement orders the resolved writes against the same reads and anchors.
    pub(super) fn with_extra(&self, extra: Vec<Counting>, unplaced: BTreeSet<String>) -> D3 {
        D3 {
            counts: Rc::clone(&self.counts),
            extra,
            extra_unplaced: unplaced,
            owner: self.owner,
            reads: Rc::clone(&self.reads),
            nodes: OnceCell::new(),
            log: self.log.as_ref().map(|_| RefCell::new(BTreeMap::new())),
        }
    }

    fn write(&self, i: usize) -> &Counting {
        let n = self.counts.writes.len();
        if i < n {
            &self.counts.writes[i]
        } else {
            &self.extra[i - n]
        }
    }

    /// The edge's finished label, then D3: an `exact` label that is not proven ordered inside its node
    /// becomes `NameOnly(SameLine)`. Applied by the two `classify_edge` call sites, so it covers the
    /// production and the frozen-oracle classifier alike.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn refine(
        d3: Option<&Rc<D3>>,
        label: FlowConfidence,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        candidates: &[usize],
        statements: &mut StatementLookup<'_>,
        line_index: &BTreeMap<Line, usize>,
        function_start: Line,
        facts: &BindingFacts,
    ) -> FlowConfidence {
        #[cfg(test)]
        if OFF.with(|off| off.get()) {
            return label;
        }
        let Some(d3) = d3.filter(|_| label.is_exact()) else {
            return label;
        };
        let nodes = d3
            .nodes
            .get_or_init(|| NodeIndex::build(&d3.counts, &d3.extra, statements));
        let def_line =
            if edge.from.line == function_start && !line_index.contains_key(&function_start) {
                Some(function_start)
            } else {
                statements.get(edge.from.line)
            };
        match (statements.get(edge.to.line), def_line) {
            (Some(u), Some(d))
                if d3
                    .refuse(parsed, edge, (u, d), nodes, candidates, facts)
                    .is_some() =>
            {
                FlowConfidence::NameOnly(FlowDoubt::SameLine)
            }
            _ => label,
        }
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
    fn refuse(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        (use_line, def_line): (usize, usize),
        nodes: &NodeIndex,
        candidates: &[usize],
        facts: &BindingFacts,
    ) -> Option<&'static str> {
        if use_line == def_line {
            return None; // one node: main's collapse and same-line rules own it
        }
        // Both sides always run: the reason is the strongest of them, and a counting write whose
        // node the mapping cannot name refuses its base whatever else the nodes hold.
        let mut failures = Failures::default();
        let base = &edge.to.path.base;
        if nodes.base_is_unknown(&self.counts, base) || self.extra_unplaced.contains(base) {
            failures.unknown = true;
        }
        self.use_side(
            parsed,
            edge,
            use_line,
            (nodes, candidates),
            facts,
            &mut failures,
        );
        self.def_side(edge, def_line, nodes, facts, &mut failures);
        let hit = failures.mechanism()?;
        if let Some(log) = &self.log {
            log.borrow_mut()
                .insert((edge.from.clone(), edge.to.clone()), hit);
        }
        Some(hit)
    }

    fn affecting<'s>(
        &'s self,
        nodes: &'s NodeIndex,
        node: usize,
        path: &'s AccessPath,
    ) -> impl Iterator<Item = &'s Counting> {
        nodes
            .by_node
            .get(&node)
            .into_iter()
            .flatten()
            .map(|&i| self.write(i))
            .filter(move |w| super::def_buckets::affects(&w.path, path))
    }

    fn use_side(
        &self,
        parsed: &ParsedFile,
        edge: &FlowEdge,
        use_line: usize,
        (nodes, candidates): (&NodeIndex, &[usize]),
        facts: &BindingFacts,
        failures: &mut Failures,
    ) {
        let writes: Vec<&Counting> = self.affecting(nodes, use_line, &edge.to.path).collect();
        if writes.is_empty() {
            return;
        }
        let Some(occurrences) = self.occurrences(parsed, edge) else {
            return; // anchored at a write's own target and no read spells the path: nothing to order
        };
        if occurrences.is_empty() {
            failures.unknown = true;
            return;
        }
        let t = Tables::shared();
        let base = &edge.to.path.base;
        for (start, end) in occurrences {
            // A read of another binding of the name (an inner `let` shadowing D's) is not D's use.
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
                .filter(|w| !facts.proven_distinct_at(base, w.probe, start))
                .collect();
            if let [only] = counting[..] {
                if only.is_operand_of(start) {
                    continue; // E2: the one write reads its operand before it writes
                }
            }
            for w in counting {
                failures.note(order(t, &place, &w.pos), Side::Use);
            }
        }
    }

    fn def_side(
        &self,
        edge: &FlowEdge,
        def_line: usize,
        nodes: &NodeIndex,
        facts: &BindingFacts,
        failures: &mut Failures,
    ) {
        let writes: Vec<&Counting> = self.affecting(nodes, def_line, &edge.from.path).collect();
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
            return;
        }
        // A def with no census site in this node (a formal on the node's line) is unplaced.
        let Some(d) = d else {
            failures.unknown = true;
            return;
        };
        let t = Tables::shared();
        for w in others {
            failures.note(order(t, &w.pos, &d.pos), Side::Def);
        }
    }

    /// The read occurrences of the edge's use: every read on its line that spells its path (a write
    /// target, which is no read, never counts) and the use's own bytes. The DFG keeps one use per
    /// path and line, at its first occurrence, so the edge speaks for the line's reads: a later
    /// occurrence the write precedes makes the whole edge unproven.
    fn occurrences(&self, parsed: &ParsedFile, edge: &FlowEdge) -> Option<Vec<(usize, usize)>> {
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
        // Every read of the path on the use's line: RD put them all in the use's node, and the order
        // against a write is `Position`'s to decide (a read in a span the write is not in is unknown).
        let on_line = reads.on_line(parsed, &to.path, to.line);
        let (targets, mut found): (Vec<_>, Vec<_>) = on_line
            .into_iter()
            .partition(|r| self.counts.write_only.contains(r));
        let own = (to.start_byte, to.end_byte);
        let real = to.end_byte > to.start_byte && !self.counts.write_only.contains(&own);
        if real && !found.contains(&own) {
            found.push(own);
        }
        if found.is_empty() && !targets.is_empty() {
            // The DFG also anchors a use at a write's own target (`value = other`, `var [f] = ..`):
            // no read of the line spells the path, so there is nothing to order.
            return None;
        }
        Some(found)
    }
}
