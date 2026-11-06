//! The narrow same-node order relation (#660 S0, DESIGN.md 7.1).
//!
//! `precedes(x, y)` is proven only when every kind on both chains is `seq` and x's time is strictly
//! before y's. Anything else is unknown, and unknown refuses. There is no clause table.

use super::tables::Tables;
use super::walk::{Form, Site};
use tree_sitter::Node;

/// A read or write positioned inside one statement span.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Position {
    /// A read is at its start byte; a write is at its construct's end byte.
    pub time: usize,
    /// Tie-break among writes with the same moment: the larger construct writes later.
    pub rank: Option<usize>,
    /// Kinds from the position (or its construct) up to the mapped statement; `None` when unmapped.
    pub chain: Option<Vec<&'static str>>,
    pub span: Option<(usize, usize)>,
}

impl Position {
    pub(crate) fn of_site(s: &Site) -> Position {
        Position {
            time: s.moment,
            rank: s.construct_start.map(|c| usize::MAX - c),
            chain: s.chain.clone(),
            span: s.span,
        }
    }

    /// A read of `node`, mapped through the same statement spans the walk used.
    pub(crate) fn of_read(node: Node<'_>, spans: &[(usize, usize)]) -> Position {
        let (s, e) = (node.start_byte(), node.end_byte());
        let span = spans
            .iter()
            .filter(|(a, b)| *a <= s && e <= *b)
            .min_by_key(|(a, b)| (b - a, std::cmp::Reverse(*a)))
            .copied();
        let chain = span.and_then(|(a, b)| {
            let mut chain = Vec::new();
            let mut cur = node.parent();
            while let Some(n) = cur {
                chain.push(n.kind());
                if n.start_byte() == a && n.end_byte() == b {
                    return Some(chain);
                }
                cur = n.parent();
            }
            None
        });
        Position {
            time: s,
            rank: None,
            chain,
            span,
        }
    }
}

/// Why `precedes(x, y)` is not proven (the trace's `d3-*` split), or `Proven`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Order {
    Proven,
    /// Both chains are all `seq` and the byte order is not x before y.
    Byte,
    /// A `loop` kind on a chain (and no `unknown` or `deferred` kind): a repeated context.
    Loop,
    /// An unmapped position, positions of different spans, or an `unknown`, `deferred` or other
    /// non-`seq` kind on a chain.
    Unknown,
}

/// The order of `x` and `y` with the reason it is not proven (`precedes` is `Proven`).
pub(crate) fn order(t: &Tables, x: &Position, y: &Position) -> Order {
    let (Some(cx), Some(cy)) = (&x.chain, &y.chain) else {
        return Order::Unknown;
    };
    if x.span != y.span || x.span.is_none() {
        return Order::Unknown;
    }
    let class = |k: &&'static str| t.order_class(k);
    let kinds = || cx.iter().chain(cy.iter());
    if kinds().any(|k| !matches!(class(k), Some("seq" | "loop"))) {
        return Order::Unknown;
    }
    if kinds().any(|k| class(k) == Some("loop")) {
        return Order::Loop;
    }
    let before = if x.time != y.time {
        x.time < y.time
    } else {
        matches!((x.rank, y.rank), (Some(a), Some(b)) if a < b)
    };
    if before {
        Order::Proven
    } else {
        Order::Byte
    }
}

/// `x` is proven to execute before `y` on every execution of their shared node.
pub(crate) fn precedes(t: &Tables, x: &Position, y: &Position) -> bool {
    order(t, x, y) == Order::Proven
}

/// The single-site exception (E2): the counting writes of the node are exactly one `RW`, update or
/// `delete` site and the use is that site's own operand read.
pub(crate) fn single_site_operand(counting: &[&Site], use_start: usize) -> bool {
    match counting {
        [w] => operand_exception(
            w.form,
            w.construct_kind.is_some(),
            w.target_start,
            use_start,
        ),
        _ => false,
    }
}

/// The per-site test of the exception: an `RW` or `delete` construct whose target is the use.
pub(crate) fn operand_exception(
    form: Form,
    has_construct: bool,
    target_start: usize,
    use_start: usize,
) -> bool {
    matches!(form, Form::ReadWrite | Form::Delete) && has_construct && target_start == use_start
}
