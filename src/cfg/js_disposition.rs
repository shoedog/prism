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
        for v in st.by_line.values() {
            st.dropped
                .extend(v[1..].iter().map(|d| (d.clone(), v[0].clone())));
        }
        st
    }

    /// Retained statements whose own line RD's lookup assigns to a shorter covering span (the #700 second
    /// mechanism). Quadratic, so computed on demand (trace and tests), never on the production path.
    pub(crate) fn covered(&self) -> Vec<(Stmt, Stmt)> {
        let kept: Vec<Stmt> = self.by_line.values().map(|v| v[0].clone()).collect();
        let mut out = Vec::new();
        for k in &kept {
            let winner = kept
                .iter()
                .filter(|c| c.line <= k.line && k.line <= c.end_line)
                .min_by_key(|c| (c.end - c.start, std::cmp::Reverse(c.start)))
                .filter(|w| w.line != k.line);
            if let Some(w) = winner {
                out.push((k.clone(), w.clone()));
            }
        }
        out
    }

    /// Whether the owner's RD selects the edge: `FileCfgEdgeIndex::for_owner` admits an edge exactly when
    /// both endpoint lines are in the owner's line index (the lines of its statement universe).
    pub(crate) fn admits(&self, r: &EdgeRecord) -> bool {
        self.by_line.contains_key(&r.from_line) && self.by_line.contains_key(&r.to_line)
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
    /// A replacement edge another tree's rewrite emitted (its exit or jump enters this tree).
    OtherTree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unres {
    /// An endpoint line is a loop boundary that another statement shares.
    SharedBoundaryLine,
    /// A then-end drawn from the entry of a wrapper that holds a loop of the tree.
    WrapperEntryThenEnd,
    /// A loop's head line also holds body execution (or a `do`'s condition has no point of its own):
    /// the test and the body share one node, so no replacement endpoint is representable.
    HeadCollapse,
    /// An unmodelled emitter's edge crosses the tree's boundary.
    LegacyCrossing,
    /// An edge leaves a loop from a statement that is neither its completion nor its header.
    NonCompletionCrossing,
    /// An endpoint line holds no statement of the owner's universe.
    NoStatement,
    /// A crossing out of a loop whose body completions this module has no rule for.
    CompletionsUnknown,
    /// A record the owner's RD admits that another callable's build emitted (or a nested callable's
    /// construct drew), and whose position bypasses or crosses a loop of the tree.
    ForeignAdmitted,
}

impl Unres {
    /// The counter name of this kind (`refused.unresolved.<key>`).
    pub(crate) fn key(self) -> &'static str {
        match self {
            Unres::SharedBoundaryLine => "shared_boundary_line",
            Unres::WrapperEntryThenEnd => "wrapper_entry_then_end",
            Unres::HeadCollapse => "head_collapse",
            Unres::LegacyCrossing => "legacy_crossing",
            Unres::NonCompletionCrossing => "non_completion_crossing",
            Unres::NoStatement => "no_statement",
            Unres::CompletionsUnknown => "completions_unknown",
            Unres::ForeignAdmitted => "foreign_admitted",
        }
    }
}

/// Records that are neither a loop tree's nor a reason to refuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Foreign {
    /// Drawn for a construct inside a callable nested in the owner (with "next statement" targets taken
    /// from the owner's line list), or emitted by another callable's build, and not selected by the
    /// owner's RD (an endpoint line is not a statement of the owner): inert.
    Inert,
    /// The same, selected by the owner's RD (`FileCfgEdgeIndex` admits an edge by its endpoint lines),
    /// and independent of every loop of the tree: kept, no reason to refuse.
    Admitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Disposition {
    OwnedReplaceable(Owned),
    Independent(Indep),
    Unresolved(Unres),
    Foreign(Foreign),
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) record: EdgeRecord,
    pub(crate) fate: Fate,
    pub(crate) disposition: Disposition,
    /// The owner's RD selects this edge: both endpoint lines are statements of the owner's universe.
    pub(crate) admitted: bool,
    /// The current rewrite dropped a record that no tree of the owner owns (`OwnedReplaceable`).
    pub(crate) unauthorized_drop: bool,
    /// A rewired tree still holds a legacy loop transition.
    pub(crate) surviving_bypass: bool,
}

