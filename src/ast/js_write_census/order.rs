//! The narrow same-node order relation (#660 S0, DESIGN.md 7.1).
//!
//! `precedes(x, y)` is proven only when every kind on both chains is `seq` and x's time is strictly
//! before y's. Anything else is unknown, and unknown refuses. There is no clause table.

use super::tables::Tables;
use super::walk::{Form, Site};
use tree_sitter::Node;

/// A read or write positioned inside one statement span.
#[derive(Clone, Debug)]
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

/// `x` is proven to execute before `y` on every execution of their shared node.
pub(crate) fn precedes(t: &Tables, x: &Position, y: &Position) -> bool {
    let (Some(cx), Some(cy)) = (&x.chain, &y.chain) else {
        return false;
    };
    if x.span != y.span || x.span.is_none() {
        return false;
    }
    let seq = |k: &&'static str| t.order_class(k) == Some("seq");
    if !cx.iter().all(&seq) || !cy.iter().all(&seq) {
        return false;
    }
    if x.time != y.time {
        return x.time < y.time;
    }
    matches!((x.rank, y.rank), (Some(a), Some(b)) if a < b)
}

/// The single-site exception (E2): the counting writes of the node are exactly one `RW`, update or
/// `delete` site and the use is that site's own operand read.
pub(crate) fn single_site_operand(counting: &[&Site], use_start: usize) -> bool {
    match counting {
        [w] => {
            matches!(w.form, Form::ReadWrite | Form::Delete)
                && w.construct_kind.is_some()
                && w.target_start == use_start
        }
        _ => false,
    }
}
