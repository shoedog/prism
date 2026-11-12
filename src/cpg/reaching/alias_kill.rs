//! #783 option A (slice 4b): a write through a certified single-step alias is the write of the aliased path, as far
//! as labels go (JS/TS/TSX owners).
//!
//! `const u = o; o.x = 0; u.x = 1; sink(o.x)`: alias-derived defs never kill in the solver (the alias may not hold
//! on every run), so `o.x@3 -> o.x@5` kept its unflagged route and stayed `exact`. The rule is label-only
//! (reaching sets, kills and edges are untouched) and has one guiding parity, relative to the direct-write
//! analysis: **inside the certified domain, a write through the alias gets the label effect the same write
//! spelled on the aliased path gets on that edge**, with the doubt the alias adds: where the direct write
//! would be a proven kill the label is `NameOnly(AliasUnstable)`, because the alias may not hold.
//!
//! **Which writes.** Only the relations the owner's certificates admit (`alias_cert.rs`: a `const|let|var u = o`
//! body declarator, `u` and `o` each bound once in the owner subtree, no dynamic-binding hazard, not in a loop,
//! positive declaration identity). A counting write (D3's eager census writes) with fields whose base `u`
//! binds the alias's own declaration at the write (`AliasCerts::alias_at`), whose node is not the
//! declarator's node, and which the alias's def reaches, is the write of `o` followed by its fields. Chains,
//! rebinding, parameter-default aliases, aliases of member values, `this`, aliases in deferred regions,
//! shadowed spellings and writes in nested callables are not certified: their labels are the rule-disabled
//! baseline's (main's).
//!
//! **Label effect**, applied only to an edge still `exact` (any other label is returned unchanged):
//! - a write in U's or D's node: D3's order proof over the resolved writes added to the node's own (the
//!   census's counts, read index and D's original census anchor are shared, not copied): `NameOnly(SameLine)`
//!   unless `precedes` proves the read before the write, or the write before the def;
//! - a write in a node between D and U: the edge becomes `NameOnly(AliasUnstable)` when every applicable
//!   reaching candidate has such a write on each unflagged route from D to U. A route that bypasses the write
//!   (`if (c) u.x = 1;` with a fall-through) is a clean route and the edge stays `exact`. Logical
//!   assignments and `for (u.x of ..)` heads are writes as the direct form is, so they block their node.
//!
//! A write whose node RD cannot name makes every D3 query of the base unknown (`SameLine`), as for a direct one.
//! The write kills a def D by `affects` (its path is a prefix of D's, `[]` matching any component).
use super::alias_cert::at;
use super::alias_cert::{AliasCerts, Verdict};
use super::census_defs::CensusFacts;
use super::census_trace::place;
use super::d3::D3;
use super::def_buckets::affects;
use super::graph::BitSet;
use super::statement_lookup::StatementLookup;
use super::{DefSite, FlowConfidence, FlowDoubt, Line};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::data_flow::FlowEdge;
use crate::js_write_census::counting::Counting;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// Trace mechanism of an edge relabelled `AliasUnstable` (`PRISM_RD_CENSUS_TRACE=1`), and of one D3 refused.
pub(super) const KILL_ALIAS: &str = "kill:alias";
pub(super) const ALIAS_D3: &str = "alias-d3";

/// The trace mechanism of an edge this rule moved to `label`.
pub(super) fn mechanism(label: FlowConfidence) -> &'static str {
    match label {
        FlowConfidence::NameOnly(FlowDoubt::SameLine) => ALIAS_D3,
        _ => KILL_ALIAS,
    }
}

/// The move records of the traced run (`rd-alias-move` lines, deduplicated), shared with the census facts.
pub(super) type MoveLog = Rc<RefCell<BTreeSet<String>>>;

/// An admitted write through an alias: the aliased path it writes, its CFG node and where it came from.
struct Blocker {
    path: AccessPath,
    node: usize,
    /// The raw write (`u.x`) and its target start byte, and the certificate (index into `AliasCerts::entries`).
    raw: AccessPath,
    site: usize,
    entry: usize,
    /// The declaration of `o` the certificate proved (an index of `BindingFacts`): only a candidate def inside
    /// its scope is affected (`o` has no other binding in the owner, so such a def writes that `o`).
    decl: usize,
}

/// One owner's alias-write state.
pub(super) struct AliasKills<'a> {
    s: Solved<'a>,
    /// D3 over the census's writes plus the resolved ones.
    d3: Rc<D3>,
    blockers: Vec<Blocker>,
    /// Unplaced raw writes proven to write through a certified alias: `(raw base, probe, certificate)`; the
    /// aliased base is unknown to D3 (trace records only).
    transfers: Vec<(String, usize, usize)>,
    certs: Rc<AliasCerts>,
    log: Option<MoveLog>,
    /// Per candidate def: `None` when no blocker affects it, else the CFG nodes reachable from its node
    /// over unflagged edges without passing through a kill of the def or a blocker node (a blocked node
    /// can be reached, never left, as for killed nodes in `DefinitionReachability`).
    rows: BTreeMap<usize, Option<BitSet>>,
}

