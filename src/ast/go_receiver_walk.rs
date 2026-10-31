//! Go receiver facts for one immutable function in a classification phase.
//! Registrations are weak and thread-local; guards own all storage and borrow
//! the parsed file. No AST nodes, locks, or facts survive the guard.
//!
//! Closure fences (P11 S2 / B1 review blockers): Go's function_node_types
//! omits func_literal, so the ordinary named-function fence is insufficient.
//! Two different leaks must stay closed in walk_receiver_bindings and here:
//! (1) ignoring a containing literal's parameter lets stale outer recovery
//! reach its own call; (2) visiting a sibling literal's local :=/var binding
//! can mint a false Exact edge for a call outside that literal. Exclude the
//! ENTIRE subtree unless the call is within its half-open byte span. Count
//! parameters only inside; an earlier outer binding keeps the first fact for
//! the shadowed R3 route. Then inspect that literal's body for later bindings.
//! Parameter lists are one transaction: first evidence is set after the list,
//! including malformed separate declarations of the same name.
//!
//! go_short_var_call_rhs has the same B1 fence: a sibling closure's x := newT()
//! must not become an unrelated later call's qualifying statement. This helper
//! relocates a FIRST-LHS receiver's single-call RHS and returns raw callee text
//! (newDemux or pkg.New); go_receiver_index::resolve_go_return_type_call owns
//! static return-type resolution. A non-first receiver (_, err := f()),
//! multi-expression RHS (a, b := f(), g()), or non-call RHS does not qualify.
//! Callers MUST already prove exactly one unshadowed receiver binding for the
//! same (receiver, call_line, call_start_byte); this helper does not recount it.
//! The binding can be untyped or provisionally typed by the NewX heuristic.
//! A receiver-first statement prunes its subtree even for a nonqualifying RHS;
//! later siblings remain searchable. Unlike binding evidence, no scope filter
//! is applied beyond the literal fence. Preserve these distinct contracts.
//!
//! Recovery text is interned during collection and shared with Rc through
//! seeds, events, segment-tree nodes and checkpoints. Only returned evidence
//! owns String copies; retained payload bytes follow distinct type text.
use super::ParsedFile;
use crate::languages::Language;
use crate::resolution::ReceiverRecovery;
use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use tree_sitter::Node;

mod auxiliary;
mod collect;
mod intervals;
#[cfg(test)]
mod tests;
mod timeline;

type Recovery = (Rc<str>, ReceiverRecovery);
type OwnedRecovery = (String, ReceiverRecovery);
type Evidence = (Option<OwnedRecovery>, usize, Option<OwnedRecovery>);

struct SpanValue<T> {
    lo: usize,
    hi: usize,
    value: T,
}
#[derive(Default)]
struct NameFacts {
    seed: timeline::Summary,
    events: Vec<timeline::Event>,
    reuses: Vec<SpanValue<Result<String, ()>>>,
    rhs: Vec<SpanValue<String>>,
    reuse_index: intervals::Intervals<Result<String, ()>>,
    rhs_index: intervals::Intervals<String>,
    timelines: [OnceCell<timeline::Timeline>; 4],
}
struct Facts {
    names: BTreeMap<String, NameFacts>,
    auxiliary: auxiliary::Facts,
}
struct Registration {
    file: usize,
    tree: usize,
    function: usize,
    facts: Weak<OnceCell<Facts>>,
}
thread_local! {
    static ACTIVE: RefCell<Vec<Registration>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct Session<'a> {
    _parsed: &'a ParsedFile,
    facts: Rc<OnceCell<Facts>>,
}
impl<'a> Session<'a> {
    pub(crate) fn new(p: &'a ParsedFile, function: Node<'_>) -> Option<Self> {
        if p.language != Language::Go {
            return None;
        }
        let facts = Rc::new(OnceCell::new());
        ACTIVE.with(|active| {
            active.borrow_mut().push(Registration {
                file: p as *const _ as usize,
                tree: p.tree.root_node().id(),
                function: function.id(),
                facts: Rc::downgrade(&facts),
            })
        });
        Some(Self { _parsed: p, facts })
    }
}
impl Drop for Session<'_> {
    fn drop(&mut self) {
        let weak = Rc::downgrade(&self.facts);
        ACTIVE.with(|active| {
            active
                .borrow_mut()
                .retain(|row| !Weak::ptr_eq(&row.facts, &weak))
        });
    }
}

