//! #660 S1d-1: the refusal classes as labels (DESIGN.md 6.1-6.1e, G6a, G6b, 8).
//!
//! `js_write_census::refusals` reads an owner once; this applies its answer to each edge. Every class
//! moves only an `exact` or `sameline` label, and only toward `NameOnly`; a label already `NameOnly`
//! (killed, alias, incomplete) is kept, as D2-c does. Order, in `CensusFacts::refine`: owner and name
//! refusals and `with` bodies (`early`) -> D2-c -> dead formals (`late`) -> D3.
//!
//! - eval (6.1b): the owner is refused; every edge becomes `CfgIncomplete`.
//! - names (6.1 `arguments`, 6.1e Annex B): an edge whose from or to base is refused is `CfgIncomplete`.
//! - defs (G6a, G6b): an edge whose from def starts at a refused byte is `CfgIncomplete`.
//! - `with` (6.1c): an edge whose use or def lies in a `with` body is `CfgIncomplete`. A write in a body
//!   is also a kill-only may-write, which the census already adds.
//! - dead formals (6.1d): an edge from the dead formal's def is `Killed { kill_line }`.
use super::{FlowConfidence, FlowDoubt};
use crate::data_flow::{FlowEdge, VarLocation};
use crate::js_write_census::refusals::{self, Classes};
use std::cell::RefCell;
use std::collections::BTreeMap;

type EdgeKey = (VarLocation, VarLocation);

#[cfg(test)]
thread_local! {
    /// Test seam: the refusal classes off, to compare the solver with and without them.
    pub(super) static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
fn off() -> bool {
    OFF.with(|o| o.get())
}

#[cfg(not(test))]
fn off() -> bool {
    false
}

/// One owner's refusal classes, shared (not copied) by the census facts' clones.
pub(super) struct Refusals {
    classes: Classes,
    /// The mechanism of each edge refused early / late, kept only while `PRISM_RD_CENSUS_TRACE` is on.
    early_log: Option<RefCell<BTreeMap<EdgeKey, &'static str>>>,
    late_log: Option<RefCell<BTreeMap<EdgeKey, &'static str>>>,
}

fn refinable(label: FlowConfidence) -> bool {
    label.is_exact() || label == FlowConfidence::NameOnly(FlowDoubt::SameLine)
}

impl Refusals {
    pub(super) fn new(classes: Classes, trace: bool) -> Option<Self> {
        (!classes.is_empty()).then(|| Refusals {
            classes,
            early_log: trace.then(|| RefCell::new(BTreeMap::new())),
            late_log: trace.then(|| RefCell::new(BTreeMap::new())),
        })
    }

    /// The mechanism that refuses the edge before D2-c, if any.
    fn early(&self, edge: &FlowEdge) -> Option<&'static str> {
        let c = &self.classes;
        if c.eval {
            return Some(refusals::EVAL);
        }
        let (from, to) = (&edge.from, &edge.to);
        for base in [&from.path.base, &to.path.base] {
            if let Some(m) = c.names.get(base) {
                return Some(m);
            }
        }
        if let Some(m) = c.defs.get(&from.start_byte) {
            return Some(m);
        }
        (in_with(&c.with_bodies, from, false) || in_with(&c.with_bodies, to, true))
            .then_some(refusals::WITH)
    }

    /// The label after the owner, name and `with` refusals.
    pub(super) fn refine_early(
        this: Option<&std::rc::Rc<Refusals>>,
        label: FlowConfidence,
        edge: &FlowEdge,
    ) -> FlowConfidence {
        let Some(this) = this.filter(|_| refinable(label) && !off()) else {
            return label;
        };
        let Some(mechanism) = this.early(edge) else {
            return label;
        };
        if let Some(log) = &this.early_log {
            log.borrow_mut()
                .insert((edge.from.clone(), edge.to.clone()), mechanism);
        }
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
    }

    /// The label after the dead-formal rule: an edge from a dead formal's def is killed at the line
    /// of the declaration (or the default) that replaces it.
    pub(super) fn refine_late(
        this: Option<&std::rc::Rc<Refusals>>,
        label: FlowConfidence,
        edge: &FlowEdge,
    ) -> FlowConfidence {
        let Some(this) = this.filter(|_| refinable(label) && !off()) else {
            return label;
        };
        let from = &edge.from;
        let Some(dead) = this.classes.dead.iter().find(|d| {
            d.name == from.path.base && d.range.0 <= from.start_byte && from.start_byte < d.range.1
        }) else {
            return label;
        };
        if let Some(log) = &this.late_log {
            log.borrow_mut()
                .insert((edge.from.clone(), edge.to.clone()), refusals::DEAD_FORMAL);
        }
        FlowConfidence::NameOnly(FlowDoubt::Killed {
            kill_line: u32::try_from(dead.line).unwrap_or(u32::MAX),
        })
    }

    /// The mechanism that refused the edge before D2-c (trace on).
    pub(super) fn logged_early(&self, edge: &FlowEdge) -> Option<&'static str> {
        let key = (edge.from.clone(), edge.to.clone());
        self.early_log.as_ref()?.borrow().get(&key).copied()
    }

    /// The mechanism that killed the edge after D2-c (trace on).
    pub(super) fn logged_late(&self, edge: &FlowEdge) -> Option<&'static str> {
        let key = (edge.from.clone(), edge.to.clone());
        self.late_log.as_ref()?.borrow().get(&key).copied()
    }
}

/// Whether the location lies in a `with` body: by its bytes, or, for a read the DFG left without
/// bytes (a member-only formal's projection), by its line (a use on a line the body covers).
/// A def always has bytes.
fn in_with(bodies: &[refusals::WithBody], loc: &VarLocation, use_side: bool) -> bool {
    if loc.end_byte > loc.start_byte || !use_side {
        return bodies
            .iter()
            .any(|b| b.bytes.0 <= loc.start_byte && loc.start_byte < b.bytes.1);
    }
    bodies
        .iter()
        .any(|b| b.lines.0 <= loc.line && loc.line <= b.lines.1)
}
