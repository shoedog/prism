//! Evaluation order inside one collapsed CFG node (#639 S2 fold 2).
//!
//! Several statements, header clauses and branch arms can share a source line
//! and so one CFG node. Their order is not byte order: `if c { w } else { r }`
//! never runs both, a `for` post runs after its body. Every write, read and
//! function-literal creation records a [`Pos`]: the start byte of its simple
//! statement and the chain of enclosing branching constructs with the arm it
//! sits in. [`order`] says whether one position can run before another in a
//! single pass through the node.
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// Arms: 0 init, 1 condition, 2 consequence, 3 alternative.
    If,
    /// Arms: 0 init, 1 header value, 2.. one per case (mutually exclusive).
    Switch,
    /// Arms: 0 init, 1 condition, 2 body, 3 post.
    For,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Step {
    /// Start byte of the construct.
    pub id: usize,
    pub kind: Kind,
    pub arm: u8,
}

pub(super) const POST: u8 = 3;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Pos {
    /// Start byte of the simple statement (or header clause) evaluating it.
    pub stmt: usize,
    /// Enclosing constructs, outermost first.
    pub route: Vec<Step>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Order {
    /// `a` can run before `b` in one pass.
    Before,
    /// `b` runs before `a` in one pass, and `a` never precedes `b`.
    After,
    /// The same simple statement: right side first, then the write.
    Same,
    /// Different arms of one branch: never both in one pass.
    Exclusive,
    /// `a` is the post of the `for` with this id and `b` its condition or
    /// body: `a` precedes `b` only around the loop.
    LoopBack(usize),
}

/// The key that orders `pos` against a sibling at route depth `depth`: the
/// start of the enclosing construct there, or the statement itself.
fn key(pos: &Pos, depth: usize) -> usize {
    pos.route.get(depth).map_or(pos.stmt, |s| s.id)
}

pub(super) fn order(a: &Pos, b: &Pos) -> Order {
    let shared = a.route.len().min(b.route.len());
    for depth in 0..shared {
        let (x, y) = (a.route[depth], b.route[depth]);
        if x.id != y.id {
            return by_key(key(a, depth), key(b, depth));
        }
        if x.arm == y.arm {
            continue;
        }
        let exclusive_from = match x.kind {
            Kind::If | Kind::Switch => 2,
            Kind::For => u8::MAX,
        };
        if x.arm >= exclusive_from && y.arm >= exclusive_from {
            return Order::Exclusive;
        }
        return match x.arm.cmp(&y.arm) {
            Ordering::Less => Order::Before,
            // The post reaches the condition and the body again only around
            // the loop; the init runs once, before it.
            _ if x.kind == Kind::For && x.arm == POST && y.arm != 0 => Order::LoopBack(x.id),
            _ => Order::After,
        };
    }
    by_key(key(a, shared), key(b, shared))
}

fn by_key(a: usize, b: usize) -> Order {
    match a.cmp(&b) {
        Ordering::Less => Order::Before,
        Ordering::Equal => Order::Same,
        Ordering::Greater => Order::After,
    }
}

impl Order {
    pub(super) fn is_before(self) -> bool {
        self == Order::Before
    }
}