fn with_facts<R>(p: &ParsedFile, function: Node<'_>, query: impl FnOnce(&Facts) -> R) -> Option<R> {
    #[cfg(test)]
    if bypassed() {
        return None;
    }
    let facts = ACTIVE.with(|active| {
        active
            .borrow()
            .iter()
            .rev()
            .find(|row| {
                row.file == p as *const _ as usize
                    && row.tree == p.tree.root_node().id()
                    && row.function == function.id()
            })
            .and_then(|row| row.facts.upgrade())
    })?;
    Some(query(facts.get_or_init(|| {
        #[cfg(test)]
        SCANS.with(|n| n.set(n.get() + 1));
        #[cfg(test)]
        TOTAL_SCANS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut rows = collect::collect(p, function);
        for row in rows.values_mut() {
            row.reuse_index = intervals::Intervals::new(std::mem::take(&mut row.reuses));
            row.rhs_index = intervals::Intervals::new(std::mem::take(&mut row.rhs));
        }
        Facts {
            names: rows,
            auxiliary: auxiliary::Facts::new(p, function),
        }
    })))
}

// Nonstandard callers may supply a line inconsistent with the byte. Preserve
// the reference's independent line gate by falling back for those queries.
fn consistent_line(p: &ParsedFile, line: usize, byte: usize) -> bool {
    byte <= p.source.len() && p.line_offsets.partition_point(|start| *start <= byte) == line
}

pub(super) fn evidence(
    p: &ParsedFile,
    function: Node<'_>,
    name: &str,
    line: usize,
    byte: usize,
    var: bool,
    reuse: bool,
) -> Option<Evidence> {
    if p.language != Language::Go || !consistent_line(p, line, byte) {
        return None;
    }
    with_facts(p, function, |rows| {
        let Some(row) = rows.names.get(name) else {
            return (None, 0, None);
        };
        row.timelines[usize::from(var) * 2 + usize::from(reuse)]
            .get_or_init(|| timeline::Timeline::new(&row.seed, &row.events, var, reuse))
            .at(byte)
    })
}

pub(super) fn reuses(
    p: &ParsedFile,
    function: Node<'_>,
    name: &str,
    byte: usize,
) -> Option<Result<Vec<String>, ()>> {
    with_facts(p, function, |rows| {
        let mut result = Ok(Vec::new());
        if let Some(row) = rows.names.get(name) {
            row.reuse_index.visit(byte, |value| {
                match (result.as_mut(), value) {
                    (Ok(out), Ok(call)) => out.push(call.clone()),
                    _ => {
                        result = Err(());
                        return false;
                    }
                }
                true
            });
        }
        result
    })
}

pub(super) fn rhs(
    p: &ParsedFile,
    function: Node<'_>,
    name: &str,
    line: usize,
    byte: usize,
) -> Option<Option<String>> {
    if !consistent_line(p, line, byte) {
        return None;
    }
    with_facts(p, function, |rows| {
        let mut result = None;
        if let Some(row) = rows.names.get(name) {
            row.rhs_index.visit(byte, |value| {
                result = Some(value.clone());
                false
            });
        }
        result
    })
}

pub(super) fn auxiliary(
    p: &ParsedFile,
    function: Node<'_>,
    name: &str,
    byte: usize,
    literal: bool,
) -> Option<bool> {
    with_facts(p, function, |facts| facts.auxiliary.at(name, byte, literal))
}

#[cfg(test)]
thread_local! {
    static BYPASS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
static TOTAL_SCANS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
#[cfg(test)]
fn bypassed() -> bool {
    BYPASS.with(std::cell::Cell::get) || std::env::var_os("PRISM_TEST_GO_WALK_REFERENCE").is_some()
}
