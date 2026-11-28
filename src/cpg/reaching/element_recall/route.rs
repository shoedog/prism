//! I1 route phases (lane 10, #847): can a store W still be live when a whole operand R is read?
//!
//! The pass is pure syntax. It starts at the commit of W, composes ordered normal exits of the
//! enclosing statement lists, recursive `if`/`elif`/`else` and Python `for`/`while`/`else` (see
//! `loops.rs`), resolves `break` and `continue` at their lexical loop, refuses every body entry
//! into a loop that encloses W, and charges `try`/`with`/`match` completion to the unsupported
//! bucket. One walk per W visits every candidate R, so route work is memoized by W.
//!
//! Only the events of [`Transfer`] kill a route: root and static prefix replacement and
//! key-blind delete/clear (`rebind.rs`). `transfer.rs` admits the store first: an independently
//! identified local root, and a receiver that is still attached at commit.
use super::python::{kids, peel_down, span, text};
use super::transfer::{self, Transfer};
use super::{OwnerScan, Span, Write};
use crate::ast::ParsedFile;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tree_sitter::Node;

/// One phase of a certified route, recorded with the construct it belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Phase {
    /// R's operand is evaluated (always the last phase of a witness).
    Read,
    /// A header expression runs: `if`/`elif` test, `while` test, `for` iterable.
    Test,
    /// An `if` (0) or `elif` (n) arm is entered.
    Arm(usize),
    Else,
    EmptyElse,
    Join,
    /// A loop body is entered, a loop header is reached again, or a loop is left.
    Enter,
    Skip,
    Advance,
    Exhaust,
    LoopElse,
    Exit,
    /// A jump statement, and the loop that resolves it.
    Break,
    Continue,
    Resolve,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Step {
    pub phase: Phase,
    pub at: Span,
    /// Jump statement resolved by this step.
    pub target: Option<Span>,
}

struct Link {
    step: Step,
    prev: Trace,
}

/// Persistent phase list: pushing is O(1) and branches share their prefix.
#[derive(Clone, Default)]
pub(super) struct Trace(Option<Rc<Link>>);

impl Trace {
    pub(super) fn push(&self, phase: Phase, at: Node) -> Trace {
        self.push_to(phase, at, None)
    }

    pub(super) fn push_to(&self, phase: Phase, at: Node, target: Option<Span>) -> Trace {
        let step = Step {
            phase,
            at: span(at),
            target,
        };
        Trace(Some(Rc::new(Link {
            step,
            prev: self.clone(),
        })))
    }

    pub(super) fn last(&self) -> Option<Span> {
        self.0.as_ref().map(|l| l.step.at)
    }

    fn steps(&self) -> Vec<Step> {
        let (mut out, mut cur) = (Vec::new(), self.0.as_ref());
        while let Some(l) = cur {
            out.push(l.step);
            cur = l.prev.0.as_ref();
        }
        out.reverse();
        out
    }
}

impl Drop for Trace {
    fn drop(&mut self) {
        let mut cur = self.0.take();
        while let Some(rc) = cur {
            cur = match Rc::try_unwrap(rc) {
                Ok(mut link) => link.prev.0.take(),
                Err(_) => None,
            };
        }
    }
}

/// Live exits of a statement. A field is `Some` when the W-carrying state reaches it.
#[derive(Default)]
pub(super) struct Flow {
    pub normal: Option<Trace>,
    pub brk: Option<Trace>,
    pub cont: Option<Trace>,
}

impl Flow {
    pub(super) fn only(t: Option<Trace>) -> Flow {
        Flow {
            normal: t,
            ..Flow::default()
        }
    }

    pub(super) fn absorb(&mut self, o: Flow) {
        self.normal = self.normal.take().or(o.normal);
        self.brk = self.brk.take().or(o.brk);
        self.cont = self.cont.take().or(o.cont);
    }

    fn live(&self) -> bool {
        self.normal.is_some() || self.brk.is_some() || self.cont.is_some()
    }
}

