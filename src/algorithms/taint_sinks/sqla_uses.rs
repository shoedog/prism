//! Occurrence rules of the Python SQL weak proof (#636 design A, round 2 owner folds).
//!
//! A local container (a display, a comprehension or a call result) is data only when every
//! occurrence of its name in the scope that binds it is on a short allow-list; any other
//! occurrence (an alias, another call, another method, a store, `del`, a nested scope) refuses.
//! Chain roots that are same-file helpers prove only through their returns (depth 2).

use super::callee::args;
use super::py_scope::{Bound, Local};
use super::sqla::{peel, Proof};
use tree_sitter::Node;

/// Methods that add to a container; every argument must be a value.
const ADDERS: &[&str] = &[
    "append",
    "extend",
    "add",
    "insert",
    "update",
    "setdefault",
    "appendleft",
    "extendleft",
];
/// Builtins that only read their argument.
const READERS: &[&str] = &["len", "list", "tuple", "set", "sorted", "str"];
/// Helper roots resolve through at most this many same-file functions.
const MAX_HELPERS: usize = 2;

/// Callees that build a mutable container.
const CONSTRUCTORS: &[&str] = &[
    "list",
    "dict",
    "set",
    "tuple",
    "frozenset",
    "deque",
    "defaultdict",
    "OrderedDict",
    "Counter",
    "ChainMap",
    "bytearray",
    "array",
    "copy",
    "deepcopy",
    "sorted",
    "reversed",
];

fn display_init(init: Node<'_>) -> bool {
    matches!(
        peel(init).kind(),
        "list"
            | "tuple"
            | "set"
            | "dictionary"
            | "list_comprehension"
            | "set_comprehension"
            | "dictionary_comprehension"
            | "generator_expression"
    )
}

impl<'a> Proof<'a> {
    /// A Python container the allow-list governs: a display, a comprehension, or a call that
    /// builds one (a container constructor by name, or a same-file function). A call into
    /// another module or a method of unknown type (`req.args.get('x')`, `select(..).where(..)`)
    /// is data from outside (D1) and stays on the mutation deny-list scan, so such a value may be
    /// read many times.
    fn container_init(&self, init: Node<'_>) -> bool {
        let init = peel(init);
        if init.kind() != "call" {
            return display_init(init);
        }
        let Some(f) = init.child_by_field_name("function") else {
            return false;
        };
        let last = match f.kind() {
            "identifier" => Some(f),
            "attribute" => f.child_by_field_name("attribute"),
            _ => None,
        };
        last.is_some_and(|l| CONSTRUCTORS.contains(&self.text(l)))
            || (f.kind() == "identifier"
                && matches!(self.local(f, self.text(f)), Local::One(Bound::Def(_))))
    }

    /// The object `ident` names, bound by `name = init`, may have been stored to: the deny-list
    /// scan for other initializers, the occurrence allow-list for a container.
    pub(super) fn stored(&self, ident: Node<'_>, init: Node<'_>) -> bool {
        if !self.container_init(init) {
            return self.stores_to(ident);
        }
        // A mutator's argument may name the container again: the depth cap answers "stored".
        let depth = self.storing.get();
        if depth >= 3 {
            return true;
        }
        self.storing.set(depth + 1);
        let refused = self.occurrences_refuse(ident, init);
        self.storing.set(depth);
        refused
    }

    fn occurrences_refuse(&self, ident: Node<'_>, init: Node<'_>) -> bool {
        let name = self.text(ident);
        // The scope that binds the name: the function around its `name = init`, else the file.
        let mut scope = init;
        let mut cur = init.parent();
        while let Some(n) = cur {
            scope = n;
            if n.kind() == "function_definition" {
                break;
            }
            cur = n.parent();
        }
        let mut stack = vec![(scope, false)];
        while let Some((n, nested)) = stack.pop() {
            if !self.tick() {
                return true;
            }
            if n.kind() == "identifier"
                && self.text(n) == name
                && n.id() != ident.id()
                && self.is_reference(n)
                && !self.own_binding(n, init)
                && (nested || !self.allowed_use(n))
            {
                return true;
            }
            let inner = nested
                || (n.id() != scope.id()
                    && matches!(
                        n.kind(),
                        "function_definition" | "lambda" | "class_definition"
                    ));
            let mut c = n.walk();
            stack.extend(n.children(&mut c).map(|k| (k, inner)));
        }
        false
    }