pub(crate) struct TreeInventory {
    pub(crate) root: Anchor,
    /// The tree's first and last line.
    pub(crate) lines: (usize, usize),
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
    /// `others` are the surviving records of the other callables' builds of the file: an edge RD selects
    /// by endpoint lines does not know its emitter, so those between two statements of this owner are
    /// foreign records it admits.
    pub(crate) fn new<'p>(
        parsed: &'p ParsedFile,
        owner: Node<'p>,
        mut records: Recorder,
        others: Vec<EdgeRecord>,
    ) -> Self {
        let statements = Statements::new(parsed, owner);
        let base = records.live.len() + records.dropped.len() + records.suppressed.len();
        let mut foreign: Vec<EdgeRecord> = others
            .into_iter()
            .filter(|r| statements.admits(r))
            .enumerate()
            .map(|(i, mut r)| {
                r.id = base + i;
                r.nested = true;
                r
            })
            .collect();
        for r in records
            .live
            .iter_mut()
            .chain(&mut records.dropped)
            .chain(&mut records.suppressed)
            .chain(&mut foreign)
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
        all.extend(foreign.into_iter().map(|r| (r, Fate::Survived)));
        all.sort_by_key(|(r, _)| r.id);
        let facts = tree_facts(owner, parsed);
        let mut trees: Vec<TreeInventory> = facts
            .iter()
            .map(|t| inventory(parsed, owner, t, &facts, &statements, &all))
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
            e.unauthorized_drop = e.fate == Fate::Dropped
                && !owned.contains(&e.record.id)
                && !matches!(e.disposition, Disposition::Foreign(_));
        }
        Self {
            callable: records.callable,
            statements,
            records,
            trees,
        }
    }
}

/// The drop authority of an owner's ADMITTED loop trees (#693/#697 option B, slice 3). A record is dropped
/// by the loop rewrite only when an admitted tree of the callable owns it (`OwnedReplaceable`, never the
/// tree's own `Replacement`): the tree then emits what replaces it (`js_loops::build`). It reads the same
/// classification the plan gates on, so a tree the plan admits drops exactly what the plan found owned.
pub(in crate::cfg) struct Authority<'p> {
    parsed: &'p ParsedFile,
    owner: Node<'p>,
    statements: Statements,
    facts: Vec<TreeFacts>,
}

impl<'p> Authority<'p> {
    /// `None` when no tree of `owner` is rewired: it owns nothing.
    pub(in crate::cfg) fn new(parsed: &'p ParsedFile, owner: Node<'p>) -> Option<Self> {
        let facts = tree_facts(owner, parsed);
        if !facts.iter().any(|t| t.rewired) {
            return None;
        }
        let statements = Statements::new(parsed, owner);
        Some(Self {
            parsed,
            owner,
            statements,
            facts,
        })
    }

    /// The root of the admitted tree that owns `record`, and how. `foreign` is a record another callable's
    /// build drew (an enclosing callable's generic collectors draw its nested callables' constructs): it
    /// counts only when this owner's RD selects it, as `Ownership` classifies foreign records.
    pub(in crate::cfg) fn owns(
        &self,
        record: &EdgeRecord,
        foreign: bool,
    ) -> Option<(Anchor, Owned)> {
        let mut r = record.clone();
        r.from_kept = self.statements.kept_at(r.from_line);
        r.to_kept = self.statements.kept_at(r.to_line);
        if foreign && !self.statements.admits(&r) {
            return None;
        }
        for tree in self.facts.iter().filter(|t| t.rewired) {
            let cx = Cx {
                parsed: self.parsed,
                owner: self.owner,
                tree,
                owner_trees: &self.facts,
                statements: &self.statements,
                replaced: std::collections::BTreeSet::new(),
            };
            if !cx.intersects(&r) {
                continue;
            }
            if let Disposition::OwnedReplaceable(o) = cx.position(&r) {
                if o != Owned::Replacement {
                    return Some((tree.root, o));
                }
            }
        }
        None
    }
}

