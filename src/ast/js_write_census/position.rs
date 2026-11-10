//! Which environment a position inside a callable is evaluated in (#660 S1d-1, round 2).
//!
//! A callable's ancestry is not lexical scope: the position under it decides. The callable's parameter
//! list sees the parameters; its body sees the parameters and the body's declarations; everything else
//! under the callable (a computed property or method name, a decorator of the callable or of one of its
//! parameters) is evaluated in the ENCLOSING environment, before and apart from the callable's own. One
//! rule, `position`, serves every scope check of `refusals.rs` that walks callable ancestry:
//! `writes_inner_name` (which declarations shadow a write) and `mapped_arguments` (whose `arguments` a
//! position reads). A position that cannot be classified is `None`, and each caller then keeps its
//! refusal: no shadow is assumed, and the callable's own `arguments` is assumed read.
use super::tables::Tables;
use super::walk::is_callable;
use crate::ast::ParsedFile;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Position {
    /// Under the callable's parameter list, parameter decorators excluded.
    Params,
    /// Under the callable's body.
    Body,
    /// Anywhere else under the callable: evaluated in the enclosing environment.
    Eager,
}

/// The position of `at` (a node inside `callable`) relative to `callable`.
pub(crate) fn position(parsed: &ParsedFile, callable: Node<'_>, at: Node<'_>) -> Option<Position> {
    let (mut cur, mut decorated) = (at, false);
    let child = loop {
        decorated |= cur.kind() == "decorator";
        let parent = cur.parent()?;
        if parent.id() == callable.id() {
            break cur;
        }
        cur = parent;
    };
    let is = |n: Option<Node<'_>>| n.is_some_and(|n| n.id() == child.id());
    let params = is(parsed.parameter_binding_region(&callable));
    // A decorator of the callable itself (a direct child), or of one of its parameters, is eager. A
    // decorator deeper in the body belongs to something nested in it, not to the callable.
    Some(if child.kind() == "decorator" || params && decorated {
        Position::Eager
    } else if params {
        Position::Params
    } else if is(callable.child_by_field_name("body")) {
        Position::Body
    } else {
        Position::Eager
    })
}

/// Whether `f`'s `arguments` object is mapped (sloppy, simple parameters) and read: by the shared
/// `js_ts_mapped_arguments_possible` (unchanged: `first_hazard` uses it, and it fences every nested
/// non-arrow callable whole), or by an `arguments` in an EAGER position of a nested non-arrow callable
/// (a computed key, a decorator), which is evaluated in `f`'s environment. The eager read is found
/// without the helper's read test, so a key named `arguments` there over-refuses.
pub(crate) fn mapped_arguments_possible(parsed: &ParsedFile, t: &Tables, f: Node<'_>) -> bool {
    if parsed.js_ts_mapped_arguments_possible(&f) {
        return true;
    }
    if parsed.js_ts_callable_is_strict(&f)
        || !parsed
            .parameter_binding_region(&f)
            .is_some_and(|p| parsed.js_ts_parameters_are_simple(p))
    {
        return false;
    }
    let mut stack = vec![f];
    while let Some(node) = stack.pop() {
        if matches!(node.kind(), "identifier" | "shorthand_property_identifier")
            && ParsedFile::js_ts_decoded_identifier(parsed.node_text(&node)).as_deref()
                == Some("arguments")
        {
            return true;
        }
        let nested = node.id() != f.id() && is_callable(t, node.kind());
        let (params, body) = (
            parsed.parameter_binding_region(&node),
            node.child_by_field_name("body"),
        );
        let mut c = node.walk();
        for child in node.named_children(&mut c) {
            let hidden = nested
                && node.kind() != "arrow_function"
                && (params.is_some_and(|p| p.id() == child.id())
                    || body.is_some_and(|b| b.id() == child.id()));
            if !hidden {
                stack.push(child);
            } else if params.is_some_and(|p| p.id() == child.id()) {
                // Parameter decorators run in the enclosing environment, parameters do not.
                let mut inner = vec![child];
                while let Some(n) = inner.pop() {
                    if n.kind() == "decorator" {
                        stack.push(n);
                    } else {
                        let mut cc = n.walk();
                        inner.extend(n.named_children(&mut cc));
                    }
                }
            }
        }
    }
    false
}
