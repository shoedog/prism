//! Run-site rule for function literals (#639 S2 fold 2, owner decision).
//!
//! A literal that is immediately invoked, or passed directly as a call
//! argument outside `go`/`defer`, runs at its creation point. A literal bound
//! once by `f := func..` / `var f = func..` whose name is only ever called as
//! `f(..)` runs at those calls. Every other literal (stored, returned, sent,
//! assigned elsewhere, under `go`/`defer`, or whose name is used other than as
//! a callee) may run at any later time. Real flow into callbacks a callee
//! stores and runs later is lost under the first two modes (a known cost).
use super::go_aggregate::Mode;
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

/// Per identifier name: for each occurrence, its start byte and whether it is
/// a plain callee of a call outside any literal and outside `go`/`defer`.
#[derive(Default)]
pub(crate) struct Idents(BTreeMap<String, Vec<(usize, bool)>>);

impl Idents {
    pub(crate) fn new(parsed: &ParsedFile, body: Node<'_>) -> Self {
        let mut out = Self::default();
        out.scan(parsed, body, false);
        out
    }

    fn scan(&mut self, parsed: &ParsedFile, node: Node<'_>, in_literal: bool) {
        if node.kind() == "identifier" {
            let callee = !in_literal
                && node.parent().is_some_and(|c| {
                    c.kind() == "call_expression"
                        && c.child_by_field_name("function")
                            .is_some_and(|f| f.id() == node.id())
                        && !under_go_or_defer(c)
                });
            self.0
                .entry(parsed.node_text(&node).to_owned())
                .or_default()
                .push((node.start_byte(), callee));
        }
        let inner = in_literal || node.kind() == "func_literal";
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.scan(parsed, child, inner);
        }
    }

    /// Start bytes of the callee identifiers of the plain calls of `name`, bound at `decl`.
    fn call_starts(&self, name: &str, decl: usize) -> Vec<usize> {
        self.0.get(name).map_or_else(Vec::new, |occurrences| {
            occurrences
                .iter()
                .filter(|&&(at, _)| at != decl)
                .map(|&(at, _)| at)
                .collect()
        })
    }

    /// `name` is bound at `decl` and every other occurrence is a later plain
    /// callee call.
    fn only_called(&self, name: &str, decl: usize, literal_end: usize) -> bool {
        self.0.get(name).is_some_and(|occurrences| {
            occurrences
                .iter()
                .all(|&(at, callee)| at == decl || (callee && at >= literal_end))
        })
    }
}

fn under_go_or_defer(call: Node<'_>) -> bool {
    call.parent()
        .is_some_and(|p| matches!(p.kind(), "go_statement" | "defer_statement"))
}

/// Outer parentheses do not change what a literal is.
pub(crate) fn peel(mut node: Node<'_>) -> Node<'_> {
    while let Some(p) = node
        .parent()
        .filter(|p| p.kind() == "parenthesized_expression")
    {
        node = p;
    }
    node
}

/// The sole name a single-name, single-value `:=` / `var` binds `literal` to.
fn bound_name<'t>(parsed: &ParsedFile, literal: Node<'t>) -> Option<Node<'t>> {
    let list = literal.parent().filter(|p| p.kind() == "expression_list")?;
    if list.named_child_count() != 1 {
        return None;
    }
    let decl = list.parent()?;
    let name = match decl.kind() {
        "short_var_declaration" => decl
            .child_by_field_name("left")
            .filter(|l| l.kind() == "expression_list" && l.named_child_count() == 1)?
            .named_child(0)?,
        "var_spec" => {
            let mut cursor = decl.walk();
            let names: Vec<_> = decl.children_by_field_name("name", &mut cursor).collect();
            let [only] = names.as_slice() else {
                return None;
            };
            *only
        }
        _ => return None,
    };
    (name.kind() == "identifier" && parsed.node_text(&name) != "_").then_some(name)
}

/// The name `node` (a literal, parentheses peeled) is bound to by a single `:=` / `var`, when that
/// name is only ever called afterwards (`f(..)`, outside any literal and `go`/`defer`). `body` is
/// the owner's body, the scope the occurrences are counted over.
fn only_called_name<'t>(
    parsed: &ParsedFile,
    node: Node<'t>,
    literal_end: usize,
    body: Node<'_>,
    idents: &mut Option<Idents>,
) -> Option<Node<'t>> {
    let name = bound_name(parsed, node)?;
    let idents = idents.get_or_insert_with(|| Idents::new(parsed, body));
    idents
        .only_called(parsed.node_text(&name), name.start_byte(), literal_end)
        .then_some(name)
}

/// The start bytes of the callee identifiers of every call of a literal bound once and only
/// called (the `AtCalls` shape of `mode`); empty for any other literal.
pub(crate) fn bound_literal_calls(
    parsed: &ParsedFile,
    literal: Node<'_>,
    body: Node<'_>,
    idents: &mut Option<Idents>,
) -> Vec<usize> {
    let Some(name) = only_called_name(parsed, peel(literal), literal.end_byte(), body, idents)
    else {
        return Vec::new();
    };
    idents.as_ref().map_or_else(Vec::new, |i| {
        i.call_starts(parsed.node_text(&name), name.start_byte())
    })
}

/// Mode of an outermost literal, and the name it is bound to (`AtCalls`).
pub(super) fn mode(
    parsed: &ParsedFile,
    literal: Node<'_>,
    body_id: Option<usize>,
    idents: &mut Option<Idents>,
) -> (Mode, Option<String>) {
    let node = peel(literal);
    let Some(parent) = node.parent() else {
        return (Mode::Escaping, None);
    };
    match parent.kind() {
        "call_expression"
            if parent
                .child_by_field_name("function")
                .is_some_and(|f| f.id() == node.id()) =>
        {
            let at = if under_go_or_defer(parent) {
                Mode::Escaping
            } else {
                Mode::AtCreation
            };
            (at, None)
        }
        "argument_list" => {
            let at = match parent.parent() {
                Some(call) if call.kind() == "call_expression" && !under_go_or_defer(call) => {
                    Mode::AtCreation
                }
                _ => Mode::Escaping,
            };
            (at, None)
        }
        "expression_list" => {
            let Some(body) = body_id else {
                return (Mode::Escaping, None);
            };
            let mut root = node;
            while root.id() != body {
                match root.parent() {
                    Some(p) => root = p,
                    None => return (Mode::Escaping, None),
                }
            }
            match only_called_name(parsed, node, literal.end_byte(), root, idents) {
                Some(name) => (Mode::AtCalls, Some(parsed.node_text(&name).to_owned())),
                None => (Mode::Escaping, None),
            }
        }
        _ => (Mode::Escaping, None),
    }
}