fn inventory<'p>(
    parsed: &'p ParsedFile,
    owner: Node<'p>,
    tree: &TreeFacts,
    owner_trees: &[TreeFacts],
    statements: &Statements,
    all: &[(EdgeRecord, Fate)],
) -> TreeInventory {
    // Line pairs this tree's own rewrite emitted (or wanted to emit): a foreign copy of a legacy loop
    // edge on one of them is the same edge.
    let replaced: std::collections::BTreeSet<(usize, usize)> = all
        .iter()
        .filter(|(r, _)| {
            matches!(r.producer, Producer::JsLoops | Producer::Jump)
                && r.tree == Some((tree.root.start, tree.root.end))
        })
        .map(|(r, _)| (r.from_line, r.to_line))
        .collect();
    let cx = Cx {
        parsed,
        owner,
        tree,
        owner_trees,
        statements,
        replaced,
    };
    // Line pairs the rewrite emits or wanted to emit: a legacy edge on one of them is the replacement.
    let wanted: std::collections::BTreeSet<(usize, usize)> = all
        .iter()
        .filter(|(r, _)| matches!(r.producer, Producer::JsLoops | Producer::Jump))
        .map(|(r, _)| (r.from_line, r.to_line))
        .collect();
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
                admitted: statements.admits(r),
                unauthorized_drop: false, // settled across the owner's trees by `Ownership::new`
                surviving_bypass: tree.rewired
                    && *fate == Fate::Survived
                    && legacy_transition
                    && !wanted.contains(&(r.from_line, r.to_line)),
            }
        })
        .collect();
    TreeInventory {
        root: tree.root,
        lines: cx.lines(),
        rewired: tree.rewired,
        entries,
    }
}

struct Cx<'a> {
    parsed: &'a ParsedFile,
    owner: Node<'a>,
    tree: &'a TreeFacts,
    /// Every tree of the owner (this one included): another tree's loop can end on this tree's first line.
    owner_trees: &'a [TreeFacts],
    statements: &'a Statements,
    /// The line pairs the tree's own rewrite emits (`replaced` in `inventory`).
    replaced: std::collections::BTreeSet<(usize, usize)>,
}

fn within(outer: Anchor, range: (usize, usize)) -> bool {
    outer.start <= range.0 && range.1 <= outer.end
}

