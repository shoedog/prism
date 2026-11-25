//! I1 binding and storage transfer (lane 10, #847): may the store W be a source along a route?
//!
//! Admission is syntax only, independent of RD's declaration ids (preview wiring combines
//! both). It requires a bare local or formal root that the owner itself binds, then follows W's
//! lifetime from receiver capture to commit. A Python assignment evaluates its right side
//! first, so a rebind there starts a new lifetime. An augmented assignment evaluates its target
//! first, so its right side runs after capture. Every index expression runs after the links
//! before it are captured. A definite rebind of the root, or a key-blind mutator of a strict
//! ancestor of the receiver, detaches the receiver from the name: the store lands in storage no
//! later read reaches. A conditional or deferred effect leaves one certified arm and is kept.
//! After commit, [`Rebind`] supplies the kills of the route walk.
use super::python::{kids, leaves, peel_down, static_path, text, FENCES};
use super::rebind::{import_binds, method_object, walk, Rebind, WINDOW_MUTATORS};
use super::route::Why;
use super::{Span, Write, WriteRefusal};
use tree_sitter::Node;

/// Kill events along a route. `atom` is a simple statement or a header expression.
pub(crate) trait Transfer {
    fn kills(&self, atom: Node<'_>, src: &str) -> bool;
    /// A loop target bound on a successful entry.
    fn kills_target(&self, target: Node<'_>, src: &str) -> bool;
    /// Per operand span inside `atom`: an expression-level replacement or clear is definitely
    /// evaluated before it (`take((d := {}), d)`). Statement-level kills run after every read.
    fn clobbered_before<'a>(&self, atom: Node<'a>, rs: &[Span], src: &str) -> Vec<bool>;
}

pub(crate) struct Admitted {
    pub tr: Rebind,
    /// A later target of a chained assignment replaces the root right after the commit.
    pub chain_killed: bool,
}

/// Admit W, or say why its contribution cannot be tracked.
pub(crate) fn admit(body: Node, w: &Write, src: &str) -> Result<Admitted, Why> {
    let tr = Rebind::of(&w.path).ok_or(Why::Syntax)?;
    let def = body.parent().filter(|p| p.kind() == "function_definition");
    if !def.is_some_and(|d| owns_binding(d, &tr.root, src)) {
        return Err(Why::Binding);
    }
    let (leaf, store) = store_of(body, w).ok_or(Why::Syntax)?;
    let k = w.path.fields.len();
    let rhs = store.child_by_field_name("right");
    let mut window: Vec<(Node, usize)> = indices(leaf).into_iter().zip(0..).collect();
    // an augmented store runs its right side after the whole receiver is captured
    if let (Some(rhs), "augmented_assignment") = (rhs, store.kind()) {
        window.push((rhs, w.path.brackets.saturating_sub(1)));
    }
    for (expr, captured) in window {
        if detaches(expr, &tr, 1 + k + captured, src) {
            return Err(Why::Capture);
        }
    }
    let mut chain_killed = false;
    let mut next = rhs.filter(|r| r.kind() == "assignment");
    while let Some(link) = next {
        let left = link.child_by_field_name("left");
        chain_killed |= left.is_some_and(|l| tr.any_leaf(l, src));
        next = link
            .child_by_field_name("right")
            .filter(|r| r.kind() == "assignment");
    }
    Ok(Admitted { tr, chain_killed })
}

/// Length of the container path `obj` when it is a static ancestor-or-self of W's storage:
/// the root, then fields, then one per bracket (key-blind).
fn link_len(obj: Node, tr: &Rebind, src: &str) -> Option<usize> {
    let (p, shape) = static_path(peel_down(obj), src);
    let rooted = p.root.as_deref() == Some(tr.root.as_str());
    if !rooted || shape == Some(WriteRefusal::DotAfterBracket) {
        return None;
    }
    let (mine, theirs) = (&tr.fields, &p.fields);
    let ancestor = match p.brackets {
        0 => theirs.len() <= mine.len() && theirs.iter().zip(mine).all(|(a, b)| a == b),
        _ => theirs == mine,
    };
    ancestor.then_some(1 + theirs.len() + p.brackets)
}

/// Does evaluating `expr`, with `held` links of the receiver chain already captured, rebind
/// the root or mutate a strict ancestor of a captured link?
fn detaches(expr: Node, tr: &Rebind, held: usize, src: &str) -> bool {
    walk(expr, &mut |n, post, cond| {
        if post || cond {
            return false;
        }
        match n.kind() {
            "named_expression" => n
                .child_by_field_name("name")
                .is_some_and(|b| text(b, src) == tr.root),
            "call" => method_object(n, &WINDOW_MUTATORS, src)
                .and_then(|o| link_len(o, tr, src))
                .is_some_and(|len| len < held),
            _ => false,
        }
    })
}

/// W's target leaf and the assignment that stores through it.
fn store_of<'a>(body: Node<'a>, w: &Write) -> Option<(Node<'a>, Node<'a>)> {
    let mut leaf = body.descendant_for_byte_range(w.span.start, w.span.end)?;
    while (leaf.start_byte(), leaf.end_byte()) != (w.span.start, w.span.end) {
        leaf = leaf.parent()?;
    }
    let mut up = leaf.parent()?;
    while up.kind() == "parenthesized_expression" {
        up = up.parent()?;
    }
    let left = up.child_by_field_name("left").map(peel_down);
    let store = matches!(up.kind(), "assignment" | "augmented_assignment");
    (store && left.is_some_and(|l| l.id() == leaf.id())).then_some((leaf, up))
}

