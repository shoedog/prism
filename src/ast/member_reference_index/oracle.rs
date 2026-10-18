//! Unchanged recursive walkers, retained only as a differential oracle.
use super::*;
impl ParsedFile {
    /// Find all references to an AccessPath within a function scope.
    ///
    /// For simple paths (no fields), delegates to `find_variable_references_scoped`.
    /// For field-qualified paths (`dev->name`), searches for matching field_expression
    /// nodes as well as bare identifier references to the base.
    pub(super) fn legacy_path_references_scoped(
        &self,
        func_node: &Node<'_>,
        path: &AccessPath,
        def_line: usize,
    ) -> BTreeSet<usize> {
        if path.is_simple() {
            return self.find_variable_references_scoped(func_node, &path.base, def_line);
        }
        // For field-qualified paths, find matching field expressions.
        // NOTE: this filters to references AFTER def_line (`line > def_line` in `collect_path_refs`),
        // which does NOT match `find_variable_references_scoped` for simple paths — that returns ALL
        // non-shadowed references, including earlier lines (so a loop-carried `def@N → use@M (M<N)`
        // edge exists for simple paths but not field paths). `cpg/trace.rs::taint_neighbors` has a
        // field-path recovery arm that re-supplies these dropped edges for the reasoning layer; this
        // production DFG path is left as-is for byte-stability (Option C). See planA-followups Round 9.
        let mut lines = BTreeSet::new();
        self.collect_path_refs(*func_node, path, def_line, &mut lines);
        lines
    }

    fn collect_path_refs(
        &self,
        node: Node<'_>,
        path: &AccessPath,
        def_line: usize,
        out: &mut BTreeSet<usize>,
    ) {
        let line = node.start_position().row + 1;

        // Check field/member access expressions (all languages)
        if Self::is_field_access_node(node.kind()) {
            super::count_normalization();
            let node_path = self
                .bounded_member_path(node)
                .map(|member| member.path)
                .unwrap_or_else(|| AccessPath::from_expr(self.node_text(&node)));
            if node_path == *path && line > def_line {
                out.insert(line);
                return; // Don't recurse into matched field expression
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_path_refs(child, path, def_line, out);
        }
    }

    /// First admitted occurrence per line, after the binding and read-role
    /// gates. Synthetic edges retain this occurrence instead of substituting
    /// a different binding's first rvalue on the same line.
    pub(super) fn legacy_path_reference_spans_fenced_in(
        &self,
        func_node: &Node<'_>,
        path: &crate::access_path::AccessPath,
        def_line: usize,
        def_byte: usize,
        within: Option<(usize, usize)>,
        reads_only: bool,
    ) -> BTreeMap<usize, (usize, usize)> {
        if !matches!(
            self.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) || (!reads_only
            && self.language.function_name(func_node).is_some()
            && self.js_ts_seam_binding(func_node, &path.base))
        {
            return self
                .legacy_path_references_scoped(func_node, path, def_line)
                .into_iter()
                .map(|line| {
                    (
                        line,
                        (self.line_start_byte(line), self.line_start_byte(line)),
                    )
                })
                .collect();
        }
        let mut lines = BTreeMap::new();
        let scope_root = within
            .and_then(|(s, e)| self.tree.root_node().descendant_for_byte_range(s, e))
            .filter(|n| within == Some((n.start_byte(), n.end_byte())))
            .unwrap_or(*func_node);
        if path.is_simple() {
            self.collect_fenced_simple_refs(
                scope_root, func_node, &path.base, def_line, def_byte, reads_only, &mut lines,
            );
        } else {
            self.collect_fenced_path_refs(
                scope_root, func_node, path, def_line, def_byte, reads_only, &mut lines,
            );
        }
        lines
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_fenced_path_refs(
        &self,
        node: Node<'_>,
        owner: &Node<'_>,
        path: &crate::access_path::AccessPath,
        def_line: usize,
        def_byte: usize,
        reads_only: bool,
        out: &mut BTreeMap<usize, (usize, usize)>,
    ) {
        let line = node.start_position().row + 1;
        if Self::is_field_access_node(node.kind()) {
            #[cfg(test)]
            super::count_normalization();
            let node_path = self
                .bounded_member_path(node)
                .map(|member| member.path)
                .unwrap_or_else(|| {
                    crate::access_path::AccessPath::from_expr(self.node_text(&node))
                });
            if node_path == *path && line > def_line {
                let refused_read = reads_only
                    && ((path.base == "this" && self.js_ts_this_rebound_between(&node, owner))
                        || self.js_ts_identifier_is_not_a_read(node)
                        || self.js_ts_reference_in_with_body(node));
                if !refused_read && !self.js_ts_reference_fenced(&node, owner, &path.base, def_byte)
                {
                    out.entry(line)
                        .or_insert((node.start_byte(), node.end_byte()));
                }
                return;
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_fenced_path_refs(child, owner, path, def_line, def_byte, reads_only, out);
        }
    }
}
