//! Value positions of the Python SQL weak proof (#636 design A, D1): what may reach a bound
//! parameter. A literal, a Python data builtin or string method, a name that resolves to one
//! binding whose initializer is a value, a value from outside the function (a parameter, a loop
//! target, an import, an attribute chain rooted at one), an allowed construct, or a call whose
//! callee and arguments are values. A raw-SQL constructor is never a value, whatever its
//! receiver; a same-file function proves by its returns.

use super::callee::{args, walk};
use super::py_scope::{Bound, Local};
use super::sqla::{peel, plain_literal, Proof};
use super::sqla_tables as t;
use tree_sitter::Node;

const MAX_DEFS: u8 = 2;

impl<'a> Proof<'a> {
    // ---- values ----------------------------------------------------------------------------

    pub(super) fn value(&self, n: Node<'_>) -> bool {
        if !self.tick() {
            return false;
        }
        let n = peel(n);
        let all = |n: Node<'_>| {
            let mut c = n.walk();
            let ok = n.named_children(&mut c).all(|k| self.value(k));
            ok
        };
        match n.kind() {
            "string" => {
                let mut c = n.walk();
                let ok = n
                    .named_children(&mut c)
                    .filter(|k| k.kind() == "interpolation")
                    .all(|k| {
                        k.child_by_field_name("expression")
                            .is_none_or(|e| self.value(e))
                    });
                ok
            }
            "integer" | "float" | "true" | "false" | "none" | "ellipsis" => true,
            "concatenated_string"
            | "binary_operator"
            | "boolean_operator"
            | "comparison_operator"
            | "list"
            | "tuple"
            | "set"
            | "expression_list"
            | "dictionary"
            | "pair"
            | "slice"
            | "not_operator"
            | "unary_operator"
            | "list_splat"
            | "dictionary_splat" => all(n),
            "conditional_expression" => {
                n.named_child(0).is_some_and(|b| self.value(b))
                    && n.named_child(2).is_some_and(|e| self.value(e))
            }
            "list_comprehension"
            | "set_comprehension"
            | "generator_expression"
            | "dictionary_comprehension" => {
                n.child_by_field_name("body").is_some_and(|b| self.value(b))
            }
            "identifier" => self.name_value(n),
            "attribute" | "subscript" => self.chain_value(n),
            "call" => self.call_value(n),
            _ => false,
        }
    }

    pub(super) fn name_value(&self, n: Node<'_>) -> bool {
        match self.local(n, self.text(n)) {
            Local::Unbound => true,
            Local::Multi => false,
            Local::One(b) => match b {
                Bound::Assign(v) => !self.stores_to(n) && self.hop_memo(b'v', v, || self.value(v)),
                Bound::Param(_) => !self.stores_to(n),
                Bound::Loop(it) => {
                    // D1: a loop target is data. An iterable built in place (a display or a
                    // comprehension) is still checked, so `for c in [literal_column(s)]` is Strong.
                    let built_here = |e: Node<'_>| {
                        matches!(
                            peel(e).kind(),
                            "list"
                                | "tuple"
                                | "set"
                                | "dictionary"
                                | "list_comprehension"
                                | "set_comprehension"
                                | "dictionary_comprehension"
                                | "generator_expression"
                        )
                    };
                    !self.stores_to(n) && it.is_none_or(|e| !built_here(e) || self.value(e))
                }
                Bound::Import(..) | Bound::Def(_) => true,
                Bound::Opaque | Bound::Declared => false,
            },
        }
    }

    /// `req.args["q"]`, `row.id`, `foo().bar`: data rooted at a name or a call.
    pub(super) fn chain_value(&self, n: Node<'_>) -> bool {
        let mut cur = n;
        while matches!(cur.kind(), "attribute" | "subscript") {
            if cur.kind() == "subscript" {
                let mut c = cur.walk();
                let ok = cur
                    .children_by_field_name("subscript", &mut c)
                    .all(|s| self.value(s));
                if !ok {
                    return false;
                }
            }
            let next = if cur.kind() == "attribute" {
                "object"
            } else {
                "value"
            };
            match cur.child_by_field_name(next) {
                Some(x) => cur = peel(x),
                None => return false,
            }
        }
        self.value(cur)
    }

    pub(super) fn args_value(&self, call: Node<'_>) -> bool {
        args(call).into_iter().all(|a| match a.kind() {
            "keyword_argument" => a
                .child_by_field_name("value")
                .is_some_and(|v| self.value(v)),
            _ => self.value(a),
        })
    }

    pub(super) fn call_value(&self, call: Node<'_>) -> bool {
        let Some(f) = call.child_by_field_name("function") else {
            return false;
        };
        if self.lib_of(f).is_some() {
            return self.expr_call(call);
        }
        match f.kind() {
            "identifier" => self.bare_value(call, f),
            "attribute" => self.method_value(call, f),
            _ => false,
        }
    }

    pub(super) fn typing_cast(&self, call: Node<'_>) -> bool {
        let a = args(call);
        a.len() == 2 && self.value(a[1])
    }

    pub(super) fn bare_value(&self, call: Node<'_>, f: Node<'_>) -> bool {
        let name = self.text(f);
        if self
            .f
            .sites
            .trusted_import(name)
            .is_some_and(|(m, x)| m == "typing" && x == "cast")
        {
            return self.typing_cast(call);
        }
        match self.local(f, name) {
            Local::Unbound => {
                // A star import or an undefined name: raw constructors by name are refused.
                !t::BELT.contains(&name) && self.args_value(call)
            }
            // Another module's function: its return is out of model (D1).
            Local::One(Bound::Import(..)) => self.args_value(call),
            Local::One(Bound::Def(d)) => {
                (d.kind() == "class_definition" || self.def_returns(d)) && self.args_value(call)
            }
            _ => false,
        }
    }

    pub(super) fn method_value(&self, call: Node<'_>, f: Node<'_>) -> bool {
        let (Some(obj), Some(m)) = (
            f.child_by_field_name("object"),
            f.child_by_field_name("attribute"),
        ) else {
            return false;
        };
        let m = self.text(m);
        if t::BELT.contains(&m) {
            return false;
        }
        if m == "cast"
            && obj.kind() == "identifier"
            && self
                .f
                .sites
                .trusted_import(self.text(obj))
                .is_some_and(|(mo, x)| mo == "typing" && x.is_empty())
        {
            return self.typing_cast(call);
        }
        if m == "join" && plain_literal(self.p, obj) {
            return self.args_value(call);
        }
        let builder = t::find(t::CHAIN, m).is_some() && !t::TERMINALS.contains(&m);
        if builder {
            return self.expr_call(call);
        }
        if t::find(t::COLUMN_METHODS, m).is_some() && self.expr_call(call) {
            return true;
        }
        if obj.kind() == "identifier" && matches!(self.text(obj), "self" | "cls") {
            if let Some(d) = self.enclosing_method(call, m) {
                return self.def_returns(d) && self.args_value(call);
            }
        }
        self.value(obj) && self.args_value(call)
    }

    /// The method `m` defined directly in the class around `at`, when there is exactly one.
    pub(super) fn enclosing_method<'t>(&self, at: Node<'t>, m: &str) -> Option<Node<'t>> {
        let mut cur = at.parent();
        while let Some(n) = cur {
            if n.kind() == "class_definition" {
                let body = n.child_by_field_name("body")?;
                let mut c = body.walk();
                let defs: Vec<Node<'t>> = body
                    .named_children(&mut c)
                    .map(|k| {
                        if k.kind() == "decorated_definition" {
                            k.child_by_field_name("definition").unwrap_or(k)
                        } else {
                            k
                        }
                    })
                    .filter(|k| {
                        k.kind() == "function_definition"
                            && k.child_by_field_name("name")
                                .is_some_and(|x| self.text(x) == m)
                    })
                    .collect();
                return (defs.len() == 1).then(|| defs[0]);
            }
            cur = n.parent();
        }
        None
    }

    /// A same-file function whose every `return` is a value (nested calls one level deep).
    pub(super) fn def_returns(&self, def: Node<'_>) -> bool {
        if def.kind() != "function_definition" || self.defs.get() >= MAX_DEFS {
            return false;
        }
        let Some(body) = def.child_by_field_name("body") else {
            return false;
        };
        self.defs.set(self.defs.get() + 1);
        let mut rets = Vec::new();
        let mut stack = vec![body];
        while let Some(n) = stack.pop() {
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
        let ok = !rets.is_empty()
            && rets
                .iter()
                .all(|r| r.named_child(0).is_none_or(|v| self.value(v)));
        self.defs.set(self.defs.get() - 1);
        ok
    }

    /// The function around `ident` (or the file) stores to the object `ident` names:
    /// `o.x = ..`, `o[k] = ..`, `setattr(o, ..)`.
    pub(super) fn stores_to(&self, ident: Node<'_>) -> bool {
        let name = self.text(ident);
        let (mut func, mut root, mut cur) = (None, ident, ident.parent());
        while let Some(n) = cur {
            if func.is_none() && n.kind() == "function_definition" {
                func = Some(n);
            }
            root = n;
            cur = n.parent();
        }
        let rooted = |mut x: Node<'_>| {
            while matches!(x.kind(), "attribute" | "subscript") {
                match x.child_by_field_name(if x.kind() == "attribute" {
                    "object"
                } else {
                    "value"
                }) {
                    Some(o) => x = o,
                    None => return false,
                }
            }
            x.kind() == "identifier" && self.text(x) == name
        };
        let mut found = false;
        walk(func.unwrap_or(root), &mut |n| match n.kind() {
            "assignment" | "augmented_assignment" => {
                if let Some(l) = n.child_by_field_name("left") {
                    walk(l, &mut |x| {
                        if matches!(x.kind(), "attribute" | "subscript") && rooted(x) {
                            found = true;
                        }
                    });
                }
            }
            "call" => {
                let is_setattr = n
                    .child_by_field_name("function")
                    .is_some_and(|f| self.text(f) == "setattr");
                if is_setattr
                    && args(n)
                        .first()
                        .is_some_and(|a| a.kind() == "identifier" && self.text(*a) == name)
                {
                    found = true;
                }
            }
            _ => {}
        });
        found
    }
}
