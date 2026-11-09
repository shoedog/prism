//! #783: a write through an alias is the write of the aliased path, as far as labels go (JS/TS/TSX owners).
//!
//! `const u = o; o.x = 0; u.x = 1; sink(o.x)`: alias-derived defs never kill in the solver (the alias may
//! not hold on every run), so `o.x@3 -> o.x@5` kept its unflagged route and stayed `exact`. The rule here
//! is label-only (reaching sets, kills and edges are untouched) and has one guiding parity: **a write
//! through an alias gets the label effect the same write spelled directly on the aliased path gets on
//! that edge** (`u.x = 1` as `o.x = 1`), with the doubt the alias adds: where the direct write would be a
//! proven kill the label is `NameOnly(AliasUnstable)`, because the alias may not hold.
//!
//! **Which writes.** Every write site the census lists (eager writes: D3's counting writes; writes in a
//! nested callable: D2-c's captured writes) and every recorded def with fields the census does not list
//! (a write in the own parameter list) whose base `u` is an alias *at that site*: a def of `u` that is an
//! alias definition (`u = o`, `const u = o`, a parameter default `u = o`, `const { k: u } = o`, `var self =
//! this`; read at the def's own byte by `js_ts_own_alias_target`, so nothing is persisted and no producer
//! fact is consulted) reaches the write's node (RD's own reaching set, so `u = {}` before the write ends
//! the alias and `u = p` after it does not) and binds the same declaration as the `u` at the write
//! (`BindingFacts::relation_at`, so a sibling block's or a nested callable's own `u` never counts). A chain
//! `w = u` resolves through the alias defs reaching the alias def. The resolved path is the alias target's
//! path followed by the write's fields.
//!
//! **Three effects**, each through the existing mechanism, with the alias-resolved write in place of the
//! raw one, applied only to an edge still `exact`:
//! - a write in U's node or D's node: D3's order proof (`NameOnly(SameLine)` unless `precedes` proves the
//!   read before the write, or the write before the def), so `return (sink(o.x), u.x = 1)` stays `exact`;
//! - a write in a nested callable: D2-c (`NameOnly(CfgIncomplete)` when the use is outside the unit);
//! - a write in a node between D and U: a may-kill. The edge becomes `NameOnly(AliasUnstable)` when every
//!   reaching candidate has such a write on each unflagged route from D to U. A route that bypasses the
//!   write (`if (c) u.x = 1;` with a fall-through) is a clean route and the edge stays `exact`: D really
//!   does reach U there. Logical assignments (`||=`) and `for (u.x of ..)` heads are writes as the direct
//!   form is (RD kills at them), so they block their node.
//!
//! The write kills a def D by `affects` (its path is a prefix of D's, `[]` matching any component).
//! Residuals: an alias the producer's `js_ts_own_alias_target` does not model (an array pattern `const [u] =
//! o`, `u = o.p`, a wrapped or call right-hand side), a write through an alias the census does not list,
//! and the alias map's own limits (no flow beyond RD's reaching sets).
use super::census_defs::CensusFacts;
use super::d2c::D2c;
use super::d3::D3;
use super::def_buckets::affects;
use super::graph::BitSet;
use super::statement_lookup::StatementLookup;
use super::{DefSite, FlowConfidence, FlowDoubt, Line};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingRelation};
use crate::data_flow::FlowEdge;
use crate::js_write_census::captured::Captured;
use crate::js_write_census::counting::{Counting, Counts};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use tree_sitter::Node;

/// Trace mechanism of an edge relabelled `AliasUnstable` (`PRISM_RD_CENSUS_TRACE=1`).
pub(super) const KILL_ALIAS: &str = "kill:alias";
/// Trace mechanisms of an edge relabelled by D3 / D2-c on an alias-resolved write.
pub(super) const ALIAS_D3: &str = "alias-d3";
pub(super) const ALIAS_D2C: &str = "alias-d2c";

