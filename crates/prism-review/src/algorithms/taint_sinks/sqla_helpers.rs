//! Same-file helpers as chain roots of the Python SQL weak proof (#636 design A, A21 and the
//! round-3 folds): a bare name of one `def` (or a lambda), or `self.m`/`cls.m` resolved through
//! the enclosing class and its same-file bases, proves only through its returns (depth 2) and
//! only when every call argument is itself proven. Unresolved, imported or other-receiver
//! callees stay opaque roots (D1).

use super::callee::args;
use super::py_scope::{Bound, Local};
use super::sqla::{peel, Proof};
use tree_sitter::Node;

/// Helper roots resolve through at most this many same-file functions.
const MAX_HELPERS: usize = 2;
/// Same-file base or subclass chains followed.
const MAX_CLASS_DEPTH: usize = 4;

/// What `self.m` names.
enum Method<'t> {
    Def(Node<'t>),
    /// Defined more than once, conditionally, ambiguously or overridden: unproven.
    Refuse,
    /// Not defined in any same-file class: another module's method (D1).
    Opaque,
}

impl<'a> Proof<'a> {
    /// `Some(proof)` when the call's callee is a same-file helper; `None` for any other callee.
    pub(super) fn helper_root(&self, call: Node<'_>, f: Node<'_>) -> Option<bool> {
        let proof = match f.kind() {
            "identifier" => self.bare_helper(f)?,
            "attribute" => {
                let obj = f.child_by_field_name("object")?;
                let m = f.child_by_field_name("attribute")?;
                if obj.kind() != "identifier" || !matches!(self.text(obj), "self" | "cls") {
                    return None;
                }
                let class = self.enclosing_class(f)?;
                let name = self.text(m);
                match self.resolve_method(class, name, 0) {
                    Method::Opaque => return None,
                    Method::Refuse => return Some(false),
                    Method::Def(_) if self.subclass_defines(class, name, 0) => return Some(false),
                    Method::Def(d) => self.helper_returns(d),
                }
            }
            _ => return None,
        };
        Some(proof && self.helper_args(call))
    }

    fn bare_helper(&self, f: Node<'_>) -> Option<bool> {
        let name = self.text(f);
        match self.local(f, name) {
            // A name bound by assignment: a lambda proves through its body, anything else
            // (`partial(..)`, an alias) is unproven.
            Local::One(Bound::Assign(v)) => {
                let v = peel(v);
                Some(v.kind() == "lambda" && self.lambda_returns(v))
            }
            _ if self.f.sites.funcs(name) == 0 => None,
            Local::One(Bound::Def(d))
                if d.kind() == "function_definition" && self.f.sites.total(name) == 1 =>
            {
                Some(self.helper_returns(d))
            }
            // A function name bound again, or ambiguous: unproven.
            _ => Some(false),
        }
    }

    /// Every argument of the helper call is a value or a proven chain: its parameters are
    /// out of model, but what the caller builds and passes in is not.
    fn helper_args(&self, call: Node<'_>) -> bool {
        args(call).into_iter().all(|a| {
            let v = if a.kind() == "keyword_argument" {
                a.child_by_field_name("value").unwrap_or(a)
            } else {
                a
            };
            self.value_arg(a) || self.hop(|| self.inner_chain(v))
        })
    }

    fn enter(&self, at: Node<'_>) -> bool {
        let mut stack = self.helpers.borrow_mut();
        if stack.len() >= MAX_HELPERS || stack.contains(&at.start_byte()) {
            return false;
        }
        stack.push(at.start_byte());
        true
    }

    fn helper_returns(&self, def: Node<'_>) -> bool {
        if def
            .parent()
            .is_some_and(|p| p.kind() == "decorated_definition")
            || !self.enter(def)
        {
            return false;
        }
        let ok = self.returns_chain(def);
        self.helpers.borrow_mut().pop();
        ok
    }