/// Compound completions the pass cannot compose (DESIGN section 4 table).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Unsupported {
    Try,
    With,
    Match,
    AsyncFor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Why {
    /// Parse error in the owner, or W's statement was not found.
    Syntax,
    /// R does not follow W's statement in bytes.
    NotAfter,
    /// W sits after a statement without a normal exit or in a literally dead arm.
    UnreachableW,
    TooDeep,
    /// No certified route reaches R.
    NoRoute,
    /// W's root is not an independently identified bare local or formal of the owner.
    Binding,
    /// The receiver was captured, then its binding or a captured link was replaced or detached
    /// before the store committed.
    Capture,
}

/// Why a pair has no certified route. The flags are observations of W's whole exploration,
/// not proof that R needed them: `reentry_cut` records a refused body entry into a loop that
/// encloses W, `killed` a route that ended at a kill, `unproven` unsupported completions
/// charged to `completion_unproven`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Refused {
    pub why: Why,
    pub reentry_cut: bool,
    pub killed: bool,
    pub unproven: Vec<(Unsupported, Span)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Route {
    Reached(Vec<Step>),
    Refused(Refused),
}

impl Route {
    pub(crate) fn is_reached(&self) -> bool {
        matches!(self, Route::Reached(_))
    }
}

/// Literal truth of a test: `True`/`1` never false, `False`/`0`/`None` never true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Truth {
    True,
    False,
    Unknown,
}

pub(super) fn truth(n: Node, src: &str) -> Truth {
    let n = peel_down(n);
    match n.kind() {
        "true" => Truth::True,
        "false" | "none" => Truth::False,
        "integer" => {
            let t = text(n, src);
            match t.bytes().all(|b| b.is_ascii_digit() || b == b'_') {
                false => Truth::Unknown,
                true if t.bytes().all(|b| b == b'0' || b == b'_') => Truth::False,
                true => Truth::True,
            }
        }
        "not_operator" => match n.child_by_field_name("argument").map(|a| truth(a, src)) {
            Some(Truth::True) => Truth::False,
            Some(Truth::False) => Truth::True,
            _ => Truth::Unknown,
        },
        _ => Truth::Unknown,
    }
}

const CLAUSES: [&str; 5] = [
    "elif_clause",
    "else_clause",
    "except_clause",
    "except_group_clause",
    "finally_clause",
];
const MAX_DEPTH: usize = 160;

/// One W's exploration. `reach` mode only asks whether a statement can complete normally.
pub(super) struct Walk<'a> {
    pub(super) src: &'a str,
    pub(super) tr: &'a dyn Transfer,
    reach: bool,
    /// Outer spans of the candidate operands, by start byte, with their input index.
    rs: Vec<(Span, usize)>,
    hits: RefCell<Vec<Option<Trace>>>,
    pub(super) cut: RefCell<Vec<Span>>,
    pub(super) killed: Cell<bool>,
    unproven: RefCell<Vec<(Unsupported, Span)>>,
    depth: Cell<usize>,
    deep: Cell<bool>,
}