/// The solved owner the rule reads (borrowed for the whole classification loop).
#[derive(Clone, Copy)]
pub(super) struct Solved<'a> {
    pub parsed: &'a ParsedFile,
    /// The owner callable's byte range.
    pub owner: (usize, usize),
    pub defs: &'a [DefSite],
    pub mapped_defs: &'a [Option<usize>],
    pub in_sets: &'a [BitSet],
    pub kill: &'a [BitSet],
    pub successors: &'a [Vec<(usize, bool)>],
    pub line_index: &'a BTreeMap<Line, usize>,
    pub function_start: Line,
    pub facts: &'a BindingFacts,
    pub census: &'a CensusFacts,
}

impl<'a> AliasKills<'a> {
    /// The owner's alias-resolved write state, or `None` when it has no admitted alias or no write through
    /// one. An owner with no admitted certificate does nothing here.
    pub(super) fn new(
        s: Solved<'a>,
        statements: &mut StatementLookup<'_>,
        log: Option<MoveLog>,
    ) -> Option<Self> {
        let certs = s.census.alias_certs().filter(|c| c.any_admitted())?;
        let base = s.census.d3.as_ref()?;
        let mut node_of = |line: Line| {
            statements
                .get(line)
                .and_then(|l| s.line_index.get(&l).copied())
        };
        let reach = super::alias_point::Reach {
            parsed: s.parsed,
            owner: s.owner,
            in_sets: s.in_sets,
            mapped_defs: s.mapped_defs,
        };
        let (mut blockers, mut extra) = (Vec::new(), Vec::new());
        let mut unplaced: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut transfers = Vec::new();
        // A write with no CFG node (a class's static field or block, a node RD cannot name) leaves the aliased
        // base unknown, as the direct write does, but only where all of it is proven: the base binds the alias's
        // declaration at the write, the declarator precedes it, and the alias def reaches the write's evaluation
        // point (`alias_point`). The unknown applies to queries inside the certified target's scope only.
        let mut transfer =
            |node_of: &mut dyn FnMut(Line) -> Option<usize>, raw: &str, probe: usize| {
                let index = certs.alias_at(s.facts, raw, probe)?;
                let e = &certs.entries[index];
                let Verdict::Admitted {
                    alias_def,
                    target_decl: BindingId::Declaration(decl),
                    ..
                } = &e.verdict
                else {
                    return None;
                };
                if !reach.reaches(probe, *alias_def, &mut |l| node_of(l)) {
                    return None;
                }
                unplaced
                    .entry(e.cand.target.clone())
                    .or_default()
                    .push(*decl);
                transfers.push((raw.to_string(), probe, index));
                Some(())
            };
        for w in base
            .counts_ref()
            .writes
            .iter()
            .filter(|w| w.path.has_fields())
        {
            let Some(index) = certs.alias_at(s.facts, &w.path.base, w.probe) else {
                continue;
            };
            let e = &certs.entries[index];
            let Verdict::Admitted {
                alias_def,
                target_decl: BindingId::Declaration(decl),
                ..
            } = &e.verdict
            else {
                continue;
            };
            let path = AccessPath {
                base: e.cand.target.clone(),
                fields: w.path.fields.clone(),
            };
            let Some(node) = node_of(w.line) else {
                transfer(&mut node_of, &w.path.base, w.probe);
                continue;
            };
            // The declarator's own node is not a place the alias is known to hold; the alias's def must
            // reach the write's node.
            let declarator = s.parsed.line_for_byte(e.cand.alias_start);
            if node_of(declarator) == Some(node) || !s.in_sets[node].contains(*alias_def) {
                continue;
            }
            extra.push((
                Counting {
                    path: path.clone(),
                    ..w.clone()
                },
                *decl,
            ));
            blockers.push(Blocker {
                path,
                node,
                raw: w.path.clone(),
                site: w.target_start,
                entry: index,
                decl: *decl,
            });
        }
        for (raw, probe, _) in &base.counts_ref().unplaced_sites {
            transfer(&mut node_of, raw, *probe);
        }
        if blockers.is_empty() && unplaced.is_empty() {
            return None;
        }
        Some(Self {
            s,
            d3: Rc::new(base.with_extra(extra, unplaced)),
            blockers,
            transfers,
            certs: Rc::clone(certs),
            log,
            rows: BTreeMap::new(),
        })
    }

