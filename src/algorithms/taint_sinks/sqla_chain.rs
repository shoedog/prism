//! Maximal chain of a clause hit (#636 design A, section 2.7): a `where`/`filter` call is weak
//! only when the whole method chain containing it proves. Methods applied to its result and
//! methods of its receiver must be on the `sqla_tables::CHAIN` list with proven arguments. The
//! root may be built elsewhere (a parameter, an attribute, a call of an unknown function); a
//! library statement or a local binding is proven.

use super::py_scope::{Bound, Local};
use super::sqla::{peel, Proof};
use super::sqla_tables as t;
use tree_sitter::Node;

impl<'a> Proof<'a> {
    /// Methods applied to the hit's result must all be on the table with proven arguments.
    pub(super) fn outer_chain(&self, call: Node<'_>) -> bool {
        let mut cur = call;
        loop {
            let Some(attr) = cur.parent().filter(|a| {
                a.kind() == "attribute" && a.child_by_field_name("object") == Some(cur)
            }) else {
                return true;
            };
            let Some(outer) = attr
                .parent()
                .filter(|o| o.kind() == "call" && o.child_by_field_name("function") == Some(attr))
            else {
                return true;
            };
            let en = attr
                .child_by_field_name("attribute")
                .and_then(|m| t::find(t::CHAIN, self.text(m)));
            if !en.is_some_and(|en| self.args_ok(outer, en)) {
                return false;
            }
            cur = outer;
        }
    }

    /// The receiver chain: table methods with proven arguments down to a root. The root may be
    /// built elsewhere (a parameter, an attribute, a call of an unknown function); a library
    /// statement or a local is proven.
    pub(super) fn inner_chain(&self, n: Node<'_>) -> bool {
        if !self.tick() {
            return false;
        }
        let n = peel(n);
        match n.kind() {
            "call" => {
                let Some(f) = n.child_by_field_name("function") else {
                    return false;
                };
                if self.lib_of(f).is_some() {
                    return self.stmt(n);
                }
                if f.kind() != "attribute" {
                    return true;
                }
                let (Some(obj), Some(m)) = (
                    f.child_by_field_name("object"),
                    f.child_by_field_name("attribute"),
                ) else {
                    return false;
                };
                let m = self.text(m);
                if m == "query" {
                    return self.args_by(n, "C", &[]);
                }
                match t::find(t::CHAIN, m) {
                    Some(en) => self.args_ok(n, en) && self.inner_chain(obj),
                    // An unknown method in the middle of a chain hides what is below it.
                    None => obj.kind() != "call",
                }
            }
            "identifier" => match self.local(n, self.text(n)) {
                Local::Multi => false,
                // A local root is proven whatever its initializer form; unpacking and walrus
                // bindings refuse.
                Local::One(Bound::Assign(v)) => self.hop(|| self.inner_chain(v)),
                Local::One(Bound::Opaque | Bound::Declared) => false,
                _ => true,
            },
            "conditional_expression" => {
                n.named_child(0).is_none_or(|b| self.inner_chain(b))
                    && n.named_child(2).is_none_or(|e| self.inner_chain(e))
            }
            "boolean_operator" => {
                n.child_by_field_name("left")
                    .is_none_or(|l| self.inner_chain(l))
                    && n.child_by_field_name("right")
                        .is_none_or(|r| self.inner_chain(r))
            }
            _ => true,
        }
    }
}
