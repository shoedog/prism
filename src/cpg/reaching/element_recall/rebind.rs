//! Events that end a route (lane 10, #847): replacement of W's root or of a static selector
//! prefix of its storage, and key-blind deletion or clear of the affected subtree.
//!
//! One evaluation-ordered walk of an atom (a simple statement or a header expression) feeds
//! every question: does the atom kill W, and is an expression-level kill evaluated before a
//! given operand. Anything under a deferred arm or a nested scope never counts as definite.
use super::python::{kids, leaves, peel_down, static_path, text};
use super::transfer::Transfer;
use super::{Span, StaticPath};
use tree_sitter::Node;

/// Methods that drop or replace entries without naming a key. Only `clear` cuts a route
/// (DESIGN section 4); the whole set detaches a child that a store already captured.
pub(super) const WINDOW_MUTATORS: [&str; 8] = [
    "clear",
    "pop",
    "popitem",
    "remove",
    "discard",
    "update",
    "__setitem__",
    "__delitem__",
];

/// The truth value of a literal constant (`True`, `1`, `'x'`, `None`, `not 0`, in parentheses),
/// or `None` for anything evaluated at run time.
fn literal_truth(n: Node, src: &str) -> Option<bool> {
    let n = peel_down(n);
    match n.kind() {
        "true" => Some(true),
        "false" | "none" => Some(false),
        "integer" | "float" => number_truth(text(n, src)),
        "string" => {
            let parts = kids(n);
            if parts.iter().any(|c| c.kind() == "interpolation") {
                return None;
            }
            let body = |c: &Node| matches!(c.kind(), "string_content" | "escape_sequence");
            Some(parts.iter().any(body))
        }
        "not_operator" => literal_truth(n.child_by_field_name("argument")?, src).map(|b| !b),
        _ => None,
    }
}

/// Nonzero-ness of an integer, float or imaginary literal, read as CPython reads it (IEEE
/// round-to-nearest, so `1e-999` is `0.0` and `1e999` is `inf`). Any spelling that is not a
/// well-formed Python 3 literal (Python 2 `0777`/`10L`, misplaced underscores, `0x1j`) is `None`.
pub(super) fn number_truth(raw: &str) -> Option<bool> {
    let low = raw.to_ascii_lowercase();
    let radix = [("0x", 16), ("0o", 8), ("0b", 2)]
        .into_iter()
        .find(|(p, _)| low.starts_with(p));
    if let Some((prefix, base)) = radix {
        let body = &low[prefix.len()..];
        let ok = |c: char| c.is_digit(base);
        let body = body.strip_prefix('_').unwrap_or(body);
        return (underscores_ok(body, ok)
            && !body.is_empty()
            && body.chars().all(|c| ok(c) || c == '_'))
        .then(|| body.chars().any(|c| c != '_' && c != '0'));
    }
    let (body, imaginary) = match low.strip_suffix('j') {
        Some(b) => (b, true),
        None => (low.as_str(), false),
    };
    let digit = |c: char| c.is_ascii_digit();
    if !underscores_ok(body, digit) || body.starts_with('_') {
        return None;
    }
    let clean: String = body.chars().filter(|c| *c != '_').collect();
    if clean.chars().all(digit) && !clean.is_empty() {
        // `0`+ or a nonzero first digit; a leading zero before a nonzero digit is Python 2
        let nonzero = clean.chars().any(|c| c != '0');
        return (imaginary || !nonzero || !clean.starts_with('0')).then_some(nonzero);
    }
    float_nonzero(&clean)
}

/// Every underscore sits between two characters accepted by `digit`.
fn underscores_ok(s: &str, digit: impl Fn(char) -> bool) -> bool {
    let cs: Vec<char> = s.chars().collect();
    cs.iter().enumerate().all(|(i, c)| {
        *c != '_' || (i > 0 && i + 1 < cs.len() && digit(cs[i - 1]) && digit(cs[i + 1]))
    })
}

/// `digits [. digits] [e [+-] digits]` (at least one mantissa digit), parsed by Rust's correctly
/// rounded `f64` reader; the text is validated first so `inf` and `nan` never reach it.
fn float_nonzero(s: &str) -> Option<bool> {
    let (mant, exp) = match s.split_once('e') {
        Some((m, e)) => (m, Some(e)),
        None => (s, None),
    };
    let dots = mant.matches('.').count();
    let mant_ok = dots <= 1
        && mant.chars().any(|c| c.is_ascii_digit())
        && mant.chars().all(|c| c.is_ascii_digit() || c == '.');
    let exp_ok = exp.is_none_or(|e| {
        let d = e.strip_prefix(['+', '-']).unwrap_or(e);
        !d.is_empty() && d.chars().all(|c| c.is_ascii_digit())
    });
    (mant_ok && exp_ok)
        .then(|| s.parse::<f64>().ok().map(|v| v != 0.0))
        .flatten()
}

