#[cfg(test)]
use super::call_membership_tests;
use super::{AstNodeSpan, FunctionAstSpans, FunctionInfo, Language, ParsedFile, Result};
use tree_sitter::Node;

impl ParsedFile {
    /// Find the smallest function/method node containing the given line (1-indexed).
    pub fn enclosing_function(&self, line: usize) -> Option<Node<'_>> {
        let row = line.saturating_sub(1); // tree-sitter uses 0-indexed rows
        self.find_enclosing_node(
            self.tree.root_node(),
            row,
            &self.language.function_node_types(),
        )
    }

    pub(super) fn find_enclosing_node<'a>(
        &self,
        node: Node<'a>,
        row: usize,
        types: &[&str],
    ) -> Option<Node<'a>> {
        let start = node.start_position().row;
        let end = node.end_position().row;

        if row < start || row > end {
            return None;
        }

        // Check children first (prefer smallest/deepest match)
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = self.find_enclosing_node(child, row, types) {
                return Some(found);
            }
        }

        if types.contains(&node.kind()) {
            Some(node)
        } else {
            None
        }
    }

    /// Function nodes, reconstructed from the eager table. On any reconstruction
    /// miss, falls back to the direct dual-path collection for the WHOLE file —
    /// never a partial sequence. The bool is the fallback-fire flag: in-module
    /// tests assert it is false for all 12 supported languages so grammar drift
    /// cannot silently route a language to the slow path.
    pub fn all_functions(&self) -> Vec<Node<'_>> {
        self.all_functions_inner().0
    }

    pub(super) fn unwrap_decorated<'a>(&self, node: Node<'a>) -> Node<'a> {
        if self.language == Language::Python && node.kind() == "decorated_definition" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "function_definition" {
                    return child;
                }
            }
        }
        node
    }

    pub(crate) fn all_functions_inner(&self) -> (Vec<Node<'_>>, bool) {
        #[cfg(test)]
        call_membership_tests::record_inventory();
        let mut out = Vec::with_capacity(self.functions.len());
        for info in &self.functions {
            match self.reconstruct_function_node(info) {
                Some(node) => out.push(node),
                None => return (self.all_functions_via_tree(), true),
            }
        }
        (out, false)
    }

    pub(super) fn reconstruct_function_node(&self, info: &FunctionInfo) -> Option<Node<'_>> {
        let mut node = self
            .tree
            .root_node()
            .descendant_for_byte_range(info.start_byte, info.end_byte)?;
        // descendant_for_byte_range returns the DEEPEST node spanning the range,
        // so recovery walks UP through same-span ancestors — a walk-down can
        // never reach a same-span ancestor.
        loop {
            if node.start_byte() == info.start_byte
                && node.end_byte() == info.end_byte
                && node.kind_id() == info.kind_id
            {
                return Some(node);
            }
            match node.parent() {
                Some(p) if p.start_byte() == info.start_byte && p.end_byte() == info.end_byte => {
                    node = p
                }
                _ => return None,
            }
        }
    }

    /// Reconstruct one callable by its exact eager-table byte identity and project
    /// edit coordinates without returning or copying source text.
    pub(crate) fn function_ast_spans(
        &self,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<FunctionAstSpans> {
        const MAX_INDENTATION_BYTES: usize = 256;

        fn span(node: Node<'_>) -> AstNodeSpan {
            AstNodeSpan {
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
            }
        }

        fn indentation_before(
            source: &str,
            byte: usize,
            non_whitespace_reason: &'static str,
        ) -> Result<String, &'static str> {
            let line_start = source.as_bytes()[..byte]
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(0, |newline| newline + 1);
            let prefix = &source[line_start..byte];
            if prefix.len() > MAX_INDENTATION_BYTES {
                return Err("line indentation exceeds 256-byte limit");
            }
            if prefix.bytes().all(|b| matches!(b, b' ' | b'\t')) {
                Ok(prefix.to_string())
            } else {
                Err(non_whitespace_reason)
            }
        }

        let info = self
            .functions
            .iter()
            .find(|info| info.start_byte == start_byte && info.end_byte == end_byte)?;
        let outer = self.reconstruct_function_node(info)?;
        let inner = self.unwrap_decorated(outer);
        let name = self.language.function_name(&inner).map(span);
        let body_node = inner.child_by_field_name("body");
        let body = body_node.map(span);
        let symbol_indentation = indentation_before(
            &self.source,
            outer.start_byte(),
            "outer callable is not preceded only by line indentation",
        );
        let body_indentation = match body_node {
            None => Err("body span unavailable"),
            Some(body) => match body.named_child(0) {
                None => Err("body has no named child"),
                Some(child) => indentation_before(
                    &self.source,
                    child.start_byte(),
                    "first named body child is not preceded only by line indentation",
                ),
            },
        };

        Some(FunctionAstSpans {
            name,
            body,
            symbol_indentation,
            body_indentation,
        })
    }

    pub(super) fn all_functions_via_tree(&self) -> Vec<Node<'_>> {
        use crate::queries::{get_query, QueryKind};
        use tree_sitter::StreamingIterator;

        let mut functions = Vec::new();

        // Use compiled tree-sitter query when available (faster: skips irrelevant subtrees).
        if let Some(query) = get_query(self.language, QueryKind::Functions) {
            let func_idx = query
                .capture_index_for_name("func")
                .expect("Functions query must have @func capture");
            let mut cursor = tree_sitter::QueryCursor::new();
            let mut matches = cursor.matches(query, self.tree.root_node(), self.source.as_bytes());
            while let Some(m) = matches.next() {
                for capture in m.captures {
                    if capture.index == func_idx {
                        functions.push(capture.node);
                    }
                }
            }
        } else {
            // Fallback: manual recursive walk.
            self.collect_functions_manual(self.tree.root_node(), &mut functions);
        }

        functions.retain(|node| !self.is_python_decorated_inner_function(node));
        functions
    }

    pub(super) fn is_python_decorated_inner_function(&self, node: &Node<'_>) -> bool {
        self.language == Language::Python
            && node.kind() == "function_definition"
            && node
                .parent()
                .is_some_and(|parent| parent.kind() == "decorated_definition")
    }

    /// Build the eager function table via the existing dual-path collection.
    pub(super) fn build_function_table(&self) -> Vec<FunctionInfo> {
        self.all_functions_via_tree()
            .into_iter()
            .map(|node| FunctionInfo {
                name: self
                    .language
                    .function_name(&node)
                    .map(|n| self.node_text(&n).to_string()),
                kind_id: node.kind_id(),
                start_byte: node.start_byte(),
                end_byte: node.end_byte(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                param_names: self.function_parameter_slots(&node),
                owner: self
                    .language
                    .method_owner(&node)
                    .map(|n| crate::resolution::owner_key(self.node_text(&n))),
                receiver_var: self
                    .language
                    .go_receiver_var(&node)
                    .map(|n| self.node_text(&n).to_string()),
            })
            .collect()
    }

    pub fn functions(&self) -> &[FunctionInfo] {
        &self.functions
    }

    /// Find the smallest function node spanning the given 1-indexed line.
    pub fn function_node_spanning(&self, line: usize) -> Option<Node<'_>> {
        self.all_functions()
            .into_iter()
            .filter(|node| {
                let (start, end) = self.node_line_range(node);
                start <= line && line <= end
            })
            .min_by_key(|node| node.end_byte().saturating_sub(node.start_byte()))
    }
}
