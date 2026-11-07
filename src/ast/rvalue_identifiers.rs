//! Identifier paths and spans in value context, and rvalue identifier queries.

use crate::access_path::AccessPath;
use crate::ast::{is_plain_ident, ParsedFile, PathSpan, RvalueOwnerDecision, RvalueQueryScope};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Check if a node kind represents a field/member access expression.
    /// Each language has a different tree-sitter node kind for this:
    /// - C/C++, Rust: field_expression
    /// - JS/TS: member_expression
    /// - Go: selector_expression
    /// - Python: attribute
    /// - Java: field_access
    /// - Lua: dot_index_expression, method_index_expression (colon syntax)
    pub(super) fn is_field_access_node(kind: &str) -> bool {
        matches!(
            kind,
            "field_expression"
                | "member_expression"
                | "selector_expression"
                | "attribute"
                | "field_access"
                | "dot_index_expression"
                | "method_index_expression"
        )
    }

    pub(super) fn is_augmented_assignment(&self, node: &Node<'_>) -> bool {
        if matches!(
            node.kind(),
            "augmented_assignment"
                | "augmented_assignment_expression"
                | "compound_assignment_expr"
                | "update_expression"
        ) {
            return true;
        }

        const AUGMENTED_ASSIGN_OPS: &[&str] = &[
            "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>=", "**=", "//=",
        ];
        let mut cursor = node.walk();
        let found = node.children(&mut cursor).any(|child| {
            if child.is_named() {
                return false;
            }
            let kind = child.kind();
            let text = self.node_text(&child);
            AUGMENTED_ASSIGN_OPS.contains(&kind) || AUGMENTED_ASSIGN_OPS.contains(&text.trim())
        });
        found
    }

    pub(super) fn collect_identifier_paths<'a>(&self, node: Node<'a>, out: &mut Vec<(AccessPath, usize)>) {
        if !self.language.is_in_erased_type_context(node) {
            self.collect_identifier_paths_in_value_context(node, out);
        }
    }

    fn collect_identifier_paths_in_value_context(
        &self,
        node: Node<'_>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let Some(node) = self.compat_value_node(node) else {
            return;
        };
        // Check for field/member access expressions — emit the full qualified
        // path instead of individual identifiers.
        if Self::is_field_access_node(node.kind()) {
            if let Some(member) = self.bounded_member_path(node) {
                out.push((member.path, node.start_position().row + 1));
                out.push((
                    AccessPath::simple(self.node_text(&member.receiver)),
                    member.receiver.start_position().row + 1,
                ));
                return;
            }
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::from_expr(&text), line));
            // Also emit the base identifier for field-insensitive fallback.
            // Different tree-sitter grammars use different field names:
            //   C/C++/Rust: "argument", Go: "operand", JS/TS/Python/Java: "object"
            //   Lua: first named child (no standard field name)
            let base_node = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("object"))
                .or_else(|| node.child_by_field_name("operand"))
                .or_else(|| node.named_child(0));
            if let Some(base) = base_node {
                if self.language.is_identifier_node(base.kind()) {
                    let base_name = self.node_text(&base).to_string();
                    out.push((AccessPath::simple(base_name), line));
                }
            }
            return; // Don't recurse into children — we've handled them
        }

        if self.language.is_identifier_node(node.kind()) {
            let name = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::simple(name), line));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_paths_in_value_context(child, out);
        }
    }

    pub(super) fn collect_identifier_paths_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        if matches!(scope, RvalueQueryScope::LegacyInventory) {
            self.collect_identifier_paths(node, out);
            return;
        }
        if !self.rvalue_scope_allows_node(scope, node, false)
            || self.language.is_in_erased_type_context(node)
        {
            return;
        }
        self.collect_identifier_paths_in_value_context_scoped(node, scope, out);
    }

    fn collect_identifier_paths_in_value_context_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let Some(node) = self.compat_value_node(node) else {
            return;
        };
        if Self::is_rvalue_class_node(node.kind()) {
            self.collect_identifier_paths_in_value_context(node, out);
            return;
        }
        if self.rvalue_is_nested_callable(scope, node) {
            if node.kind() == "method_definition" {
                if let Some(name) = node.child_by_field_name("name") {
                    if name.kind() == "computed_property_name" {
                        self.collect_identifier_paths_in_value_context_scoped(name, scope, out);
                    }
                }
            }
            return;
        }
        if Self::is_field_access_node(node.kind()) {
            if let Some(member) = self.bounded_member_path(node) {
                out.push((member.path, node.start_position().row + 1));
                out.push((
                    AccessPath::simple(self.node_text(&member.receiver)),
                    member.receiver.start_position().row + 1,
                ));
                return;
            }
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((AccessPath::from_expr(&text), line));
            let base_node = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("object"))
                .or_else(|| node.child_by_field_name("operand"))
                .or_else(|| node.named_child(0));
            if let Some(base) = base_node {
                if self.language.is_identifier_node(base.kind()) {
                    out.push((
                        AccessPath::simple(self.node_text(&base)),
                        base.start_position().row + 1,
                    ));
                }
            }
            return;
        }
        if self.language.is_identifier_node(node.kind()) {
            out.push((
                AccessPath::simple(self.node_text(&node)),
                node.start_position().row + 1,
            ));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_paths_in_value_context_scoped(child, scope, out);
        }
    }

    pub(super) fn collect_identifier_path_spans(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        if !self.language.is_in_erased_type_context(node) {
            self.collect_identifier_spans_in_value_context(node, out);
        }
    }

    fn collect_identifier_spans_in_value_context(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let Some(node) = self.compat_value_node(node) else {
            return;
        };
        if Self::is_field_access_node(node.kind()) {
            if let Some(member) = self.bounded_member_path(node) {
                out.push(PathSpan {
                    path: member.path,
                    line: node.start_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
                self.push_identifier_path_span(member.receiver, out);
                return;
            }
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push(PathSpan {
                path: AccessPath::from_expr(&text),
                line,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });

            if let Some(base) = self.leftmost_receiver_identifier(node) {
                self.push_identifier_path_span(base, out);
            }
            return;
        }

        if self.language.is_identifier_node(node.kind()) {
            self.push_identifier_path_span(node, out);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_spans_in_value_context(child, out);
        }
    }

    pub(super) fn collect_identifier_path_spans_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<PathSpan>,
    ) {
        if matches!(scope, RvalueQueryScope::LegacyInventory) {
            self.collect_identifier_path_spans(node, out);
            return;
        }
        if !self.rvalue_scope_allows_node(scope, node, false)
            || self.language.is_in_erased_type_context(node)
        {
            return;
        }
        self.collect_identifier_spans_in_value_context_scoped(node, scope, out);
    }

    fn collect_identifier_spans_in_value_context_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<PathSpan>,
    ) {
        let Some(node) = self.compat_value_node(node) else {
            return;
        };
        if Self::is_rvalue_class_node(node.kind()) {
            self.collect_identifier_spans_in_value_context(node, out);
            return;
        }
        if self.rvalue_is_nested_callable(scope, node) {
            if node.kind() == "method_definition" {
                if let Some(name) = node.child_by_field_name("name") {
                    if name.kind() == "computed_property_name" {
                        self.collect_identifier_spans_in_value_context_scoped(name, scope, out);
                    }
                }
            }
            return;
        }
        if Self::is_field_access_node(node.kind()) {
            if let Some(member) = self.bounded_member_path(node) {
                out.push(PathSpan {
                    path: member.path,
                    line: node.start_position().row + 1,
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                });
                self.push_identifier_path_span(member.receiver, out);
                return;
            }
            let text = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push(PathSpan {
                path: AccessPath::from_expr(&text),
                line,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
            if let Some(base) = self.leftmost_receiver_identifier(node) {
                self.push_identifier_path_span(base, out);
            }
            return;
        }
        if self.language.is_identifier_node(node.kind()) {
            self.push_identifier_path_span(node, out);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifier_spans_in_value_context_scoped(child, scope, out);
        }
    }

    fn leftmost_receiver_identifier<'a>(&self, mut node: Node<'a>) -> Option<Node<'a>> {
        loop {
            let receiver = node
                .child_by_field_name("argument")
                .or_else(|| node.child_by_field_name("object"))
                .or_else(|| node.child_by_field_name("operand"))
                .or_else(|| node.named_child(0))?;
            if Self::is_field_access_node(receiver.kind()) {
                node = receiver;
                continue;
            }
            return self
                .language
                .is_identifier_node(receiver.kind())
                .then_some(receiver);
        }
    }

    pub(super) fn push_identifier_path_span(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        let name = self.node_text(&node).to_string();
        if is_plain_ident(&name) {
            out.push(PathSpan {
                path: AccessPath::simple(name),
                line: node.start_position().row + 1,
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
            });
        }
    }

    pub(super) fn collect_return_value_identifier_spans(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
        scope: &RvalueQueryScope,
        out: &mut Vec<PathSpan>,
    ) {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Returns) {
            let ret_idx = query
                .capture_index_for_name("ret")
                .expect("Returns query must have @ret capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ret_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && matches!(
                                self.rvalue_capture_owner(func_node, scope, capture.node),
                                RvalueOwnerDecision::Owned
                            )
                        {
                            self.collect_return_node_value_spans_scoped(capture.node, scope, out);
                        }
                    }
                }
            }
        }
    }

    fn collect_return_node_value_spans(&self, node: Node<'_>, out: &mut Vec<PathSpan>) {
        if self.language == Language::Go {
            if let Some(child) = node.named_child(0) {
                self.collect_identifier_path_spans(child, out);
            }
            return;
        }

        if let Some(child) = node.named_child(0) {
            self.collect_identifier_path_spans(child, out);
        }
    }

    pub(super) fn collect_return_node_value_spans_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<PathSpan>,
    ) {
        if matches!(scope, RvalueQueryScope::LegacyInventory) {
            self.collect_return_node_value_spans(node, out);
            return;
        }
        if !self.rvalue_scope_allows_node(scope, node, false) {
            return;
        }
        if let Some(child) = node.named_child(0) {
            self.collect_identifier_path_spans_scoped(child, scope, out);
        }
    }

    /// Find all R-value identifiers on diff lines within a function (excluding L-values).
    pub fn rvalue_identifiers_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let scope = self.rvalue_query_scope(func_node);

        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut rvalues = Vec::new();

            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches =
                cursor.matches(assign_query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line)
                            && self.language.is_assignment_node(capture.node.kind())
                            && matches!(
                                self.rvalue_capture_owner(func_node, &scope, capture.node),
                                RvalueOwnerDecision::Owned
                            )
                        {
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_all_identifiers_scoped(rhs, &scope, &mut rvalues);
                            }
                        }
                    }
                }
            }

            // Also collect from call arguments
            if let Some(call_query) = get_query(self.language, QueryKind::Calls) {
                let call_idx = call_query
                    .capture_index_for_name("call")
                    .expect("Calls query must have @call capture");
                let mut cursor2 = tree_sitter::QueryCursor::new();
                cursor2.set_byte_range(func_node.byte_range());
                let mut matches2 =
                    cursor2.matches(call_query, self.tree.root_node(), self.source.as_bytes());
                while let Some(m) = matches2.next() {
                    for capture in m.captures {
                        if capture.index == call_idx {
                            let line = capture.node.start_position().row + 1;
                            if lines.contains(&line)
                                && matches!(
                                    self.rvalue_capture_owner(func_node, &scope, capture.node),
                                    RvalueOwnerDecision::Owned
                                )
                            {
                                if let Some(args) = self.language.call_arguments(&capture.node) {
                                    self.collect_all_identifiers_scoped(args, &scope, &mut rvalues);
                                }
                                if let Some(func_name_node) = self.call_value_name(&capture.node) {
                                    let name = self.node_text(&func_name_node).to_string();
                                    rvalues.push((name, line));
                                }
                            }
                        }
                    }
                }
            }

            return rvalues;
        }

        let mut rvalues = Vec::new();
        self.collect_rvalues_manual(*func_node, lines, &mut rvalues);
        rvalues
    }

    /// Manual recursive R-value collection (pre-query fallback).
    pub(super) fn collect_rvalues_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let scope = self.rvalue_query_scope(&node);
        self.collect_rvalues_manual_scoped(node, lines, &scope, true, out);
    }

    fn collect_rvalues_manual_scoped(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        scope: &RvalueQueryScope,
        query_root: bool,
        out: &mut Vec<(String, usize)>,
    ) {
        if !self.rvalue_scope_allows_node(scope, node, query_root) {
            return;
        }
        if !query_root && matches!(scope, RvalueQueryScope::CallableBody { .. }) {
            if Self::is_rvalue_class_node(node.kind()) {
                self.collect_rvalues_manual_scoped(
                    node,
                    lines,
                    &RvalueQueryScope::LegacyInventory,
                    false,
                    out,
                );
                return;
            }
            if self.rvalue_is_nested_callable(scope, node) {
                if node.kind() == "method_definition" {
                    if let Some(name) = node.child_by_field_name("name") {
                        if name.kind() == "computed_property_name"
                            && lines.contains(&(name.start_position().row + 1))
                        {
                            self.collect_all_identifiers_scoped(name, scope, out);
                        }
                    }
                }
                return;
            }
        }
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_all_identifiers_scoped(rhs, scope, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_all_identifiers_scoped(args, scope, out);
            }
            if let Some(func_name_node) = self.call_value_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalues_manual_scoped(child, lines, scope, false, out);
        }
    }

    pub(super) fn collect_all_identifiers<'a>(&self, node: Node<'a>, out: &mut Vec<(String, usize)>) {
        if !self.language.is_in_erased_type_context(node) {
            self.collect_identifiers_in_value_context(node, out);
        }
    }

    fn collect_identifiers_in_value_context(&self, node: Node<'_>, out: &mut Vec<(String, usize)>) {
        // Descendants need only a local boundary check: scanning every ancestor
        // at every recursive step would turn deep value walks quadratic.
        let Some(node) = self.compat_value_node(node) else {
            return;
        };
        if self.language.is_identifier_node(node.kind()) {
            let name = self.node_text(&node).to_string();
            let line = node.start_position().row + 1;
            out.push((name, line));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers_in_value_context(child, out);
        }
    }

    fn collect_all_identifiers_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<(String, usize)>,
    ) {
        if matches!(scope, RvalueQueryScope::LegacyInventory) {
            self.collect_all_identifiers(node, out);
            return;
        }
        if !self.rvalue_scope_allows_node(scope, node, false)
            || self.language.is_in_erased_type_context(node)
        {
            return;
        }
        self.collect_identifiers_in_value_context_scoped(node, scope, out);
    }

    fn collect_identifiers_in_value_context_scoped(
        &self,
        node: Node<'_>,
        scope: &RvalueQueryScope,
        out: &mut Vec<(String, usize)>,
    ) {
        let Some(node) = self.compat_value_node(node) else {
            return;
        };
        if Self::is_rvalue_class_node(node.kind()) {
            self.collect_identifiers_in_value_context(node, out);
            return;
        }
        if self.rvalue_is_nested_callable(scope, node) {
            if node.kind() == "method_definition" {
                if let Some(name) = node.child_by_field_name("name") {
                    if name.kind() == "computed_property_name" {
                        self.collect_identifiers_in_value_context_scoped(name, scope, out);
                    }
                }
            }
            return;
        }
        if self.language.is_identifier_node(node.kind()) {
            out.push((
                self.node_text(&node).to_string(),
                node.start_position().row + 1,
            ));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_identifiers_in_value_context_scoped(child, scope, out);
        }
    }
}