/// True for the named child that may not execute: conditional-expression arms, the right
/// operand of `and`/`or`, later operands of a comparison chain and comprehension parts.
/// A literal constant decides the first two: `True and X`, `False or X` and the arm a literal
/// test selects always run, so they are not deferred; `False and X`, `True or X` and the other
/// arm never run and stay deferred (they cannot kill anything).
fn deferred(parent: Node, index: usize, src: &str) -> bool {
    match parent.kind() {
        "conditional_expression" => {
            let named: Vec<_> = kids(parent).into_iter().filter(|c| c.is_named()).collect();
            let test = named.get(1).and_then(|t| literal_truth(*t, src));
            match test {
                Some(taken) if index != 1 => index != if taken { 0 } else { 2 },
                _ => index != 1,
            }
        }
        "boolean_operator" => {
            let left = parent.child_by_field_name("left");
            let op = parent.child_by_field_name("operator").map(|o| text(o, src));
            let runs = match (op, left.and_then(|l| literal_truth(l, src))) {
                (Some("and"), Some(true)) | (Some("or"), Some(false)) => true,
                _ => false,
            };
            index == 1 && !runs
        }
        "comparison_operator" => index >= 2,
        "list_comprehension"
        | "set_comprehension"
        | "dictionary_comprehension"
        | "generator_expression" => true,
        _ => false,
    }
}

/// Named children in evaluation order, each with its deferral flag. An assignment evaluates
/// its right side before its targets, a conditional expression its test before either arm.
fn ordered<'a>(n: Node<'a>, src: &str) -> Vec<(Node<'a>, bool)> {
    let named = kids(n).into_iter().filter(|c| c.is_named());
    let mut out: Vec<_> = named
        .enumerate()
        .map(|(i, c)| (c, deferred(n, i, src)))
        .collect();
    match n.kind() {
        "assignment" => out.sort_by_key(|(c, _)| n.child_by_field_name("right") != Some(*c)),
        "conditional_expression" if out.len() == 3 => {
            let test = out.remove(1);
            out.insert(0, test);
        }
        _ => {}
    }
    out
}

/// Depth-first walk of one atom in evaluation order. `f(node, post, cond)` runs when a node is
/// entered (`post` false) and, for a node that is not under a deferred arm, again when it has
/// been evaluated. `cond` marks a node that may not execute. Returns true when `f` did.
pub(super) fn walk<'a>(
    atom: Node<'a>,
    src: &str,
    f: &mut dyn FnMut(Node<'a>, bool, bool) -> bool,
) -> bool {
    enum Fr<'a> {
        In(Node<'a>, bool),
        Out(Node<'a>),
    }
    let mut stack = vec![Fr::In(atom, false)];
    while let Some(fr) = stack.pop() {
        match fr {
            Fr::Out(n) => {
                if f(n, true, false) {
                    return true;
                }
            }
            Fr::In(n, cond) => {
                if f(n, false, cond) {
                    return true;
                }
                let scope = ["lambda", "function_definition", "class_definition"];
                if scope.contains(&n.kind()) || n.kind() == "decorated_definition" {
                    continue;
                }
                if !cond {
                    stack.push(Fr::Out(n));
                }
                for (c, d) in ordered(n, src).into_iter().rev() {
                    stack.push(Fr::In(c, cond || d));
                }
            }
        }
    }
    false
}

/// `x.<name>(..)` with `name` in `names`: the node `x`.
pub(super) fn method_object<'a>(call: Node<'a>, names: &[&str], src: &str) -> Option<Node<'a>> {
    let f = peel_down(call.child_by_field_name("function")?);
    let a = f.child_by_field_name("attribute")?;
    let hit = f.kind() == "attribute" && names.contains(&text(a, src));
    hit.then(|| f.child_by_field_name("object")).flatten()
}

/// Names bound by an import statement.
pub(super) fn import_binds(n: Node, name: &str, src: &str) -> bool {
    n.children_by_field_name("name", &mut n.walk()).any(|c| {
        let bound = match c.kind() {
            "aliased_import" => c.child_by_field_name("alias"),
            _ => kids(c).into_iter().find(|k| k.kind() == "identifier"),
        };
        bound.is_some_and(|b| text(b, src) == name)
    })
}

