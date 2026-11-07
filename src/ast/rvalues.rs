//! Rvalue query scope and owner rules, and rvalue path/span queries.

use crate::access_path::AccessPath;
use crate::ast::{byte_range_eq, ParsedFile, PathSpan, RvalueOwnerDecision, RvalueQueryScope};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Query byte ranges also admit overlapping ancestor nodes. For JS/TS/TSX,
    /// an rvalue-producing capture must be wholly inside the requested callable
    /// (or file root), not merely overlap its lines.
    fn rvalue_capture_is_contained(&self, function: &Node<'_>, capture: Node<'_>) -> bool {
        !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || (function.start_byte() <= capture.start_byte()
            && capture.end_byte() <= function.end_byte())
    }

    pub(super) fn rvalue_query_scope(&self, function: &Node<'_>) -> RvalueQueryScope {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || function.parent().is_none()
        {
            return RvalueQueryScope::LegacyInventory;
        }
        if function.has_error()
            || !self
                .language
                .callable_boundary_node_types()
                .contains(&function.kind())
            || !self.all_functions().iter().any(|candidate| {
                candidate.kind() == function.kind() && byte_range_eq(candidate, function)
            })
        {
            return RvalueQueryScope::Unresolved;
        }
        let Some(body) = function.child_by_field_name("body") else {
            return RvalueQueryScope::Unresolved;
        };
        if body.start_byte() < function.start_byte() || function.end_byte() < body.end_byte() {
            return RvalueQueryScope::Unresolved;
        }
        RvalueQueryScope::CallableBody {
            owner_start_byte: function.start_byte(),
            owner_end_byte: function.end_byte(),
            body_start_byte: body.start_byte(),
            body_end_byte: body.end_byte(),
        }
    }

    pub(super) fn is_rvalue_class_node(kind: &str) -> bool {
        matches!(
            kind,
            "class" | "class_declaration" | "abstract_class_declaration"
        )
    }

    pub(super) fn rvalue_has_class_ancestor_before_owner(
        &self,
        mut node: Node<'_>,
        owner_start_byte: usize,
        owner_end_byte: usize,
    ) -> bool {
        while let Some(parent) = node.parent() {
            if parent.start_byte() == owner_start_byte && parent.end_byte() == owner_end_byte {
                return false;
            }
            if Self::is_rvalue_class_node(parent.kind()) {
                return true;
            }
            node = parent;
        }
        false
    }

    fn rvalue_is_computed_method_key_capture(
        &self,
        mut node: Node<'_>,
        owner_start_byte: usize,
        owner_end_byte: usize,
    ) -> bool {
        loop {
            if node.start_byte() == owner_start_byte && node.end_byte() == owner_end_byte {
                return false;
            }
            if node.kind() == "computed_property_name" {
                if let Some(method) = node.parent() {
                    if method.kind() == "method_definition"
                        && method
                            .child_by_field_name("name")
                            .is_some_and(|name| byte_range_eq(&name, &node))
                    {
                        return true;
                    }
                }
            }
            let Some(parent) = node.parent() else {
                return false;
            };
            node = parent;
        }
    }

    pub(super) fn rvalue_capture_owner(
        &self,
        function: &Node<'_>,
        scope: &RvalueQueryScope,
        capture: Node<'_>,
    ) -> RvalueOwnerDecision {
        match *scope {
            RvalueQueryScope::LegacyInventory => RvalueOwnerDecision::Owned,
            RvalueQueryScope::Unresolved => RvalueOwnerDecision::Unsupported,
            RvalueQueryScope::CallableBody {
                owner_start_byte,
                owner_end_byte,
                body_start_byte,
                body_end_byte,
            } => {
                if !self.rvalue_capture_is_contained(function, capture) {
                    return RvalueOwnerDecision::Outside;
                }
                if capture.start_byte() < body_start_byte || body_end_byte < capture.end_byte() {
                    return RvalueOwnerDecision::Signature;
                }
                if self.rvalue_is_computed_method_key_capture(
                    capture,
                    owner_start_byte,
                    owner_end_byte,
                ) {
                    return RvalueOwnerDecision::Owned;
                }
                // Class evaluation phases are intentionally outside this first
                // repair. Preserve their legacy inventory until separately proved.
                if self.rvalue_has_class_ancestor_before_owner(
                    capture,
                    owner_start_byte,
                    owner_end_byte,
                ) {
                    return RvalueOwnerDecision::Owned;
                }
                let mut current = Some(capture);
                while let Some(node) = current {
                    if node.start_byte() == owner_start_byte && node.end_byte() == owner_end_byte {
                        return RvalueOwnerDecision::Owned;
                    }
                    if self
                        .language
                        .callable_boundary_node_types()
                        .contains(&node.kind())
                    {
                        return RvalueOwnerDecision::Nested;
                    }
                    current = node.parent();
                }
                RvalueOwnerDecision::Outside
            }
        }
    }

    pub(super) fn rvalue_scope_allows_node(
        &self,
        scope: &RvalueQueryScope,
        node: Node<'_>,
        query_root: bool,
    ) -> bool {
        match *scope {
            RvalueQueryScope::LegacyInventory => true,
            RvalueQueryScope::Unresolved => false,
            RvalueQueryScope::CallableBody {
                owner_start_byte,
                owner_end_byte,
                body_start_byte,
                body_end_byte,
            } => {
                (query_root
                    && node.start_byte() == owner_start_byte
                    && node.end_byte() == owner_end_byte)
                    || (body_start_byte <= node.start_byte() && node.end_byte() <= body_end_byte)
            }
        }
    }

    pub(super) fn rvalue_is_nested_callable(
        &self,
        scope: &RvalueQueryScope,
        node: Node<'_>,
    ) -> bool {
        let RvalueQueryScope::CallableBody {
            owner_start_byte,
            owner_end_byte,
            ..
        } = *scope
        else {
            return false;
        };
        (node.start_byte() != owner_start_byte || node.end_byte() != owner_end_byte)
            && self
                .language
                .callable_boundary_node_types()
                .contains(&node.kind())
    }

    /// Like `rvalue_identifiers_on_lines`, but returns structured `AccessPath`s.
    /// Used by the DFG for field-sensitive tracking.
    pub fn rvalue_identifier_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let scope = self.rvalue_query_scope(func_node);

        // Use the Assignments query to find assignment nodes, then extract RHS.
        // Also use the Calls query for call arguments on diff lines.
        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut paths = Vec::new();

            // Collect R-values from assignments
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
                                self.collect_identifier_paths_scoped(rhs, &scope, &mut paths);
                            }
                        }
                    }
                }
            }

            // Collect R-values from call arguments using Calls query
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
                                    self.collect_identifier_paths_scoped(args, &scope, &mut paths);
                                }
                                if let Some(func_name_node) = self.call_value_name(&capture.node) {
                                    let name = self.node_text(&func_name_node).to_string();
                                    paths.push((AccessPath::simple(name), line));
                                }
                            }
                        }
                    }
                }
            }

            return paths;
        }

        let mut paths = Vec::new();
        self.collect_rvalue_paths_manual(*func_node, lines, &mut paths);
        paths
    }

    /// Byte-bearing sibling of `rvalue_identifier_paths_on_lines`.
    pub fn rvalue_identifier_spans_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<PathSpan> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let scope = self.rvalue_query_scope(func_node);

        if let Some(assign_query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = assign_query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut spans = Vec::new();

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
                            if self.is_augmented_assignment(&capture.node) {
                                if let Some(lhs) = self.language.assignment_target(&capture.node) {
                                    self.collect_identifier_path_spans_scoped(
                                        lhs, &scope, &mut spans,
                                    );
                                }
                            }
                            if let Some(rhs) = self.language.assignment_value(&capture.node) {
                                self.collect_identifier_path_spans_scoped(rhs, &scope, &mut spans);
                            }
                        }
                    }
                }
            }

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
                                    self.collect_identifier_path_spans_scoped(
                                        args, &scope, &mut spans,
                                    );
                                }
                                if let Some(func_name_node) = self.call_value_name(&capture.node) {
                                    self.push_identifier_path_span(func_name_node, &mut spans);
                                }
                            }
                        }
                    }
                }
            }

            self.collect_return_value_identifier_spans(func_node, lines, &scope, &mut spans);
            spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
            return spans;
        }

        let mut spans = Vec::new();
        self.collect_rvalue_spans_manual(*func_node, lines, &mut spans);
        spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
        spans
    }

    /// Manual recursive R-value path collection (pre-query fallback).
    pub(super) fn collect_rvalue_paths_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let scope = self.rvalue_query_scope(&node);
        self.collect_rvalue_paths_manual_scoped(node, lines, &scope, true, out);
    }

    fn collect_rvalue_paths_manual_scoped(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        scope: &RvalueQueryScope,
        query_root: bool,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        if !self.rvalue_scope_allows_node(scope, node, query_root) {
            return;
        }
        if !query_root && matches!(scope, RvalueQueryScope::CallableBody { .. }) {
            if Self::is_rvalue_class_node(node.kind()) {
                self.collect_rvalue_paths_manual_scoped(
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
                            self.collect_identifier_paths_scoped(name, scope, out);
                        }
                    }
                }
                return;
            }
        }
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_identifier_paths_scoped(rhs, scope, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_identifier_paths_scoped(args, scope, out);
            }
            if let Some(func_name_node) = self.call_value_name(&node) {
                let name = self.node_text(&func_name_node).to_string();
                out.push((AccessPath::simple(name), line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalue_paths_manual_scoped(child, lines, scope, false, out);
        }
    }

    /// Manual recursive R-value span collection (pre-query fallback).
    pub(super) fn collect_rvalue_spans_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let scope = self.rvalue_query_scope(&node);
        self.collect_rvalue_spans_manual_scoped(node, lines, &scope, true, out);
    }

    fn collect_rvalue_spans_manual_scoped(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        scope: &RvalueQueryScope,
        query_root: bool,
        out: &mut Vec<PathSpan>,
    ) {
        if !self.rvalue_scope_allows_node(scope, node, query_root) {
            return;
        }
        if !query_root && matches!(scope, RvalueQueryScope::CallableBody { .. }) {
            if Self::is_rvalue_class_node(node.kind()) {
                self.collect_rvalue_spans_manual_scoped(
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
                            self.collect_identifier_path_spans_scoped(name, scope, out);
                        }
                    }
                }
                return;
            }
        }
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if self.is_augmented_assignment(&node) {
                if let Some(lhs) = self.language.assignment_target(&node) {
                    self.collect_identifier_path_spans_scoped(lhs, scope, out);
                }
            }
            if let Some(rhs) = self.language.assignment_value(&node) {
                self.collect_identifier_path_spans_scoped(rhs, scope, out);
            }
        }

        if lines.contains(&line) && self.language.is_call_node(node.kind()) {
            if let Some(args) = self.language.call_arguments(&node) {
                self.collect_identifier_path_spans_scoped(args, scope, out);
            }
            if let Some(func_name_node) = self.call_value_name(&node) {
                self.push_identifier_path_span(func_name_node, out);
            }
        }

        if lines.contains(&line) && self.language.is_return_node(node.kind()) {
            self.collect_return_node_value_spans_scoped(node, scope, out);
            return;
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_rvalue_spans_manual_scoped(child, lines, scope, false, out);
        }
    }
}