/// The trace mechanism of an edge this rule moved to `label`.
pub(super) fn mechanism(label: FlowConfidence) -> &'static str {
    match label {
        FlowConfidence::NameOnly(FlowDoubt::SameLine) => ALIAS_D3,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => ALIAS_D2C,
        _ => KILL_ALIAS,
    }
}

/// An alias definition of a name: the def, the alias target it names, and its node.
struct AliasDef {
    def: usize,
    target: AccessPath,
    node: Option<usize>,
}

/// One owner's alias-write state.
pub(super) struct AliasKills {
    /// D3 over the alias-resolved eager writes (endpoint-node order).
    d3: Option<Rc<D3>>,
    /// D2-c over the alias-resolved writes in nested callables.
    d2c: Option<Rc<D2c>>,
    /// The aliased path each eager write kills, and its CFG node (D3 owns the endpoints).
    blockers: Vec<(AccessPath, usize)>,
    /// Per candidate def: `None` when no blocker affects it, else the CFG nodes reachable from its node
    /// over unflagged edges without passing through a kill of the def or a blocker node (a blocked node
    /// can be reached, never left, as for killed nodes in `DefinitionReachability`).
    rows: BTreeMap<usize, Option<BitSet>>,
}

/// What `AliasKills::new` reads: the solved owner.
pub(super) struct Solved<'a, 'p> {
    pub parsed: &'a ParsedFile,
    pub func_node: &'a Node<'p>,
    pub defs: &'a [DefSite],
    pub n_recorded: usize,
    pub mapped_defs: &'a [Option<usize>],
    pub in_sets: &'a [BitSet],
    pub facts: &'a BindingFacts,
    pub census: &'a CensusFacts,
}

/// The alias definitions of `bases`: recorded simple defs whose own bytes spell an alias.
fn alias_defs(s: &Solved<'_, '_>, bases: &BTreeSet<String>) -> BTreeMap<String, Vec<AliasDef>> {
    let mut out: BTreeMap<String, Vec<AliasDef>> = BTreeMap::new();
    for (index, def) in s.defs.iter().enumerate().take(s.n_recorded) {
        if def.alias_derived
            || !def.path.fields.is_empty()
            || !bases.contains(def.path.base.as_str())
        {
            continue;
        }
        let end = def.start_byte + def.path.base.len();
        if let Some(target) = alias_target(s.parsed, def.start_byte, end) {
            out.entry(def.path.base.clone())
                .or_default()
                .push(AliasDef {
                    def: index,
                    target: AccessPath::from_expr(&target),
                    node: s.mapped_defs[index],
                });
        }
    }
    out
}

/// The alias a def's own identifier bytes establish: the producer's own-scope rule, plus `x = this`.
fn alias_target(parsed: &ParsedFile, start: usize, end: usize) -> Option<String> {
    if let Some(target) = parsed.js_ts_own_alias_target(start, end) {
        return Some(target);
    }
    let ident = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(start, end)
        .filter(|n| n.start_byte() == start && n.end_byte() == end && n.kind() == "identifier")?;
    let parent = ident.parent()?;
    let (slot, value) = match parent.kind() {
        "variable_declarator" => ("name", "value"),
        "assignment_expression" => ("left", "right"),
        _ => return None,
    };
    let is_target = parent.child_by_field_name(slot).map(|n| n.id()) == Some(ident.id());
    let is_this = parent
        .child_by_field_name(value)
        .is_some_and(|n| n.kind() == "this");
    (is_target && is_this).then(|| "this".to_string())
}

/// The counting writes with fields.
fn counted(c: &Counts) -> impl Iterator<Item = &Counting> {
    c.writes.iter().filter(|w| w.path.has_fields())
}

