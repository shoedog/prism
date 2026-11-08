//! ReturnSink (#694 slice B): a value that leaves a literal through its call result.
//!
//! A def inside a literal called in place (`err = func() error { err := do(); return err }()`), or
//! inside one bound once and only called (`fn := func..; errs = append(errs, fn(1)...)`), reaches
//! the variable the call result is assigned to through the result, which no edge connects, so rule
//! S keeps the pair the name coincidence draws (main's behaviour). Soundness rules:
//!
//! * the call sits on the right of an assignment, `:=` or `var` naming plain variables on the left
//!   (an identifier under an index, selector or `*` on the left is a read, not a written variable);
//!   a call among another call's arguments counts, as prism's line model feeds the arguments of a
//!   right-hand side call into the left-hand side (`x := g(y)` gives `y` flow into `x`);
//! * the read resolves to the declaration the statement leaves the written variable bound to (a
//!   later `if err := ..` or block-local redeclaration of the name is another variable; a name
//!   that is not an explicit declaration gives no sink);
//! * the def's nearest enclosing callable is the sink literal, or an in-place literal nested in it
//!   whose call is itself in a `return` of the sink (recursively), never a `go`/`defer` or other
//!   nested literal;
//! * the use lies outside the sink literal and does not precede the statement, unless a `for`
//!   statement of the owner contains both (the statement is one node in the line model, so a read
//!   inside it counts).
use crate::ast::ParsedFile;
use crate::cpg::binding_facts::{BindingFacts, BindingId};
use crate::cpg::{bound_literal_calls, peel, GoLiteralIdents};
use std::collections::BTreeMap;
use tree_sitter::Node;

struct Sink {
    /// The sink literal.
    literal: (usize, usize),
    /// Start bytes of the callables a def may sit in: the literal and the in-place literals it
    /// returns out of (recursively).
    carriers: Vec<usize>,
    statement: (usize, usize),
    /// The `for` statements that contain the statement.
    loops: Vec<(usize, usize)>,
    /// Each written variable and the declaration the statement leaves it bound to.
    names: BTreeMap<String, usize>,
}

#[derive(Default)]
pub(super) struct ReturnSinks {
    sinks: Vec<Sink>,
}