impl Cx<'_> {
    fn lines(&self) -> (usize, usize) {
        let root = &self.tree.loops[0];
        (root.start_line, root.end_line)
    }

    /// Whether the record's endpoints touch the tree's lines, or an anchor of it encloses the tree.
    fn intersects(&self, r: &EdgeRecord) -> bool {
        let (s, e) = self.lines();
        let on = |l: usize| s <= l && l <= e;
        // An unmodelled emitter's edge (a `try` around the tree drawing its `catch` edge) touches the
        // tree by lines only: enclosing it says nothing about the tree's transitions.
        let encloses = |a: Option<Anchor>| {
            r.role != Role::Legacy && a.is_some_and(|a| a.contains(self.tree.root))
        };
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

    /// Whether the then-end wrapper `from` is delegated to this tree: this tree is its CLAIMANT (one record,
    /// one authority) and its completion is resolvable (`Cx::wrapper_completion`: closed rule list, every
    /// jump inside it is routed by an admitted tree or by the wrapper itself).
    fn delegable(&self, from: Anchor) -> bool {
        self.claims(from)
            && super::js_loops::wrapper_resolvable(self.parsed, self.owner, from, self.tree.root)
    }

    /// Whether this tree is the one that owns the then-end record of the wrapper `from`. A wrapper inside
    /// the tree's own loops is the tree's; one that holds whole trees of the owner (a loop-bearing `if`
    /// between sibling loops) is claimed by the first of them that is rewired, else the first of them, so
    /// the other trees see the record as another tree's.
    fn claims(&self, from: Anchor) -> bool {
        let inside: Vec<&TreeFacts> = self
            .owner_trees
            .iter()
            .filter(|t| from.contains(t.root))
            .collect();
        match inside.iter().find(|t| t.rewired).or(inside.first()) {
            Some(first) => first.root.same_range(self.tree.root),
            None => true,
        }
    }

    /// The innermost loop of the tree that holds the wrapper `from` and that the record leaves (its
    /// target statement is outside the loop): the wrapper ends a loop body's last `if` arm.
    fn crossed_by(&self, r: &EdgeRecord, from: Anchor) -> Option<&LoopFacts> {
        if !super::is_compound(self.parsed, from.kind) {
            return None;
        }
        let tk = r.to_kept?;
        self.tree
            .loops
            .iter()
            .filter(|l| within(l.node, (from.start, from.end)) && !within(l.node, tk))
            .min_by_key(|l| l.node.end - l.node.start)
    }

    fn loop_of(&self, a: Anchor) -> Option<&LoopFacts> {
        self.tree.loops.iter().find(|l| l.node.same_range(a))
    }

    /// Whether the loop's head line holds execution that the head's test must be told apart from: a
    /// statement of its body starting on that line (`while (c) x = f();`, `while (c) { x = f();`), or a
    /// `do` whose condition has no point of its own. A header declaration (`for (let i = 0;`) and a
    /// header spread over lines are not body execution.
    fn collapsed(&self, l: &LoopFacts) -> bool {
        l.condition_collapsed
            || l.body.is_some_and(|b| {
                self.statements
                    .by_line
                    .get(&l.head)
                    .is_some_and(|v| v.iter().any(|s| within(b, s.range())))
            })
    }

    /// Whether every statement starting on `line` lies in the loop's body.
    fn line_in_body(&self, l: &LoopFacts, line: usize) -> bool {
        l.body.is_some_and(|b| {
            self.statements
                .by_line
                .get(&line)
                .is_some_and(|v| v.iter().all(|s| within(b, s.range())))
        })
    }

    /// `owned` for loop `l`, unless its head is collapsed: no replacement endpoint is representable.
    fn grant(&self, l: Option<&LoopFacts>, owned: Owned) -> Disposition {
        match l {
            Some(l) if self.collapsed(l) => Disposition::Unresolved(Unres::HeadCollapse),
            _ => Disposition::OwnedReplaceable(owned),
        }
    }

    fn classify(&self, r: &EdgeRecord) -> Disposition {
        use Disposition::{Foreign as F, Independent as I, Unresolved as U};
        if !r.nested {
            return self.position(r);
        }
        // Another callable's edge: RD cannot tell its emitter, only its endpoint lines. One RD does not
        // select is inert; one it selects is read by position, and an edge that is not independent of the
        // tree's loops (a bypass, a crossing, an unplaceable line) cannot be owned by the tree.
        if !self.statements.admits(r) {
            return F(Foreign::Inert);
        }
        // A generic collector of an enclosing callable draws a nested loop's legacy back and exit edges
        // too; one on a line pair the tree's own rewrite emits is that very edge (a duplicate), not a bypass.
        match self.position(r) {
            I(_) | F(_) => F(Foreign::Admitted),
            Disposition::OwnedReplaceable(o)
                if o != Owned::Replacement && self.replaced.contains(&(r.from_line, r.to_line)) =>
            {
                F(Foreign::Admitted)
            }
            _ => U(Unres::ForeignAdmitted),
        }
    }

    fn position(&self, r: &EdgeRecord) -> Disposition {
        use Disposition::{Independent as I, Unresolved as U};
        if self.shared_boundary(r.from_line) || self.shared_boundary(r.to_line) {
            return U(Unres::SharedBoundaryLine);
        }
        // A record on a collapsed head line cannot be placed on either execution point.
        if self
            .tree
            .loops
            .iter()
            .any(|l| self.collapsed(l) && (r.from_line == l.head || r.to_line == l.head))
        {
            return U(Unres::HeadCollapse);
        }
        let (s, e) = self.lines();
        let on = |l: usize| s <= l && l <= e;
        match r.producer {
            Producer::LoopBack | Producer::LoopExit
                if r.depends_on.and_then(|d| self.loop_of(d)).is_some() =>
            {
                let l = r.depends_on.and_then(|d| self.loop_of(d));
                return self.grant(
                    l,
                    if r.producer == Producer::LoopBack {
                        Owned::LoopBack
                    } else {
                        Owned::LoopExit
                    },
                );
            }
            Producer::JsLoops | Producer::Jump => {
                // Only the tree whose rewrite emitted the edge owns it.
                let root = (self.tree.root.start, self.tree.root.end);
                return if r.tree == Some(root) {
                    self.grant(
                        r.depends_on.and_then(|d| self.loop_of(d)),
                        Owned::Replacement,
                    )
                } else {
                    I(Indep::OtherTree)
                };
            }
            Producer::ThenEnd => {
                if let Some(from) = r.from {
                    if let Some(l) = self.loop_of(from) {
                        return self.grant(Some(l), Owned::DelegatedThenEnd);
                    }
                    // A label on the loop's own line is the loop's alias.
                    let alias = self.tree.loops.iter().find(|l| {
                        from.kind == "labeled_statement"
                            && from.contains(l.node)
                            && from.end == l.node.end
                            && r.from_line == l.start_line
                    });
                    if alias.is_some() {
                        return self.grant(alias, Owned::DelegatedThenEnd);
                    }
                    // A compound last statement is no completion line: its entry is. The arm's completion
                    // is delegated to the tree when the wrapper's own completions are a closed rule list
                    // and the tree emits them (`Cx::wrapper_completion`, #693 slice 3).
                    let held = self.tree.loops.iter().find(|l| from.contains(l.node));
                    if let Some(l) = held {
                        return if !self.claims(from) {
                            I(Indep::OtherTree)
                        } else if self.delegable(from) {
                            self.grant(Some(l), Owned::DelegatedThenEnd)
                        } else {
                            U(Unres::WrapperEntryThenEnd)
                        };
                    }
                    if let Some(l) = self.crossed_by(r, from) {
                        if self.delegable(from) {
                            return self.grant(Some(l), Owned::DelegatedThenEnd);
                        }
                    }
                }
            }
            _ if r.role == Role::Legacy => {
                return if on(r.from_line) && on(r.to_line) {
                    I(Indep::InteriorLegacy)
                } else if r.to_line == s && r.from_line < s {
                    // An unmodelled emitter's edge (`try` -> `finally`/`catch`) that ENTERS the tree at its
                    // first line from outside: the rewrite changes no edge into the head, so it owns nothing
                    // of it and nothing of it is replaced (slice 3).
                    I(Indep::EntryFromOutside)
                } else {
                    U(Unres::LegacyCrossing)
                };
            }
            _ => {}
        }
        let (Some(fk), Some(tk)) = (r.from_kept, r.to_kept) else {
            return U(Unres::NoStatement);
        };
        let branch = r.role == Role::BranchEntry;
        let loops = &self.tree.loops;
        let size = |l: &&LoopFacts| l.node.end - l.node.start;
        let (f_in, t_in) = (
            |l: &LoopFacts| within(l.node, fk),
            |l: &LoopFacts| within(l.node, tk),
        );
        // Exits first: the innermost loop (by containment) the edge leaves.
        if let Some(l) = loops
            .iter()
            .filter(|l| f_in(l) && !t_in(l))
            .min_by_key(size)
        {
            let completes = l.completions.iter().any(|(line, a)| {
                *line == r.from_line && (within(*a, fk) || self.line_in_body(l, r.from_line))
            });
            return if completes {
                self.grant(Some(l), Owned::BodyBypass)
            } else if r.from_line == l.head {
                self.grant(Some(l), Owned::LoopExit)
            } else if !l.completions_known {
                U(Unres::CompletionsUnknown)
            } else {
                U(Unres::NonCompletionCrossing)
            };
        }
        if loops.iter().any(|l| !f_in(l) && t_in(l)) {
            return I(if branch {
                Indep::BranchEntry
            } else {
                Indep::EntryFromOutside
            });
        }
        if let Some(l) = loops.iter().filter(|l| f_in(l) && t_in(l)).min_by_key(size) {
            return I(if branch {
                Indep::BranchEntry
            } else if r.from_line == l.head {
                Indep::LoopEntry
            } else {
                Indep::InteriorFlow
            });
        }
        I(if branch {
            Indep::BranchEntry
        } else {
            Indep::OutsideTree
        })
    }
}