impl AliasKills {
    /// The owner's alias-resolved write state, or `None` when it has no write through an alias. Owners with
    /// no write with fields, or no alias definition of a written base, do no more than a scan of their defs.
    pub(super) fn new(
        s: &Solved<'_, '_>,
        statements: &mut StatementLookup<'_>,
        line_index: &BTreeMap<Line, usize>,
    ) -> Option<Self> {
        let (counts, captured) = s.census.alias_inputs();
        let listed: BTreeSet<usize> = counts
            .map(|c| c.writes.iter().map(|w| w.target_start).collect())
            .unwrap_or_default();
        // Recorded defs with fields the census does not list (a write in the own parameter list).
        let unlisted: Vec<usize> = (0..s.n_recorded)
            .filter(|&i| {
                let d = &s.defs[i];
                !d.alias_derived && d.path.has_fields() && !listed.contains(&d.start_byte)
            })
            .collect();
        let mut bases: BTreeSet<&str> = BTreeSet::new();
        bases.extend(
            counts
                .into_iter()
                .flat_map(counted)
                .map(|w| w.path.base.as_str()),
        );
        bases.extend(
            captured
                .iter()
                .filter(|c| c.path.has_fields())
                .map(|c| c.path.base.as_str()),
        );
        bases.extend(unlisted.iter().map(|&i| s.defs[i].path.base.as_str()));
        if bases.is_empty() {
            return None;
        }
        // A chain `w = u = o` needs the alias defs of `u` too: close the bases over the targets found.
        let mut all: BTreeSet<String> = bases.iter().map(|b| b.to_string()).collect();
        let mut aliases = alias_defs(s, &all);
        for _ in 0..4 {
            let fresh: Vec<String> = aliases
                .values()
                .flatten()
                .map(|a| a.target.base.clone())
                .filter(|b| !all.contains(b.as_str()))
                .collect();
            if fresh.is_empty() {
                break;
            }
            all.extend(fresh);
            aliases = alias_defs(s, &all);
        }
        if aliases.is_empty() {
            return None;
        }
        let node_of = |statements: &mut StatementLookup<'_>, line: Line| {
            statements
                .get(line)
                .and_then(|l| line_index.get(&l).copied())
        };
        let mut blockers = Vec::new();
        let mut eager = Vec::new();
        for w in counts.into_iter().flat_map(counted) {
            let Some(node) = node_of(statements, w.line) else {
                continue;
            };
            for path in resolve(s, &aliases, &w.path, w.probe, node) {
                blockers.push((path.clone(), node));
                eager.push(Counting { path, ..w.clone() });
            }
        }
        for &i in &unlisted {
            let d = &s.defs[i];
            let Some(node) = s.mapped_defs[i] else {
                continue;
            };
            for path in resolve(s, &aliases, &d.path, d.start_byte, node) {
                blockers.push((path, node));
            }
        }
        let mut nested = Vec::new();
        for c in captured.iter().filter(|c| c.path.has_fields()) {
            let line = s.parsed.line_for_byte(c.start);
            let Some(node) = node_of(statements, line) else {
                continue;
            };
            for path in resolve(s, &aliases, &c.path, c.start, node) {
                nested.push(Captured {
                    path,
                    start: c.start,
                    shadow: None,
                });
            }
        }
        if blockers.is_empty() && nested.is_empty() {
            return None;
        }
        let d3 = counts.and_then(|c| {
            D3::new(
                Counts {
                    writes: eager,
                    spans: c.spans.clone(),
                    write_only: c.write_only.clone(),
                    unplaced: BTreeSet::new(),
                },
                s.func_node,
                false,
            )
        });
        Some(Self {
            d3: d3.map(Rc::new),
            d2c: D2c::new(nested, false).map(Rc::new),
            blockers,
            rows: BTreeMap::new(),
        })
    }

    /// The edge's label after the rule: only an `exact` label moves.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn refine(
        this: Option<&mut Self>,
        label: FlowConfidence,
        c: &mut RefineCtx<'_, '_>,
    ) -> FlowConfidence {
        let (Some(this), true) = (this, label.is_exact()) else {
            return label;
        };
        let label = D2c::refine(
            this.d2c.as_ref(),
            label,
            c.edge,
            c.candidates,
            c.regions,
            c.facts,
        );
        let label = D3::refine(
            this.d3.as_ref(),
            label,
            c.parsed,
            c.edge,
            c.candidates,
            c.statements,
            c.line_index,
            c.function_start,
            c.facts,
        );
        if !label.is_exact() {
            return label;
        }
        this.may_kill(label, c)
    }

    fn may_kill(&mut self, label: FlowConfidence, c: &mut RefineCtx<'_, '_>) -> FlowConfidence {
        let Some(use_node) = c
            .statements
            .get(c.edge.to.line)
            .and_then(|line| c.line_index.get(&line).copied())
        else {
            return label;
        };
        let mut blocked_any = false;
        for &candidate in c.candidates {
            let Some(from) = c.mapped_defs[candidate] else {
                continue;
            };
            if !c.in_sets[use_node].contains(candidate) {
                continue;
            }
            let blockers = &self.blockers;
            let row = self.rows.entry(candidate).or_insert_with(|| {
                let nodes: BTreeSet<usize> = blockers
                    .iter()
                    .filter(|(path, _)| affects(path, &c.defs[candidate].path))
                    .map(|&(_, node)| node)
                    .collect();
                (!nodes.is_empty())
                    .then(|| clean_row(candidate, from, &nodes, c.kill, c.successors))
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
}

/// What `refine` reads for one edge.
pub(super) struct RefineCtx<'a, 'p> {
    pub parsed: &'a ParsedFile,
    pub edge: &'a FlowEdge,
    pub candidates: &'a [usize],
    pub defs: &'a [DefSite],
    pub mapped_defs: &'a [Option<usize>],
    pub in_sets: &'a [BitSet],
    pub kill: &'a [BitSet],
    pub successors: &'a [Vec<(usize, bool)>],
    pub statements: &'a mut StatementLookup<'p>,
    pub line_index: &'a BTreeMap<Line, usize>,
    pub function_start: Line,
    pub facts: &'a BindingFacts,
    pub regions: Option<&'a super::capture_regions::JsRegions<'a>>,
}

/// The paths a write of `path` at `probe` in `node` writes through an alias: for each alias definition
/// of its base that reaches `node` and binds the same declaration, the target's path then the write's
/// fields (a chain resolves through the alias defs reaching the alias def itself).
fn resolve(
    s: &Solved<'_, '_>,
    aliases: &BTreeMap<String, Vec<AliasDef>>,
    path: &AccessPath,
    probe: usize,
    node: usize,
) -> Vec<AccessPath> {
    resolve_depth(s, aliases, path, probe, node, 0)
}

fn resolve_depth(
    s: &Solved<'_, '_>,
    aliases: &BTreeMap<String, Vec<AliasDef>>,
    path: &AccessPath,
    probe: usize,
    node: usize,
    depth: usize,
) -> Vec<AccessPath> {
    let mut out = Vec::new();
    for a in aliases.get(&path.base).into_iter().flatten() {
        let reaches = s.in_sets[node].contains(a.def) || a.node == Some(node);
        if !reaches
            || matches!(
                s.facts.relation_at(&path.base, probe, a.def),
                BindingRelation::KilledAt(_)
            )
        {
            continue;
        }
        let mut fields = a.target.fields.clone();
        fields.extend(path.fields.iter().cloned());
        let resolved = AccessPath {
            base: a.target.base.clone(),
            fields,
        };
        if resolved.base == path.base {
            continue;
        }
        let deeper = match (depth < 8, a.node) {
            (true, Some(def_node)) => {
                let start = s.defs[a.def].start_byte;
                resolve_depth(s, aliases, &resolved, start, def_node, depth + 1)
            }
            _ => Vec::new(),
        };
        if deeper.is_empty() {
            out.push(resolved);
        } else {
            out.extend(deeper);
        }
    }
    out
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