impl ReturnSinks {
    pub(super) fn new(parsed: &ParsedFile, owner: Node<'_>, facts: &BindingFacts) -> Self {
        let mut out = Self::default();
        let body = owner.child_by_field_name("body").unwrap_or(owner);
        let mut idents: Option<GoLiteralIdents> = None;
        let mut stack = vec![owner];
        while let Some(node) = stack.pop() {
            if node.kind() == "func_literal" {
                let mut calls = Vec::new();
                let outer = peel(node);
                if let Some(call) = outer.parent().filter(|p| {
                    p.kind() == "call_expression"
                        && p.child_by_field_name("function")
                            .is_some_and(|f| f.id() == outer.id())
                }) {
                    calls.push(call);
                }
                for at in bound_literal_calls(parsed, node, body, &mut idents) {
                    let callee = parsed
                        .tree
                        .root_node()
                        .descendant_for_byte_range(at, at)
                        .and_then(|n| n.parent())
                        .filter(|p| p.kind() == "call_expression");
                    calls.extend(callee);
                }
                for call in calls {
                    if let Some((names, statement)) = statement_of(parsed, facts, call) {
                        out.sinks.push(Sink {
                            literal: (node.start_byte(), node.end_byte()),
                            carriers: carriers(node),
                            statement: (statement.start_byte(), statement.end_byte()),
                            loops: loops_around(statement),
                            names,
                        });
                    }
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        out
    }

    /// The def of `name` sits (nearest enclosing callable `def_callable`) in a literal that
    /// returns its value into a variable `name`, and the read at `use_start` can see it.
    pub(super) fn carries(
        &self,
        facts: &BindingFacts,
        name: &str,
        def_callable: Option<usize>,
        use_start: usize,
    ) -> bool {
        let Some(callable) = def_callable else {
            return false;
        };
        let used = facts.binding_at(name, use_start);
        self.sinks.iter().any(|s| {
            s.names
                .get(name)
                .is_some_and(|d| used == BindingId::Declaration(*d))
                && s.carriers.contains(&callable)
                && !(s.literal.0 <= use_start && use_start < s.literal.1)
                && (use_start >= s.statement.0
                    || s.loops
                        .iter()
                        .any(|(a, b)| *a <= use_start && use_start < *b))
        })
    }
}

fn unparen(mut node: Node<'_>) -> Node<'_> {
    while node.kind() == "parenthesized_expression" {
        match node.named_child(0) {
            Some(inner) => node = inner,
            None => break,
        }
    }
    node
}

/// `literal` and the in-place literals returned, directly or through further returns, out of it.
fn carriers(literal: Node<'_>) -> Vec<usize> {
    let mut out = vec![literal.start_byte()];
    let mut todo = vec![literal];
    while let Some(callable) = todo.pop() {
        let Some(body) = callable.child_by_field_name("body") else {
            continue;
        };
        let mut stack = vec![body];
        while let Some(node) = stack.pop() {
            if node.kind() == "func_literal" {
                continue;
            }
            if node.kind() == "return_statement" {
                let mut values = Vec::new();
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if child.kind() == "expression_list" {
                        let mut inner = child.walk();
                        values.extend(child.named_children(&mut inner));
                    } else {
                        values.push(child);
                    }
                }
                for value in values {
                    let value = unparen(value);
                    let callee = (value.kind() == "call_expression")
                        .then(|| value.child_by_field_name("function").map(unparen))
                        .flatten()
                        .filter(|f| f.kind() == "func_literal");
                    if let Some(inner) = callee {
                        out.push(inner.start_byte());
                        todo.push(inner);
                    }
                }
                continue;
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
    }
    out
}

fn loops_around(statement: Node<'_>) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut node = statement.parent();
    while let Some(n) = node {
        if n.kind() == "for_statement" {
            out.push((n.start_byte(), n.end_byte()));
        }
        node = n.parent();
    }
    out
}

/// The statement node and the plain variables written on the left of the assignment or declaration
/// whose right side holds `call`.
fn statement_of<'t>(
    parsed: &ParsedFile,
    facts: &BindingFacts,
    call: Node<'t>,
) -> Option<(BTreeMap<String, usize>, Node<'t>)> {
    let mut child = call;
    while let Some(node) = child.parent() {
        let (left, right) = match node.kind() {
            "short_var_declaration" | "assignment_statement" => ("left", "right"),
            "var_spec" => ("name", "value"),
            k if k == "block"
                || k == "func_literal"
                || k.ends_with("_statement")
                || k.ends_with("_declaration") =>
            {
                return None;
            }
            _ => {
                child = node;
                continue;
            }
        };
        let side = node.child_by_field_name(right)?;
        if !(side.start_byte() <= child.start_byte() && child.end_byte() <= side.end_byte()) {
            return None;
        }
        let mut targets = Vec::new();
        let mut cursor = node.walk();
        for field in node.children_by_field_name(left, &mut cursor) {
            if field.kind() == "expression_list" {
                let mut inner = field.walk();
                targets.extend(field.named_children(&mut inner));
            } else {
                targets.push(field);
            }
        }
        // The declaration the name is bound to once the statement has run (`:=` declares or reuses,
        // `=` writes the existing one). Anything not an explicit declaration gives no sink.
        let mut names = BTreeMap::new();
        for target in targets.into_iter().filter(|t| t.kind() == "identifier") {
            let name = parsed.node_text(&target);
            if name == "_" {
                continue;
            }
            if let BindingId::Declaration(d) = facts.binding_at(name, node.end_byte()) {
                names.insert(name.to_owned(), d);
            }
        }
        return Some((names, node));
    }
    None
}
