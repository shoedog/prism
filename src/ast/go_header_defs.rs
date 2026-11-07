//! Go binding forms the Assignments query has no node for (#694 slice A):
//! range and receive `:=` targets, type-switch aliases, and the `var`/`const`
//! names beyond the first name of the first direct spec. Each becomes a def at
//! its identifier, like the target of `x := e`; the right-hand side's reads sit
//! on the same line and need nothing else.
//!
//! A type-switch alias is ONE def at the header identifier (so the guard's reads
//! and the def share a line); `BindingFacts` groups it with the alias's per-clause
//! declarations, so every clause's uses are reached by it.
use super::{ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Append the spans of those declarations not already in `out`.
    pub(super) fn collect_go_header_def_spans(
        &self,
        func_node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        if self.language != Language::Go {
            return;
        }
        let seen: BTreeSet<usize> = out.iter().map(|s| s.start_byte).collect();
        let mut found = Vec::new();
        self.walk_go_header_defs(func_node, &mut found);
        out.extend(
            found
                .into_iter()
                .filter(|s| lines.contains(&s.line) && !seen.contains(&s.start_byte)),
        );
    }

    fn walk_go_header_defs(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        match node.kind() {
            "range_clause" | "receive_statement" => {
                let mut cursor = node.walk();
                let declares = node.children(&mut cursor).any(|c| c.kind() == ":=");
                if declares {
                    if let Some(left) = node.child_by_field_name("left") {
                        self.push_go_names(left, out);
                    }
                }
            }
            "type_switch_statement" => {
                if let Some(alias) = node.child_by_field_name("alias") {
                    self.push_go_names(alias, out);
                }
            }
            "var_declaration" | "const_declaration" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    match child.kind() {
                        "var_spec" | "const_spec" => self.push_spec_names(child, out),
                        "var_spec_list" => {
                            let mut inner = child.walk();
                            for spec in child.named_children(&mut inner) {
                                if matches!(spec.kind(), "var_spec" | "const_spec") {
                                    self.push_spec_names(spec, out);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.walk_go_header_defs(child, out);
        }
    }

    fn push_spec_names(&self, spec: Node<'_>, out: &mut Vec<PathSpan>) {
        let mut cursor = spec.walk();
        for name in spec.children_by_field_name("name", &mut cursor) {
            self.push_go_names(name, out);
        }
    }

    fn push_go_names(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        if node.kind() == "identifier" {
            self.push_simple_lvalue_span(node, out);
        } else if node.kind() == "expression_list" {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "identifier" {
                    self.push_simple_lvalue_span(child, out);
                }
            }
        }
    }
}