impl<'a> Walk<'a> {
    fn new(src: &'a str, tr: &'a dyn Transfer, rs: &[Span], reach: bool) -> Walk<'a> {
        let mut by_start: Vec<_> = rs.iter().copied().zip(0..).collect();
        by_start.sort_by_key(|(s, i)| (s.start, *i));
        Walk {
            src,
            tr,
            reach,
            rs: by_start,
            hits: RefCell::new(vec![None; rs.len()]),
            cut: RefCell::default(),
            killed: Cell::new(false),
            unproven: RefCell::default(),
            depth: Cell::new(0),
            deep: Cell::new(false),
        }
    }

    /// Evaluate a simple statement or header expression: record each operand it contains, then
    /// apply the transfer's kills. `None` when the state died here.
    pub(super) fn atom(&self, n: Node<'a>, t: Trace) -> Option<Trace> {
        if self.reach {
            return Some(t);
        }
        let (s, e) = (n.start_byte(), n.end_byte());
        let first = self.rs.partition_point(|(r, _)| r.start < s);
        let inside: Vec<_> = (self.rs[first..].iter())
            .take_while(|(r, _)| r.start < e)
            .filter(|(r, i)| r.end <= e && self.hits.borrow()[*i].is_none())
            .collect();
        // an operand evaluated after an expression-level kill of the same atom reads the
        // replacement (`take((d := {}), d)`), so it carries no contribution of W
        let spans: Vec<Span> = inside.iter().map(|(r, _)| *r).collect();
        let late = match spans.is_empty() {
            true => Vec::new(),
            false => self.tr.clobbered_before(n, &spans, self.src),
        };
        for ((r, i), late) in inside.into_iter().zip(late) {
            let mut hits = self.hits.borrow_mut();
            if !late {
                let step = Step {
                    phase: Phase::Read,
                    at: *r,
                    target: None,
                };
                let link = Link {
                    step,
                    prev: t.clone(),
                };
                hits[*i] = Some(Trace(Some(Rc::new(link))));
            }
        }
        if self.tr.kills(n, self.src) {
            self.killed.set(true);
            return None;
        }
        Some(t)
    }

    fn unsupported(&self, kind: Unsupported, n: Node<'a>, t: Trace) -> Flow {
        if self.reach {
            return Flow::only(Some(t));
        }
        self.unproven.borrow_mut().push((kind, span(n)));
        Flow::default()
    }

    pub(super) fn exec(&self, n: Node<'a>, t: Trace) -> Flow {
        if self.depth.get() >= MAX_DEPTH {
            self.deep.set(true);
            return Flow::default();
        }
        self.depth.set(self.depth.get() + 1);
        let flow = self.exec_inner(n, t);
        self.depth.set(self.depth.get() - 1);
        flow
    }

    fn exec_inner(&self, n: Node<'a>, t: Trace) -> Flow {
        match n.kind() {
            "return_statement" | "raise_statement" => {
                self.atom(n, t);
                Flow::default()
            }
            "break_statement" => Flow {
                brk: Some(t.push(Phase::Break, n)),
                ..Flow::default()
            },
            "continue_statement" => Flow {
                cont: Some(t.push(Phase::Continue, n)),
                ..Flow::default()
            },
            "if_statement" => self.exec_if(n, t),
            "while_statement" => self.exec_while(n, t),
            "for_statement" if kids(n).iter().any(|c| c.kind() == "async") => {
                self.unsupported(Unsupported::AsyncFor, n, t)
            }
            "for_statement" => self.exec_for(n, t),
            "try_statement" => self.unsupported(Unsupported::Try, n, t),
            "with_statement" => self.unsupported(Unsupported::With, n, t),
            "match_statement" => self.unsupported(Unsupported::Match, n, t),
            _ => Flow::only(self.atom(n, t)),
        }
    }

    pub(super) fn exec_block(&self, block: Node<'a>, t: Trace) -> Flow {
        self.seq(&statements(block), Some(t))
    }

    fn seq(&self, stmts: &[Node<'a>], live: Option<Trace>) -> Flow {
        let (mut acc, mut cur) = (Flow::default(), live);
        for s in stmts {
            let Some(t) = cur.take() else { break };
            let mut f = self.exec(*s, t);
            cur = f.normal.take();
            acc.absorb(f);
        }
        acc.normal = cur;
        acc
    }

    fn exec_if(&self, n: Node<'a>, t: Trace) -> Flow {
        let mut clauses = vec![(n, n.child_by_field_name("condition"))];
        let mut other = None;
        for c in kids(n) {
            match c.kind() {
                "elif_clause" => clauses.push((c, c.child_by_field_name("condition"))),
                "else_clause" => other = Some(c),
                _ => {}
            }
        }
        let (mut out, mut live) = (Flow::default(), Some(t));
        for (i, (clause, cond)) in clauses.into_iter().enumerate() {
            let (Some(t), Some(cond)) = (live.take(), cond) else {
                break;
            };
            let lit = truth(cond, self.src);
            live = self.atom(cond, t.push(Phase::Test, cond));
            let block = clause.child_by_field_name("consequence");
            if let (Some(t), Some(block), true) = (live.clone(), block, lit != Truth::False) {
                out.absorb(self.exec_block(block, t.push(Phase::Arm(i), clause)));
            }
            if lit == Truth::True {
                live = None;
            }
        }
        if let Some(t) = live {
            let body = other.and_then(|e| e.child_by_field_name("body"));
            match (other, body) {
                (Some(e), Some(b)) => out.absorb(self.exec_block(b, t.push(Phase::Else, e))),
                _ => out.normal = out.normal.take().or(Some(t.push(Phase::EmptyElse, n))),
            }
        }
        out.normal = out.normal.map(|t| t.push(Phase::Join, n));
        out
    }

    /// Walk up from W's statement, completing every enclosing construct. Returns whether W
    /// itself is reachable (earlier statements complete, no literally dead arm).
    fn climb(&self, wstmt: Node<'a>) -> bool {
        let rw = Walk::new(self.src, self.tr, &[], true);
        let mut flow = Flow::only(Some(Trace::default()));
        let mut cur = wstmt;
        loop {
            if cur.kind() != "block" {
                let Some(block) = cur.parent().filter(|p| p.kind() == "block") else {
                    return true;
                };
                let all = statements(block);
                let at = all.iter().position(|s| s.id() == cur.id()).unwrap_or(0);
                if all[..at]
                    .iter()
                    .any(|s| rw.exec(*s, Trace::default()).normal.is_none())
                {
                    return false;
                }
                let mut after = self.seq(&all[at + 1..], flow.normal.take());
                after.brk = flow.brk.take().or(after.brk);
                after.cont = flow.cont.take().or(after.cont);
                flow = after;
                cur = block;
                continue;
            }
            let Some((part, comp)) = construct_of(cur) else {
                return true;
            };
            match comp.kind() {
                "if_statement" => {
                    if !self.arm_reachable(comp, part) {
                        return false;
                    }
                    flow.normal = flow.normal.map(|t| t.push(Phase::Join, comp));
                }
                "while_statement" | "for_statement" => {
                    let in_else = part.kind() == "else_clause";
                    if !self.loop_part_reachable(comp, in_else) {
                        return false;
                    }
                    if !in_else {
                        flow = self.loop_back(comp, flow, true);
                    }
                }
                "try_statement" | "with_statement" | "match_statement" => {
                    if flow.live() {
                        let kind = match comp.kind() {
                            "try_statement" => Unsupported::Try,
                            "with_statement" => Unsupported::With,
                            _ => Unsupported::Match,
                        };
                        self.unproven.borrow_mut().push((kind, span(comp)));
                    }
                    return true;
                }
                _ => return true,
            }
            cur = comp;
        }
    }

    /// Earlier tests of the `if` chain may be false and the arm's own test may be true.
    fn arm_reachable(&self, comp: Node<'a>, part: Node<'a>) -> bool {
        let tests: Vec<_> = std::iter::once(comp)
            .chain(kids(comp).into_iter().filter(|c| c.kind() == "elif_clause"))
            .filter_map(|c| Some((c.id(), truth(c.child_by_field_name("condition")?, self.src))))
            .collect();
        let own = tests.iter().position(|(id, _)| *id == part.id());
        let before = own.unwrap_or(tests.len());
        let earlier = tests[..before].iter().all(|(_, t)| *t != Truth::True);
        earlier && own.is_none_or(|i| tests[i].1 != Truth::False)
    }
}

