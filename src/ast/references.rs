//! Variable references, shadowing, identifiers on a line and code-text checks.

#[cfg(test)]
use crate::ast::{binding_verdict_memo, identifier_row_tests};
use crate::ast::{shadow_walk, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Find all identifiers (variable references) on a given line (1-indexed).
    ///
    /// Uses the manual recursive walk because it needs to match the broad set
    /// of node types in `Language::is_identifier_node()` (property_identifier,
    /// field_identifier, etc.), not just the core `identifier` type.
    pub fn identifiers_on_line(&self, line: usize) -> Vec<Node<'_>> {
        let row = line.saturating_sub(1);
        let mut result = Vec::new();
        self.collect_identifiers_at_row(self.tree.root_node(), row, &mut result);
        result
    }

    /// Manual recursive identifier-at-row collection (pre-query fallback).
    fn collect_identifiers_at_row<'a>(&self, node: Node<'a>, row: usize, out: &mut Vec<Node<'a>>) {
        #[cfg(test)]
        identifier_row_tests::record_visit();
        if node.start_position().row == row && self.language.is_identifier_node(node.kind()) {
            out.push(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.start_position().row > row {
                break;
            }
            // Inclusive end row keeps zero-width nodes and multiline containers.
            if child.end_position().row >= row {
                self.collect_identifiers_at_row(child, row, out);
            }
        }
    }

    /// Check whether `text` appears on `line` (1-indexed) in actual code,
    /// i.e. NOT inside a comment or string literal AST node.
    ///
    /// For each occurrence of `text` on the line, we ask tree-sitter for the
    /// smallest node covering that byte range. If every occurrence lands in a
    /// comment or string, returns `false`.
    pub fn line_has_code_text(&self, line: usize, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        let row = line.saturating_sub(1);
        let source = self.source.as_bytes();

        // O(1) line→byte offset lookup via precomputed table.
        let line_start = self.line_offsets.get(row).copied().unwrap_or(source.len());
        let line_end = self
            .line_offsets
            .get(row + 1)
            .map(|&off| off.saturating_sub(1)) // exclude the newline
            .unwrap_or(source.len());
        let line_bytes = &source[line_start..line_end];
        let line_str = std::str::from_utf8(line_bytes).unwrap_or("");

        // For each occurrence of `text` in this line, check the AST node.
        let text_bytes = text.as_bytes();
        let mut search_start = 0;
        while let Some(pos) = line_str[search_start..].find(text) {
            let abs_start = line_start + search_start + pos;
            let abs_end = abs_start + text_bytes.len();

            // Find the smallest AST node covering this byte range.
            let node = self
                .tree
                .root_node()
                .descendant_for_byte_range(abs_start, abs_end);
            if let Some(n) = node {
                // Walk up to check if any ancestor (up to the line boundary) is
                // a comment or string. The immediate node might be an identifier
                // inside a string interpolation, so we check ancestors too.
                if !self.is_inside_comment_or_string(n) {
                    return true; // At least one occurrence is in real code.
                }
            } else {
                // No AST node found — conservative: treat as code.
                return true;
            }
            search_start += pos + text_bytes.len();
        }
        false
    }

    /// Walk up from `node` to check if it (or any ancestor) is a comment or
    /// string literal node.
    fn is_inside_comment_or_string(&self, node: Node<'_>) -> bool {
        let mut current = Some(node);
        while let Some(n) = current {
            if self.language.is_comment_or_string_node(n.kind()) {
                return true;
            }
            current = n.parent();
        }
        false
    }

    /// Find all lines in a function scope where a variable name is referenced.
    pub fn find_variable_references(
        &self,
        func_node: &Node<'_>,
        var_name: &str,
    ) -> BTreeSet<usize> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        if let Some(query) = get_query(self.language, QueryKind::Identifiers) {
            let ident_idx = query
                .capture_index_for_name("ident")
                .expect("Identifiers query must have @ident capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            cursor.set_byte_range(func_node.byte_range());
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            let mut lines = BTreeSet::new();
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == ident_idx
                        && self.node_text(&capture.node) == var_name
                        && !self.is_go_legacy_type_context(capture.node)
                    {
                        lines.insert(capture.node.start_position().row + 1);
                    }
                }
            }
            return lines;
        }

        let mut lines = BTreeSet::new();
        self.collect_variable_refs_manual(*func_node, var_name, &mut lines);
        lines
    }

    /// Manual recursive variable ref collection (pre-query fallback).
    fn collect_variable_refs_manual(
        &self,
        node: Node<'_>,
        var_name: &str,
        out: &mut BTreeSet<usize>,
    ) {
        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs_manual(child, var_name, out);
        }
    }

    /// Find variable references with basic scope awareness.
    ///
    /// Like `find_variable_references`, but filters out references that lie inside
    /// an inner scope block which re-declares the same variable name — i.e., the
    /// reference would be bound to the inner declaration, not the one at `def_line`.
    pub fn find_variable_references_scoped(
        &self,
        func_node: &Node<'_>,
        var_name: &str,
        def_line: usize,
    ) -> BTreeSet<usize> {
        crate::profile::gates::scoped_refs::<false>(self, func_node, var_name, def_line)
    }

    /// Check whether a reference node is shadowed by a re-declaration in an inner scope.
    /// Tests ancestors bottom-up, collecting them with a single top-down descent.
    pub(crate) fn is_shadowed_at(
        &self,
        node: &Node<'_>,
        func_node: &Node<'_>,
        var_name: &str,
        def_line: usize,
    ) -> bool {
        shadow_walk::is_shadowed_at(self, *node, *func_node, var_name, def_line)
    }

    /// Manual recursive scoped variable ref collection (pre-query fallback).
    pub(crate) fn collect_variable_refs_scoped_manual(
        &self,
        node: Node<'_>,
        var_name: &str,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let node_start = node.start_position().row + 1;
        let node_end = node.end_position().row + 1;

        if self.language.is_scope_block(node.kind()) {
            let def_in_scope = def_line >= node_start && def_line <= node_end;
            if !def_in_scope && self.scope_has_declaration(node, var_name) {
                return;
            }
        }

        if self.language.is_identifier_node(node.kind()) && self.node_text(&node) == var_name {
            out.insert(node.start_position().row + 1);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_variable_refs_scoped_manual(child, var_name, def_line, out);
        }
    }

    /// Check whether a scope block directly declares a variable with the given name.
    /// Does not recurse into nested scope blocks (those have their own scope).
    pub(super) fn scope_has_declaration_uncached(&self, node: Node<'_>, var_name: &str) -> bool {
        #[cfg(test)]
        binding_verdict_memo::probe::observe("declaration", node.id(), 0, var_name);
        let mut cursor = node.walk();
        let found = node.children(&mut cursor).any(|child| {
            (self.language.is_declaration_node(child.kind())
                && self
                    .language
                    .declaration_name(&child)
                    .is_some_and(|name| self.node_text(&name) == var_name))
                || (!self.language.is_scope_block(child.kind())
                    && self.scope_has_declaration_uncached(child, var_name))
        });
        found
    }

    pub(super) fn function_body_node<'a>(&self, func_node: &Node<'a>) -> Option<Node<'a>> {
        let func_node = self.unwrap_decorated(*func_node);
        func_node
            .child_by_field_name("body")
            .or_else(|| func_node.child_by_field_name("consequence"))
    }
}
