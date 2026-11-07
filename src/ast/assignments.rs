//! Assignment lvalue queries and their gap/manual collectors.

use crate::access_path::AccessPath;
use crate::ast::lvalues::extract_lvalue_paths;
use crate::ast::{is_plain_ident, AssignmentCallInfo, ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Find all assignment targets (L-values) on diff lines within a function scope.
    pub fn assignment_lvalues_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(String, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lvalues = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalues(&capture.node, line, &mut lvalues);
                        }
                    }
                }
            }
            return lvalues;
        }

        let mut lvalues = Vec::new();
        self.collect_assignments_manual(*func_node, lines, &mut lvalues);
        lvalues
    }

    /// Extract L-value names from a matched assignment/declaration node.
    fn extract_assignment_lvalues(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(String, usize)>,
    ) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }
    }

    /// Manual recursive assignment collection (pre-query fallback).
    pub(crate) fn collect_assignments_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(String, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                let lhs_text = self.node_text(&lhs).to_string();
                for name in extract_lvalue_names(&lhs_text) {
                    out.push((name, line));
                }
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                let name = self.node_text(&name_node).to_string();
                out.push((name, line));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignments_manual(child, lines, out);
        }
    }

    /// Like `assignment_lvalues_on_lines`, but returns structured `AccessPath`s
    /// instead of plain variable name strings. Used by the DFG for field-sensitive tracking.
    pub fn assignment_lvalue_paths_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<(AccessPath, usize)> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut paths = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalue_paths(&capture.node, line, &mut paths);
                        }
                    }
                }
            }
            // The query may miss for_in_statement and as_pattern since they aren't
            // assignment/declaration nodes. Run the gap handlers on the full tree.
            self.collect_assignment_paths_gaps(*func_node, lines, &mut paths);
            return paths;
        }

        let mut paths = Vec::new();
        self.collect_assignment_paths_manual(*func_node, lines, &mut paths);
        paths
    }

    /// Byte-bearing sibling of `assignment_lvalue_paths_on_lines`.
    pub fn assignment_lvalue_spans_on_lines(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<PathSpan> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Assignments) {
            let assign_idx = query
                .capture_index_for_name("assign")
                .expect("Assignments query must have @assign capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut spans = Vec::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == assign_idx {
                        let line = capture.node.start_position().row + 1;
                        if lines.contains(&line) {
                            self.extract_assignment_lvalue_spans(&capture.node, &mut spans);
                        }
                    }
                }
            }
            self.collect_assignment_span_gaps(*func_node, lines, &mut spans);
            self.collect_go_header_def_spans(*func_node, lines, &mut spans);
            spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
            return spans;
        }

        let mut spans = Vec::new();
        self.collect_assignment_spans_manual(*func_node, lines, &mut spans);
        spans.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
        spans
    }

    /// Recover the exact AST parent for a call that is itself an assignment or
    /// declaration RHS. Nested calls are rejected: only the RHS slot whose
    /// byte range exactly equals the supplied call range is admitted.
    pub fn assignment_for_exact_call(
        &self,
        call_start_byte: usize,
        call_end_byte: usize,
    ) -> Option<AssignmentCallInfo> {
        let leaf = self
            .tree
            .root_node()
            .descendant_for_byte_range(call_start_byte, call_end_byte)?;
        let mut current = Some(leaf);
        while let Some(node) = current {
            if self.language.is_assignment_node(node.kind())
                || self.language.is_declaration_node(node.kind())
            {
                let rhs = self
                    .language
                    .assignment_value(&node)
                    .or_else(|| self.language.declaration_value(&node))?;
                let rhs_slots: Vec<Node<'_>> =
                    if matches!(rhs.kind(), "expression_list" | "variable_list") {
                        let mut cursor = rhs.walk();
                        rhs.named_children(&mut cursor).collect()
                    } else {
                        vec![rhs]
                    };
                let rhs_slot = rhs_slots.iter().position(|slot| {
                    slot.start_byte() == call_start_byte && slot.end_byte() == call_end_byte
                })?;

                let lhs = self
                    .language
                    .assignment_target(&node)
                    .or_else(|| self.language.declaration_name(&node))?;
                let mut lvalues = Vec::new();
                self.extract_lvalue_spans_from_node(lhs, &mut lvalues);
                lvalues.sort_by_key(|span| (span.start_byte, span.end_byte));
                if rhs_slots.len() > 1 {
                    if rhs_slots.len() != lvalues.len() {
                        return None;
                    }
                    lvalues = vec![lvalues[rhs_slot].clone()];
                }
                return Some(AssignmentCallInfo {
                    start_byte: node.start_byte(),
                    end_byte: node.end_byte(),
                    rhs_start_byte: rhs.start_byte(),
                    rhs_end_byte: rhs.end_byte(),
                    lvalues,
                });
            }
            current = node.parent();
        }
        None
    }

    /// True when a byte-bearing DFG Use is a semantic value/receiver use, not
    /// a call's callee label or a Python keyword-argument label.
    pub fn is_semantic_value_use(&self, start_byte: usize, end_byte: usize) -> bool {
        let Some(leaf) = self
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
        else {
            return false;
        };
        let mut current = Some(leaf);
        while let Some(node) = current {
            if self.language.is_erased_type_boundary(node) {
                return false;
            }
            if node.kind() == "keyword_argument" {
                if node.child_by_field_name("name").is_some_and(|name| {
                    name.start_byte() <= start_byte && end_byte <= name.end_byte()
                }) {
                    return false;
                }
            }
            if self.language.is_call_node(node.kind()) {
                if self.call_function_name(&node).is_some_and(|name| {
                    name.start_byte() <= start_byte && end_byte <= name.end_byte()
                }) {
                    return false;
                }
            }
            current = node.parent();
        }
        true
    }

    /// Extract L-value AccessPaths from a matched assignment/declaration node.
    fn extract_assignment_lvalue_paths(
        &self,
        node: &Node<'_>,
        line: usize,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                if lhs.kind() == "pattern_list" || lhs.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&lhs, line, out);
                } else if let Some(member) = self.bounded_member_path(lhs) {
                    out.push((member.path, line));
                } else {
                    let lhs_text = self.node_text(&lhs).to_string();
                    for path in extract_lvalue_paths(&lhs_text) {
                        out.push((path, line));
                    }
                }
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(node) {
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }
    }

    fn extract_assignment_lvalue_spans(&self, node: &Node<'_>, out: &mut Vec<PathSpan>) {
        if self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(node) {
                self.extract_lvalue_spans_from_node(lhs, out);
            }
        }
        if self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = node
                .child_by_field_name("pattern")
                .or_else(|| node.child_by_field_name("name"))
                .or_else(|| node.child_by_field_name("declarator"))
                .or_else(|| self.language.declaration_name(node))
            {
                self.extract_lvalue_spans_from_node(name_node, out);
            }
        }
    }

    /// Handle gap patterns not covered by the Assignments query (for_in, as_pattern).
    fn collect_assignment_paths_gaps(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_paths_gaps(child, lines, out);
        }
    }

    /// Byte-bearing gap patterns not covered by the Assignments query.
    fn collect_assignment_span_gaps(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalue_spans(&node, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    self.push_simple_lvalue_span(child, out);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_span_gaps(child, lines, out);
        }
    }

    /// Manual recursive assignment path collection (pre-query fallback).
    pub(super) fn collect_assignment_paths_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<(AccessPath, usize)>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                if lhs.kind() == "pattern_list" || lhs.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&lhs, line, out);
                } else if let Some(member) = self.bounded_member_path(lhs) {
                    out.push((member.path, line));
                } else {
                    let lhs_text = self.node_text(&lhs).to_string();
                    for path in extract_lvalue_paths(&lhs_text) {
                        out.push((path, line));
                    }
                }
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = self.language.declaration_name(&node) {
                if name_node.kind() == "expression_list" {
                    self.extract_multi_target_lvalues(&name_node, line, out);
                } else {
                    let name = self.node_text(&name_node).to_string();
                    out.push((AccessPath::simple(name), line));
                }
            }
        }

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalues(&node, line, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    let name = self.node_text(&child).to_string();
                    if is_plain_ident(&name) {
                        out.push((AccessPath::simple(name), line));
                    }
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_paths_manual(child, lines, out);
        }
    }

    /// Manual recursive assignment span collection (pre-query fallback).
    pub(super) fn collect_assignment_spans_manual(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<PathSpan>,
    ) {
        let line = node.start_position().row + 1;

        if lines.contains(&line) && self.language.is_assignment_node(node.kind()) {
            if let Some(lhs) = self.language.assignment_target(&node) {
                self.extract_lvalue_spans_from_node(lhs, out);
            }
        }

        if lines.contains(&line) && self.language.is_declaration_node(node.kind()) {
            if let Some(name_node) = node
                .child_by_field_name("pattern")
                .or_else(|| node.child_by_field_name("name"))
                .or_else(|| node.child_by_field_name("declarator"))
                .or_else(|| self.language.declaration_name(&node))
            {
                self.extract_lvalue_spans_from_node(name_node, out);
            }
        }

        if lines.contains(&line)
            && node.kind() == "for_in_statement"
            && matches!(
                self.language,
                Language::JavaScript | Language::TypeScript | Language::Tsx
            )
        {
            self.extract_for_in_lvalue_spans(&node, out);
        }

        if lines.contains(&line)
            && node.kind() == "as_pattern"
            && matches!(self.language, Language::Python)
        {
            let mut c = node.walk();
            for child in node.children(&mut c) {
                if child.kind() == "as_pattern_target" {
                    self.push_simple_lvalue_span(child, out);
                }
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_assignment_spans_manual(child, lines, out);
        }
    }
}