/// The clause (or body block's parent) holding `block`, and the compound statement owning it.
fn construct_of(block: Node) -> Option<(Node, Node)> {
    let part = block.parent()?;
    let mut comp = part;
    while CLAUSES.contains(&comp.kind()) || comp.kind() == "case_clause" {
        comp = comp.parent()?;
        if comp.kind() == "block" {
            comp = comp.parent().filter(|p| p.kind() == "match_statement")?;
        }
    }
    Some((part, comp))
}

pub(super) fn statements(block: Node) -> Vec<Node> {
    kids(block).into_iter().filter(|c| c.is_named()).collect()
}

/// Routes of every admitted pair of an owner scan, in `OwnerScan::pairs` order.
pub(crate) fn routes(parsed: &ParsedFile, scan: &OwnerScan) -> Vec<((usize, usize), Route)> {
    let body = parsed.all_functions().into_iter().find_map(|f| {
        let def = kids(f)
            .into_iter()
            .find(|c| c.kind() == "function_definition");
        let same = span(f).start == scan.span.start && span(f).end == scan.span.end;
        same.then(|| def.unwrap_or(f).child_by_field_name("body"))
            .flatten()
    });
    let mut out: Vec<_> = scan
        .pairs()
        .into_iter()
        .map(|p| (p, Route::Refused(refused(Why::Syntax))))
        .collect();
    let Some(body) = body else { return out };
    let mut writes: Vec<usize> = out.iter().map(|(p, _)| p.0).collect();
    writes.dedup();
    for w in writes {
        let slots: Vec<usize> = (0..out.len()).filter(|k| out[*k].0 .0 == w).collect();
        let spans: Vec<Span> = slots
            .iter()
            .map(|k| scan.operands[out[*k].0 .1].outer)
            .collect();
        let results = route_one(parsed, body, &scan.writes[w], &spans);
        for (k, route) in slots.into_iter().zip(results) {
            out[k].1 = route;
        }
    }
    out
}