/// Whether `PRISM_CFG_OWNERSHIP_TRACE` asks for the inventory on stderr.
pub(crate) fn trace_enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("PRISM_CFG_OWNERSHIP_TRACE").is_some_and(|v| v != "0"))
}

/// Print the owner's inventory to stderr when the trace is on. Stdout and every result are untouched.
pub(super) fn trace(parsed: &ParsedFile, owner: Node<'_>, records: Recorder) {
    if !trace_enabled() || super::js_loops::main_loop_cfg() {
        return;
    }
    let o = Ownership::new(parsed, owner, records, Vec::new());
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
        for (k, w) in &self.statements.covered() {
            let _ = writeln!(
                out,
                "own-covered {path} line={} stmt={}[{},{}) covering={}[{},{})",
                k.line, k.kind, k.start, k.end, w.kind, w.start, w.end
            );
        }
        for t in &self.trees {
            let _ = writeln!(
                out,
                "own-tree {path} {}[{},{}) lines={}-{} rewired={}",
                t.root.kind, t.root.start, t.root.end, t.lines.0, t.lines.1, t.rewired
            );
            for e in &t.entries {
                let r = &e.record;
                let _ = writeln!(
                    out,
                    "own-edge {path} #{} {:?}/{:?} L{}->L{} {:?} {:?} {:?} admitted={}{}{}",
                    r.id,
                    r.producer,
                    r.role,
                    r.from_line,
                    r.to_line,
                    r.own,
                    e.fate,
                    e.disposition,
                    e.admitted,
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

/// The inventory of every JS-family owner of `parsed`, built with records under the trees main's own
/// policy routes (the first, all-candidates round of the plan; the plan itself may refuse some).
#[cfg(test)]
pub(crate) fn ownership_of(parsed: &ParsedFile) -> Vec<Ownership> {
    let candidates = super::js_loops::legacy_candidates(parsed);
    super::js_loops::with_admission(parsed, &candidates, || {
        super::js_loops::ownerships(parsed, false)
    })
    .into_iter()
    .map(|(_, o)| o)
    .collect()
}

/// The file's CFG edges as `ownership_of` sees them: built under the all-candidates admission.
#[cfg(test)]
pub(crate) fn tentative_file_edges(
    parsed: &ParsedFile,
) -> Vec<(super::CfgEdge, super::ArmProvenance)> {
    let candidates = super::js_loops::legacy_candidates(parsed);
    super::js_loops::with_admission(parsed, &candidates, || {
        super::build_cfg_edges_with_arms(parsed)
    })
}

#[cfg(test)]
#[path = "js_ownership_tests.rs"]
mod tests;
