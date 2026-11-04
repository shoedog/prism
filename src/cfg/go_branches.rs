//! Go normal flow follows syntax rather than globally sorted statement lines.
//!
//! In particular, completing an arm never enters its sibling. Case anchors
//! represent value/communication evaluation; fallthrough skips that evaluation.
//! Jumps and normal completion use the same structural successor lookup.
use super::go_jumps::{entry, successor, Successor};
use super::{CfgEdge, EdgeSink};
use crate::{ast::ParsedFile, go_statements, languages::Language};
use tree_sitter::Node;

pub(super) fn build(owner: Node<'_>, parsed: &ParsedFile, lines: &[usize], edges: &mut EdgeSink) {
    if let Some(body) = owner.child_by_field_name("body") {
        Builder {
            owner,
            parsed,
            lines,
            edges,
        }
        .sequence(body);
    }
}

struct Builder<'a, 'tree> {
    owner: Node<'tree>,
    parsed: &'a ParsedFile,
    lines: &'a [usize],
    edges: &'a mut EdgeSink,
}
impl<'tree> Builder<'_, 'tree> {
    fn emit(&mut self, from: Node<'_>, to: Option<usize>) {
        if let Some(to_line) = to {
            self.edges.push(CfgEdge {
                file: self.parsed.path.clone(),
                from_line: from.start_position().row + 1,
                to_line,
            });
        }
    }
    fn after(&self, node: Node<'_>) -> Option<usize> {
        match successor(node, self.owner, self.lines) {
            Successor::Line(line) => Some(line),
            Successor::Exit | Successor::Unknown => None,
        }
    }
    fn entry(&self, node: Node<'_>) -> Option<usize> {
        match entry(node, self.owner, self.lines) {
            Successor::Line(line) => Some(line),
            Successor::Exit | Successor::Unknown => None,
        }
    }
    fn dispatch(&mut self, node: Node<'tree>) -> Node<'tree> {
        if let Some(init) = node.child_by_field_name("initializer") {
            let line = init.start_position().row + 1;
            if line != node.start_position().row + 1 && self.lines.binary_search(&line).is_ok() {
                // The keyword is the entry, then the mandatory initializer
                // executes before either condition outcome or case dispatch.
                self.emit(node, Some(line));
                return init;
            }
        }
        // Unanchored inc/dec initializers share the existing header anchor.
        node
    }
    fn sequence(&mut self, node: Node<'tree>) {
        for child in go_statements::children(node, Language::Go)
            .filter(|n| n.is_named() && n.kind() != "comment")
        {
            self.statement(child);
        }
    }
    fn statement(&mut self, node: Node<'tree>) {
        match node.kind() {
            "block" => self.sequence(node),
            "if_statement" => {
                let dispatch = self.dispatch(node);
                let after = self.after(node);
                for field in ["consequence", "alternative"] {
                    let child = node.child_by_field_name(field);
                    // Only an empty block may take the successor directly;
                    // an unknown entry must not be skipped to fabricate flow.
                    let target = match child {
                        Some(n) if n.kind() == "block" && first(n).is_none() => after,
                        Some(n) => self.entry(n),
                        None => after,
                    };
                    self.emit(dispatch, target);
                    if let Some(child) = child {
                        self.statement(child);
                    }
                }
            }
            "for_statement" => {
                if let Some(body) = node.child_by_field_name("body") {
                    let target = if first(body).is_none() {
                        Some(node.start_position().row + 1)
                    } else {
                        self.entry(body)
                    };
                    self.emit(node, target);
                    self.sequence(body);
                }
                // Retain the existing loop-condition false path, now landing
                // on a following switch header instead of its first body.
                self.emit(node, self.after(node));
            }
            "expression_switch_statement" | "type_switch_statement" | "select_statement" => {
                let dispatch = self.dispatch(node);
                let cases: Vec<_> = go_statements::children(node, Language::Go)
                    .filter(|n| is_case(n.kind()))
                    .collect();
                let after = self.after(node);
                if node.kind() != "select_statement"
                    && !cases.iter().any(|c| c.kind() == "default_case")
                {
                    self.emit(dispatch, after);
                }
                for case in cases {
                    self.emit(dispatch, self.entry(case));
                    self.emit(
                        case,
                        first_body(case).and_then(|n| self.entry(n)).or_else(|| {
                            // A selected empty arm exits; an unknown body doesn't.
                            first_body(case).is_none().then_some(after).flatten()
                        }),
                    );
                    for child in body_statements(case) {
                        self.statement(child);
                    }
                }
            }
            "fallthrough_statement" => {
                if let Some(case) = go_statements::parent(node).filter(|c| is_case(c.kind())) {
                    let mut next = go_statements::next_named_sibling(case);
                    while next.is_some_and(|n| n.kind() == "comment") {
                        next = next.and_then(go_statements::next_named_sibling);
                    }
                    if let Some(next) = next.filter(|n| is_case(n.kind())) {
                        let target = match first_body(next) {
                            Some(body) => self.entry(body),
                            None => self.after(next),
                        };
                        self.emit(node, target);
                    }
                }
            }
            "labeled_statement" => {
                if let Some(body) = go_statements::children(node, Language::Go)
                    .find(|n| n.is_named() && n.kind() != "label_name" && n.kind() != "comment")
                {
                    if body.start_position().row != node.start_position().row {
                        self.emit(node, self.entry(body));
                    }
                    self.statement(body);
                }
            }
            "break_statement" | "continue_statement" | "goto_statement" | "return_statement" => {}
            _ if self.parsed.language.is_statement_node(node.kind()) => {
                self.emit(node, self.after(node));
            }
            _ => {} // Unknown syntax never supplies a guessed bypass.
        }
    }
}
fn is_case(kind: &str) -> bool {
    matches!(
        kind,
        "expression_case" | "type_case" | "communication_case" | "default_case"
    )
}
fn first(node: Node<'_>) -> Option<Node<'_>> {
    go_statements::children(node, Language::Go).find(|n| n.is_named() && n.kind() != "comment")
}
fn body_statements(case: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    // Only the statement_list is the body. A send/receive in the case header
    // executes at the case anchor, not again as an ordinary body statement.
    let mut cursor = case.walk();
    let list = case
        .named_children(&mut cursor)
        .find(|n| n.kind() == "statement_list");
    list.into_iter()
        .flat_map(|list| go_statements::children(list, Language::Go))
        .filter(|n| n.is_named() && n.kind() != "comment")
}
fn first_body(case: Node<'_>) -> Option<Node<'_>> {
    body_statements(case).next()
}