/// Index expressions of a store target, innermost bracket first (evaluation order).
fn indices(leaf: Node) -> Vec<Node> {
    let (mut out, mut n) = (Vec::new(), peel_down(leaf));
    loop {
        let next = match n.kind() {
            "subscript" => {
                out.extend(n.children_by_field_name("subscript", &mut n.walk()).take(1));
                n.child_by_field_name("value")
            }
            "attribute" => n.child_by_field_name("object"),
            _ => break,
        };
        match next {
            Some(v) => n = peel_down(v),
            None => break,
        }
    }
    out.reverse();
    out
}

/// The identifier a parameter binds.
fn param_name(p: Node) -> Option<Node> {
    if p.kind() == "identifier" {
        return Some(p);
    }
    let first = p
        .child_by_field_name("name")
        .or_else(|| kids(p).into_iter().find(|c| c.is_named()))?;
    param_name(first)
}

/// Does a node of the owner's own scope bind `name`?
fn binds(n: Node, name: &str, src: &str) -> bool {
    let is = |t: Node| text(t, src) == name;
    let any_leaf = |t: Option<Node>| {
        let (mut found, mut nested) = (Vec::new(), false);
        t.into_iter()
            .for_each(|t| leaves(t, &mut found, &mut nested));
        found.iter().any(|l| l.kind() == "identifier" && is(*l))
    };
    match n.kind() {
        "assignment" | "augmented_assignment" | "for_statement" => {
            any_leaf(n.child_by_field_name("left"))
        }
        "named_expression" => n.child_by_field_name("name").is_some_and(is),
        "function_definition" | "class_definition" => n.child_by_field_name("name").is_some_and(is),
        "decorated_definition" => n
            .child_by_field_name("definition")
            .and_then(|d| d.child_by_field_name("name"))
            .is_some_and(is),
        "as_pattern_target" | "delete_statement" => kids(n)
            .into_iter()
            .filter(|c| c.is_named())
            .any(|c| any_leaf(Some(c))),
        "import_statement" | "import_from_statement" => import_binds(n, name, src),
        _ => false,
    }
}

/// The owner binds `name` itself: a formal, or a binding construct of its own scope (nested
/// scopes excluded), and no `global`/`nonlocal` declaration redirects it.
fn owns_binding(def: Node, name: &str, src: &str) -> bool {
    let formal = def.child_by_field_name("parameters").is_some_and(|ps| {
        let named = kids(ps).into_iter().filter(|c| c.is_named());
        named.filter_map(param_name).any(|p| text(p, src) == name)
    });
    let (mut bound, mut redirected) = (formal, false);
    let mut stack: Vec<Node> = def.child_by_field_name("body").into_iter().collect();
    while let Some(n) = stack.pop() {
        match n.kind() {
            "global_statement" | "nonlocal_statement" => {
                redirected |= kids(n)
                    .iter()
                    .any(|i| i.is_named() && text(*i, src) == name);
            }
            _ => bound |= binds(n, name, src),
        }
        if !FENCES.contains(&n.kind()) {
            stack.extend(kids(n));
        }
    }
    bound && !redirected
}
