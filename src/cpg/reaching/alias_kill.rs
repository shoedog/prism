//! #783: a write through an alias may-kills an earlier direct def of the same path (JS/TS/TSX owners).
//!
//! `const u = o; o.x = 0; u.x = 1; sink(o.x)` records the write `u.x = 1` and its alias twin, a def of `o.x`
//! (`alias_derived`). Alias-derived defs never kill in the solver (the alias may not hold on every run), so
//! `o.x@3 -> o.x@5` kept its unflagged route and stayed `exact`. Here a write through an alias is a MAY-kill:
//! the solver, its reaching sets and every edge stay as they were (label-only), and an `exact` edge D -> U
//! is relabelled `NameOnly(AliasUnstable)` when no unflagged route from D to U avoids such a write. A route
//! that bypasses the write (`if (c) u.x = 1;` with a fall-through) is a clean route, so the edge stays
//! `exact`: D really does reach U there.
//!
//! Two sources of writes through an alias, both resolved to the aliased path (`u.x` -> `o.x`):
//! - **The twin** the producer records. An alias-derived def is the twin of a write when a recorded,
//!   non-derived def at the same occurrence (line and start byte) has a different base and non-empty fields
//!   that the twin's fields end with (`u.x` for `o.x`, `a.x` for `o.k.x` through `const { k: a } = o`). The
//!   twin a declaration makes (`const { name } = o`: the def `o.name`, whose raw def is the field-less
//!   `name`) is not a write and never counts.
//! - **The census kill-only defs** for writes the producer records no def for (`u.x++`, `for (u.x of ..)`,
//!   `[u.x] = z`, `delete u.x`): a kill-only def with fields whose base is an alias in the owner's alias map.
//!   The map is re-derived here from the AST (the producer's own derivation, `owner_alias_map`); it is
//!   transient and only built for an owner that has such a def. It is flow-insensitive, so it can only
//!   add may-kills, never remove one the producer has.
//!
//! A write kills a def D by `affects` (its path is a prefix of D's, `[]` matching any component), as a
//! recorded write does. Bindings are not consulted (alias facts are name-based): the cost is a label,
//! never an edge. A write in the same CFG node as D or U is not a blocker (D3 owns order inside a node).
use super::def_buckets::affects;
use super::graph::BitSet;
use super::{DefSite, FlowConfidence, FlowDoubt};
use crate::access_path::AccessPath;
use crate::ast::ParsedFile;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Trace mechanism of an edge this rule relabelled (`PRISM_RD_CENSUS_TRACE=1`).
pub(super) const KILL_ALIAS: &str = "kill:alias";

/// The owner's writes through an alias, and the rows of per-def clean reachability computed so far.
pub(super) struct AliasKills {
    /// The aliased path each write kills, and the CFG node it sits in.
    writes: Vec<(AccessPath, usize)>,
    /// Per candidate def: the CFG nodes reachable from its node over unflagged edges without passing
    /// through a kill of the def or a node of a write that may-kills it (a blocked node can be reached,
    /// never left, as for killed nodes in `DefinitionReachability`).
    rows: BTreeMap<usize, BitSet>,
}

impl AliasKills {
    /// The owner's alias writes, or `None` when it has none (no edge can move). Owners with no
    /// alias-derived def and no kill-only def with fields do no work.
    pub(super) fn new(
        parsed: &ParsedFile,
        func_node: &Node<'_>,
        defs: &[DefSite],
        n_recorded: usize,
        mapped_defs: &[Option<usize>],
    ) -> Option<Self> {
        let mut writes = Vec::new();
        if defs[..n_recorded].iter().any(|d| d.alias_derived) {
            twin_writes(defs, n_recorded, mapped_defs, &mut writes);
        }
        if defs[n_recorded..].iter().any(|d| !d.path.fields.is_empty()) {
            census_writes(
                parsed,
                func_node,
                defs,
                n_recorded,
                mapped_defs,
                &mut writes,
            );
        }
        (!writes.is_empty()).then(|| Self {
            writes,
            rows: BTreeMap::new(),
        })
    }

    /// The edge's label after the rule: an `exact` edge whose every reaching candidate has an alias write
    /// on each unflagged route to the use becomes `NameOnly(AliasUnstable)`; any other label is kept.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn refine(
        this: Option<&mut Self>,
        label: FlowConfidence,
        candidates: &[usize],
        use_node: Option<usize>,
        defs: &[DefSite],
        mapped_defs: &[Option<usize>],
        in_use: Option<&BitSet>,
        kill: &[BitSet],
        successors: &[Vec<(usize, bool)>],
    ) -> FlowConfidence {
        let (Some(this), Some(use_node), Some(in_use), true) =
            (this, use_node, in_use, label.is_exact())
        else {
            return label;
        };
        let mut blocked_any = false;
        for &candidate in candidates {
            let Some(from) = mapped_defs[candidate] else {
                continue;
            };
            if !in_use.contains(candidate) {
                continue;
            }
            let blockers: BTreeSet<usize> = this
                .writes
                .iter()
                .filter(|(path, _)| affects(path, &defs[candidate].path))
                .map(|&(_, node)| node)
                .collect();
            if blockers.is_empty() {
                // Unaffected by any alias write: the classifier's verdict stands for this candidate.
                return label;
            }
            blocked_any = true;
            let row = this
                .rows
                .entry(candidate)
                .or_insert_with(|| clean_row(candidate, from, &blockers, kill, successors));
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

/// The writes whose twin the producer recorded.
fn twin_writes(
    defs: &[DefSite],
    n_recorded: usize,
    mapped_defs: &[Option<usize>],
    out: &mut Vec<(AccessPath, usize)>,
) {
    let mut raw: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (index, def) in defs.iter().enumerate().take(n_recorded) {
        if !def.alias_derived {
            raw.entry((def.line, def.start_byte))
                .or_default()
                .push(index);
        }
    }
    for (index, twin) in defs.iter().enumerate().take(n_recorded) {
        let is_write = twin.alias_derived
            && raw
                .get(&(twin.line, twin.start_byte))
                .is_some_and(|sources| {
                    sources.iter().any(|&j| {
                        let r = &defs[j].path;
                        !r.fields.is_empty()
                            && r.base != twin.path.base
                            && twin.path.fields.ends_with(&r.fields)
                    })
                });
        if let (true, Some(node)) = (is_write, mapped_defs[index]) {
            out.push((twin.path.clone(), node));
        }
    }
}

/// The census kill-only writes through a name the owner's alias map resolves.
fn census_writes(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    defs: &[DefSite],
    n_recorded: usize,
    mapped_defs: &[Option<usize>],
    out: &mut Vec<(AccessPath, usize)>,
) {
    let aliases = crate::data_flow::owner_alias_map(parsed, func_node);
    if aliases.is_empty() {
        return;
    }
    for (index, def) in defs.iter().enumerate().skip(n_recorded) {
        let (Some(target), Some(node)) = (aliases.get(&def.path.base), mapped_defs[index]) else {
            continue;
        };
        if def.path.fields.is_empty() {
            continue;
        }
        let target = AccessPath::from_expr(target);
        if target.base == def.path.base {
            continue;
        }
        let mut fields = target.fields;
        fields.extend(def.path.fields.iter().cloned());
        out.push((
            AccessPath {
                base: target.base,
                fields,
            },
            node,
        ));
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