fn extract_lvalue_names(lhs_text: &str) -> Vec<String> {
    let lhs = lhs_text.trim();

    // Pointer dereference: *p, **p
    if lhs.starts_with('*') {
        let inner = lhs.trim_start_matches('*').trim();
        // Strip surrounding parens: (*p)
        let inner = inner.trim_start_matches('(').trim_end_matches(')').trim();
        if !inner.is_empty() && is_plain_ident(inner) {
            return vec![inner.to_string()];
        }
        return vec![];
    }

    // Field via arrow: dev->field
    if let Some(arrow) = lhs.find("->") {
        let base = lhs[..arrow].trim();
        let field = lhs[arrow + 2..].trim();
        let mut names = Vec::new();
        if !field.is_empty() && is_plain_ident(field) {
            names.push(field.to_string());
        }
        if !base.is_empty() && is_plain_ident(base) {
            names.push(base.to_string());
        }
        return names;
    }

    // Array subscript: buf[i]  — only track the base name
    if let Some(bracket) = lhs.find('[') {
        let base = lhs[..bracket].trim();
        if !base.is_empty() && is_plain_ident(base) {
            return vec![base.to_string()];
        }
        return vec![];
    }

    // Simple identifier (also covers `obj.field` by treating `obj` as the def).
    // We intentionally ignore dot access for non-pointer structs here; the base
    // identifier appears as a separate rvalue and will be tracked via rvalue edges.
    if !lhs.is_empty() && is_plain_ident(lhs) {
        return vec![lhs.to_string()];
    }

    vec![]
}