/// Replacement of W's root or of a static selector prefix of its storage.
pub(crate) struct Rebind {
    pub(super) root: String,
    pub(super) fields: Vec<String>,
}

impl Rebind {
    pub(crate) fn of(path: &StaticPath) -> Option<Rebind> {
        Some(Rebind {
            root: path.root.clone()?,
            fields: path.fields.clone(),
        })
    }

    fn prefix_of_mine(&self, fields: &[String]) -> bool {
        fields.len() <= self.fields.len() && fields.iter().zip(&self.fields).all(|(a, b)| a == b)
    }

    /// A target replacing the root, or an attribute chain naming a prefix of W's selectors.
    fn hits(&self, target: Node, src: &str) -> bool {
        let mut n = peel_down(target);
        let mut fields = Vec::new();
        while n.kind() == "attribute" {
            fields.extend(n.child_by_field_name("attribute").map(|a| text(a, src)));
            let Some(o) = n.child_by_field_name("object") else {
                return false;
            };
            n = peel_down(o);
        }
        let fields: Vec<String> = fields.into_iter().rev().map(str::to_string).collect();
        n.kind() == "identifier" && text(n, src) == self.root && self.prefix_of_mine(&fields)
    }

    /// Key-blind: the path's prefix before its first bracket is a static prefix of W's storage.
    fn blind(&self, target: Node, src: &str) -> bool {
        let (p, _) = static_path(peel_down(target), src);
        p.root.as_deref() == Some(self.root.as_str()) && self.prefix_of_mine(&p.fields)
    }

    pub(super) fn any_leaf(&self, target: Node, src: &str) -> bool {
        let (mut found, mut nested) = (Vec::new(), false);
        leaves(target, &mut found, &mut nested);
        found.into_iter().any(|l| self.hits(l, src))
    }

    fn def_named(&self, def: Node, src: &str) -> bool {
        let name = def.child_by_field_name("name");
        name.is_some_and(|n| text(n, src) == self.root)
    }

    /// A definite node that replaces the root or a prefix of W's storage.
    fn replaces(&self, n: Node, src: &str) -> bool {
        let left = n.child_by_field_name("left");
        match n.kind() {
            "function_definition" | "class_definition" => self.def_named(n, src),
            "decorated_definition" => {
                let def = n.child_by_field_name("definition");
                def.is_some_and(|d| self.def_named(d, src))
            }
            "assignment" => {
                let value = n.child_by_field_name("right").is_some();
                value && left.is_some_and(|l| self.any_leaf(l, src))
            }
            "augmented_assignment" => left.is_some_and(|l| self.hits(l, src)),
            "named_expression" => self.walrus(n, src),
            "import_statement" | "import_from_statement" => import_binds(n, &self.root, src),
            _ => false,
        }
    }

    fn walrus(&self, n: Node, src: &str) -> bool {
        let name = n.child_by_field_name("name");
        name.is_some_and(|b| text(b, src) == self.root)
    }

    /// `del` of anything under W's subtree, key-blind, or a direct `x.clear()`.
    fn cuts(&self, n: Node, src: &str) -> bool {
        match n.kind() {
            "delete_statement" => {
                let (mut found, mut nested) = (Vec::new(), false);
                for t in kids(n).into_iter().filter(|c| c.is_named()) {
                    leaves(t, &mut found, &mut nested);
                }
                found.into_iter().any(|l| self.blind(l, src))
            }
            "call" => self.clears(n, src),
            _ => false,
        }
    }

    fn clears(&self, call: Node, src: &str) -> bool {
        method_object(call, &["clear"], src).is_some_and(|o| self.blind(o, src))
    }
}

impl Transfer for Rebind {
    fn kills(&self, atom: Node<'_>, src: &str) -> bool {
        walk(atom, src, &mut |n, post, cond| {
            !post && !cond && (self.replaces(n, src) || self.cuts(n, src))
        })
    }

    fn kills_target(&self, target: Node<'_>, src: &str) -> bool {
        self.any_leaf(target, src)
    }

    fn clobbered_before<'a>(&self, atom: Node<'a>, rs: &[Span], src: &str) -> Vec<bool> {
        let (mut late, mut dead) = (vec![false; rs.len()], false);
        walk(atom, src, &mut |n, post, cond| {
            if !post {
                for (i, r) in rs.iter().enumerate() {
                    if (r.start, r.end) == (n.start_byte(), n.end_byte()) {
                        late[i] = dead;
                    }
                }
            } else if !cond {
                let walrus = n.kind() == "named_expression" && self.walrus(n, src);
                dead |= walrus || (n.kind() == "call" && self.clears(n, src));
            }
            false
        });
        late
    }
}
