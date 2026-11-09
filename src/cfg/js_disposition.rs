//! The disposition inventory over the JS CFG ownership records (#693, #697; option B, slice 1).
//!
//! OBSERVATIONAL: it reads the records (`js_ownership`) and the loop trees (`js_loops::tree_facts`),
//! changes no edge, and exists so that slices 2 and 3 can gate on it. For each loop tree it lists
//! EVERY record (surviving, dropped by the current loop rewrite, or skipped as a duplicate) whose
//! endpoints intersect the tree's line footprint, or whose anchor encloses the tree (the enclosing
//! arm's completion), with one disposition:
//!
//! * `OwnedReplaceable`: the edge is a loop's own transition (its back edge, its exit, a body
//!   completion that bypasses the loop, the then-end of an arm whose terminal statement is the loop),
//!   or a replacement edge of the rewrite itself;
//! * `Independent`: unrelated to the loop's transitions, with the reason;
//! * `Unresolved`: ownership cannot be established from the records (an endpoint line shared with a
//!   loop boundary, a then-end drawn from an enclosing wrapper's entry, a nested callable, an
//!   unmodelled emitter, a crossing that is no completion).
//!
//! Two flags compare the inventory with what the CURRENT rewrite does (they are the gate of the next
//! slices): `unauthorized_drop` (the rewrite dropped a record that is not `OwnedReplaceable`) and
//! `surviving_bypass` (a rewired tree still holds a legacy edge that is a loop transition).
//! The #700 inventory (`Statements`) lists statements dropped by start-line deduplication and
//! retained statements whose line RD's lookup assigns to a shorter covering span.
use super::js_loops::{tree_facts, LoopFacts, TreeFacts};
use super::js_ownership::{Anchor, Doubt, EdgeRecord, Own, Producer, Recorder, Role};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// A statement of the owner's pre-dedup universe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Stmt {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) line: usize,
    pub(crate) end_line: usize,
    pub(crate) kind: String,
}

