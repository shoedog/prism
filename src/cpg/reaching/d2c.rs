//! #660 S1c: D2-c, the captured-write refusal (DESIGN.md 6.2, 8).
//!
//! A write in a region (a nested callable, an instance field initializer: `capture_regions`) can run
//! between an outer def and an outer read, at a time reaching definitions does not model. For an edge
//! D->U that would be `exact` (or `sameline`), D2-c refuses it when a write `p` in a region R
//! - is not provably scoped by a declaration inside R (`census::captured`),
//! - writes the binding the candidate def belongs to (B, the innermost declaration by scope alone, TDZ
//!   aside: a closure writes the `let` it names even when that is declared later; or any binding when B is
//!   unknown or the def's is), with `affects(p, U.path)`, and
//! - U lies outside R.
//!
//! There is no order proof and no creation exception (E5 removed, recovery #667): the read may follow any
//! run of R. A callable is one unit (its parameter list and body: a use anywhere in it is inside R). Each
//! unit enclosing the write is judged on its own, so a write two units deep also refuses the use
//! between the two. Label-only: it moves `exact` and `sameline` to `NameOnly(CfgIncomplete)`; a label
//! that is already `NameOnly` (killed, alias, incomplete) is kept.
use super::capture_regions::{JsRegions, Placement};
use super::{FlowConfidence, FlowDoubt};
use crate::access_path::AccessPath;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::data_flow::{FlowEdge, VarLocation};
use crate::js_write_census::captured::{Captured, Scope};
use std::cell::{OnceCell, RefCell};
use std::collections::{BTreeMap, BTreeSet};

/// Trace mechanism (DESIGN.md 10).
pub(super) const D2C: &str = "d2c";

#[cfg(test)]
thread_local! {
    /// Test seam: D2-c off, to compare the solver with and without it.
    pub(super) static OFF: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

type EdgeKey = (VarLocation, VarLocation);

/// What a write outside a unit targets.
#[derive(PartialEq)]
enum Target {
    /// No declaration inside a region scopes it: the innermost declaration by scope alone (a flat
    /// fallback when there is none).
    Outer(BindingId),
    /// A declaration in an enclosing unit scopes it: the binding is that declaration's, which the facts
    /// may not know. A def binds it exactly when it lies in the declaration's scope, which excludes the
    /// narrower same-name declarations nested in it (their defs bind those instead).
    Declared(Scope),
}

/// One write judged against one enclosing region of its own.
struct Level {
    path: AccessPath,
    /// The unit (callable or field value) the write is in; `None`: no region holds the write's byte.
    group: Option<usize>,
    target: Target,
}

/// One owner's D2-c state, shared (not copied) by the census facts' clones.
pub(super) struct D2c {
    captured: Vec<Captured>,
    /// The levels by base, built once per owner on the first edge D2-c sees.
    levels: OnceCell<BTreeMap<String, Vec<Level>>>,
    /// The edges D2-c refused, kept only while `PRISM_RD_CENSUS_TRACE` is on.
    log: Option<RefCell<BTreeSet<EdgeKey>>>,
}

impl D2c {
    pub(super) fn new(captured: Vec<Captured>, trace: bool) -> Option<Self> {
        (!captured.is_empty()).then(|| D2c {
            captured,
            levels: OnceCell::new(),
            log: trace.then(|| RefCell::new(BTreeSet::new())),
        })
    }

    /// The edge's label after D2-c: an `exact` or `sameline` edge a captured write may reach becomes
    /// `NameOnly(CfgIncomplete)`.
    pub(super) fn refine(
        d2c: Option<&std::rc::Rc<D2c>>,
        label: FlowConfidence,
        edge: &FlowEdge,
        candidates: &[usize],
        js: Option<&JsRegions<'_>>,
        facts: &BindingFacts,
    ) -> FlowConfidence {
        #[cfg(test)]
        if OFF.with(|off| off.get()) {
            return label;
        }
        let refinable = label.is_exact() || label == FlowConfidence::NameOnly(FlowDoubt::SameLine);
        let (Some(d2c), Some(js), true) = (d2c, js, refinable) else {
            return label;
        };
        if !d2c.refuses(edge, candidates, js, facts) {
            return label;
        }
        if let Some(log) = &d2c.log {
            log.borrow_mut()
                .insert((edge.from.clone(), edge.to.clone()));
        }
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete)
    }

    /// Whether D2-c refused the edge (trace on).
    pub(super) fn logged(&self, edge: &FlowEdge) -> bool {
        self.log
            .as_ref()
            .is_some_and(|log| log.borrow().contains(&(edge.from.clone(), edge.to.clone())))
    }

    fn levels(&self, js: &JsRegions<'_>, facts: &BindingFacts) -> &BTreeMap<String, Vec<Level>> {
        self.levels.get_or_init(|| {
            let mut out: BTreeMap<String, Vec<Level>> = BTreeMap::new();
            for c in &self.captured {
                let base = &c.path.base;
                let mut push = |group: Option<usize>, target: Target| {
                    let levels = out.entry(base.clone()).or_default();
                    let same =
                        |l: &Level| l.group == group && l.target == target && l.path == c.path;
                    if !levels.iter().any(same) {
                        levels.push(Level {
                            path: c.path.clone(),
                            group,
                            target,
                        });
                    }
                };
                let groups = js.groups_at(c.start);
                if groups.is_empty() {
                    // The census puts the write in a region and the regions do not: refuse the base.
                    push(None, Target::Outer(BindingId::FlatFallback(base.clone())));
                }
                for group in groups {
                    let (start, end) = js.unit(group);
                    let target = match &c.shadow {
                        // the innermost declaration is inside the unit: not an outer write
                        Some(d) if start <= d.start && d.start < end => continue,
                        Some(d) => Target::Declared(d.scope.clone()),
                        None => Target::Outer(facts.binding_by_scope_at(base, start)),
                    };
                    push(Some(group), target);
                }
            }
            out
        })
    }

    fn refuses(
        &self,
        edge: &FlowEdge,
        candidates: &[usize],
        js: &JsRegions<'_>,
        facts: &BindingFacts,
    ) -> bool {
        let Some(levels) = self.levels(js, facts).get(&edge.to.path.base) else {
            return false;
        };
        let to = &edge.to;
        let use_groups = OnceCell::new();
        levels.iter().any(|level| {
            if !super::def_buckets::affects(&level.path, &to.path) {
                return false;
            }
            let binding_hit = match &level.target {
                Target::Outer(BindingId::FlatFallback(_)) => true,
                Target::Outer(b) => candidates.iter().any(|&c| {
                    let def = facts.binding_of_def(c);
                    def == b || matches!(def, BindingId::FlatFallback(_))
                }),
                Target::Declared(scope) => scope.contains(edge.from.start_byte),
            };
            binding_hit
                && level.group.is_none_or(|g| {
                    let groups = use_groups.get_or_init(|| use_groups_of(js, edge));
                    !groups.contains(&g)
                })
        })
    }
}

/// The units holding the use: by its real bytes, else by the line-keyed placement S1a uses (a use that
/// cannot be placed, or whose reads disagree, is in none: outside every region).
fn use_groups_of(js: &JsRegions<'_>, edge: &FlowEdge) -> Vec<usize> {
    let to = &edge.to;
    if to.end_byte > to.start_byte {
        return js.groups_at(to.start_byte);
    }
    match js.line_region(&to.path, to.line) {
        Some(Placement::Region(region)) => js.groups_of(region),
        _ => Vec::new(),
    }
}
