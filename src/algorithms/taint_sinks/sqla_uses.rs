//! Occurrence rules of the Python SQL weak proof (#636 design A, round 2 owner folds).
//!
//! A local container (a display, a comprehension or a call result) is data only when every
//! occurrence of its name in the scope that binds it is on a short allow-list; any other
//! occurrence (an alias, another call, another method, a store, `del`, a nested scope) refuses.
//! A container that holds a mutable element (a nested display or comprehension) refuses, and an
//! adder counts only as a whole statement, so no element can be reached and mutated.

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

/// Callees that build a mutable container.
const MUTABLE: &[&str] = &[
    "list",
    "dictionary",
    "set",
    "list_comprehension",
    "set_comprehension",
    "dictionary_comprehension",
];

/// Any strict descendant of `n` is a mutable display or comprehension.
fn descendant_mutable(n: Node<'_>) -> bool {
    let mut stack = vec![n];
    while let Some(x) = stack.pop() {
        let mut c = x.walk();
        for k in x.children(&mut c) {
            if MUTABLE.contains(&k.kind()) {
                return true;
            }
            stack.push(k);
        }
    }
    false
}

/// The elements of the container `n` builds hold a mutable object: a nested display or
/// comprehension (operands of `+`, branches of a conditional and a sliced value are the
/// container itself, not its elements).
fn elements_mutable(n: Node<'_>) -> bool {
    let n = peel(n);
    match n.kind() {
        "binary_operator" => ["left", "right"]
            .iter()
            .filter_map(|f| n.child_by_field_name(f))
            .any(elements_mutable),
        "conditional_expression" => [0, 2]
            .iter()
            .filter_map(|i| n.named_child(*i))
            .any(elements_mutable),
        "subscript" => n.child_by_field_name("value").is_some_and(elements_mutable),
        _ => descendant_mutable(n),
    }
}

/// An adder argument that would store a mutable object in the container (for `extend`-like
/// adders the argument's own elements are stored, not the argument).
fn arg_mutable(a: Node<'_>, spreads: bool) -> bool {
    let a = if a.kind() == "keyword_argument" {
        a.child_by_field_name("value").unwrap_or(a)
    } else {
        a
    };
    if spreads {
        elements_mutable(a)
    } else {
        MUTABLE.contains(&peel(a).kind()) || descendant_mutable(a)
    }
}

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
        match init.kind() {
            // `a + [..]`: a list built from a display operand.
            "binary_operator" => {
                init.child_by_field_name("operator")
                    .is_some_and(|o| self.text(o) == "+")
                    && ["left", "right"]
                        .iter()
                        .filter_map(|f| init.child_by_field_name(f))
                        .any(|o| self.container_init(o))
            }
            "conditional_expression" => [0, 2]
                .iter()
                .all(|i| init.named_child(*i).is_some_and(|b| self.container_init(b))),
            // `[..][:]`: a slice of a display.
            "subscript" => {
                init.child_by_field_name("subscript")
                    .is_some_and(|s| s.kind() == "slice")
                    && init
                        .child_by_field_name("value")
                        .is_some_and(|v| self.container_init(v))
            }
            "call" => self.container_call(init),
            _ => display_init(init),
        }
    }

    fn container_call(&self, init: Node<'_>) -> bool {
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
        // A container of mutable elements can be mutated through an element.
        if elements_mutable(init) {
            return true;
        }
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
            let name = attr.child_by_field_name("attribute").map(|m| self.text(m));
            let spreads = matches!(name, Some("extend" | "extendleft" | "update"));
            // A whole expression statement: its result (`setdefault` returns an element) is
            // discarded.
            return adder
                && call.is_some_and(|c| {
                    c.parent()
                        .is_some_and(|p| p.kind() == "expression_statement")
                        && args(c)
                            .into_iter()
                            .all(|a| self.value_arg(a) && !arg_mutable(a, spreads))
                });
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
        if self.truth_test(top) {
            return true;
        }
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

    /// A read-only truth test: the name (through `not`, `and`, `or` and parentheses) as the
    /// condition of `if`/`elif`/`while`/`assert` or the test of a conditional expression, or the
    /// right operand of `in`/`not in`.
    fn truth_test(&self, start: Node<'_>) -> bool {
        let mut cur = start;
        while let Some(p) = cur.parent() {
            match p.kind() {
                "parenthesized_expression" | "not_operator" | "boolean_operator" => cur = p,
                "if_statement" | "elif_clause" | "while_statement" => {
                    return p.child_by_field_name("condition") == Some(cur)
                }
                "assert_statement" => return p.named_child(0) == Some(cur),
                "conditional_expression" => return p.named_child(1) == Some(cur),
                "comparison_operator" => {
                    let mut c = p.walk();
                    let ops_in = p
                        .children(&mut c)
                        .filter(|k| !k.is_named())
                        .all(|k| matches!(self.text(k), "in" | "not in"));
                    return ops_in && p.named_child(0) != Some(cur);
                }
                _ => return false,
            }
        }
        false
    }
}