impl Stmt {
    fn range(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

/// The statement universe before and after start-line deduplication (#700's two mechanisms).
#[derive(Debug, Default)]
pub(crate) struct Statements {
    /// Every statement starting on a line, the one the line node keeps first.
    pub(crate) by_line: BTreeMap<usize, Vec<Stmt>>,
    /// Statements removed by start-line deduplication, with the statement kept on their line.
    pub(crate) dropped: Vec<(Stmt, Stmt)>,
    /// Retained statements whose own line RD's lookup assigns to a shorter covering span.
    pub(crate) covered: Vec<(Stmt, Stmt)>,
}

impl Statements {
    fn new(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut st = Self::default();
        for s in parsed.statement_spans_before_dedup(&owner) {
            let stmt = Stmt {
                start: s.start_byte,
                end: s.end_byte,
                line: s.line,
                end_line: parsed.line_for_byte(s.end_byte.saturating_sub(1)),
                kind: s.kind,
            };
            // The collector's ordinary and nested modes can both emit one node: a statement is
            // counted once, at its first emission (the one the line keeps if it is first).
            let on_line = st.by_line.entry(s.line).or_default();
            if !on_line.contains(&stmt) {
                on_line.push(stmt);
            }
        }
        let kept: Vec<Stmt> = st.by_line.values().map(|v| v[0].clone()).collect();
        for v in st.by_line.values() {
            st.dropped
                .extend(v[1..].iter().map(|d| (d.clone(), v[0].clone())));
        }
        // RD's lookup: the shortest retained span holding the line (ties: later start byte).
        for k in &kept {
            let winner = kept
                .iter()
                .filter(|c| c.line <= k.line && k.line <= c.end_line)
                .min_by_key(|c| (c.end - c.start, std::cmp::Reverse(c.start)))
                .filter(|w| w.line != k.line);
            if let Some(w) = winner {
                st.covered.push((k.clone(), w.clone()));
            }
        }
        st
    }

    fn kept_at(&self, line: usize) -> Option<(usize, usize)> {
        self.by_line.get(&line).map(|v| v[0].range())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Fate {
    Survived,
    /// Removed by the current loop rewrite's edge drop.
    Dropped,
    /// A rewrite emission skipped because its line pair was already an edge.
    Suppressed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Owned {
    LoopBack,
    LoopExit,
    /// A body completion that leaves the loop without passing its header.
    BodyBypass,
    /// The then-end of an arm whose terminal statement is a loop of the tree.
    DelegatedThenEnd,
    /// An edge the loop rewrite itself emits (loop transitions and jumps).
    Replacement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Indep {
    InteriorFlow,
    LoopEntry,
    EntryFromOutside,
    BranchEntry,
    OutsideTree,
    /// Both endpoints are inside the tree's lines and the emitter is not modelled.
    InteriorLegacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unres {
    /// An endpoint line is a loop boundary that another statement shares.
    SharedBoundaryLine,
    /// A then-end drawn from the entry of a wrapper that encloses the tree.
    WrapperEntryThenEnd,
    NestedCallable,
    /// An unmodelled emitter's edge crosses the tree's boundary.
    LegacyCrossing,
    /// An edge leaves a loop from a statement that is neither its completion nor its header.
    NonCompletionCrossing,
    /// An endpoint line holds no statement of the owner's universe.
    NoStatement,
    /// A crossing out of a loop whose body completions this module has no rule for.
    CompletionsUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    OwnedReplaceable(Owned),
    Independent(Indep),
    Unresolved(Unres),
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) record: EdgeRecord,
    pub(crate) fate: Fate,
    pub(crate) disposition: Disposition,
    /// The current rewrite dropped a record that no tree of the owner owns (`OwnedReplaceable`).
    pub(crate) unauthorized_drop: bool,
    /// A rewired tree still holds a legacy loop transition.
    pub(crate) surviving_bypass: bool,
}

pub(crate) struct TreeInventory {
    pub(crate) root: Anchor,
    pub(crate) rewired: bool,
    pub(crate) entries: Vec<Entry>,
}

/// One owner's records, statement universe and per-tree inventory.
pub(crate) struct Ownership {
    pub(crate) callable: (usize, usize),
    pub(crate) statements: Statements,
    /// Every record of the owner (also those no tree's inventory lists), kept anchors filled.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) records: Recorder,
    pub(crate) trees: Vec<TreeInventory>,
}

impl Ownership {
    pub(crate) fn new(parsed: &ParsedFile, owner: Node<'_>, mut records: Recorder) -> Self {
        let statements = Statements::new(parsed, owner);
        for r in records
            .live
            .iter_mut()
            .chain(&mut records.dropped)
            .chain(&mut records.suppressed)
        {
            r.from_kept = statements.kept_at(r.from_line);
            r.to_kept = statements.kept_at(r.to_line);
            let shared = |line| statements.by_line.get(&line).is_some_and(|v| v.len() > 1);
            if r.own == Own::Known && (shared(r.from_line) || shared(r.to_line)) {
                r.own = Own::Ambiguous(Doubt::SharedLine);
            }
        }
        let mut all: Vec<(EdgeRecord, Fate)> = Vec::new();
        all.extend(records.live.iter().map(|r| (r.clone(), Fate::Survived)));
        all.extend(records.dropped.iter().map(|r| (r.clone(), Fate::Dropped)));
        all.extend(
            records
                .suppressed
                .iter()
                .map(|r| (r.clone(), Fate::Suppressed)),
        );
        all.sort_by_key(|(r, _)| r.id);
        let facts = tree_facts(owner, parsed);
        let mut trees: Vec<TreeInventory> = facts
            .iter()
            .map(|t| inventory(t, &facts, &statements, &all))
            .collect();
        // A drop is authorized when SOME tree owns the record (a sibling's bypass is another tree's
        // independent entry), unauthorized when none does.
        let owned: std::collections::BTreeSet<usize> = trees
            .iter()
            .flat_map(|t| &t.entries)
            .filter(|e| matches!(e.disposition, Disposition::OwnedReplaceable(_)))
            .map(|e| e.record.id)
            .collect();
        for e in trees.iter_mut().flat_map(|t| &mut t.entries) {
            e.unauthorized_drop = e.fate == Fate::Dropped && !owned.contains(&e.record.id);
        }
        Self {
            callable: records.callable,
            statements,
            records,
            trees,
        }
    }
}

fn inventory(
    tree: &TreeFacts,
    owner_trees: &[TreeFacts],
    statements: &Statements,
    all: &[(EdgeRecord, Fate)],
) -> TreeInventory {
    let cx = Cx {
        tree,
        owner_trees,
        statements,
    };
    let entries = all
        .iter()
        .filter(|(r, _)| cx.intersects(r))
        .map(|(r, fate)| {
            let disposition = cx.classify(r);
            let legacy_transition = matches!(
                disposition,
                Disposition::OwnedReplaceable(o) if o != Owned::Replacement
            );
            Entry {
                record: r.clone(),
                fate: *fate,
                disposition,
                unauthorized_drop: false, // settled across the owner's trees by `Ownership::new`
                surviving_bypass: tree.rewired && *fate == Fate::Survived && legacy_transition,
            }
        })
        .collect();
    TreeInventory {
        root: tree.root,
        rewired: tree.rewired,
        entries,
    }
}

struct Cx<'a> {
    tree: &'a TreeFacts,
    /// Every tree of the owner (this one included): another tree's loop can end on this tree's first line.
    owner_trees: &'a [TreeFacts],
    statements: &'a Statements,
}

fn within(outer: Anchor, range: (usize, usize)) -> bool {
    outer.start <= range.0 && range.1 <= outer.end
}

impl Cx<'_> {
    fn lines(&self) -> (usize, usize) {
        let root = &self.tree.loops[0];
        (root.start_line, root.end_line)
    }

    /// Loops innermost first.
    fn loops(&self) -> Vec<&LoopFacts> {
        let mut v: Vec<&LoopFacts> = self.tree.loops.iter().collect();
        v.sort_by_key(|l| l.node.end - l.node.start);
        v
    }

    /// Whether the record's endpoints touch the tree's lines, or an anchor of it encloses the tree.
    fn intersects(&self, r: &EdgeRecord) -> bool {
        let (s, e) = self.lines();
        let on = |l: usize| s <= l && l <= e;
        let encloses = |a: Option<Anchor>| a.is_some_and(|a| a.contains(self.tree.root));
        on(r.from_line)
            || on(r.to_line)
            || encloses(r.from)
            || encloses(r.to)
            || encloses(r.depends_on)
    }

    /// Whether `line` is a loop boundary (the first or last line of some loop of the tree) that
    /// another construct shares: a statement starting outside that loop (a label that ends with the
    /// loop is the loop's alias), or a boundary line of a disjoint loop (a sibling's closing line is
    /// invisible to start lines).
    fn shared_boundary(&self, line: usize) -> bool {
        let bound = |l: &LoopFacts| line == l.start_line || line == l.end_line;
        self.tree.loops.iter().filter(|l| bound(l)).any(|l| {
            let foreign_statement = self.statements.by_line.get(&line).is_some_and(|v| {
                v.iter().any(|s| {
                    let alias = s.kind == "labeled_statement"
                        && s.start <= l.node.start
                        && s.end == l.node.end;
                    !(within(l.node, s.range()) || alias)
                })
            });
            let disjoint_loop = self
                .owner_trees
                .iter()
                .flat_map(|t| &t.loops)
                .any(|m| bound(m) && !l.node.contains(m.node) && !m.node.contains(l.node));
            foreign_statement || disjoint_loop
        })
    }

    fn is_tree_loop(&self, a: Anchor) -> bool {
        self.tree.loops.iter().any(|l| l.node.same_range(a))
    }

    fn classify(&self, r: &EdgeRecord) -> Disposition {
        use Disposition::{Independent as I, OwnedReplaceable as O, Unresolved as U};
        if r.own == Own::Ambiguous(Doubt::NestedCallable) {
            return U(Unres::NestedCallable);
        }
        if self.shared_boundary(r.from_line) || self.shared_boundary(r.to_line) {
            return U(Unres::SharedBoundaryLine);
        }
        let (s, e) = self.lines();
        let on = |l: usize| s <= l && l <= e;
        match r.producer {
            Producer::LoopBack | Producer::LoopExit
                if r.depends_on.is_some_and(|d| self.is_tree_loop(d)) =>
            {
                return O(if r.producer == Producer::LoopBack {
                    Owned::LoopBack
                } else {
                    Owned::LoopExit
                });
            }
            Producer::JsLoops | Producer::Jump => return O(Owned::Replacement),
            Producer::ThenEnd => {
                if let Some(from) = r.from {
                    if self.is_tree_loop(from) {
                        return O(Owned::DelegatedThenEnd);
                    }
                    if from.contains(self.tree.root) {
                        return U(Unres::WrapperEntryThenEnd);
                    }
                }
            }
            _ if r.role == Role::Legacy => {
                return if on(r.from_line) && on(r.to_line) {
                    I(Indep::InteriorLegacy)
                } else {
                    U(Unres::LegacyCrossing)
                };
            }
            _ => {}
        }
        let (Some(fk), Some(tk)) = (r.from_kept, r.to_kept) else {
            return U(Unres::NoStatement);
        };
        for l in self.loops() {
            let (f_in, t_in) = (within(l.node, fk), within(l.node, tk));
            match (f_in, t_in) {
                (true, false) => {
                    let completes = l
                        .completions
                        .iter()
                        .any(|(line, a)| *line == r.from_line && within(*a, fk));
                    return if completes {
                        O(Owned::BodyBypass)
                    } else if r.from_line == l.head {
                        O(Owned::LoopExit)
                    } else if !l.completions_known {
                        U(Unres::CompletionsUnknown)
                    } else {
                        U(Unres::NonCompletionCrossing)
                    };
                }
                // A branch edge that stays inside or enters a loop is the `if`'s own.
                (false, true) if r.role == Role::BranchEntry => return I(Indep::BranchEntry),
                (true, true) if r.role == Role::BranchEntry => return I(Indep::BranchEntry),
                (false, true) => return I(Indep::EntryFromOutside),
                (true, true) => {
                    return I(if r.from_line == l.head {
                        Indep::LoopEntry
                    } else {
                        Indep::InteriorFlow
                    });
                }
                (false, false) => {}
            }
        }
        I(if r.role == Role::BranchEntry {
            Indep::BranchEntry
        } else {
            Indep::OutsideTree
        })
    }
}

/// Whether `PRISM_CFG_OWNERSHIP_TRACE` asks for the inventory on stderr.
fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PRISM_CFG_OWNERSHIP_TRACE").is_some_and(|v| v != "0"))
}

/// Print the owner's inventory to stderr when the trace is on. Stdout and every result are untouched.
pub(super) fn trace(parsed: &ParsedFile, owner: Node<'_>, records: Recorder) {
    if !trace_enabled() || super::js_loops::main_loop_cfg() {
        return;
    }
    let o = Ownership::new(parsed, owner, records);
    let mut out = String::new();
    o.render(&parsed.path, &mut out);
    eprint!("{out}");
}

impl Ownership {
    fn render(&self, path: &str, out: &mut String) {
        use std::fmt::Write;
        let (a, b) = self.callable;
        let _ = writeln!(out, "own-owner {path} [{a},{b})");
        for (d, k) in &self.statements.dropped {
            let _ = writeln!(
                out,
                "own-dedup {path} line={} dropped={}[{},{}) kept={}[{},{})",
                d.line, d.kind, d.start, d.end, k.kind, k.start, k.end
            );
        }
        for (k, w) in &self.statements.covered {
            let _ = writeln!(
                out,
                "own-covered {path} line={} stmt={}[{},{}) covering={}[{},{})",
                k.line, k.kind, k.start, k.end, w.kind, w.start, w.end
            );
        }
        for t in &self.trees {
            let _ = writeln!(
                out,
                "own-tree {path} {}[{},{}) rewired={}",
                t.root.kind, t.root.start, t.root.end, t.rewired
            );
            for e in &t.entries {
                let r = &e.record;
                let _ = writeln!(
                    out,
                    "own-edge {path} #{} {:?}/{:?} L{}->L{} {:?} {:?} {:?}{}{}",
                    r.id,
                    r.producer,
                    r.role,
                    r.from_line,
                    r.to_line,
                    r.own,
                    e.fate,
                    e.disposition,
                    if e.unauthorized_drop {
                        " UNAUTHORIZED-DROP"
                    } else {
                        ""
                    },
                    if e.surviving_bypass {
                        " SURVIVING-BYPASS"
                    } else {
                        ""
                    },
                );
            }
        }
    }
}

/// The inventory of every JS-family owner of `parsed`, by running the CFG builder with records on.
#[cfg(test)]
pub(crate) fn ownership_of(parsed: &ParsedFile) -> Vec<Ownership> {
    let mut sink = super::EdgeSink::new();
    let mut out = Vec::new();
    for owner in parsed.all_functions() {
        super::build_function_cfg(owner, parsed, &mut sink);
        if let Some(rec) = sink.take_records() {
            out.push(Ownership::new(parsed, owner, rec));
        }
        sink.drain().for_each(drop);
    }
    out
}

#[cfg(test)]
#[path = "js_ownership_tests.rs"]
mod tests;