    /// An identifier that is not a reference: `o.name`, `f(name=..)`.
    fn is_reference(&self, n: Node<'_>) -> bool {
        let Some(p) = n.parent() else {
            return true;
        };
        let field = |f: &str| p.child_by_field_name(f) == Some(n);
        !((p.kind() == "attribute" && field("attribute"))
            || (p.kind() == "keyword_argument" && field("name")))
    }

    fn own_binding(&self, n: Node<'_>, init: Node<'_>) -> bool {
        n.parent().is_some_and(|p| {
            p.kind() == "assignment"
                && p.child_by_field_name("left") == Some(n)
                && p.child_by_field_name("right")
                    .is_some_and(|r| r.id() == init.id())
        })
    }

    fn allowed_use(&self, n: Node<'_>) -> bool {
        // `name.adder(<values>)`
        if let Some(attr) = n
            .parent()
            .filter(|p| p.kind() == "attribute" && p.child_by_field_name("object") == Some(n))
        {
            let call = attr
                .parent()
                .filter(|c| c.kind() == "call" && c.child_by_field_name("function") == Some(attr));
            let adder = attr
                .child_by_field_name("attribute")
                .is_some_and(|m| ADDERS.contains(&self.text(m)));
            return adder && call.is_some_and(|c| args(c).into_iter().all(|a| self.value_arg(a)));
        }
        // The iterable of a `for` statement or comprehension, or a reader builtin's argument.
        let mut top = n;
        while let Some(p) = top
            .parent()
            .filter(|p| p.kind() == "parenthesized_expression")
        {
            top = p;
        }
        let Some(p) = top.parent() else {
            return false;
        };
        match p.kind() {
            "for_statement" | "for_in_clause" => p.child_by_field_name("right") == Some(top),
            "argument_list" => p
                .parent()
                .filter(|c| c.kind() == "call")
                .and_then(|c| c.child_by_field_name("function"))
                .is_some_and(|f| {
                    f.kind() == "identifier"
                        && READERS.contains(&self.text(f))
                        && matches!(self.local(f, self.text(f)), Local::Unbound)
                }),
            _ => false,
        }
    }

    // ---- same-file helpers as chain roots ---------------------------------------------------

    /// `Some(proof)` when the call's callee is a same-file helper (a bare name of one `def`, or
    /// `self.m`/`cls.m` of a method defined once in the class): it proves only through its
    /// returns. `None` for any other callee, which stays an opaque root.
    pub(super) fn helper_root(&self, f: Node<'_>) -> Option<bool> {
        let def = match f.kind() {
            "identifier" => {
                let name = self.text(f);
                if self.f.sites.funcs(name) == 0 {
                    return None;
                }
                match self.local(f, name) {
                    Local::One(Bound::Def(d))
                        if d.kind() == "function_definition" && self.f.sites.total(name) == 1 =>
                    {
                        d
                    }
                    // A function name bound again, or ambiguous: unproven.
                    _ => return Some(false),
                }
            }
            "attribute" => {
                let obj = f.child_by_field_name("object")?;
                let m = f.child_by_field_name("attribute")?;
                if obj.kind() != "identifier" || !matches!(self.text(obj), "self" | "cls") {
                    return None;
                }
                self.enclosing_method(f, self.text(m))?
            }
            _ => return None,
        };
        Some(self.helper_returns(def))
    }

    fn helper_returns(&self, def: Node<'_>) -> bool {
        let mut stack = self.helpers.borrow_mut();
        if def
            .parent()
            .is_some_and(|p| p.kind() == "decorated_definition")
            || stack.len() >= MAX_HELPERS
            || stack.contains(&def.start_byte())
        {
            return false;
        }
        stack.push(def.start_byte());
        drop(stack);
        let ok = self.returns_chain(def);
        self.helpers.borrow_mut().pop();
        ok
    }

    fn returns_chain(&self, def: Node<'_>) -> bool {
        let Some(body) = def.child_by_field_name("body") else {
            return false;
        };
        let mut rets = Vec::new();
        let mut stack = vec![body];
        while let Some(n) = stack.pop() {
            if !self.tick() {
                return false;
            }
            if n.kind() == "return_statement" {
                rets.push(n);
            }
            if matches!(
                n.kind(),
                "function_definition" | "lambda" | "class_definition"
            ) {
                continue;
            }
            let mut c = n.walk();
            stack.extend(n.named_children(&mut c));
        }
        !rets.is_empty()
            && rets.iter().all(|r| {
                r.named_child(0)
                    .is_some_and(|v| self.hop(|| self.inner_chain(v)))
            })
    }
}