    /// The edge's label after the rule: only an `exact` label moves.
    pub(super) fn refine(
        this: Option<&mut Self>,
        label: FlowConfidence,
        edge: &FlowEdge,
        candidates: &[usize],
        statements: &mut StatementLookup<'_>,
    ) -> FlowConfidence {
        let (Some(this), true) = (this, label.is_exact()) else {
            return label;
        };
        let s = this.s;
        let moved = D3::refine(
            Some(&this.d3),
            label,
            s.parsed,
            edge,
            candidates,
            statements,
            s.line_index,
            s.function_start,
            s.facts,
        );
        let moved = if moved.is_exact() {
            this.may_kill(moved, edge, candidates, statements)
        } else {
            moved
        };
        if moved != label {
            this.note(mechanism(moved), edge);
        }
        moved
    }

    fn may_kill(
        &mut self,
        label: FlowConfidence,
        edge: &FlowEdge,
        candidates: &[usize],
        statements: &mut StatementLookup<'_>,
    ) -> FlowConfidence {
        let s = self.s;
        let Some(use_node) = statements
            .get(edge.to.line)
            .and_then(|line| s.line_index.get(&line).copied())
        else {
            return label;
        };
        let mut blocked_any = false;
        for &candidate in candidates {
            let Some(from) = s.mapped_defs[candidate] else {
                continue;
            };
            if !s.in_sets[use_node].contains(candidate) {
                continue;
            }
            let blockers = &self.blockers;
            let row = self.rows.entry(candidate).or_insert_with(|| {
                let nodes: BTreeSet<usize> = blockers
                    .iter()
                    .filter(|b| {
                        affects(&b.path, &s.defs[candidate].path)
                            && s.facts
                                .declaration_scope_contains(b.decl, s.defs[candidate].start_byte)
                    })
                    .map(|b| b.node)
                    .collect();
                (!nodes.is_empty())
                    .then(|| clean_row(candidate, from, &nodes, s.kill, s.successors))
            });
            let Some(row) = row else {
                // Unaffected by any alias write: the classifier's verdict stands for this candidate.
                return label;
            };
            blocked_any = true;
            if row.contains(use_node) {
                return label;
            }
        }
        if blocked_any {
            FlowConfidence::NameOnly(FlowDoubt::AliasUnstable)
        } else {
            label
        }
    }

    /// Trace on: one `rd-alias-move` record per admitted write that affects the moved edge's candidate path
    /// (raw site, alias def, source and target declarations, resolved path, the edge, the mechanism).
    fn note(&self, mechanism: &str, edge: &FlowEdge) {
        let Some(log) = &self.log else { return };
        let (from, to) = (&edge.from, &edge.to);
        for b in self
            .blockers
            .iter()
            .filter(|b| affects(&b.path, &from.path) || affects(&b.path, &to.path))
        {
            let e = &self.certs.entries[b.entry].cand;
            let alias = at(self.s.parsed, &e.alias, e.alias_start);
            let target = e
                .target_start
                .map_or("-".into(), |t| at(self.s.parsed, &e.target, t));
            log.borrow_mut().insert(format!(
                "rd-alias-move\t{mechanism}\t{}\t{}\t{}\t{}\t{}\t{alias}\t{alias}\t{target}\t{}",
                to.file,
                from.function,
                place(from),
                place(to),
                at(self.s.parsed, &b.raw.to_string(), b.site),
                b.path,
            ));
        }
        for (base, probe, index) in &self.transfers {
            let e = &self.certs.entries[*index].cand;
            if e.target != from.path.base && e.target != to.path.base {
                continue;
            }
            let alias = at(self.s.parsed, &e.alias, e.alias_start);
            let target = e
                .target_start
                .map_or("-".into(), |t| at(self.s.parsed, &e.target, t));
            log.borrow_mut().insert(format!(
                "rd-alias-move\t{mechanism}\t{}\t{}\t{}\t{}\t{}\t{alias}\t{alias}\t{target}\t{}(unplaced)",
                to.file,
                from.function,
                place(from),
                place(to),
                at(self.s.parsed, base, *probe),
                e.target,
            ));
        }
    }
}

fn clean_row(
    def_index: usize,
    from: usize,
    blockers: &BTreeSet<usize>,
    kill: &[BitSet],
    successors: &[Vec<(usize, bool)>],
) -> BitSet {
    let mut seen = BitSet::new(successors.len());
    seen.insert(from);
    let mut stack = vec![from];
    while let Some(node) = stack.pop() {
        for &(next, incomplete) in &successors[node] {
            if !incomplete && !seen.contains(next) {
                seen.insert(next);
                if !kill[next].contains(def_index) && !blockers.contains(&next) {
                    stack.push(next);
                }
            }
        }
    }
    seen
}