    fn lambda_returns(&self, lambda: Node<'_>) -> bool {
        if !self.enter(lambda) {
            return false;
        }
        let ok = lambda
            .child_by_field_name("body")
            .is_some_and(|b| self.hop(|| self.inner_chain(b)));
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

    // ---- classes ---------------------------------------------------------------------------

    fn enclosing_class<'t>(&self, at: Node<'t>) -> Option<Node<'t>> {
        let mut cur = at.parent();
        while let Some(n) = cur {
            if n.kind() == "class_definition" {
                return Some(n);
            }
            cur = n.parent();
        }
        None
    }

    fn all_classes<'t>(&self, at: Node<'t>) -> Vec<Node<'t>> {
        let mut root = at;
        while let Some(p) = root.parent() {
            root = p;
        }
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            if n.kind() == "class_definition" {
                out.push(n);
            }
            let mut c = n.walk();
            stack.extend(n.named_children(&mut c));
        }
        out
    }

    fn class_name(&self, class: Node<'_>) -> &'a str {
        class
            .child_by_field_name("name")
            .map_or("", |n| self.text(n))
    }

    /// Identifier bases of a class (`class R(B, Mixin)`); dotted or called bases are other
    /// modules' and not listed.
    fn base_names(&self, class: Node<'_>) -> Vec<&'a str> {
        let Some(list) = class.child_by_field_name("superclasses") else {
            return Vec::new();
        };
        let mut c = list.walk();
        let names = list
            .named_children(&mut c)
            .filter(|b| b.kind() == "identifier")
            .map(|b| self.text(b))
            .collect();
        names
    }

    /// `m` defined in the class body, at any depth outside nested function and class bodies:
    /// the direct definitions (decorated or not) and the count of all of them.
    fn method_defs<'t>(&self, class: Node<'t>, m: &str) -> (Vec<Node<'t>>, usize) {
        let Some(body) = class.child_by_field_name("body") else {
            return (Vec::new(), 0);
        };
        let (mut direct, mut total) = (Vec::new(), 0);
        let mut stack = vec![body];
        while let Some(n) = stack.pop() {
            if n.kind() == "function_definition" {
                if n.child_by_field_name("name")
                    .is_some_and(|x| self.text(x) == m)
                {
                    total += 1;
                    let top = n
                        .parent()
                        .filter(|p| p.kind() == "decorated_definition")
                        .unwrap_or(n);
                    if top.parent() == Some(body) {
                        direct.push(n);
                    }
                }
                continue;
            }
            if n.kind() == "class_definition" {
                continue;
            }
            let mut c = n.walk();
            stack.extend(n.named_children(&mut c));
        }
        (direct, total)
    }

    fn resolve_method<'t>(&self, class: Node<'t>, m: &str, depth: usize) -> Method<'t> {
        let (direct, total) = self.method_defs(class, m);
        if total > 0 {
            return if total == 1 && direct.len() == 1 {
                Method::Def(direct[0])
            } else {
                Method::Refuse
            };
        }
        if depth >= MAX_CLASS_DEPTH {
            return Method::Refuse;
        }
        let classes = self.all_classes(class);
        let mut found: Vec<Node<'t>> = Vec::new();
        for base in self.base_names(class) {
            let same: Vec<Node<'t>> = classes
                .iter()
                .copied()
                .filter(|c| self.class_name(*c) == base)
                .collect();
            match same.as_slice() {
                [] => {}
                [b] => match self.resolve_method(*b, m, depth + 1) {
                    Method::Def(d) => found.push(d),
                    Method::Refuse => return Method::Refuse,
                    Method::Opaque => {}
                },
                _ => return Method::Refuse,
            }
        }
        match found.as_slice() {
            [] => Method::Opaque,
            [d] => Method::Def(*d),
            [d, rest @ ..] if rest.iter().all(|x| x.id() == d.id()) => Method::Def(*d),
            _ => Method::Refuse,
        }
    }

    /// A same-file class derived (by identifier base, transitively) from `class` defines `m`:
    /// `self.m` may run the override.
    fn subclass_defines(&self, class: Node<'_>, m: &str, depth: usize) -> bool {
        if depth >= MAX_CLASS_DEPTH {
            return true;
        }
        let name = self.class_name(class);
        self.all_classes(class).into_iter().any(|c| {
            c.id() != class.id()
                && self.base_names(c).contains(&name)
                && (self.method_defs(c, m).1 > 0 || self.subclass_defines(c, m, depth + 1))
        })
    }
}