fn refused(why: Why) -> Refused {
    Refused {
        why,
        reentry_cut: false,
        killed: false,
        unproven: Vec::new(),
    }
}

/// W's statement: the `expression_statement` holding the store.
fn store_statement<'a>(body: Node<'a>, w: &Write) -> Option<Node<'a>> {
    let mut n = body.descendant_for_byte_range(w.span.start, w.span.end)?;
    while n.kind() != "expression_statement" {
        n = n.parent().filter(|p| p.id() != body.id())?;
    }
    Some(n)
}

/// True when a loop of the owner encloses both W's statement and the byte `at`.
fn shares_loop(wstmt: Node, at: usize) -> bool {
    let mut n = wstmt;
    while let Some(p) = n.parent() {
        let loop_ = matches!(p.kind(), "for_statement" | "while_statement");
        if loop_ && p.start_byte() <= at && at < p.end_byte() {
            return true;
        }
        if p.kind() == "function_definition" {
            break;
        }
        n = p;
    }
    false
}

fn route_one(parsed: &ParsedFile, body: Node, w: &Write, rs: &[Span]) -> Vec<Route> {
    let mut all = vec![Route::Refused(refused(Why::Syntax)); rs.len()];
    let Some(wstmt) = store_statement(body, w) else {
        return all;
    };
    if body.has_error() {
        return all;
    }
    let mut live = Vec::new();
    for (i, r) in rs.iter().enumerate() {
        if r.start < wstmt.end_byte() {
            let mut why = refused(Why::NotAfter);
            why.reentry_cut = shares_loop(wstmt, r.start);
            all[i] = Route::Refused(why);
        } else {
            live.push(i);
        }
    }
    let src = parsed.source.as_str();
    let adm = match transfer::admit(body, w, src) {
        Ok(a) => a,
        Err(why) => {
            live.iter()
                .for_each(|i| all[*i] = Route::Refused(refused(why)));
            return all;
        }
    };
    if adm.chain_killed {
        let mut why = refused(Why::NoRoute);
        why.killed = true;
        live.iter()
            .for_each(|i| all[*i] = Route::Refused(why.clone()));
        return all;
    }
    let spans: Vec<Span> = live.iter().map(|i| rs[*i]).collect();
    let walk = Walk::new(src, &adm.tr, &spans, false);
    let reachable = walk.climb(wstmt);
    let hits = walk.hits.borrow();
    for (k, i) in live.into_iter().enumerate() {
        all[i] = match &hits[k] {
            Some(t) if reachable => Route::Reached(t.steps()),
            _ => {
                let why = match (reachable, walk.deep.get()) {
                    (false, _) => Why::UnreachableW,
                    (_, true) => Why::TooDeep,
                    _ => Why::NoRoute,
                };
                Route::Refused(Refused {
                    why,
                    reentry_cut: !walk.cut.borrow().is_empty(),
                    killed: walk.killed.get(),
                    unproven: walk.unproven.borrow().clone(),
                })
            }
        };
    }
    all
}
